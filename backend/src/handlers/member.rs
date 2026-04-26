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
use chrono::Utc;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{
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
        // Subscription
        .route("/subscription", get(get_subscription))
        .route("/billing-portal", post(post_billing_portal))
        .route("/subscription/cancel", post(post_subscription_cancel))
        .route("/subscription/resume", post(post_subscription_resume))
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

    let updated = sqlx::query_as::<_, crate::models::User>(
        r#"UPDATE users SET
            name = $1, avatar_url = $2, bio = $3, position = $4,
            website_url = $5, twitter_url = $6, linkedin_url = $7,
            youtube_url = $8, instagram_url = $9, updated_at = NOW()
           WHERE id = $10 RETURNING *"#,
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
        match stripe_api::cancel_subscription_immediately(&state, &sub.stripe_subscription_id)
            .await
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

#[derive(Debug, serde::Serialize, ToSchema)]
pub struct ApplyCouponResponse {
    pub ok: bool,
    pub coupon_id: Uuid,
    pub applied_at: chrono::DateTime<Utc>,
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
    let mut tx = state.db.begin().await?;
    let usage: CouponUsage = sqlx::query_as(
        r#"
        INSERT INTO coupon_usages
            (id, coupon_id, user_id, subscription_id, discount_applied_cents, used_at)
        VALUES ($1, $2, $3, $4, 0, NOW())
        RETURNING *
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(coupon.id)
    .bind(auth.user_id)
    .bind(Some(subscription.id))
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

    Ok(Json(ApplyCouponResponse {
        ok: true,
        coupon_id: coupon.id,
        applied_at: usage.used_at,
    }))
}
