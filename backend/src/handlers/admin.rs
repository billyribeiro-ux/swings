use axum::{
    extract::{Path, Query, State},
    routing::{delete, get, post, put},
    Json, Router,
};
use uuid::Uuid;
use validator::Validate;

use crate::{
    db,
    error::{AppError, AppResult},
    extractors::AdminUser,
    models::*,
    AppState,
};

pub fn router() -> Router<AppState> {
    Router::new()
        // Dashboard
        .route("/stats", get(dashboard_stats))
        // Members
        .route("/members", get(list_members))
        .route("/members/{id}", get(get_member))
        .route("/members/{id}/role", put(update_member_role))
        .route("/members/{id}", delete(delete_member))
        // Watchlists
        .route("/watchlists", get(list_watchlists))
        .route("/watchlists", post(create_watchlist))
        .route("/watchlists/{id}", get(get_watchlist))
        .route("/watchlists/{id}", put(update_watchlist))
        .route("/watchlists/{id}", delete(delete_watchlist))
        // Watchlist Alerts
        .route("/watchlists/{id}/alerts", get(get_watchlist_alerts))
        .route("/watchlists/{id}/alerts", post(create_alert))
        .route("/alerts/{id}", put(update_alert))
        .route("/alerts/{id}", delete(delete_alert))
}

// ── Dashboard ───────────────────────────────────────────────────────────

async fn dashboard_stats(
    State(state): State<AppState>,
    _admin: AdminUser,
) -> AppResult<Json<AdminStats>> {
    let (users, total_members) = db::list_users(&state.db, 0, 1).await?;
    let _ = users;
    let active_subscriptions = db::count_active_subscriptions(&state.db).await?;
    let monthly = db::count_subscriptions_by_plan(&state.db, &SubscriptionPlan::Monthly).await?;
    let annual = db::count_subscriptions_by_plan(&state.db, &SubscriptionPlan::Annual).await?;
    let total_watchlists = db::count_watchlists(&state.db).await?;
    let total_enrollments = db::count_enrollments(&state.db).await?;
    let recent = db::recent_members(&state.db, 5).await?;

    Ok(Json(AdminStats {
        total_members,
        active_subscriptions,
        monthly_subscriptions: monthly,
        annual_subscriptions: annual,
        total_watchlists,
        total_enrollments,
        recent_members: recent.into_iter().map(UserResponse::from).collect(),
    }))
}

// ── Members ─────────────────────────────────────────────────────────────

async fn list_members(
    State(state): State<AppState>,
    _admin: AdminUser,
    Query(params): Query<PaginationParams>,
) -> AppResult<Json<PaginatedResponse<UserResponse>>> {
    let per_page = params.per_page();
    let offset = params.offset();
    let page = params.page.unwrap_or(1).max(1);

    let (users, total) = db::list_users(&state.db, offset, per_page).await?;
    let total_pages = (total as f64 / per_page as f64).ceil() as i64;

    Ok(Json(PaginatedResponse {
        data: users.into_iter().map(UserResponse::from).collect(),
        total,
        page,
        per_page,
        total_pages,
    }))
}

async fn get_member(
    State(state): State<AppState>,
    _admin: AdminUser,
    Path(id): Path<Uuid>,
) -> AppResult<Json<UserResponse>> {
    let user = db::find_user_by_id(&state.db, id)
        .await?
        .ok_or(AppError::NotFound("Member not found".to_string()))?;
    Ok(Json(user.into()))
}

#[derive(serde::Deserialize)]
struct RoleUpdate {
    role: UserRole,
}

async fn update_member_role(
    State(state): State<AppState>,
    _admin: AdminUser,
    Path(id): Path<Uuid>,
    Json(req): Json<RoleUpdate>,
) -> AppResult<Json<UserResponse>> {
    let user = db::update_user_role(&state.db, id, &req.role).await?;
    Ok(Json(user.into()))
}

