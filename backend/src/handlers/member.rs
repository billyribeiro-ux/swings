use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use axum::{
    extract::{Path, Query, State},
    http::HeaderMap,
    routing::{delete, get, post, put},
    Json, Router,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{
    commerce::{orders as orders_repo, subscriptions as subs_repo},
    db,
    error::{AppError, AppResult},
    extractors::{AuthUser, ClientInfo},
    handlers::auth::{auth_cookie_headers, clear_auth_cookie_headers, generate_tokens},
    models::*,
    services::audit::audit_admin_under_impersonation,
    stripe_api, AppState,
};

pub fn router() -> Router<AppState> {
    // FDN-08: 120/min/user governs the whole authenticated member surface.
    // Keying prefers the Bearer `sub` when the Postgres backend is active;
    // the in-process governor falls back to IP.
    Router::new()
        // Profile
        .route("/profile", get(get_profile))
        .route("/profile", put(update_profile))
        // Subscription (singleton — current/active)
        .route("/subscription", get(get_subscription))
        .route("/billing-portal", post(post_billing_portal))
        .route("/subscription/cancel", post(post_subscription_cancel))
        .route("/subscription/resume", post(post_subscription_resume))
        // Subscriptions history + per-subscription actions (Phase 5).
        .route("/subscriptions", get(list_subscriptions))
        .route("/subscriptions/{id}", get(get_subscription_detail))
        .route("/subscriptions/{id}/cancel", post(post_cancel_subscription))
        .route("/subscriptions/{id}/resume", post(post_resume_subscription))
        .route("/subscriptions/{id}/pause", post(post_pause_subscription))
        .route(
            "/subscriptions/{id}/unpause",
            post(post_unpause_subscription),
        )
        .route(
            "/subscriptions/{id}/switch-plan",
            post(post_switch_subscription_plan),
        )
        .route(
            "/subscriptions/{id}/switch-plan/preview",
            get(get_switch_plan_preview),
        )
        // Orders history + detail
        .route("/orders", get(list_orders))
        .route("/orders/{id}", get(get_order_detail))
        // Coupon redemption history
        .route("/coupons/redeemed", get(list_redeemed_coupons))
        // Watchlists
        .route("/watchlists", get(list_watchlists))
        .route("/watchlists/{id}", get(get_watchlist))
        // Courses
        .route("/courses", get(get_enrollments))
        .route("/courses/{course_id}/progress", put(update_progress))
        // Phase 4.6: member self-service (password / account / coupon-apply)
        .route("/password", post(post_change_password))
        .route("/account", delete(delete_account))
        .route("/coupons/apply", post(post_apply_coupon))
        // Native Stripe Elements payment-method management.
        .route("/payment-methods", get(list_payment_methods))
        .route("/payment-methods/setup-intent", post(post_setup_intent))
        .route(
            "/payment-methods/{pm_id}/set-default",
            post(post_set_default_payment_method),
        )
        .route("/payment-methods/{pm_id}", delete(delete_payment_method))
        .layer(crate::middleware::rate_limit::member_layer())
}

// ── Profile ─────────────────────────────────────────────────────────────

async fn get_profile(
    State(state): State<AppState>,
    auth: AuthUser,
) -> AppResult<Json<UserResponse>> {
    let user = db::find_user_by_id(&state.db, auth.user_id)
        .await?
        .ok_or(AppError::NotFound("User not found".to_string()))?;
    Ok(Json(user.into()))
}

#[derive(serde::Deserialize, ToSchema)]
pub struct UpdateProfileRequest {
    name: Option<String>,
    avatar_url: Option<String>,
    bio: Option<String>,
    position: Option<String>,
    website_url: Option<String>,
    twitter_url: Option<String>,
    linkedin_url: Option<String>,
    youtube_url: Option<String>,
    instagram_url: Option<String>,
    /// Phase 5: free-text phone number. Validation is intentionally minimal
    /// at the API edge — the DB CHECK in migration 079 caps it at 32 chars.
    #[serde(default)]
    phone: Option<String>,
    /// Phase 5: billing address. Mirrors the Stripe `Address` shape so the
    /// SPA can pass a single object through to both backends. Each field
    /// inside `BillingAddress` is independently optional; missing fields
    /// leave the corresponding column untouched.
    #[serde(default)]
    billing_address: Option<BillingAddress>,
}

#[utoipa::path(
    put,
    path = "/api/member/profile",
    tag = "member",
    security(("bearer_auth" = [])),
    request_body = UpdateProfileRequest,
    responses(
        (status = 200, description = "Profile updated", body = UserResponse),
        (status = 401, description = "Unauthorized")
    )
)]
pub(crate) async fn update_profile(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<UpdateProfileRequest>,
) -> AppResult<Json<UserResponse>> {
    let user = db::find_user_by_id(&state.db, auth.user_id)
        .await?
        .ok_or(AppError::NotFound("User not found".to_string()))?;

    let name = req.name.as_deref().unwrap_or(&user.name);
    let avatar_url = req.avatar_url.as_deref().or(user.avatar_url.as_deref());
    let bio = req.bio.as_deref().or(user.bio.as_deref());
    let position = req.position.as_deref().or(user.position.as_deref());
    let website_url = req.website_url.as_deref().or(user.website_url.as_deref());
    let twitter_url = req.twitter_url.as_deref().or(user.twitter_url.as_deref());
    let linkedin_url = req.linkedin_url.as_deref().or(user.linkedin_url.as_deref());
    let youtube_url = req.youtube_url.as_deref().or(user.youtube_url.as_deref());
    let instagram_url = req
        .instagram_url
        .as_deref()
        .or(user.instagram_url.as_deref());

    // Phase 5: COALESCE-style overlay for phone + billing fields. `None`
    // anywhere in the request leaves the existing column untouched so a
    // partial PATCH can land a single field without zeroing the rest.
    let phone = req.phone.as_deref().or(user.phone.as_deref());
    let addr = req.billing_address.as_ref();
    let billing_line1 = addr
        .and_then(|a| a.line1.as_deref())
        .or(user.billing_line1.as_deref());
    let billing_line2 = addr
        .and_then(|a| a.line2.as_deref())
        .or(user.billing_line2.as_deref());
    let billing_city = addr
        .and_then(|a| a.city.as_deref())
        .or(user.billing_city.as_deref());
    let billing_state = addr
        .and_then(|a| a.state.as_deref())
        .or(user.billing_state.as_deref());
    let billing_postal_code = addr
        .and_then(|a| a.postal_code.as_deref())
        .or(user.billing_postal_code.as_deref());
    // Stripe normalises country codes to upper-case; mirror that contract
    // so the value persisted here is identical to what the admin path
    // would push through `stripe_api::update_customer_address`.
    let normalised_country: Option<String> = addr
        .and_then(|a| a.country.as_deref())
        .map(|c| c.trim().to_ascii_uppercase())
        .or_else(|| user.billing_country.clone());
    let billing_country = normalised_country.as_deref();

    let updated = sqlx::query_as::<_, crate::models::User>(
        r#"UPDATE users SET
            name = $1, avatar_url = $2, bio = $3, position = $4,
            website_url = $5, twitter_url = $6, linkedin_url = $7,
            youtube_url = $8, instagram_url = $9,
            phone = $10,
            billing_line1 = $11, billing_line2 = $12, billing_city = $13,
            billing_state = $14, billing_postal_code = $15, billing_country = $16,
            updated_at = NOW()
           WHERE id = $17 RETURNING *"#,
    )
    .bind(name)
    .bind(avatar_url)
    .bind(bio)
    .bind(position)
    .bind(website_url)
    .bind(twitter_url)
    .bind(linkedin_url)
    .bind(youtube_url)
    .bind(instagram_url)
    .bind(phone)
    .bind(billing_line1)
    .bind(billing_line2)
    .bind(billing_city)
    .bind(billing_state)
    .bind(billing_postal_code)
    .bind(billing_country)
    .bind(auth.user_id)
    .fetch_one(&state.db)
    .await?;

    Ok(Json(updated.into()))
}

// ── Subscription ────────────────────────────────────────────────────────

