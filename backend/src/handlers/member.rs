use axum::{
    extract::{Path, Query, State},
    routing::{get, post, put},
    Json, Router,
};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{
    db,
    error::{AppError, AppResult},
    extractors::AuthUser,
    models::*,
    stripe_api, AppState,
};

pub fn router() -> Router<AppState> {
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
