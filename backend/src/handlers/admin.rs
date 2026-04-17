use axum::{
    extract::{Path, Query, State},
    routing::{delete, get, post, put},
    Json, Router,
};
use chrono::{Duration, NaiveDate, NaiveTime};
use utoipa::ToSchema;
use uuid::Uuid;
use validator::Validate;

use crate::{
    db,
    error::{AppError, AppResult},
    extractors::AdminUser,
    models::*,
    stripe_api, AppState,
};

pub fn router() -> Router<AppState> {
    Router::new()
        // Dashboard
        .route("/stats", get(dashboard_stats))
        .route("/analytics/summary", get(analytics_summary))
        .route("/analytics/revenue", get(analytics_revenue))
        // Members
        .route("/members", get(list_members))
        .route("/members/{id}", get(get_member))
        .route(
            "/members/{id}/subscription",
            get(get_member_subscription_admin),
        )
        .route(
            "/members/{id}/billing-portal",
            post(admin_member_billing_portal),
        )
        .route(
            "/members/{id}/subscription/cancel",
            post(admin_member_subscription_cancel),
        )
        .route(
            "/members/{id}/subscription/resume",
            post(admin_member_subscription_resume),
        )
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

async fn analytics_summary(
    State(state): State<AppState>,
    _admin: AdminUser,
    Query(q): Query<AnalyticsSummaryQuery>,
) -> AppResult<Json<AnalyticsSummary>> {
    let from_date = NaiveDate::parse_from_str(&q.from, "%Y-%m-%d")
        .map_err(|_| AppError::BadRequest("invalid from date (use YYYY-MM-DD)".to_string()))?;
    let to_date = NaiveDate::parse_from_str(&q.to, "%Y-%m-%d")
        .map_err(|_| AppError::BadRequest("invalid to date (use YYYY-MM-DD)".to_string()))?;
    if to_date < from_date {
        return Err(AppError::BadRequest(
            "to must be on or after from".to_string(),
        ));
    }

    let start = from_date.and_time(NaiveTime::MIN).and_utc();
    let end_exclusive = (to_date + Duration::days(1))
        .and_time(NaiveTime::MIN)
        .and_utc();

    let (total_page_views, total_sessions, total_impressions) =
        db::analytics_totals(&state.db, start, end_exclusive).await?;

    let (bounced_sessions, bounce_eligible_sessions) =
        db::analytics_bounce_stats(&state.db, start, end_exclusive).await?;
    let bounce_rate = if bounce_eligible_sessions > 0 {
        bounced_sessions as f64 / bounce_eligible_sessions as f64
    } else {
        0.0
    };

    let (mrr_cents, arr_cents, active_subscribers) =
        db::admin_estimated_mrr_arr_cents(&state.db).await?;
    let period_revenue_cents =
        db::analytics_sales_revenue_total_cents(&state.db, start, end_exclusive).await?;

    let days = db::analytics_time_series(&state.db, start, end_exclusive).await?;
    let tops = db::analytics_top_pages(&state.db, start, end_exclusive, 25).await?;
    let ctr_rows = db::analytics_ctr_breakdown(&state.db, start, end_exclusive).await?;
    let recent_rows = db::analytics_recent_sales(&state.db, start, end_exclusive, 20).await?;

    let time_series = days
        .into_iter()
        .map(|d| AnalyticsTimeBucket {
            date: d.day.format("%Y-%m-%d").to_string(),
            page_views: d.page_views,
            unique_sessions: d.unique_sessions,
            impressions: d.impressions,
        })
        .collect();

    let top_pages = tops
        .into_iter()
        .map(|t| AnalyticsTopPage {
            path: t.path,
            views: t.views,
            sessions: t.sessions,
        })
        .collect();

    let ctr_series = ctr_rows
        .into_iter()
        .map(|r| {
            let ctr = if r.impressions > 0 {
                r.clicks as f64 / r.impressions as f64
            } else {
                0.0
            };
            AnalyticsCtrPoint {
                date: r.day.format("%Y-%m-%d").to_string(),
                cta_id: r.cta_id,
                impressions: r.impressions,
                clicks: r.clicks,
                ctr,
            }
        })
        .collect();

    let recent_sales = recent_rows
        .into_iter()
        .map(|r| AnalyticsRecentSale {
            id: r.id,
            event_type: r.event_type,
            amount_cents: r.amount_cents,
            user_email: r.user_email,
            created_at: r.created_at,
        })
        .collect();

    Ok(Json(AnalyticsSummary {
        from: q.from.clone(),
        to: q.to.clone(),
        total_page_views,
        total_sessions,
        total_impressions,
        bounced_sessions,
        bounce_eligible_sessions,
        bounce_rate,
        mrr_cents,
        arr_cents,
        active_subscribers,
        period_revenue_cents,
        time_series,
        top_pages,
        ctr_series,
        recent_sales,
    }))
}

async fn analytics_revenue(
    State(state): State<AppState>,
    _admin: AdminUser,
    Query(q): Query<AnalyticsSummaryQuery>,
) -> AppResult<Json<AdminRevenueResponse>> {
    let from_date = NaiveDate::parse_from_str(&q.from, "%Y-%m-%d")
        .map_err(|_| AppError::BadRequest("invalid from date (use YYYY-MM-DD)".to_string()))?;
    let to_date = NaiveDate::parse_from_str(&q.to, "%Y-%m-%d")
        .map_err(|_| AppError::BadRequest("invalid to date (use YYYY-MM-DD)".to_string()))?;
    if to_date < from_date {
        return Err(AppError::BadRequest(
            "to must be on or after from".to_string(),
        ));
    }

    let start = from_date.and_time(NaiveTime::MIN).and_utc();
    let end_exclusive = (to_date + Duration::days(1))
        .and_time(NaiveTime::MIN)
        .and_utc();

    let rows = db::analytics_sales_revenue_daily(&state.db, start, end_exclusive).await?;
    let data = rows
        .into_iter()
        .map(|r| DailyRevenuePoint {
            date: r.day.format("%Y-%m-%d").to_string(),
            revenue_cents: r.revenue_cents,
        })
        .collect();

    Ok(Json(AdminRevenueResponse { data }))
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

async fn get_member_subscription_admin(
    State(state): State<AppState>,
    _admin: AdminUser,
    Path(user_id): Path<Uuid>,
) -> AppResult<Json<SubscriptionStatusResponse>> {
    db::find_user_by_id(&state.db, user_id)
        .await?
        .ok_or(AppError::NotFound("Member not found".to_string()))?;

    let sub = db::find_subscription_by_user(&state.db, user_id).await?;
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
    path = "/api/admin/members/{id}/billing-portal",
    tag = "admin",
    security(("bearer_auth" = [])),
    params(("id" = Uuid, Path, description = "Member id")),
    request_body = BillingPortalRequest,
    responses(
        (status = 200, description = "Stripe billing portal URL", body = BillingPortalResponse),
        (status = 400, description = "Member has no subscription"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Member not found")
    )
)]
pub(crate) async fn admin_member_billing_portal(
    State(state): State<AppState>,
    _admin: AdminUser,
    Path(user_id): Path<Uuid>,
    Json(req): Json<BillingPortalRequest>,
) -> AppResult<Json<BillingPortalResponse>> {
    db::find_user_by_id(&state.db, user_id)
        .await?
        .ok_or(AppError::NotFound("Member not found".to_string()))?;

    let sub = db::find_subscription_by_user(&state.db, user_id)
        .await?
        .ok_or_else(|| AppError::BadRequest("Member has no subscription".to_string()))?;

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
    path = "/api/admin/members/{id}/subscription/cancel",
    tag = "admin",
    security(("bearer_auth" = [])),
    params(("id" = Uuid, Path, description = "Member id")),
    responses(
        (status = 200, description = "Subscription scheduled to cancel at period end"),
        (status = 400, description = "Member has no subscription"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Member not found")
    )
)]
pub(crate) async fn admin_member_subscription_cancel(
    State(state): State<AppState>,
    _admin: AdminUser,
    Path(user_id): Path<Uuid>,
) -> AppResult<Json<serde_json::Value>> {
    db::find_user_by_id(&state.db, user_id)
        .await?
        .ok_or(AppError::NotFound("Member not found".to_string()))?;

    let sub = db::find_subscription_by_user(&state.db, user_id)
        .await?
        .ok_or_else(|| AppError::BadRequest("Member has no subscription".to_string()))?;

    stripe_api::set_subscription_cancel_at_period_end(&state, &sub.stripe_subscription_id, true)
        .await?;

    Ok(Json(
        serde_json::json!({ "ok": true, "cancel_at_period_end": true }),
    ))
}

#[utoipa::path(
    post,
    path = "/api/admin/members/{id}/subscription/resume",
    tag = "admin",
    security(("bearer_auth" = [])),
    params(("id" = Uuid, Path, description = "Member id")),
    responses(
        (status = 200, description = "Subscription cancellation reversed"),
        (status = 400, description = "Member has no subscription"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Member not found")
    )
)]
pub(crate) async fn admin_member_subscription_resume(
    State(state): State<AppState>,
    _admin: AdminUser,
    Path(user_id): Path<Uuid>,
) -> AppResult<Json<serde_json::Value>> {
    db::find_user_by_id(&state.db, user_id)
        .await?
        .ok_or(AppError::NotFound("Member not found".to_string()))?;

    let sub = db::find_subscription_by_user(&state.db, user_id)
        .await?
        .ok_or_else(|| AppError::BadRequest("Member has no subscription".to_string()))?;

    stripe_api::set_subscription_cancel_at_period_end(&state, &sub.stripe_subscription_id, false)
        .await?;

    Ok(Json(
        serde_json::json!({ "ok": true, "cancel_at_period_end": false }),
    ))
}

#[derive(serde::Deserialize, ToSchema)]
pub struct RoleUpdate {
    role: UserRole,
}

#[utoipa::path(
    put,
    path = "/api/admin/members/{id}/role",
    tag = "admin",
    security(("bearer_auth" = [])),
    params(("id" = Uuid, Path, description = "Member id")),
    request_body = RoleUpdate,
    responses(
        (status = 200, description = "Role updated", body = UserResponse),
        (status = 403, description = "Forbidden")
    )
)]
pub(crate) async fn update_member_role(
    State(state): State<AppState>,
    _admin: AdminUser,
    Path(id): Path<Uuid>,
    Json(req): Json<RoleUpdate>,
) -> AppResult<Json<UserResponse>> {
    let user = db::update_user_role(&state.db, id, &req.role).await?;
    Ok(Json(user.into()))
}