async fn get_subscription(
    State(state): State<AppState>,
    auth: AuthUser,
) -> AppResult<Json<SubscriptionStatusResponse>> {
    let sub = db::find_subscription_by_user(&state.db, auth.user_id).await?;
    let is_active = sub
        .as_ref()
        .map(|s| s.status == SubscriptionStatus::Active || s.status == SubscriptionStatus::Trialing)
        .unwrap_or(false);

    Ok(Json(SubscriptionStatusResponse {
        subscription: sub,
        is_active,
    }))
}

#[utoipa::path(
    post,
    path = "/api/member/billing-portal",
    tag = "member",
    security(("bearer_auth" = [])),
    request_body = BillingPortalRequest,
    responses(
        (status = 200, description = "Stripe billing portal session URL", body = BillingPortalResponse),
        (status = 400, description = "No subscription on file"),
        (status = 401, description = "Unauthorized")
    )
)]
pub(crate) async fn post_billing_portal(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<BillingPortalRequest>,
) -> AppResult<Json<BillingPortalResponse>> {
    let sub = db::find_subscription_by_user(&state.db, auth.user_id)
        .await?
        .ok_or_else(|| AppError::BadRequest("No subscription on file".to_string()))?;

    let base = state.config.frontend_url.trim_end_matches('/');
    let return_url = req
        .return_url
        .unwrap_or_else(|| format!("{base}/dashboard/account"));

    let url =
        stripe_api::create_billing_portal_session(&state, &sub.stripe_customer_id, &return_url)
            .await?;

    Ok(Json(BillingPortalResponse { url }))
}

#[utoipa::path(
    post,
    path = "/api/member/subscription/cancel",
    tag = "member",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Subscription scheduled to cancel at period end"),
        (status = 400, description = "No subscription on file"),
        (status = 401, description = "Unauthorized")
    )
)]
pub(crate) async fn post_subscription_cancel(
    State(state): State<AppState>,
    auth: AuthUser,
) -> AppResult<Json<serde_json::Value>> {
    let sub = db::find_subscription_by_user(&state.db, auth.user_id)
        .await?
        .ok_or_else(|| AppError::BadRequest("No subscription on file".to_string()))?;

    stripe_api::set_subscription_cancel_at_period_end(&state, &sub.stripe_subscription_id, true)
        .await?;

    Ok(Json(
        serde_json::json!({ "ok": true, "cancel_at_period_end": true }),
    ))
}

#[utoipa::path(
    post,
    path = "/api/member/subscription/resume",
    tag = "member",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Subscription cancellation reversed"),
        (status = 400, description = "No subscription on file"),
        (status = 401, description = "Unauthorized")
    )
)]
pub(crate) async fn post_subscription_resume(
    State(state): State<AppState>,
    auth: AuthUser,
) -> AppResult<Json<serde_json::Value>> {
    let sub = db::find_subscription_by_user(&state.db, auth.user_id)
        .await?
        .ok_or_else(|| AppError::BadRequest("No subscription on file".to_string()))?;

    stripe_api::set_subscription_cancel_at_period_end(&state, &sub.stripe_subscription_id, false)
        .await?;

    Ok(Json(
        serde_json::json!({ "ok": true, "cancel_at_period_end": false }),
    ))
}

// ── Watchlists ──────────────────────────────────────────────────────────

async fn list_watchlists(
    State(state): State<AppState>,
    _auth: AuthUser,
    Query(params): Query<PaginationParams>,
) -> AppResult<Json<PaginatedResponse<Watchlist>>> {
    let per_page = params.per_page();
    let offset = params.offset();
    let page = params.page.unwrap_or(1).max(1);

    let (watchlists, total) = db::list_watchlists(&state.db, offset, per_page, true).await?;
    let total_pages = (total as f64 / per_page as f64).ceil() as i64;

    Ok(Json(PaginatedResponse {
        data: watchlists,
        total,
        page,
        per_page,
        total_pages,
    }))
}

async fn get_watchlist(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path(id): Path<Uuid>,
) -> AppResult<Json<WatchlistWithAlerts>> {
    let watchlist = db::get_watchlist(&state.db, id)
        .await?
        .ok_or(AppError::NotFound("Watchlist not found".to_string()))?;

    if !watchlist.published {
        return Err(AppError::NotFound("Watchlist not found".to_string()));
    }

    let alerts = db::get_alerts_for_watchlist(&state.db, id).await?;
    Ok(Json(WatchlistWithAlerts { watchlist, alerts }))
}

// ── Courses ─────────────────────────────────────────────────────────────

async fn get_enrollments(
    State(state): State<AppState>,
    auth: AuthUser,
) -> AppResult<Json<Vec<CourseEnrollment>>> {
    let enrollments = db::get_user_enrollments(&state.db, auth.user_id).await?;
    Ok(Json(enrollments))
}

#[derive(serde::Deserialize, ToSchema)]
pub struct ProgressUpdate {
    progress: i32,
}

#[utoipa::path(
    put,
    path = "/api/member/courses/{course_id}/progress",
    tag = "member",
    security(("bearer_auth" = [])),
    params(("course_id" = String, Path, description = "Course identifier")),
    request_body = ProgressUpdate,
    responses(
        (status = 200, description = "Enrollment progress updated", body = CourseEnrollment),
        (status = 401, description = "Unauthorized")
    )
)]
pub(crate) async fn update_progress(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(course_id): Path<String>,
    Json(req): Json<ProgressUpdate>,
) -> AppResult<Json<CourseEnrollment>> {
    let enrollment =
        db::update_course_progress(&state.db, auth.user_id, &course_id, req.progress).await?;
    Ok(Json(enrollment))
}

// ── Self-service: password / account / coupon (Phase 4.6) ───────────────

/// Phase 4.6: change-password request body for `POST /api/member/password`.
#[derive(Debug, serde::Deserialize, ToSchema)]
pub struct ChangePasswordRequest {
    pub current_password: String,
    pub new_password: String,
}

#[derive(Debug, serde::Serialize, ToSchema)]
pub struct ChangePasswordResponse {
    pub ok: bool,
}

#[utoipa::path(
    post,
    path = "/api/member/password",
    tag = "member",
    security(("bearer_auth" = [])),
    request_body = ChangePasswordRequest,
    responses(
        (status = 200, description = "Password changed and a fresh access+refresh pair issued as cookies", body = ChangePasswordResponse),
        (status = 400, description = "New password too short or current_password missing"),
        (status = 401, description = "Unauthorized or current_password mismatch")
    )
)]
pub(crate) async fn post_change_password(
    State(state): State<AppState>,
    auth: AuthUser,
    client: ClientInfo,
    Json(req): Json<ChangePasswordRequest>,
) -> AppResult<(HeaderMap, Json<ChangePasswordResponse>)> {
    if req.current_password.is_empty() {
        return Err(AppError::BadRequest(
            "current_password is required".to_string(),
        ));
    }
    if req.new_password.len() < 8 {
        return Err(AppError::BadRequest(
            "new_password must be at least 8 characters".to_string(),
        ));
    }

    // Pull the user row (carries the existing Argon2 hash to verify against).
    let user = db::find_user_by_id(&state.db, auth.user_id)
        .await?
        .ok_or(AppError::Unauthorized)?;

    // Verify current_password against the stored Argon2 hash. A parse
    // failure (e.g. legacy plain-text hash that should never exist in
    // production) is treated as an authentication failure rather than
    // a 500 — the caller cannot do anything useful with a parse error.
    let parsed = PasswordHash::new(&user.password_hash).map_err(|_| AppError::Unauthorized)?;
    if Argon2::default()
        .verify_password(req.current_password.as_bytes(), &parsed)
        .is_err()
    {
        return Err(AppError::Unauthorized);
    }

    // Hash + persist the new password. New salt every time per Argon2
    // recommendation; the salt is embedded in the resulting PHC string.
    let salt = SaltString::generate(&mut OsRng);
    let new_hash = Argon2::default()
        .hash_password(req.new_password.as_bytes(), &salt)
        .map_err(|e| AppError::BadRequest(format!("Password hash error: {e}")))?
        .to_string();

    db::update_user_password(&state.db, auth.user_id, &new_hash).await?;

    // Revoke every active refresh token for this user — any other live
    // session in another browser tab / device is now signed out, matching
    // the contract of `POST /api/auth/reset-password`.
    db::delete_user_refresh_tokens(&state.db, auth.user_id).await?;

    // Mint a fresh access+refresh pair so the calling session keeps
    // working seamlessly. Without this the user would be silently logged
    // out after the next request because we just deleted their refresh
    // token above.
    let refreshed = db::find_user_by_id(&state.db, auth.user_id)
        .await?
        .ok_or(AppError::Unauthorized)?;
    let (access_token, refresh_token) = generate_tokens(&state, &refreshed).await?;
    let cookies = auth_cookie_headers(&state, &access_token, &refresh_token)?;

    audit_admin_under_impersonation(
        &state.db,
        &auth,
        &client,
        "member.password.change",
        "user",
        auth.user_id,
        serde_json::json!({
            "self_service": true,
        }),
    )
    .await;

    Ok((cookies, Json(ChangePasswordResponse { ok: true })))
}

