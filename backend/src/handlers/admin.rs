use axum::{
    extract::{Path, Query, State},
    routing::{delete, get, patch, post, put},
    Json, Router,
};
use chrono::{DateTime, Datelike, Duration, NaiveDate, NaiveTime, TimeZone, Utc};
use utoipa::ToSchema;
use uuid::Uuid;
use validator::Validate;

use crate::{
    db::{self, UserStatusFilter},
    error::{AppError, AppResult},
    extractors::{AdminUser, ClientInfo, PrivilegedUser},
    models::*,
    services::audit::{audit_admin_priv, record_admin_action_best_effort, AdminAction},
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
        .route("/members/{id}", patch(update_member_profile))
        .route("/members/{id}/detail", get(member_detail))
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

/// Resolve a [`DashboardRange`] (plus optional `from`/`to` inclusive `YYYY-MM-DD`
/// strings) into a half-open UTC window `[start, end_exclusive)`.
///
/// Anchors `now = Utc::now()` once so every metric on the same response sees a
/// consistent endpoint — without this, two consecutive `Utc::now()` reads can
/// fall on opposite sides of a day boundary mid-request and cause the deltas
/// to disagree with the totals by one row.
fn resolve_dashboard_window(
    range: DashboardRange,
    from: Option<&str>,
    to: Option<&str>,
) -> AppResult<(DateTime<Utc>, DateTime<Utc>)> {
    let now = Utc::now();
    match range {
        DashboardRange::Last7Days => Ok((now - Duration::days(7), now)),
        DashboardRange::Last30Days => Ok((now - Duration::days(30), now)),
        DashboardRange::Last90Days => Ok((now - Duration::days(90), now)),
        DashboardRange::YearToDate => {
            let start = Utc
                .with_ymd_and_hms(now.year(), 1, 1, 0, 0, 0)
                .single()
                .ok_or_else(|| {
                    AppError::Internal(anyhow::anyhow!("failed to resolve start of UTC year"))
                })?;
            Ok((start, now))
        }
        DashboardRange::Custom => {
            let from_str = from.ok_or_else(|| {
                AppError::BadRequest("range=custom requires `from` (YYYY-MM-DD)".to_string())
            })?;
            let to_str = to.ok_or_else(|| {
                AppError::BadRequest("range=custom requires `to` (YYYY-MM-DD)".to_string())
            })?;
            let from_date = NaiveDate::parse_from_str(from_str, "%Y-%m-%d").map_err(|_| {
                AppError::BadRequest("invalid from date (use YYYY-MM-DD)".to_string())
            })?;
            let to_date = NaiveDate::parse_from_str(to_str, "%Y-%m-%d").map_err(|_| {
                AppError::BadRequest("invalid to date (use YYYY-MM-DD)".to_string())
            })?;
            if to_date < from_date {
                return Err(AppError::BadRequest(
                    "to must be on or after from".to_string(),
                ));
            }
            let start = from_date.and_time(NaiveTime::MIN).and_utc();
            let end_exclusive = (to_date + Duration::days(1))
                .and_time(NaiveTime::MIN)
                .and_utc();
            Ok((start, end_exclusive))
        }
    }
}

async fn collect_period_window(
    state: &AppState,
    start: DateTime<Utc>,
    end_exclusive: DateTime<Utc>,
) -> AppResult<PeriodWindow> {
    Ok(PeriodWindow {
        new_members: db::count_users_created_between(&state.db, start, end_exclusive).await?,
        new_subscriptions: db::count_subscriptions_created_between(&state.db, start, end_exclusive)
            .await?,
        canceled_subscriptions: db::count_subscriptions_canceled_between(
            &state.db,
            start,
            end_exclusive,
        )
        .await?,
        new_enrollments: db::count_enrollments_created_between(&state.db, start, end_exclusive)
            .await?,
        new_watchlists: db::count_watchlists_created_between(&state.db, start, end_exclusive)
            .await?,
        revenue_cents: db::analytics_sales_revenue_total_cents(&state.db, start, end_exclusive)
            .await?,
    })
}

async fn dashboard_stats(
    State(state): State<AppState>,
    _admin: AdminUser,
    Query(q): Query<DashboardStatsQuery>,
) -> AppResult<Json<AdminStats>> {
    // Lifetime totals — these mirror what the dashboard "as-of-now" KPIs
    // need; the period-scoped block below adds the deltas.
    let (users, total_members) = db::list_users(&state.db, 0, 1).await?;
    let _ = users;
    let active_subscriptions = db::count_active_subscriptions(&state.db).await?;
    let monthly = db::count_subscriptions_by_plan(&state.db, &SubscriptionPlan::Monthly).await?;
    let annual = db::count_subscriptions_by_plan(&state.db, &SubscriptionPlan::Annual).await?;
    let total_watchlists = db::count_watchlists(&state.db).await?;
    let total_enrollments = db::count_enrollments(&state.db).await?;
    let recent = db::recent_members(&state.db, 5).await?;

    // Range-scoped block.
    let range = q.range.unwrap_or_default();
    let (from_ts, to_ts) = resolve_dashboard_window(range, q.from.as_deref(), q.to.as_deref())?;
    let window_len = to_ts - from_ts;
    let prev_to = from_ts;
    let prev_from = from_ts - window_len;

    let period = collect_period_window(&state, from_ts, to_ts).await?;
    let previous_period = collect_period_window(&state, prev_from, prev_to).await?;

    Ok(Json(AdminStats {
        total_members,
        active_subscriptions,
        monthly_subscriptions: monthly,
        annual_subscriptions: annual,
        total_watchlists,
        total_enrollments,
        recent_members: recent.into_iter().map(UserResponse::from).collect(),
        range,
        from: from_ts,
        to: to_ts,
        period,
        previous_period,
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

/// Query string accepted by `GET /api/admin/members`.
///
/// Backwards-compatible with the original `page` / `per_page` shape;
/// adds optional `search`, `role`, and `status` filters that route
/// through [`db::search_users`]. The list page on the SPA uses the same
/// extractor so a GET without any new params behaves exactly as it did
/// before this change.
#[derive(Debug, serde::Deserialize, ToSchema)]
pub struct ListMembersQuery {
    #[serde(default)]
    pub page: Option<i64>,
    #[serde(default)]
    pub per_page: Option<i64>,
    #[serde(default)]
    pub search: Option<String>,
    #[serde(default)]
    pub role: Option<String>,
    #[serde(default)]
    pub status: Option<String>,
}

async fn list_members(
    State(state): State<AppState>,
    _admin: AdminUser,
    Query(q): Query<ListMembersQuery>,
) -> AppResult<Json<PaginatedResponse<UserResponse>>> {
    let per_page = q.per_page.unwrap_or(20).clamp(1, 100);
    let page = q.page.unwrap_or(1).max(1);
    let offset = (page - 1) * per_page;

    let role_filter = match q.role.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        None => None,
        Some(r) => Some(
            UserRole::from_str_lower(&r.to_lowercase())
                .ok_or_else(|| AppError::BadRequest(format!("unknown role: {r}")))?,
        ),
    };
    let status_filter = match q.status.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        None => None,
        Some(s) => Some(
            UserStatusFilter::from_wire(&s.to_lowercase())
                .ok_or_else(|| AppError::BadRequest(format!("unknown status: {s}")))?,
        ),
    };

    let any_filter = q
        .search
        .as_deref()
        .map(str::trim)
        .is_some_and(|s| !s.is_empty())
        || role_filter.is_some()
        || status_filter.is_some();

    let (users, total) = if any_filter {
        db::search_users(
            &state.db,
            q.search.as_deref(),
            role_filter.as_ref(),
            status_filter,
            per_page,
            offset,
        )
        .await?
    } else {
        db::list_users(&state.db, offset, per_page).await?
    };

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
    admin: AdminUser,
    client: ClientInfo,
    Path(user_id): Path<Uuid>,
    Json(req): Json<BillingPortalRequest>,
) -> AppResult<Json<BillingPortalResponse>> {
    admin.require(&state.policy, "admin.member.billing_portal")?;

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

    let actor_role = UserRole::from_str_lower(&admin.role).unwrap_or(UserRole::Admin);
    record_admin_action_best_effort(
        &state.db,
        AdminAction::new(
            admin.user_id,
            actor_role,
            "subscription.billing_portal.issue",
            "subscription",
        )
        .with_target_id(sub.stripe_customer_id.clone())
        .with_client(&client)
        .with_metadata(serde_json::json!({
            "user_id": user_id,
            "return_url": return_url,
        })),
    )
    .await;

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
    admin: AdminUser,
    client: ClientInfo,
    Path(user_id): Path<Uuid>,
) -> AppResult<Json<serde_json::Value>> {
    admin.require(&state.policy, "admin.member.subscription.manage")?;

    db::find_user_by_id(&state.db, user_id)
        .await?
        .ok_or(AppError::NotFound("Member not found".to_string()))?;

    let sub = db::find_subscription_by_user(&state.db, user_id)
        .await?
        .ok_or_else(|| AppError::BadRequest("Member has no subscription".to_string()))?;

    stripe_api::set_subscription_cancel_at_period_end(&state, &sub.stripe_subscription_id, true)
        .await?;

    let actor_role = UserRole::from_str_lower(&admin.role).unwrap_or(UserRole::Admin);
    record_admin_action_best_effort(
        &state.db,
        AdminAction::new(
            admin.user_id,
            actor_role,
            "subscription.cancel_at_period_end",
            "subscription",
        )
        .with_target_id(sub.stripe_subscription_id.clone())
        .with_client(&client)
        .with_metadata(serde_json::json!({
            "user_id": user_id,
            "cancel_at_period_end": true,
        })),
    )
    .await;

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
    admin: AdminUser,
    client: ClientInfo,
    Path(user_id): Path<Uuid>,
) -> AppResult<Json<serde_json::Value>> {
    admin.require(&state.policy, "admin.member.subscription.manage")?;

    db::find_user_by_id(&state.db, user_id)
        .await?
        .ok_or(AppError::NotFound("Member not found".to_string()))?;

    let sub = db::find_subscription_by_user(&state.db, user_id)
        .await?
        .ok_or_else(|| AppError::BadRequest("Member has no subscription".to_string()))?;

    stripe_api::set_subscription_cancel_at_period_end(&state, &sub.stripe_subscription_id, false)
        .await?;

    let actor_role = UserRole::from_str_lower(&admin.role).unwrap_or(UserRole::Admin);
    record_admin_action_best_effort(
        &state.db,
        AdminAction::new(
            admin.user_id,
            actor_role,
            "subscription.resume",
            "subscription",
        )
        .with_target_id(sub.stripe_subscription_id.clone())
        .with_client(&client)
        .with_metadata(serde_json::json!({
            "user_id": user_id,
            "cancel_at_period_end": false,
        })),
    )
    .await;

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
    admin: AdminUser,
    client: ClientInfo,
    Path(id): Path<Uuid>,
    Json(req): Json<RoleUpdate>,
) -> AppResult<Json<UserResponse>> {
    admin.require(&state.policy, "admin.member.role.update")?;

    // Capture the prior role for the audit trail so a privilege escalation
    // can be reconstructed from the log alone (without the actor having to
    // carry the previous state separately).
    let prior_role = db::find_user_by_id(&state.db, id).await?.map(|u| u.role);

    // Demote-last-admin guard: refuse to change the role of the only
    // remaining `admin` user away from `admin`. Mirrors the self-lock
    // pattern in `admin_roles::replace_role_permissions`. Without this,
    // the matrix can be left with zero admins and recovery requires a
    // SQL console.
    if matches!(prior_role, Some(UserRole::Admin)) && req.role != UserRole::Admin {
        let admin_count = db::count_admins(&state.db).await?;
        if admin_count <= 1 {
            return Err(AppError::Conflict(
                "refusing to demote the last admin — promote another user to admin first"
                    .to_string(),
            ));
        }
        // Self-lock: an actor cannot demote *themselves* even when other
        // admins exist, because the demotion would invalidate the JWT
        // mid-request and leave the session in an inconsistent state on
        // the next call. The actor must ask another admin to do it.
        if id == admin.user_id {
            return Err(AppError::Conflict(
                "refusing to self-demote — ask another admin to perform the change".to_string(),
            ));
        }
    }

    let user = db::update_user_role(&state.db, id, &req.role).await?;

    let actor_role = UserRole::from_str_lower(&admin.role).unwrap_or(UserRole::Admin);
    record_admin_action_best_effort(
        &state.db,
        AdminAction::new(admin.user_id, actor_role, "user.role.update", "user")
            .with_target_id(id.to_string())
            .with_client(&client)
            .with_metadata(serde_json::json!({
                "from_role": prior_role,
                "to_role": req.role,
            })),
    )
    .await;

    Ok(Json(user.into()))
}

#[utoipa::path(
    patch,
    path = "/api/admin/members/{id}",
    tag = "admin",
    security(("bearer_auth" = [])),
    params(("id" = Uuid, Path, description = "Member id")),
    request_body = UpdateMemberRequest,
    responses(
        (status = 200, description = "Member profile updated", body = UserResponse),
        (status = 400, description = "Validation failed"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Member not found"),
        (status = 409, description = "Email already in use")
    )
)]
pub(crate) async fn update_member_profile(
    State(state): State<AppState>,
    admin: PrivilegedUser,
    client: ClientInfo,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateMemberRequest>,
) -> AppResult<Json<UserResponse>> {
    admin.require(&state.policy, "admin.member.update")?;

    let existing = db::find_user_by_id(&state.db, id)
        .await?
        .ok_or_else(|| AppError::NotFound("Member not found".to_string()))?;

    // Validate the input bundle. We rely on the lightweight `validator`
    // crate for trivial checks and do email-uniqueness in a follow-up
    // query so the conflict error is distinct from a 400.
    if let Some(name) = req.name.as_deref() {
        let trimmed = name.trim();
        if trimmed.is_empty() || trimmed.chars().count() > 200 {
            return Err(AppError::BadRequest(
                "name must be 1-200 characters".to_string(),
            ));
        }
    }
    let normalised_email = if let Some(email) = req.email.as_deref() {
        let trimmed = email.trim();
        if !trimmed.contains('@') || trimmed.len() < 3 || trimmed.len() > 320 {
            return Err(AppError::BadRequest("invalid email address".to_string()));
        }
        let normalised = db::normalize_email(trimmed);
        if normalised != existing.email {
            if let Some(other) = db::find_user_by_email(&state.db, &normalised).await? {
                if other.id != existing.id {
                    return Err(AppError::Conflict(
                        "Another account already uses that email".to_string(),
                    ));
                }
            }
        }
        Some(normalised)
    } else {
        None
    };

    if let Some(phone) = req.phone.as_deref() {
        let trimmed = phone.trim();
        if !trimmed.is_empty() && (trimmed.len() < 4 || trimmed.len() > 32) {
            return Err(AppError::BadRequest(
                "phone must be 4-32 characters when provided".to_string(),
            ));
        }
    }

    let address_was_provided = req.billing_address.is_some();
    let address = req.billing_address.unwrap_or_default();
    let country_normalised = match address.country.as_deref().map(str::trim) {
        Some(c) if !c.is_empty() => {
            if c.len() != 2 || !c.chars().all(|ch| ch.is_ascii_alphabetic()) {
                return Err(AppError::BadRequest(
                    "country must be ISO 3166-1 alpha-2 (e.g. \"US\")".to_string(),
                ));
            }
            Some(c.to_uppercase())
        }
        _ => None,
    };

    // Email change forces re-verification — the operator can flip it
    // back on via the dedicated `verify-email` endpoint after they've
    // confirmed the new address belongs to the account holder.
    let verified_change = if normalised_email
        .as_deref()
        .is_some_and(|e| e != existing.email)
    {
        Some(None)
    } else {
        None
    };

    let updated = db::update_member_profile(
        &state.db,
        id,
        req.name.as_deref().map(str::trim),
        normalised_email.as_deref(),
        req.phone.as_deref().map(str::trim),
        address.line1.as_deref().map(str::trim),
        address.line2.as_deref().map(str::trim),
        address.city.as_deref().map(str::trim),
        address.state.as_deref().map(str::trim),
        address.postal_code.as_deref().map(str::trim),
        country_normalised.as_deref(),
        address_was_provided,
        verified_change,
    )
    .await?;

    // Best-effort: keep Stripe Customer's address aligned if the member
    // has one. The local row is already saved — a Stripe outage cannot
    // roll it back.
    let mut stripe_outcome = serde_json::json!(null);
    if address_was_provided || req.phone.is_some() {
        if let Some(sub) = db::find_subscription_by_user(&state.db, id).await? {
            match stripe_api::update_customer_address(
                &state,
                &sub.stripe_customer_id,
                address.line1.as_deref().map(str::trim),
                address.line2.as_deref().map(str::trim),
                address.city.as_deref().map(str::trim),
                address.state.as_deref().map(str::trim),
                address.postal_code.as_deref().map(str::trim),
                country_normalised.as_deref(),
                req.phone.as_deref().map(str::trim),
            )
            .await
            {
                Ok(()) => {
                    stripe_outcome = serde_json::json!({ "result": "synced" });
                }
                Err(e) => {
                    tracing::warn!(
                        user_id = %id,
                        customer_id = %sub.stripe_customer_id,
                        error = %e,
                        "stripe customer.update failed; local profile saved regardless"
                    );
                    stripe_outcome = serde_json::json!({
                        "result": "stripe_error",
                        "error":  e.to_string(),
                    });
                }
            }
        }
    }

    audit_admin_priv(
        &state.db,
        &admin,
        &client,
        "user.update",
        "user",
        id.to_string(),
        serde_json::json!({
            "name_changed":    req.name.is_some(),
            "email_changed":   normalised_email.as_deref().is_some_and(|e| e != existing.email),
            "phone_changed":   req.phone.is_some(),
            "address_changed": address_was_provided,
            "stripe_sync":     stripe_outcome,
        }),
    )
    .await;

    Ok(Json(updated.into()))
}

#[utoipa::path(
    get,
    path = "/api/admin/members/{id}/detail",
    tag = "admin",
    security(("bearer_auth" = [])),
    params(("id" = Uuid, Path, description = "Member id")),
    responses(
        (status = 200, description = "Composite member detail", body = MemberDetailResponse),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Member not found")
    )
)]
pub(crate) async fn member_detail(
    State(state): State<AppState>,
    admin: PrivilegedUser,
    Path(id): Path<Uuid>,
) -> AppResult<Json<MemberDetailResponse>> {
    admin.require(&state.policy, "admin.member.read")?;

    let user = db::find_user_by_id(&state.db, id)
        .await?
        .ok_or_else(|| AppError::NotFound("Member not found".to_string()))?;
    let subscription = db::find_subscription_by_user(&state.db, id).await?;
    // 20 is enough to surface the recent-actions strip on the page
    // without forcing pagination; the dedicated audit viewer picks up
    // anything older.
    let activity = db::recent_admin_actions_for_user(&state.db, id, 20).await?;
    let payment_failures = db::recent_payment_failures_for_user(&state.db, id, 10).await?;

    Ok(Json(MemberDetailResponse {
        user: user.into(),
        subscription,
        activity,
        payment_failures,
    }))
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
    admin: AdminUser,
    client: ClientInfo,
    Path(id): Path<Uuid>,
) -> AppResult<Json<serde_json::Value>> {
    admin.require(&state.policy, "admin.member.delete")?;

    // Snapshot identity before the row disappears so the audit log keeps a
    // human-readable record (admin_actions has no FK to users; deletes
    // wouldn't cascade through the audit table — that's intentional).
    let snapshot = db::find_user_by_id(&state.db, id).await?;

    // Don't let the actor delete themselves: the JWT survives the row
    // drop and would loop on every subsequent request as the per-request
    // lifecycle gate fails to find the user.
    if id == admin.user_id {
        return Err(AppError::Conflict(
            "refusing to self-delete — ask another admin to perform the deletion".to_string(),
        ));
    }

    // Refuse to delete the last admin row for the same reason
    // `update_member_role` refuses to demote it: zero-admin recovery
    // requires a SQL console.
    if matches!(snapshot.as_ref().map(|u| u.role), Some(UserRole::Admin)) {
        let admin_count = db::count_admins(&state.db).await?;
        if admin_count <= 1 {
            return Err(AppError::Conflict(
                "refusing to delete the last admin — promote another user to admin first"
                    .to_string(),
            ));
        }
    }

    // ADM-15: cancel any active Stripe subscription before tearing the
    // user row down — leaving it open would orphan the recurring charge
    // since the local row goes away. Failures are logged but never
    // block the delete.
    let mut stripe_outcome = serde_json::json!(null);
    if let Some(sub) = db::find_subscription_by_user(&state.db, id).await? {
        match stripe_api::cancel_subscription_immediately(&state, &sub.stripe_subscription_id).await
        {
            Ok(()) => {
                stripe_outcome = serde_json::json!({
                    "stripe_subscription_id": sub.stripe_subscription_id,
                    "result": "canceled",
                });
            }
            Err(e) => {
                tracing::warn!(
                    user_id = %id,
                    subscription_id = %sub.stripe_subscription_id,
                    error = %e,
                    "stripe immediate-cancel failed during delete; row dropped regardless"
                );
                stripe_outcome = serde_json::json!({
                    "stripe_subscription_id": sub.stripe_subscription_id,
                    "result": "stripe_error",
                    "error":  e.to_string(),
                });
            }
        }
    }

    db::delete_user(&state.db, id).await?;

    let actor_role = UserRole::from_str_lower(&admin.role).unwrap_or(UserRole::Admin);
    record_admin_action_best_effort(
        &state.db,
        AdminAction::new(admin.user_id, actor_role, "user.delete", "user")
            .with_target_id(id.to_string())
            .with_client(&client)
            .with_metadata(serde_json::json!({
                "email": snapshot.as_ref().map(|u| u.email.clone()),
                "role": snapshot.as_ref().map(|u| u.role),
                "stripe_cancel": stripe_outcome,
            })),
    )
    .await;

    Ok(Json(serde_json::json!({ "message": "Member deleted" })))
}