#[utoipa::path(
    delete,
    path = "/api/admin/members/{id}",
    tag = "admin",
    security(("bearer_auth" = [])),
    params(("id" = Uuid, Path, description = "Member id")),
    responses(
        (status = 200, description = "Member deleted"),
        (status = 403, description = "Forbidden")
    )
)]
pub(crate) async fn delete_member(
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

#[utoipa::path(
    post,
    path = "/api/admin/watchlists",
    tag = "admin",
    security(("bearer_auth" = [])),
    request_body = CreateWatchlistRequest,
    responses(
        (status = 200, description = "Watchlist created", body = Watchlist),
        (status = 403, description = "Forbidden"),
        (status = 422, description = "Validation error")
    )
)]
pub(crate) async fn create_watchlist(
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

#[utoipa::path(
    put,
    path = "/api/admin/watchlists/{id}",
    tag = "admin",
    security(("bearer_auth" = [])),
    params(("id" = Uuid, Path, description = "Watchlist id")),
    request_body = UpdateWatchlistRequest,
    responses(
        (status = 200, description = "Watchlist updated", body = Watchlist),
        (status = 403, description = "Forbidden")
    )
)]
pub(crate) async fn update_watchlist(
    State(state): State<AppState>,
    _admin: AdminUser,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateWatchlistRequest>,
) -> AppResult<Json<Watchlist>> {
    let watchlist = db::update_watchlist(&state.db, id, &req).await?;
    Ok(Json(watchlist))
}