#[utoipa::path(
    delete,
    path = "/api/member/account",
    tag = "member",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Account deleted; auth cookies cleared. Cancels any active Stripe subscription before deletion."),
        (status = 401, description = "Unauthorized")
    )
)]
pub(crate) async fn delete_account(
    State(state): State<AppState>,
    auth: AuthUser,
    client: ClientInfo,
) -> AppResult<(HeaderMap, Json<serde_json::Value>)> {
    // Best-effort cancel any active Stripe subscription before we drop
    // the user row. Stripe failures must not panic — propagate as
    // AppError so the caller sees a 4xx/5xx and can retry. If there's
    // no subscription on file we skip silently (graceful for users who
    // never paid).
    let stripe_cancelled = if let Some(sub) = db::find_subscription_by_user(&state.db, auth.user_id)
        .await?
        .filter(|s| !s.stripe_subscription_id.is_empty())
    {
        match stripe_api::cancel_subscription_immediately(&state, &sub.stripe_subscription_id).await
        {
            Ok(()) => true,
            Err(e) => {
                // Surface Stripe failures rather than swallowing —
                // we'd rather the user retry than have a billable
                // subscription survive an account deletion.
                tracing::warn!(
                    user_id = %auth.user_id,
                    sub_id = %sub.stripe_subscription_id,
                    error = %e,
                    "stripe cancel failed during member self-delete; aborting"
                );
                return Err(e);
            }
        }
    } else {
        false
    };

    // Revoke every refresh token before the row goes away so any
    // concurrent session that races us cannot keep using a stale token.
    db::delete_user_refresh_tokens(&state.db, auth.user_id).await?;

    // Delete the user. We mirror the existing admin delete path
    // (`db::delete_user`) so the same cascade semantics apply.
    db::delete_user(&state.db, auth.user_id).await?;

    audit_admin_under_impersonation(
        &state.db,
        &auth,
        &client,
        "member.account.delete",
        "user",
        auth.user_id,
        serde_json::json!({
            "self_service": true,
            "stripe_cancelled": stripe_cancelled,
        }),
    )
    .await;

    let cookies = clear_auth_cookie_headers(&state)?;
    Ok((cookies, Json(serde_json::json!({ "ok": true }))))
}

/// Phase 4.6: apply-coupon request body for
/// `POST /api/member/coupons/apply`.
#[derive(Debug, serde::Deserialize, ToSchema)]
pub struct ApplyCouponRequest {
    pub code: String,
}

/// Response returned by [`post_apply_coupon`].
///
/// Carries both the audit-plan canonical fields (`ok`, `coupon_id`,
/// `applied_at`) and the legacy [`CouponValidationResponse`] aliases
/// (`valid`, `message`) so the existing dashboard page at
/// `routes/dashboard/account/+page.svelte` keeps rendering after the
/// Phase 4.6 rollout. `valid` mirrors `ok` because every 200 response
/// represents a successful redemption — failure paths land on 4xx with
/// an `AppError` body instead.
#[derive(Debug, serde::Serialize, ToSchema)]
pub struct ApplyCouponResponse {
    pub ok: bool,
    pub coupon_id: Uuid,
    pub applied_at: chrono::DateTime<Utc>,
    /// Legacy alias for `ok` — frontend compatibility with the
    /// pre-existing `CouponValidationResponse` shape.
    pub valid: bool,
    /// Human-readable success message; surfaced verbatim by the SPA.
    pub message: String,
}

#[utoipa::path(
    post,
    path = "/api/member/coupons/apply",
    tag = "member",
    security(("bearer_auth" = [])),
    request_body = ApplyCouponRequest,
    responses(
        (status = 200, description = "Coupon applied to the member's active subscription", body = ApplyCouponResponse),
        (status = 400, description = "Coupon is inactive, expired, exhausted, or no active subscription"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Coupon code not found"),
        (status = 409, description = "Coupon already redeemed by this user")
    )
)]
pub(crate) async fn post_apply_coupon(
    State(state): State<AppState>,
    auth: AuthUser,
    client: ClientInfo,
    Json(req): Json<ApplyCouponRequest>,
) -> AppResult<Json<ApplyCouponResponse>> {
    let code = req.code.trim();
    if code.is_empty() {
        return Err(AppError::BadRequest("Coupon code is required".to_string()));
    }

    // 1. Lookup the coupon. 404 when missing — distinct from 400 for the
    //    "exists-but-invalid" path so the SPA can render different copy.
    let coupon: Coupon = sqlx::query_as("SELECT * FROM coupons WHERE UPPER(code) = UPPER($1)")
        .bind(code)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| AppError::NotFound("Coupon code not found".to_string()))?;

    // 2. Validate (active, in window, global limit, per-user limit).
    if !coupon.is_active {
        return Err(AppError::BadRequest("Coupon is not active".to_string()));
    }
    let now = Utc::now();
    if let Some(starts_at) = coupon.starts_at {
        if now < starts_at {
            return Err(AppError::BadRequest("Coupon is not yet valid".to_string()));
        }
    }
    if let Some(expires_at) = coupon.expires_at {
        if now > expires_at {
            return Err(AppError::BadRequest("Coupon has expired".to_string()));
        }
    }
    if let Some(limit) = coupon.usage_limit {
        if coupon.usage_count >= limit {
            return Err(AppError::BadRequest(
                "Coupon usage limit has been reached".to_string(),
            ));
        }
    }
    let user_redemptions: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM coupon_usages WHERE coupon_id = $1 AND user_id = $2",
    )
    .bind(coupon.id)
    .bind(auth.user_id)
    .fetch_one(&state.db)
    .await?;
    if user_redemptions >= i64::from(coupon.per_user_limit) {
        return Err(AppError::Conflict("Coupon already redeemed".to_string()));
    }

    // 3. Locate the member's active subscription. Without one there's
    //    nowhere to attach the coupon — Stripe's UpdateSubscription is
    //    keyed off a subscription id.
    let subscription = db::find_subscription_by_user(&state.db, auth.user_id)
        .await?
        .ok_or_else(|| AppError::BadRequest("No active subscription on file".to_string()))?;

    // 4. Apply on Stripe when the coupon has a Stripe twin id. Locally-
    //    only coupons (no `stripe_coupon_id`) still record the
    //    redemption so we can audit the intent — but they cannot
    //    discount a Stripe-billed invoice.
    let stripe_applied = if let Some(stripe_coupon_id) = coupon.stripe_coupon_id.as_deref() {
        if !subscription.stripe_subscription_id.is_empty() {
            stripe_api::apply_coupon_to_subscription(
                &state,
                &subscription.stripe_subscription_id,
                stripe_coupon_id,
            )
            .await?;
            true
        } else {
            false
        }
    } else {
        false
    };

    // 5. Insert the redemption + bump usage_count atomically so a
    //    Stripe-applied coupon can never end up unrecorded locally.
    //    `currency` falls back to `'usd'` because `coupons` does not yet
    //    carry a per-coupon currency column; `order_id` is NULL because
    //    apply-coupon attaches to a subscription, not an order.
    let mut tx = state.db.begin().await?;
    let usage: CouponUsage = sqlx::query_as(
        r#"
        INSERT INTO coupon_usages
            (id, coupon_id, user_id, subscription_id, discount_applied_cents,
             currency, order_id, used_at)
        VALUES ($1, $2, $3, $4, 0, $5, NULL, NOW())
        RETURNING *
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(coupon.id)
    .bind(auth.user_id)
    .bind(Some(subscription.id))
    .bind("usd")
    .fetch_one(&mut *tx)
    .await?;
    sqlx::query(
        "UPDATE coupons SET usage_count = usage_count + 1, updated_at = NOW() WHERE id = $1",
    )
    .bind(coupon.id)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;

    audit_admin_under_impersonation(
        &state.db,
        &auth,
        &client,
        "member.coupon.apply",
        "coupon",
        coupon.id,
        serde_json::json!({
            "coupon_code": coupon.code,
            "subscription_id": subscription.id,
            "stripe_applied": stripe_applied,
        }),
    )
    .await;

    let message = if stripe_applied {
        format!("Coupon {} applied to your subscription.", coupon.code)
    } else {
        format!("Coupon {} redeemed.", coupon.code)
    };

    Ok(Json(ApplyCouponResponse {
        ok: true,
        coupon_id: coupon.id,
        applied_at: usage.used_at,
        valid: true,
        message,
    }))
}