// ── Watchlists ──────────────────────────────────────────────────────────

async fn list_watchlists(
    State(state): State<AppState>,
    admin: AdminUser,
    Query(params): Query<PaginationParams>,
) -> AppResult<Json<PaginatedResponse<Watchlist>>> {
    admin.require(&state.policy, "admin.watchlist.read")?;

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
    admin: AdminUser,
    client: ClientInfo,
    Json(req): Json<CreateWatchlistRequest>,
) -> AppResult<Json<Watchlist>> {
    admin.require(&state.policy, "admin.watchlist.manage")?;

    req.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;

    let published = req.published.unwrap_or(false);
    let watchlist = db::create_watchlist(
        &state.db,
        &req.title,
        req.week_of,
        req.video_url.as_deref(),
        req.notes.as_deref(),
        published,
    )
    .await?;

    let actor_role = UserRole::from_str_lower(&admin.role).unwrap_or(UserRole::Admin);
    record_admin_action_best_effort(
        &state.db,
        AdminAction::new(admin.user_id, actor_role, "watchlist.create", "watchlist")
            .with_target_id(watchlist.id.to_string())
            .with_client(&client)
            .with_metadata(serde_json::json!({
                "title": watchlist.title,
                "week_of": watchlist.week_of,
                "published": published,
            })),
    )
    .await;

    Ok(Json(watchlist))
}

