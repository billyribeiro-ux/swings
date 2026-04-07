use axum::{
    extract::{Path, Query, State},
    routing::{get, put},
    Json, Router,
};
use uuid::Uuid;

use crate::{
    db,
    error::{AppError, AppResult},
    extractors::AuthUser,
    models::*,
    AppState,
};

pub fn router() -> Router<AppState> {
    Router::new()
        // Profile
        .route("/profile", get(get_profile))
        .route("/profile", put(update_profile))
        // Subscription
        .route("/subscription", get(get_subscription))
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

#[derive(serde::Deserialize)]
struct UpdateProfileRequest {
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

async fn update_profile(
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
    let instagram_url = req.instagram_url.as_deref().or(user.instagram_url.as_deref());

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

#[derive(serde::Serialize)]
struct SubscriptionResponse {
    subscription: Option<Subscription>,
    is_active: bool,
}

async fn get_subscription(
    State(state): State<AppState>,
    auth: AuthUser,
) -> AppResult<Json<SubscriptionResponse>> {
    let sub = db::find_subscription_by_user(&state.db, auth.user_id).await?;
    let is_active = sub
        .as_ref()
        .map(|s| s.status == SubscriptionStatus::Active || s.status == SubscriptionStatus::Trialing)
        .unwrap_or(false);

    Ok(Json(SubscriptionResponse {
        subscription: sub,
        is_active,
    }))
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

#[derive(serde::Deserialize)]
struct ProgressUpdate {
    progress: i32,
}

async fn update_progress(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(course_id): Path<String>,
    Json(req): Json<ProgressUpdate>,
) -> AppResult<Json<CourseEnrollment>> {
    let enrollment =
        db::update_course_progress(&state.db, auth.user_id, &course_id, req.progress).await?;
    Ok(Json(enrollment))
}