// ── Phase 5: Orders history ────────────────────────────────────────────

/// Compact order row returned by `GET /api/member/orders` and embedded
/// inside [`MemberSubscriptionDetailResponse::related_orders`]. Strict
/// subset of the admin `Order` shape — the SPA only renders the columns
/// actually visible on the member dashboard.
#[derive(Debug, Serialize, ToSchema)]
pub struct MemberOrderListItem {
    pub id: Uuid,
    pub number: String,
    pub status: String,
    pub currency: String,
    pub total_cents: i64,
    pub item_count: i64,
    pub placed_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

/// Lightweight refund row exposed to members. We deliberately omit
/// `created_by` (operator id) — that's privileged context.
#[derive(Debug, Serialize, ToSchema)]
pub struct MemberOrderRefund {
    pub id: Uuid,
    pub amount_cents: i64,
    pub reason: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// One transition in the order's state log; mirrors the
/// `order_state_transitions` row shape minus the actor id.
#[derive(Debug, Serialize, ToSchema, FromRow)]
pub struct MemberOrderStateTransition {
    pub from_status: String,
    pub to_status: String,
    pub reason: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct MemberOrderDetailResponse {
    pub order: orders_repo::Order,
    pub items: Vec<orders_repo::OrderItem>,
    pub refunds: Vec<MemberOrderRefund>,
    pub state_log: Vec<MemberOrderStateTransition>,
}

/// OpenAPI wrapper around `PaginatedResponse<MemberOrderListItem>` so the
/// snapshot doesn't leak the generic into `ApiDoc`.
#[derive(Debug, Serialize, ToSchema)]
pub struct PaginatedMemberOrdersResponse {
    pub data: Vec<MemberOrderListItem>,
    pub total: i64,
    pub page: i64,
    pub per_page: i64,
    pub total_pages: i64,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct MemberOrdersListQuery {
    pub page: Option<i64>,
    pub per_page: Option<i64>,
}

#[utoipa::path(
    get,
    path = "/api/member/orders",
    tag = "member",
    security(("bearer_auth" = [])),
    params(
        ("page" = Option<i64>, Query, description = "1-based page number (default 1)"),
        ("per_page" = Option<i64>, Query, description = "Page size (default 20, max 50)"),
    ),
    responses(
        (status = 200, description = "Paginated orders for the authenticated member", body = PaginatedMemberOrdersResponse),
        (status = 401, description = "Unauthorized")
    )
)]
pub(crate) async fn list_orders(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(q): Query<MemberOrdersListQuery>,
) -> AppResult<Json<PaginatedResponse<MemberOrderListItem>>> {
    let page = q.page.unwrap_or(1).max(1);
    let per_page = q.per_page.unwrap_or(20).clamp(1, 50);
    let offset = (page - 1) * per_page;

    let rows = sqlx::query_as::<_, MemberOrderListRow>(
        r#"
        SELECT o.id,
               o.number,
               o.status::text AS status,
               o.currency,
               o.total_cents,
               COALESCE((SELECT COUNT(*) FROM order_items WHERE order_id = o.id), 0)::bigint
                   AS item_count,
               o.placed_at,
               o.completed_at,
               o.created_at
          FROM orders o
         WHERE o.user_id = $1
         ORDER BY o.created_at DESC
         LIMIT $2 OFFSET $3
        "#,
    )
    .bind(auth.user_id)
    .bind(per_page)
    .bind(offset)
    .fetch_all(&state.db)
    .await?;

    let total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM orders WHERE user_id = $1")
        .bind(auth.user_id)
        .fetch_one(&state.db)
        .await?;

    let total_pages = if per_page > 0 {
        (total + per_page - 1) / per_page
    } else {
        0
    };

    let data = rows.into_iter().map(MemberOrderListItem::from).collect();

    Ok(Json(PaginatedResponse {
        data,
        total,
        page,
        per_page,
        total_pages,
    }))
}

/// FromRow shim used by the orders list query. Mirrors the columns the
/// SELECT projects so we don't have to write a manual `try_get` per field.
#[derive(Debug, FromRow)]
struct MemberOrderListRow {
    id: Uuid,
    number: String,
    status: String,
    currency: String,
    total_cents: i64,
    item_count: i64,
    placed_at: Option<DateTime<Utc>>,
    completed_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
}

impl From<MemberOrderListRow> for MemberOrderListItem {
    fn from(r: MemberOrderListRow) -> Self {
        Self {
            id: r.id,
            number: r.number,
            status: r.status,
            currency: r.currency,
            total_cents: r.total_cents,
            item_count: r.item_count,
            placed_at: r.placed_at,
            completed_at: r.completed_at,
            created_at: r.created_at,
        }
    }
}

#[utoipa::path(
    get,
    path = "/api/member/orders/{id}",
    tag = "member",
    security(("bearer_auth" = [])),
    params(("id" = Uuid, Path, description = "Order id")),
    responses(
        (status = 200, description = "Order detail", body = MemberOrderDetailResponse),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Order not found or not owned by the member")
    )
)]
pub(crate) async fn get_order_detail(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> AppResult<Json<MemberOrderDetailResponse>> {
    // Ownership check folded into the lookup — anything that isn't the
    // member's own order returns 404 (NOT 403) to avoid leaking
    // existence to a probing client.
    let order = orders_repo::get_order(&state.db, id)
        .await?
        .filter(|o| o.user_id == Some(auth.user_id))
        .ok_or_else(|| AppError::NotFound("Order not found".to_string()))?;

    let items = sqlx::query_as::<_, orders_repo::OrderItem>(
        "SELECT * FROM order_items WHERE order_id = $1 ORDER BY created_at",
    )
    .bind(id)
    .fetch_all(&state.db)
    .await?;

    let refunds = sqlx::query_as::<_, orders_repo::OrderRefund>(
        "SELECT * FROM order_refunds WHERE order_id = $1 ORDER BY created_at",
    )
    .bind(id)
    .fetch_all(&state.db)
    .await?
    .into_iter()
    .map(|r| MemberOrderRefund {
        id: r.id,
        amount_cents: r.amount_cents,
        reason: r.reason,
        created_at: r.created_at,
    })
    .collect();

    let state_log = sqlx::query_as::<_, MemberOrderStateTransition>(
        r#"
        SELECT from_status::text AS from_status,
               to_status::text   AS to_status,
               reason,
               created_at
          FROM order_state_transitions
         WHERE order_id = $1
         ORDER BY created_at
        "#,
    )
    .bind(id)
    .fetch_all(&state.db)
    .await?;

    Ok(Json(MemberOrderDetailResponse {
        order,
        items,
        refunds,
        state_log,
    }))
}

// ── Phase 5: Subscriptions history + actions ───────────────────────────

/// Compact subscription row for `GET /api/member/subscriptions`. Joins
/// `pricing_plans` so the SPA can render the catalog name + price without
/// a follow-up call.
#[derive(Debug, Serialize, ToSchema)]
pub struct MemberSubscriptionListItem {
    pub id: Uuid,
    pub plan: SubscriptionPlan,
    pub status: SubscriptionStatus,
    pub current_period_start: DateTime<Utc>,
    pub current_period_end: DateTime<Utc>,
    pub paused_at: Option<DateTime<Utc>>,
    pub pause_resumes_at: Option<DateTime<Utc>>,
    /// Derived from `subscriptions.cancel_at IS NOT NULL`. Stripe stores
    /// the boolean separately; we mirror the admin handler convention so
    /// both surfaces agree.
    pub cancel_at_period_end: bool,
    pub pricing_plan_id: Option<Uuid>,
    pub plan_name: Option<String>,
    pub amount_cents: Option<i64>,
    pub currency: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct PaginatedMemberSubscriptionsResponse {
    pub data: Vec<MemberSubscriptionListItem>,
    pub total: i64,
    pub page: i64,
    pub per_page: i64,
    pub total_pages: i64,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct MemberSubscriptionsListQuery {
    pub page: Option<i64>,
    pub per_page: Option<i64>,
}

/// Mirror of `subscription_invoices` with only the columns we surface to
/// members (no `attempt_count`, no actor metadata).
#[derive(Debug, Serialize, ToSchema, FromRow)]
pub struct MemberSubscriptionInvoice {
    pub id: Uuid,
    pub stripe_invoice_id: String,
    pub status: String,
    pub amount_due_cents: i64,
    pub amount_paid_cents: i64,
    pub currency: String,
    pub period_start: Option<DateTime<Utc>>,
    pub period_end: Option<DateTime<Utc>>,
    pub paid_at: Option<DateTime<Utc>>,
    /// Stripe-hosted invoice page (member-friendly receipt). Populated by
    /// the `invoice.paid` / `invoice.payment_failed` webhooks once Stripe
    /// has finalized the invoice. NULL on drafts.
    pub hosted_invoice_url: Option<String>,
    /// Direct link to the PDF rendering of the invoice. Same lifecycle as
    /// `hosted_invoice_url` — populated when Stripe finalizes the invoice.
    pub invoice_pdf: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct MemberSubscriptionDetailResponse {
    pub subscription: Subscription,
    pub plan: Option<PricingPlan>,
    pub invoices: Vec<MemberSubscriptionInvoice>,
    pub related_orders: Vec<MemberOrderListItem>,
}

#[utoipa::path(
    get,
    path = "/api/member/subscriptions",
    tag = "member",
    security(("bearer_auth" = [])),
    params(
        ("page" = Option<i64>, Query, description = "1-based page number (default 1)"),
        ("per_page" = Option<i64>, Query, description = "Page size (default 20, max 50)"),
    ),
    responses(
        (status = 200, description = "Full subscription history for the authenticated member", body = PaginatedMemberSubscriptionsResponse),
        (status = 401, description = "Unauthorized")
    )
)]
pub(crate) async fn list_subscriptions(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(q): Query<MemberSubscriptionsListQuery>,
) -> AppResult<Json<PaginatedResponse<MemberSubscriptionListItem>>> {
    let page = q.page.unwrap_or(1).max(1);
    let per_page = q.per_page.unwrap_or(20).clamp(1, 50);
    let offset = (page - 1) * per_page;

    let rows = sqlx::query_as::<_, MemberSubscriptionListRow>(
        r#"
        SELECT s.id,
               s.plan,
               s.status,
               s.current_period_start,
               s.current_period_end,
               s.paused_at,
               s.pause_resumes_at,
               (s.cancel_at IS NOT NULL) AS cancel_at_period_end,
               s.pricing_plan_id,
               p.name           AS plan_name,
               p.amount_cents   AS plan_amount_cents,
               p.currency       AS plan_currency,
               s.created_at
          FROM subscriptions s
          LEFT JOIN pricing_plans p ON p.id = s.pricing_plan_id
         WHERE s.user_id = $1
         ORDER BY s.created_at DESC
         LIMIT $2 OFFSET $3
        "#,
    )
    .bind(auth.user_id)
    .bind(per_page)
    .bind(offset)
    .fetch_all(&state.db)
    .await?;

    let total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM subscriptions WHERE user_id = $1")
        .bind(auth.user_id)
        .fetch_one(&state.db)
        .await?;

    let total_pages = if per_page > 0 {
        (total + per_page - 1) / per_page
    } else {
        0
    };

    let data = rows
        .into_iter()
        .map(MemberSubscriptionListItem::from)
        .collect();

    Ok(Json(PaginatedResponse {
        data,
        total,
        page,
        per_page,
        total_pages,
    }))
}

#[derive(Debug, FromRow)]
struct MemberSubscriptionListRow {
    id: Uuid,
    plan: SubscriptionPlan,
    status: SubscriptionStatus,
    current_period_start: DateTime<Utc>,
    current_period_end: DateTime<Utc>,
    paused_at: Option<DateTime<Utc>>,
    pause_resumes_at: Option<DateTime<Utc>>,
    cancel_at_period_end: bool,
    pricing_plan_id: Option<Uuid>,
    plan_name: Option<String>,
    plan_amount_cents: Option<i64>,
    plan_currency: Option<String>,
    created_at: DateTime<Utc>,
}

impl From<MemberSubscriptionListRow> for MemberSubscriptionListItem {
    fn from(r: MemberSubscriptionListRow) -> Self {
        Self {
            id: r.id,
            plan: r.plan,
            status: r.status,
            current_period_start: r.current_period_start,
            current_period_end: r.current_period_end,
            paused_at: r.paused_at,
            pause_resumes_at: r.pause_resumes_at,
            cancel_at_period_end: r.cancel_at_period_end,
            pricing_plan_id: r.pricing_plan_id,
            plan_name: r.plan_name,
            amount_cents: r.plan_amount_cents,
            currency: r.plan_currency,
            created_at: r.created_at,
        }
    }
}

#[utoipa::path(
    get,
    path = "/api/member/subscriptions/{id}",
    tag = "member",
    security(("bearer_auth" = [])),
    params(("id" = Uuid, Path, description = "Subscription id")),
    responses(
        (status = 200, description = "Subscription detail (plan + invoices + related orders)", body = MemberSubscriptionDetailResponse),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Subscription not found or not owned by the member")
    )
)]
pub(crate) async fn get_subscription_detail(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> AppResult<Json<MemberSubscriptionDetailResponse>> {
    let subscription = load_owned_subscription(&state, auth.user_id, id).await?;

    let plan = if let Some(plan_id) = subscription.pricing_plan_id {
        sqlx::query_as::<_, PricingPlan>("SELECT * FROM pricing_plans WHERE id = $1")
            .bind(plan_id)
            .fetch_optional(&state.db)
            .await?
    } else {
        None
    };

    let invoices = sqlx::query_as::<_, MemberSubscriptionInvoice>(
        r#"
        SELECT id, stripe_invoice_id, status, amount_due_cents, amount_paid_cents,
               currency, period_start, period_end, paid_at,
               hosted_invoice_url, invoice_pdf, created_at
          FROM subscription_invoices
         WHERE subscription_id = $1 OR (user_id = $2 AND stripe_subscription_id = $3)
         ORDER BY created_at DESC
        "#,
    )
    .bind(subscription.id)
    .bind(auth.user_id)
    .bind(&subscription.stripe_subscription_id)
    .fetch_all(&state.db)
    .await?;

    // Related orders: anything where the order metadata names this
    // subscription, or anything that shared the same Stripe customer id
    // (covers post-checkout digital-good orders that don't carry the
    // subscription id explicitly).
    let related_orders = sqlx::query_as::<_, MemberOrderListRow>(
        r#"
        SELECT o.id,
               o.number,
               o.status::text AS status,
               o.currency,
               o.total_cents,
               COALESCE((SELECT COUNT(*) FROM order_items WHERE order_id = o.id), 0)::bigint
                   AS item_count,
               o.placed_at,
               o.completed_at,
               o.created_at
          FROM orders o
         WHERE o.user_id = $1
           AND (
                (o.metadata ->> 'subscription_id') = $2
                OR ($3::text <> '' AND o.stripe_customer_id = $3)
           )
         ORDER BY o.created_at DESC
         LIMIT 50
        "#,
    )
    .bind(auth.user_id)
    .bind(subscription.id.to_string())
    .bind(&subscription.stripe_customer_id)
    .fetch_all(&state.db)
    .await?
    .into_iter()
    .map(MemberOrderListItem::from)
    .collect();

    Ok(Json(MemberSubscriptionDetailResponse {
        subscription,
        plan,
        invoices,
        related_orders,
    }))
}

/// Look up a subscription by id, enforcing member ownership. Returns
/// `404` (not `403`) on a foreign id so callers cannot enumerate
/// subscription primary keys.
async fn load_owned_subscription(
    state: &AppState,
    user_id: Uuid,
    subscription_id: Uuid,
) -> AppResult<Subscription> {
    let sub = sqlx::query_as::<_, Subscription>("SELECT * FROM subscriptions WHERE id = $1")
        .bind(subscription_id)
        .fetch_optional(&state.db)
        .await?
        .filter(|s| s.user_id == user_id)
        .ok_or_else(|| AppError::NotFound("Subscription not found".to_string()))?;
    Ok(sub)
}

/// Re-fetch a subscription after a mutation so the response carries the
/// freshly-persisted shape (paused_at, status, etc.).
async fn fetch_subscription(state: &AppState, subscription_id: Uuid) -> AppResult<Subscription> {
    sqlx::query_as::<_, Subscription>("SELECT * FROM subscriptions WHERE id = $1")
        .bind(subscription_id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| AppError::NotFound("Subscription not found".to_string()))
}

#[utoipa::path(
    post,
    path = "/api/member/subscriptions/{id}/cancel",
    tag = "member",
    security(("bearer_auth" = [])),
    params(("id" = Uuid, Path, description = "Subscription id")),
    responses(
        (status = 200, description = "Cancel-at-period-end flag set on the subscription", body = Subscription),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Subscription not found or not owned by the member")
    )
)]
pub(crate) async fn post_cancel_subscription(
    State(state): State<AppState>,
    auth: AuthUser,
    client: ClientInfo,
    Path(id): Path<Uuid>,
) -> AppResult<Json<Subscription>> {
    let sub = load_owned_subscription(&state, auth.user_id, id).await?;

    // Only call Stripe when there's a real Stripe twin id. Local-only
    // rows (created via tests / comp grants) skip the Stripe round trip
    // but still mirror the cancel state in the DB.
    if !sub.stripe_subscription_id.is_empty() {
        stripe_api::set_subscription_cancel_at_period_end(
            &state,
            &sub.stripe_subscription_id,
            true,
        )
        .await?;
    }

    sqlx::query(
        "UPDATE subscriptions SET cancel_at = current_period_end, updated_at = NOW() WHERE id = $1",
    )
    .bind(sub.id)
    .execute(&state.db)
    .await?;

    audit_admin_under_impersonation(
        &state.db,
        &auth,
        &client,
        "member.subscription.cancel",
        "subscription",
        sub.id,
        serde_json::json!({
            "self_service":           true,
            "stripe_subscription_id": sub.stripe_subscription_id,
        }),
    )
    .await;

    let refreshed = fetch_subscription(&state, sub.id).await?;
    Ok(Json(refreshed))
}