async fn get_watchlist(
    State(state): State<AppState>,
    admin: AdminUser,
    Path(id): Path<Uuid>,
) -> AppResult<Json<WatchlistWithAlerts>> {
    admin.require(&state.policy, "admin.watchlist.read")?;

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
    admin: AdminUser,
    client: ClientInfo,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateWatchlistRequest>,
) -> AppResult<Json<Watchlist>> {
    admin.require(&state.policy, "admin.watchlist.manage")?;

    let watchlist = db::update_watchlist(&state.db, id, &req).await?;

    let actor_role = UserRole::from_str_lower(&admin.role).unwrap_or(UserRole::Admin);
    record_admin_action_best_effort(
        &state.db,
        AdminAction::new(admin.user_id, actor_role, "watchlist.update", "watchlist")
            .with_target_id(id.to_string())
            .with_client(&client)
            .with_metadata(serde_json::json!({
                "title_changed":     req.title.is_some(),
                "week_of_changed":   req.week_of.is_some(),
                "video_url_changed": req.video_url.is_some(),
                "notes_changed":     req.notes.is_some(),
                "published_changed": req.published.is_some(),
            })),
    )
    .await;

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
    admin: AdminUser,
    client: ClientInfo,
    Path(id): Path<Uuid>,
) -> AppResult<Json<serde_json::Value>> {
    admin.require(&state.policy, "admin.watchlist.manage")?;

    // Snapshot before delete so the audit row carries title/week_of for
    // post-incident review even after the row is gone.
    let snapshot = db::get_watchlist(&state.db, id).await?;
    db::delete_watchlist(&state.db, id).await?;

    let actor_role = UserRole::from_str_lower(&admin.role).unwrap_or(UserRole::Admin);
    record_admin_action_best_effort(
        &state.db,
        AdminAction::new(admin.user_id, actor_role, "watchlist.delete", "watchlist")
            .with_target_id(id.to_string())
            .with_client(&client)
            .with_metadata(serde_json::json!({
                "title":   snapshot.as_ref().map(|w| w.title.clone()),
                "week_of": snapshot.as_ref().map(|w| w.week_of),
            })),
    )
    .await;

    Ok(Json(serde_json::json!({ "message": "Watchlist deleted" })))
}