async fn delete_member(
    State(state): State<AppState>,
    _admin: AdminUser,
    Path(id): Path<Uuid>,
) -> AppResult<Json<serde_json::Value>> {
    db::delete_user(&state.db, id).await?;
    Ok(Json(serde_json::json!({ "message": "Member deleted" })))
}

// ── Watchlists ──────────────────────────────────────────────────────────

async fn list_watchlists(
    State(state): State<AppState>,
    _admin: AdminUser,
    Query(params): Query<PaginationParams>,
) -> AppResult<Json<PaginatedResponse<Watchlist>>> {
    let per_page = params.per_page();
    let offset = params.offset();
    let page = params.page.unwrap_or(1).max(1);

    let (watchlists, total) = db::list_watchlists(&state.db, offset, per_page, false).await?;
    let total_pages = (total as f64 / per_page as f64).ceil() as i64;

    Ok(Json(PaginatedResponse {
        data: watchlists,
        total,
        page,
        per_page,
        total_pages,
    }))
}

async fn create_watchlist(
    State(state): State<AppState>,
    _admin: AdminUser,
    Json(req): Json<CreateWatchlistRequest>,
) -> AppResult<Json<Watchlist>> {
    req.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;

    let watchlist = db::create_watchlist(
        &state.db,
        &req.title,
        req.week_of,
        req.video_url.as_deref(),
        req.notes.as_deref(),
        req.published.unwrap_or(false),
    )
    .await?;

    Ok(Json(watchlist))
}

async fn get_watchlist(
    State(state): State<AppState>,
    _admin: AdminUser,
    Path(id): Path<Uuid>,
) -> AppResult<Json<WatchlistWithAlerts>> {
    let watchlist = db::get_watchlist(&state.db, id)
        .await?
        .ok_or(AppError::NotFound("Watchlist not found".to_string()))?;

    let alerts = db::get_alerts_for_watchlist(&state.db, id).await?;

    Ok(Json(WatchlistWithAlerts { watchlist, alerts }))
}

async fn update_watchlist(
    State(state): State<AppState>,
    _admin: AdminUser,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateWatchlistRequest>,
) -> AppResult<Json<Watchlist>> {
    let watchlist = db::update_watchlist(&state.db, id, &req).await?;
    Ok(Json(watchlist))
}

async fn delete_watchlist(
    State(state): State<AppState>,
    _admin: AdminUser,
    Path(id): Path<Uuid>,
) -> AppResult<Json<serde_json::Value>> {
    db::delete_watchlist(&state.db, id).await?;
    Ok(Json(serde_json::json!({ "message": "Watchlist deleted" })))
}

// ── Alerts ──────────────────────────────────────────────────────────────

async fn get_watchlist_alerts(
    State(state): State<AppState>,
    _admin: AdminUser,
    Path(id): Path<Uuid>,
) -> AppResult<Json<Vec<WatchlistAlert>>> {
    let alerts = db::get_alerts_for_watchlist(&state.db, id).await?;
    Ok(Json(alerts))
}

async fn create_alert(
    State(state): State<AppState>,
    _admin: AdminUser,
    Path(watchlist_id): Path<Uuid>,
    Json(req): Json<CreateAlertRequest>,
) -> AppResult<Json<WatchlistAlert>> {
    req.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;

    let alert = db::create_alert(&state.db, watchlist_id, &req).await?;
    Ok(Json(alert))
}

async fn update_alert(
    State(state): State<AppState>,
    _admin: AdminUser,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateAlertRequest>,
) -> AppResult<Json<WatchlistAlert>> {
    let alert = db::update_alert(&state.db, id, &req).await?;
    Ok(Json(alert))
}

async fn delete_alert(
    State(state): State<AppState>,
    _admin: AdminUser,
    Path(id): Path<Uuid>,
) -> AppResult<Json<serde_json::Value>> {
    db::delete_alert(&state.db, id).await?;
    Ok(Json(serde_json::json!({ "message": "Alert deleted" })))
}