#[utoipa::path(
    post,
    path = "/api/member/subscriptions/{id}/resume",
    tag = "member",
    security(("bearer_auth" = [])),
    params(("id" = Uuid, Path, description = "Subscription id")),
    responses(
        (status = 200, description = "Cancel-at-period-end flag cleared", body = Subscription),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Subscription not found or not owned by the member")
    )
)]
pub(crate) async fn post_resume_subscription(
    State(state): State<AppState>,
    auth: AuthUser,
    client: ClientInfo,
    Path(id): Path<Uuid>,
) -> AppResult<Json<Subscription>> {
    let sub = load_owned_subscription(&state, auth.user_id, id).await?;

    if !sub.stripe_subscription_id.is_empty() {
        stripe_api::set_subscription_cancel_at_period_end(
            &state,
            &sub.stripe_subscription_id,
            false,
        )
        .await?;
    }

    sqlx::query("UPDATE subscriptions SET cancel_at = NULL, updated_at = NOW() WHERE id = $1")
        .bind(sub.id)
        .execute(&state.db)
        .await?;

    audit_admin_under_impersonation(
        &state.db,
        &auth,
        &client,
        "member.subscription.resume",
        "subscription",
        sub.id,
        serde_json::json!({
            "self_service":           true,
            "stripe_subscription_id": sub.stripe_subscription_id,
        }),
    )
    .await;

    let refreshed = fetch_subscription(&state, sub.id).await?;
    Ok(Json(refreshed))
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct PauseSubscriptionRequest {
    /// Optional RFC3339 timestamp at which the subscription should auto-resume.
    /// `None` ⇒ open-ended pause (member must call `unpause` to lift).
    #[serde(default)]
    pub resume_at: Option<DateTime<Utc>>,
}

#[utoipa::path(
    post,
    path = "/api/member/subscriptions/{id}/pause",
    tag = "member",
    security(("bearer_auth" = [])),
    params(("id" = Uuid, Path, description = "Subscription id")),
    request_body = PauseSubscriptionRequest,
    responses(
        (status = 200, description = "Subscription paused", body = Subscription),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Subscription not found or not owned by the member")
    )
)]
pub(crate) async fn post_pause_subscription(
    State(state): State<AppState>,
    auth: AuthUser,
    client: ClientInfo,
    Path(id): Path<Uuid>,
    Json(req): Json<PauseSubscriptionRequest>,
) -> AppResult<Json<Subscription>> {
    let sub = load_owned_subscription(&state, auth.user_id, id).await?;

    subs_repo::pause(&state.db, sub.id, req.resume_at).await?;

    audit_admin_under_impersonation(
        &state.db,
        &auth,
        &client,
        "member.subscription.pause",
        "subscription",
        sub.id,
        serde_json::json!({
            "self_service": true,
            "resume_at":    req.resume_at,
        }),
    )
    .await;

    let refreshed = fetch_subscription(&state, sub.id).await?;
    Ok(Json(refreshed))
}