// ── Alerts ──────────────────────────────────────────────────────────────

async fn get_watchlist_alerts(
    State(state): State<AppState>,
    admin: AdminUser,
    Path(id): Path<Uuid>,
) -> AppResult<Json<Vec<WatchlistAlert>>> {
    admin.require(&state.policy, "admin.watchlist.alert.read")?;

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
    admin: AdminUser,
    client: ClientInfo,
    Path(watchlist_id): Path<Uuid>,
    Json(req): Json<CreateAlertRequest>,
) -> AppResult<Json<WatchlistAlert>> {
    admin.require(&state.policy, "admin.watchlist.alert.manage")?;

    req.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;

    let alert = db::create_alert(&state.db, watchlist_id, &req).await?;

    let actor_role = UserRole::from_str_lower(&admin.role).unwrap_or(UserRole::Admin);
    record_admin_action_best_effort(
        &state.db,
        AdminAction::new(
            admin.user_id,
            actor_role,
            "watchlist.alert.create",
            "watchlist_alert",
        )
        .with_target_id(alert.id.to_string())
        .with_client(&client)
        .with_metadata(serde_json::json!({
            "watchlist_id": watchlist_id,
            "ticker": alert.ticker,
            "direction": alert.direction,
        })),
    )
    .await;

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
    admin: AdminUser,
    client: ClientInfo,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateAlertRequest>,
) -> AppResult<Json<WatchlistAlert>> {
    admin.require(&state.policy, "admin.watchlist.alert.manage")?;

    let alert = db::update_alert(&state.db, id, &req).await?;

    let actor_role = UserRole::from_str_lower(&admin.role).unwrap_or(UserRole::Admin);
    record_admin_action_best_effort(
        &state.db,
        AdminAction::new(
            admin.user_id,
            actor_role,
            "watchlist.alert.update",
            "watchlist_alert",
        )
        .with_target_id(id.to_string())
        .with_client(&client)
        .with_metadata(serde_json::json!({
            "ticker_changed":       req.ticker.is_some(),
            "direction_changed":    req.direction.is_some(),
            "entry_zone_changed":   req.entry_zone.is_some(),
            "invalidation_changed": req.invalidation.is_some(),
            "profit_zones_changed": req.profit_zones.is_some(),
            "notes_changed":        req.notes.is_some(),
            "chart_url_changed":    req.chart_url.is_some(),
        })),
    )
    .await;

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
    admin: AdminUser,
    client: ClientInfo,
    Path(id): Path<Uuid>,
) -> AppResult<Json<serde_json::Value>> {
    admin.require(&state.policy, "admin.watchlist.alert.manage")?;

    db::delete_alert(&state.db, id).await?;

    let actor_role = UserRole::from_str_lower(&admin.role).unwrap_or(UserRole::Admin);
    record_admin_action_best_effort(
        &state.db,
        AdminAction::new(
            admin.user_id,
            actor_role,
            "watchlist.alert.delete",
            "watchlist_alert",
        )
        .with_target_id(id.to_string())
        .with_client(&client)
        .with_metadata(serde_json::Value::Null),
    )
    .await;

    Ok(Json(serde_json::json!({ "message": "Alert deleted" })))
}