#[utoipa::path(
    delete,
    path = "/api/admin/watchlists/{id}",
    tag = "admin",
    security(("bearer_auth" = [])),
    params(("id" = Uuid, Path, description = "Watchlist id")),
    responses(
        (status = 200, description = "Watchlist deleted"),
        (status = 403, description = "Forbidden")
    )
)]
pub(crate) async fn delete_watchlist(
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

#[utoipa::path(
    post,
    path = "/api/admin/watchlists/{id}/alerts",
    tag = "admin",
    security(("bearer_auth" = [])),
    params(("id" = Uuid, Path, description = "Watchlist id")),
    request_body = CreateAlertRequest,
    responses(
        (status = 200, description = "Alert created", body = WatchlistAlert),
        (status = 403, description = "Forbidden"),
        (status = 422, description = "Validation error")
    )
)]
pub(crate) async fn create_alert(
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

#[utoipa::path(
    put,
    path = "/api/admin/alerts/{id}",
    tag = "admin",
    security(("bearer_auth" = [])),
    params(("id" = Uuid, Path, description = "Alert id")),
    request_body = UpdateAlertRequest,
    responses(
        (status = 200, description = "Alert updated", body = WatchlistAlert),
        (status = 403, description = "Forbidden")
    )
)]
pub(crate) async fn update_alert(
    State(state): State<AppState>,
    _admin: AdminUser,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateAlertRequest>,
) -> AppResult<Json<WatchlistAlert>> {
    let alert = db::update_alert(&state.db, id, &req).await?;
    Ok(Json(alert))
}

#[utoipa::path(
    delete,
    path = "/api/admin/alerts/{id}",
    tag = "admin",
    security(("bearer_auth" = [])),
    params(("id" = Uuid, Path, description = "Alert id")),
    responses(
        (status = 200, description = "Alert deleted"),
        (status = 403, description = "Forbidden")
    )
)]
pub(crate) async fn delete_alert(
    State(state): State<AppState>,
    _admin: AdminUser,
    Path(id): Path<Uuid>,
) -> AppResult<Json<serde_json::Value>> {
    db::delete_alert(&state.db, id).await?;
    Ok(Json(serde_json::json!({ "message": "Alert deleted" })))
}