#[utoipa::path(
    post,
    path = "/api/member/subscriptions/{id}/unpause",
    tag = "member",
    security(("bearer_auth" = [])),
    params(("id" = Uuid, Path, description = "Subscription id")),
    responses(
        (status = 200, description = "Subscription resumed", body = Subscription),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Subscription not found or not owned by the member")
    )
)]
pub(crate) async fn post_unpause_subscription(
    State(state): State<AppState>,
    auth: AuthUser,
    client: ClientInfo,
    Path(id): Path<Uuid>,
) -> AppResult<Json<Subscription>> {
    let sub = load_owned_subscription(&state, auth.user_id, id).await?;

    subs_repo::resume(&state.db, sub.id).await?;

    audit_admin_under_impersonation(
        &state.db,
        &auth,
        &client,
        "member.subscription.unpause",
        "subscription",
        sub.id,
        serde_json::json!({ "self_service": true }),
    )
    .await;

    let refreshed = fetch_subscription(&state, sub.id).await?;
    Ok(Json(refreshed))
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct SwitchPlanRequest {
    /// Target `pricing_plans.id` to switch the subscription onto.
    pub pricing_plan_id: Uuid,
    /// When `true` (default) Stripe creates prorations for the swap.
    /// `false` defers the new price to the next renewal — useful for
    /// downgrades that should not refund mid-cycle.
    #[serde(default = "default_prorate")]
    pub prorate: bool,
}

fn default_prorate() -> bool {
    true
}

#[utoipa::path(
    post,
    path = "/api/member/subscriptions/{id}/switch-plan",
    tag = "member",
    security(("bearer_auth" = [])),
    params(("id" = Uuid, Path, description = "Subscription id")),
    request_body = SwitchPlanRequest,
    responses(
        (status = 200, description = "Subscription switched to the new plan", body = Subscription),
        (status = 400, description = "Target plan invalid (no Stripe price id, missing line item, etc.)"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Subscription or pricing plan not found")
    )
)]
pub(crate) async fn post_switch_subscription_plan(
    State(state): State<AppState>,
    auth: AuthUser,
    client: ClientInfo,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
    Json(req): Json<SwitchPlanRequest>,
) -> AppResult<Json<Subscription>> {
    let sub = load_owned_subscription(&state, auth.user_id, id).await?;

    let target_plan = sqlx::query_as::<_, PricingPlan>(
        "SELECT * FROM pricing_plans WHERE id = $1 AND is_active = TRUE",
    )
    .bind(req.pricing_plan_id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| AppError::NotFound("Pricing plan not found".to_string()))?;

    let stripe_price_id = target_plan.stripe_price_id.as_deref().ok_or_else(|| {
        AppError::BadRequest(
            "Target pricing plan has no stripe_price_id; cannot switch via Stripe".to_string(),
        )
    })?;

    // Talk to Stripe only when the local row carries a real Stripe twin —
    // otherwise we mirror the switch in the DB and call it done.
    if !sub.stripe_subscription_id.is_empty() {
        let stripe_sub =
            stripe_api::retrieve_subscription(&state, &sub.stripe_subscription_id).await?;
        let item_id = stripe_sub
            .items
            .data
            .first()
            .map(|i| i.id.clone())
            .ok_or_else(|| {
                AppError::BadRequest("Stripe subscription has no line items to swap".to_string())
            })?;
        let idempotency_key = headers
            .get("idempotency-key")
            .and_then(|v| v.to_str().ok())
            .map(str::to_string);
        stripe_api::swap_subscription_price_with_proration(
            &state,
            &sub.stripe_subscription_id,
            &item_id,
            stripe_price_id,
            req.prorate,
            idempotency_key.as_deref(),
        )
        .await?;
    }

    // Persist the local switch + log into `subscription_changes` so the
    // history view shows the upgrade/downgrade. We keep the cadence enum
    // consistent with the catalog row's interval (`month`/`year`).
    let new_cadence = match target_plan.interval.as_str() {
        "year" => SubscriptionPlan::Annual,
        _ => SubscriptionPlan::Monthly,
    };
    sqlx::query(
        r#"
        UPDATE subscriptions
           SET pricing_plan_id = $1,
               plan            = $2,
               updated_at      = NOW()
         WHERE id = $3
        "#,
    )
    .bind(target_plan.id)
    .bind(new_cadence)
    .bind(sub.id)
    .execute(&state.db)
    .await?;

    let _change = subs_repo::record_change(
        &state.db,
        sub.id,
        "switch_plan",
        sub.pricing_plan_id,
        Some(target_plan.id),
        0,
        Some(auth.user_id),
        Some("member.self_service"),
    )
    .await?;

    audit_admin_under_impersonation(
        &state.db,
        &auth,
        &client,
        "member.subscription.switch_plan",
        "subscription",
        sub.id,
        serde_json::json!({
            "self_service":     true,
            "from_plan_id":     sub.pricing_plan_id,
            "to_plan_id":       target_plan.id,
            "prorate":          req.prorate,
            "stripe_price_id":  stripe_price_id,
        }),
    )
    .await;

    let refreshed = fetch_subscription(&state, sub.id).await?;
    Ok(Json(refreshed))
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct SwitchPlanPreviewQuery {
    pub pricing_plan_id: Uuid,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct SwitchPlanPreviewResponse {
    pub proration_credit_cents: i64,
    pub proration_charge_cents: i64,
    pub immediate_total_cents: i64,
    pub next_invoice_total_cents: i64,
    pub currency: String,
}

#[utoipa::path(
    get,
    path = "/api/member/subscriptions/{id}/switch-plan/preview",
    tag = "member",
    security(("bearer_auth" = [])),
    params(
        ("id" = Uuid, Path, description = "Subscription id"),
        ("pricing_plan_id" = Uuid, Query, description = "Target pricing_plans.id to preview"),
    ),
    responses(
        (status = 200, description = "Stripe upcoming-invoice proration preview", body = SwitchPlanPreviewResponse),
        (status = 400, description = "Target plan invalid or local subscription has no Stripe twin"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Subscription or pricing plan not found")
    )
)]
pub(crate) async fn get_switch_plan_preview(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
    Query(q): Query<SwitchPlanPreviewQuery>,
) -> AppResult<Json<SwitchPlanPreviewResponse>> {
    let sub = load_owned_subscription(&state, auth.user_id, id).await?;
    if sub.stripe_subscription_id.is_empty() {
        return Err(AppError::BadRequest(
            "Subscription has no Stripe twin; preview unavailable".to_string(),
        ));
    }
    let target_plan = sqlx::query_as::<_, PricingPlan>(
        "SELECT * FROM pricing_plans WHERE id = $1 AND is_active = TRUE",
    )
    .bind(q.pricing_plan_id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| AppError::NotFound("Pricing plan not found".to_string()))?;
    let stripe_price_id = target_plan.stripe_price_id.as_deref().ok_or_else(|| {
        AppError::BadRequest(
            "Target pricing plan has no stripe_price_id; cannot preview".to_string(),
        )
    })?;

    let stripe_sub = stripe_api::retrieve_subscription(&state, &sub.stripe_subscription_id).await?;
    let item_id = stripe_sub
        .items
        .data
        .first()
        .map(|i| i.id.clone())
        .ok_or_else(|| {
            AppError::BadRequest("Stripe subscription has no line items to preview".to_string())
        })?;

    let preview = stripe_api::preview_subscription_change(
        &state,
        &sub.stripe_subscription_id,
        &item_id,
        stripe_price_id,
    )
    .await?;

    Ok(Json(SwitchPlanPreviewResponse {
        proration_credit_cents: preview.proration_credit_cents,
        proration_charge_cents: preview.proration_charge_cents,
        immediate_total_cents: preview.immediate_total_cents,
        next_invoice_total_cents: preview.next_invoice_total_cents,
        currency: preview.currency,
    }))
}

// ── Phase 5: Coupon redemptions history ────────────────────────────────

#[derive(Debug, Serialize, ToSchema)]
pub struct MemberCouponRedemptionResponse {
    pub id: Uuid,
    pub coupon_code: String,
    pub discount_applied_cents: i64,
    pub redeemed_at: DateTime<Utc>,
    pub subscription_id: Option<Uuid>,
    /// ISO 4217 currency code (lowercase) the discount was denominated
    /// in at redemption time. Frontend formats `discount_applied_cents`
    /// against this code so members see e.g. `−$5.00 USD` or
    /// `−€10.00 EUR` rather than a bare cent value.
    pub currency: String,
    /// FK to `orders.id` when the redemption was tied to a concrete
    /// order (vs. attached only to a subscription). NULL when no order
    /// was involved — the member UI hides the "view order" link in that
    /// case.
    pub order_id: Option<Uuid>,
}

#[derive(Debug, FromRow)]
struct MemberCouponRedemptionRow {
    id: Uuid,
    coupon_code: String,
    discount_applied_cents: i64,
    redeemed_at: DateTime<Utc>,
    subscription_id: Option<Uuid>,
    currency: String,
    order_id: Option<Uuid>,
}

#[utoipa::path(
    get,
    path = "/api/member/coupons/redeemed",
    tag = "member",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Coupon redemptions for the authenticated member", body = [MemberCouponRedemptionResponse]),
        (status = 401, description = "Unauthorized")
    )
)]
pub(crate) async fn list_redeemed_coupons(
    State(state): State<AppState>,
    auth: AuthUser,
) -> AppResult<Json<Vec<MemberCouponRedemptionResponse>>> {
    let rows = sqlx::query_as::<_, MemberCouponRedemptionRow>(
        r#"
        SELECT u.id              AS id,
               c.code            AS coupon_code,
               u.discount_applied_cents,
               u.used_at         AS redeemed_at,
               u.subscription_id,
               u.currency,
               u.order_id
          FROM coupon_usages u
          JOIN coupons c ON c.id = u.coupon_id
         WHERE u.user_id = $1
         ORDER BY u.used_at DESC
        "#,
    )
    .bind(auth.user_id)
    .fetch_all(&state.db)
    .await?;

    let data = rows
        .into_iter()
        .map(|r| MemberCouponRedemptionResponse {
            id: r.id,
            coupon_code: r.coupon_code,
            discount_applied_cents: r.discount_applied_cents,
            redeemed_at: r.redeemed_at,
            subscription_id: r.subscription_id,
            currency: r.currency,
            order_id: r.order_id,
        })
        .collect();

    Ok(Json(data))
}

// ── Native payment-methods management ──────────────────────────────────
//
// These endpoints let the SPA list / set-default / delete saved cards and
// mint a SetupIntent for the Stripe Elements iframe — replacing the
// Stripe-portal redirect that previously powered the "Payment Methods"
// page. The PCI scope still belongs to Stripe (the SPA never sees a raw
// PAN); we only round-trip `pm_*` ids on the wire.
//
// The customer id is resolved through the most recent Stripe-twinned
// `subscriptions` row for the user (Stripe customers are first created
// at checkout). Members without any subscription cannot manage saved
// cards yet — we return 404 instead of inventing a customer at GET
// time, mirroring the behaviour of the billing-portal endpoint.

/// Wire shape returned by `GET /api/member/payment-methods`.
#[derive(Debug, Serialize, ToSchema)]
pub struct MemberPaymentMethodsResponse {
    pub payment_methods: Vec<stripe_api::PaymentMethodSummary>,
    pub default_payment_method_id: Option<String>,
}

/// Response body for `POST /api/member/payment-methods/setup-intent`.
#[derive(Debug, Serialize, ToSchema)]
pub struct SetupIntentResponse {
    pub client_secret: String,
}

/// Response body for `POST /api/member/payment-methods/{pm_id}/set-default`.
#[derive(Debug, Serialize, ToSchema)]
pub struct SetDefaultPaymentMethodResponse {
    pub default_payment_method_id: String,
}

/// Response body for `DELETE /api/member/payment-methods/{pm_id}`.
#[derive(Debug, Serialize, ToSchema)]
pub struct DeletePaymentMethodResponse {
    pub deleted: bool,
}

/// Resolve the Stripe customer id for the calling member. We look at
/// every subscription on the user (active or otherwise) and pick the
/// first one carrying a non-empty `stripe_customer_id` — returning 404
/// when nothing matches. We deliberately don't fall back to a fresh
/// `POST /v1/customers` here: customers should be born at checkout so
/// the local mirror always has a row to attach the `pm_*` to.
async fn resolve_member_stripe_customer(state: &AppState, user_id: Uuid) -> AppResult<String> {
    let customer_id: Option<String> = sqlx::query_scalar(
        r#"
        SELECT stripe_customer_id
          FROM subscriptions
         WHERE user_id = $1
           AND stripe_customer_id <> ''
         ORDER BY created_at DESC
         LIMIT 1
        "#,
    )
    .bind(user_id)
    .fetch_optional(&state.db)
    .await?;

    customer_id.ok_or_else(|| {
        AppError::NotFound(
            "No Stripe customer on file. Subscribe first to manage payment methods.".to_string(),
        )
    })
}

#[utoipa::path(
    get,
    path = "/api/member/payment-methods",
    tag = "member",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Saved payment methods + the customer's current default", body = MemberPaymentMethodsResponse),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Member has no Stripe customer on file"),
    )
)]
pub(crate) async fn list_payment_methods(
    State(state): State<AppState>,
    auth: AuthUser,
) -> AppResult<Json<MemberPaymentMethodsResponse>> {
    let customer_id = resolve_member_stripe_customer(&state, auth.user_id).await?;
    // Two Stripe round-trips — the list itself and the customer GET so we
    // can stamp `is_default` on the matching row. We could squeeze into
    // a single call by expanding `default_payment_method` on the list
    // endpoint, but that endpoint doesn't accept the expand parameter
    // (Stripe quirk), so we accept the extra round trip.
    let mut payment_methods =
        stripe_api::list_customer_payment_methods(&state, &customer_id).await?;
    let default_payment_method_id =
        stripe_api::get_customer_default_payment_method(&state, &customer_id).await?;
    if let Some(default_id) = default_payment_method_id.as_deref() {
        for pm in payment_methods.iter_mut() {
            if pm.id == default_id {
                pm.is_default = true;
            }
        }
    }
    Ok(Json(MemberPaymentMethodsResponse {
        payment_methods,
        default_payment_method_id,
    }))
}

#[utoipa::path(
    post,
    path = "/api/member/payment-methods/setup-intent",
    tag = "member",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Stripe SetupIntent client_secret for Stripe Elements", body = SetupIntentResponse),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Member has no Stripe customer on file"),
    )
)]
pub(crate) async fn post_setup_intent(
    State(state): State<AppState>,
    auth: AuthUser,
    headers: HeaderMap,
) -> AppResult<Json<SetupIntentResponse>> {
    let customer_id = resolve_member_stripe_customer(&state, auth.user_id).await?;
    // Forward the SPA-supplied Idempotency-Key (auto-attached by the
    // BFF client) so a retry of the modal "Add card" submit reuses the
    // existing SetupIntent rather than minting a second one.
    let idempotency_key = headers
        .get("idempotency-key")
        .and_then(|v| v.to_str().ok())
        .map(str::to_string);
    let client_secret =
        stripe_api::create_setup_intent(&state, &customer_id, idempotency_key.as_deref()).await?;
    Ok(Json(SetupIntentResponse { client_secret }))
}

#[utoipa::path(
    post,
    path = "/api/member/payment-methods/{pm_id}/set-default",
    tag = "member",
    security(("bearer_auth" = [])),
    params(("pm_id" = String, Path, description = "Stripe payment method id (`pm_*`)")),
    responses(
        (status = 200, description = "Default payment method updated", body = SetDefaultPaymentMethodResponse),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Payment method not found or not owned by the member"),
    )
)]
pub(crate) async fn post_set_default_payment_method(
    State(state): State<AppState>,
    auth: AuthUser,
    client: ClientInfo,
    Path(pm_id): Path<String>,
) -> AppResult<Json<SetDefaultPaymentMethodResponse>> {
    let customer_id = resolve_member_stripe_customer(&state, auth.user_id).await?;
    // Ownership probe: GET the PM and refuse if the `customer` field
    // doesn't match the caller's customer id. Returns 404 (not 403) so
    // a probing client can't enumerate `pm_*` ids belonging to others.
    let pm = stripe_api::get_payment_method(&state, &pm_id).await?;
    if pm.customer.as_deref() != Some(customer_id.as_str()) {
        return Err(AppError::NotFound("Payment method not found".to_string()));
    }
    stripe_api::set_default_payment_method(&state, &customer_id, &pm_id).await?;

    audit_admin_under_impersonation(
        &state.db,
        &auth,
        &client,
        "member.payment_method.set_default",
        "payment_method",
        pm_id.clone(),
        serde_json::json!({
            "self_service": true,
            "stripe_customer_id": customer_id,
        }),
    )
    .await;

    Ok(Json(SetDefaultPaymentMethodResponse {
        default_payment_method_id: pm_id,
    }))
}

#[utoipa::path(
    delete,
    path = "/api/member/payment-methods/{pm_id}",
    tag = "member",
    security(("bearer_auth" = [])),
    params(("pm_id" = String, Path, description = "Stripe payment method id (`pm_*`)")),
    responses(
        (status = 200, description = "Payment method detached", body = DeletePaymentMethodResponse),
        (status = 400, description = "Refused: card is the active subscription's default"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Payment method not found or not owned by the member"),
    )
)]
pub(crate) async fn delete_payment_method(
    State(state): State<AppState>,
    auth: AuthUser,
    client: ClientInfo,
    Path(pm_id): Path<String>,
) -> AppResult<Json<DeletePaymentMethodResponse>> {
    let customer_id = resolve_member_stripe_customer(&state, auth.user_id).await?;
    let pm = stripe_api::get_payment_method(&state, &pm_id).await?;
    if pm.customer.as_deref() != Some(customer_id.as_str()) {
        return Err(AppError::NotFound("Payment method not found".to_string()));
    }

    // Refuse to detach the default card while a subscription is active —
    // otherwise the next renewal silently fails. The member should set
    // another card as default first. We only check active/trialing
    // subscriptions; canceled ones can have their default detached
    // freely.
    let default_id = stripe_api::get_customer_default_payment_method(&state, &customer_id).await?;
    if default_id.as_deref() == Some(pm_id.as_str()) {
        let active_count: i64 = sqlx::query_scalar(
            r#"
            SELECT COUNT(*) FROM subscriptions
             WHERE user_id = $1
               AND status IN ('active', 'trialing')
            "#,
        )
        .bind(auth.user_id)
        .fetch_one(&state.db)
        .await?;
        if active_count > 0 {
            return Err(AppError::BadRequest(
                "Cannot remove the default payment method while a subscription is active. Set another card as default first.".to_string(),
            ));
        }
    }

    stripe_api::detach_payment_method(&state, &pm_id).await?;

    audit_admin_under_impersonation(
        &state.db,
        &auth,
        &client,
        "member.payment_method.delete",
        "payment_method",
        pm_id.clone(),
        serde_json::json!({
            "self_service": true,
            "stripe_customer_id": customer_id,
            "was_default": default_id.as_deref() == Some(pm_id.as_str()),
        }),
    )
    .await;

    Ok(Json(DeletePaymentMethodResponse { deleted: true }))
}
