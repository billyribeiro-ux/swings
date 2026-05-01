//! ADM-11: manual subscription operations admin surface.
//!
//! Mounted at `/api/admin/subscriptions`. Three operator workflows:
//!
//!   * `POST /comp`               — mint a `memberships` row (no Stripe).
//!   * `POST /{id}/extend`        — push `current_period_end` by N days.
//!   * `POST /{id}/billing-cycle` — override `billing_cycle_anchor`.
//!   * `GET  /by-user/{user_id}`  — read a user's subscription + active
//!     memberships in one round-trip.
//!
//! Each mutation writes a single row to `subscription_changes` so the
//! per-subscription history is the canonical audit trail (the broader
//! `admin_actions` log also gets a row for ops dashboards).
//!
//! Stripe sync is **deliberately out of scope** for these workflows.
//! Extend / cycle-override are local-mirror mutations; if the operator
//! needs Stripe to follow they must use the existing billing portal /
//! Stripe-direct path. The change-log row makes any divergence
//! discoverable at the user level.

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use sqlx::Row;
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;
use validator::Validate;

use crate::{
    db,
    error::{AppError, AppResult},
    extractors::{ClientInfo, PrivilegedUser},
    models::{PaginatedResponse, Subscription, SubscriptionStatus, SubscriptionStatusResponse},
    services::audit::audit_admin_priv,
    AppState,
};

const PERM_READ: &str = "admin.subscription.read";
const PERM_COMP: &str = "admin.subscription.comp";
const PERM_EXTEND: &str = "admin.subscription.extend";
const PERM_CYCLE: &str = "admin.subscription.cycle";

/// Cap a single extension at one year — beyond that the operator
/// should be running a comp grant or a fresh billing cycle, not
/// stretching an existing period indefinitely.
const MAX_EXTEND_DAYS: i64 = 366;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(list))
        .route("/stats", get(stats))
        .route("/comp", post(comp_grant))
        .route("/by-user/{user_id}", get(by_user))
        .route("/{id}/extend", post(extend_period))
        .route("/{id}/billing-cycle", post(override_billing_cycle))
}

/// Status labels accepted by the list filter and emitted in row payloads.
/// Mirrors the lowercase Postgres `subscription_status` enum from
/// `001_initial.sql` + `057_subscription_status_paused.sql`.
const VALID_STATUSES: &[&str] = &[
    "active", "past_due", "canceled", "trialing", "unpaid", "paused",
];

/// Plan labels accepted by the list filter and emitted in row payloads.
const VALID_PLANS: &[&str] = &["monthly", "annual"];

// ── DTOs ───────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct CompGrantRequest {
    /// Recipient — must already exist in `users`.
    pub user_id: Uuid,
    /// Membership plan id from `membership_plans`.
    pub plan_id: Uuid,
    /// Length of the comp grant in days; `None` ⇒ open-ended (the
    /// access engine treats `ends_at IS NULL` as "until cancelled").
    #[validate(range(min = 1, max = 3650, message = "duration_days must be 1-3650"))]
    pub duration_days: Option<i64>,
    /// Free-text reason captured on the audit row.
    #[validate(length(max = 500))]
    pub notes: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct CompGrantResponse {
    pub membership_id: Uuid,
    pub starts_at: DateTime<Utc>,
    pub ends_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct ExtendRequest {
    /// Number of days to add to `current_period_end`. Must be 1..=366.
    #[validate(range(min = 1, max = 366))]
    pub days: i64,
    #[validate(length(max = 500))]
    pub notes: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ExtendResponse {
    pub subscription_id: Uuid,
    pub previous_current_period_end: DateTime<Utc>,
    pub new_current_period_end: DateTime<Utc>,
}

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct CycleOverrideRequest {
    /// New `billing_cycle_anchor`. Must be in the future (you cannot
    /// retroactively shift a billing cycle in this admin path; if you
    /// need to backdate, do it via Stripe and let the webhook
    /// reconcile).
    pub anchor: DateTime<Utc>,
    #[validate(length(max = 500))]
    pub notes: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct CycleOverrideResponse {
    pub subscription_id: Uuid,
    pub previous_anchor: Option<DateTime<Utc>>,
    pub new_anchor: DateTime<Utc>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct MembershipRow {
    pub id: Uuid,
    pub user_id: Uuid,
    pub plan_id: Uuid,
    pub granted_by: String,
    pub status: String,
    pub starts_at: DateTime<Utc>,
    pub ends_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct UserSubscriptionView {
    pub subscription: SubscriptionStatusResponse,
    /// All non-expired memberships attached to the user, newest first.
    pub memberships: Vec<MembershipRow>,
}

// ── Handlers ───────────────────────────────────────────────────────────

#[utoipa::path(
    get,
    path = "/api/admin/subscriptions/by-user/{user_id}",
    tag = "admin-subscriptions",
    operation_id = "admin_subscriptions_by_user",
    security(("bearer_auth" = [])),
    params(("user_id" = Uuid, Path, description = "Member id")),
    responses(
        (status = 200, description = "Subscription + memberships", body = UserSubscriptionView),
        (status = 401, description = "Unauthenticated"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Member not found")
    )
)]
pub async fn by_user(
    State(state): State<AppState>,
    privileged: PrivilegedUser,
    Path(user_id): Path<Uuid>,
) -> AppResult<Json<UserSubscriptionView>> {
    privileged.require(&state.policy, PERM_READ)?;

    db::find_user_by_id(&state.db, user_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Member not found".to_string()))?;

    let sub = db::find_subscription_by_user(&state.db, user_id).await?;
    let is_active = sub
        .as_ref()
        .map(|s| s.status == SubscriptionStatus::Active || s.status == SubscriptionStatus::Trialing)
        .unwrap_or(false);

    let memberships = fetch_user_memberships(&state.db, user_id).await?;

    Ok(Json(UserSubscriptionView {
        subscription: SubscriptionStatusResponse {
            subscription: sub,
            is_active,
        },
        memberships,
    }))
}

#[utoipa::path(
    post,
    path = "/api/admin/subscriptions/comp",
    tag = "admin-subscriptions",
    operation_id = "admin_subscriptions_comp_grant",
    security(("bearer_auth" = [])),
    request_body = CompGrantRequest,
    responses(
        (status = 201, description = "Comp membership minted", body = CompGrantResponse),
        (status = 400, description = "Validation failed"),
        (status = 401, description = "Unauthenticated"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Member or plan not found")
    )
)]
pub async fn comp_grant(
    State(state): State<AppState>,
    privileged: PrivilegedUser,
    client: ClientInfo,
    Json(req): Json<CompGrantRequest>,
) -> AppResult<(StatusCode, Json<CompGrantResponse>)> {
    privileged.require(&state.policy, PERM_COMP)?;
    req.validate()
        .map_err(|e| AppError::BadRequest(e.to_string()))?;

    db::find_user_by_id(&state.db, req.user_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Member not found".to_string()))?;

    // Ensure the plan exists. We hit the table directly because the
    // membership_plans catalogue has no shared DB helper today.
    let plan_exists: Option<(Uuid,)> =
        sqlx::query_as("SELECT id FROM membership_plans WHERE id = $1")
            .bind(req.plan_id)
            .fetch_optional(&state.db)
            .await?;
    if plan_exists.is_none() {
        return Err(AppError::NotFound("Membership plan not found".to_string()));
    }

    let starts_at = Utc::now();
    let ends_at = req.duration_days.map(|d| starts_at + Duration::days(d));
    let metadata = serde_json::json!({
        "comp_notes":  req.notes.clone().unwrap_or_default(),
        "issued_by":   privileged.user_id,
        "issued_role": privileged.role.as_str(),
    });

    let row = sqlx::query(
        r#"
        INSERT INTO memberships
            (id, user_id, plan_id, granted_by, status, starts_at, ends_at, metadata)
        VALUES
            ($1, $2, $3, 'manual', 'active', $4, $5, $6)
        RETURNING id, starts_at, ends_at
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(req.user_id)
    .bind(req.plan_id)
    .bind(starts_at)
    .bind(ends_at)
    .bind(&metadata)
    .fetch_one(&state.db)
    .await?;
    let membership_id: Uuid = row
        .try_get("id")
        .map_err(|e| AppError::Internal(e.into()))?;

    audit_admin_priv(
        &state.db,
        &privileged,
        &client,
        "admin.subscription.comp",
        "membership",
        membership_id.to_string(),
        serde_json::json!({
            "user_id":       req.user_id,
            "plan_id":       req.plan_id,
            "duration_days": req.duration_days,
            "notes":         req.notes,
        }),
    )
    .await;

    Ok((
        StatusCode::CREATED,
        Json(CompGrantResponse {
            membership_id,
            starts_at,
            ends_at,
        }),
    ))
}

#[utoipa::path(
    post,
    path = "/api/admin/subscriptions/{id}/extend",
    tag = "admin-subscriptions",
    operation_id = "admin_subscriptions_extend_period",
    security(("bearer_auth" = [])),
    params(("id" = Uuid, Path, description = "Subscription id")),
    request_body = ExtendRequest,
    responses(
        (status = 200, description = "Period extended", body = ExtendResponse),
        (status = 400, description = "Validation failed"),
        (status = 401, description = "Unauthenticated"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Subscription not found")
    )
)]
pub async fn extend_period(
    State(state): State<AppState>,
    privileged: PrivilegedUser,
    client: ClientInfo,
    Path(subscription_id): Path<Uuid>,
    Json(req): Json<ExtendRequest>,
) -> AppResult<Json<ExtendResponse>> {
    privileged.require(&state.policy, PERM_EXTEND)?;
    req.validate()
        .map_err(|e| AppError::BadRequest(e.to_string()))?;
    if req.days > MAX_EXTEND_DAYS {
        return Err(AppError::BadRequest(format!(
            "days must be <= {MAX_EXTEND_DAYS}"
        )));
    }

    let mut tx = state.db.begin().await?;

    // SELECT … FOR UPDATE so two concurrent operators don't double-extend.
    let current: Subscription =
        sqlx::query_as::<_, Subscription>("SELECT * FROM subscriptions WHERE id = $1 FOR UPDATE")
            .bind(subscription_id)
            .fetch_optional(&mut *tx)
            .await?
            .ok_or_else(|| AppError::NotFound("Subscription not found".to_string()))?;

    let previous_end = current.current_period_end;
    let new_end = previous_end + Duration::days(req.days);

    sqlx::query(
        "UPDATE subscriptions SET current_period_end = $1, updated_at = NOW() WHERE id = $2",
    )
    .bind(new_end)
    .bind(subscription_id)
    .execute(&mut *tx)
    .await?;

    sqlx::query(
        r#"
        INSERT INTO subscription_changes
            (subscription_id, kind, actor_id, notes)
        VALUES ($1, 'manual_extend', $2, $3)
        "#,
    )
    .bind(subscription_id)
    .bind(privileged.user_id)
    .bind(req.notes.clone().unwrap_or_default())
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    audit_admin_priv(
        &state.db,
        &privileged,
        &client,
        "admin.subscription.extend",
        "subscription",
        subscription_id.to_string(),
        serde_json::json!({
            "days":              req.days,
            "previous_end":      previous_end,
            "new_end":           new_end,
            "stripe_drift_warn": true,
            "notes":             req.notes,
        }),
    )
    .await;

    Ok(Json(ExtendResponse {
        subscription_id,
        previous_current_period_end: previous_end,
        new_current_period_end: new_end,
    }))
}

#[utoipa::path(
    post,
    path = "/api/admin/subscriptions/{id}/billing-cycle",
    tag = "admin-subscriptions",
    operation_id = "admin_subscriptions_override_billing_cycle",
    security(("bearer_auth" = [])),
    params(("id" = Uuid, Path, description = "Subscription id")),
    request_body = CycleOverrideRequest,
    responses(
        (status = 200, description = "Billing cycle anchor overridden", body = CycleOverrideResponse),
        (status = 400, description = "Validation failed (anchor must be in the future)"),
        (status = 401, description = "Unauthenticated"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Subscription not found")
    )
)]
pub async fn override_billing_cycle(
    State(state): State<AppState>,
    privileged: PrivilegedUser,
    client: ClientInfo,
    Path(subscription_id): Path<Uuid>,
    Json(req): Json<CycleOverrideRequest>,
) -> AppResult<Json<CycleOverrideResponse>> {
    privileged.require(&state.policy, PERM_CYCLE)?;
    req.validate()
        .map_err(|e| AppError::BadRequest(e.to_string()))?;

    if req.anchor <= Utc::now() {
        return Err(AppError::BadRequest(
            "anchor must be a timestamp in the future".to_string(),
        ));
    }

    let mut tx = state.db.begin().await?;

    let row =
        sqlx::query("SELECT id, billing_cycle_anchor FROM subscriptions WHERE id = $1 FOR UPDATE")
            .bind(subscription_id)
            .fetch_optional(&mut *tx)
            .await?;
    let row = row.ok_or_else(|| AppError::NotFound("Subscription not found".to_string()))?;

    let previous: Option<DateTime<Utc>> = row
        .try_get("billing_cycle_anchor")
        .map_err(|e| AppError::Internal(e.into()))?;

    sqlx::query(
        "UPDATE subscriptions SET billing_cycle_anchor = $1, updated_at = NOW() WHERE id = $2",
    )
    .bind(req.anchor)
    .bind(subscription_id)
    .execute(&mut *tx)
    .await?;

    sqlx::query(
        r#"
        INSERT INTO subscription_changes
            (subscription_id, kind, actor_id, notes)
        VALUES ($1, 'cycle_override', $2, $3)
        "#,
    )
    .bind(subscription_id)
    .bind(privileged.user_id)
    .bind(req.notes.clone().unwrap_or_default())
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    audit_admin_priv(
        &state.db,
        &privileged,
        &client,
        "admin.subscription.cycle",
        "subscription",
        subscription_id.to_string(),
        serde_json::json!({
            "previous_anchor":  previous,
            "new_anchor":       req.anchor,
            "stripe_drift_warn": true,
            "notes":            req.notes,
        }),
    )
    .await;

    Ok(Json(CycleOverrideResponse {
        subscription_id,
        previous_anchor: previous,
        new_anchor: req.anchor,
    }))
}

// ── List + Stats ───────────────────────────────────────────────────────

/// Query string for the paginated subscriptions list.
#[derive(Debug, Deserialize, IntoParams)]
pub struct ListSubscriptionsQuery {
    /// 1-based page number. Defaults to 1.
    pub page: Option<i64>,
    /// Page size. Capped to 100 to keep the join + count cheap.
    pub per_page: Option<i64>,
    /// Case-insensitive substring against `users.email`.
    pub search: Option<String>,
    /// One of `active|past_due|canceled|trialing|unpaid|paused`.
    pub status: Option<String>,
    /// One of `monthly|annual`.
    pub plan: Option<String>,
}

/// One subscription row in the admin list response.
///
/// Field naming intentionally tracks the SvelteKit consumer at
/// `src/routes/admin/subscriptions/+page.svelte` (which binds to
/// `member_id` / `member_name` / `member_email` / `plan_name` /
/// `interval` / `amount_cents` / `start_date` / `next_renewal`) so the
/// existing page renders without any frontend changes.
#[derive(Debug, Serialize, ToSchema)]
pub struct SubscriptionRow {
    /// Subscription primary key.
    pub id: Uuid,
    /// Member id — frontend route key for `/admin/members/{id}`.
    pub member_id: Uuid,
    pub member_name: String,
    pub member_email: String,
    pub stripe_subscription_id: String,
    pub stripe_customer_id: String,
    /// Plan cadence (`monthly` / `annual`) — what `subscriptions.plan` stores.
    pub plan: String,
    /// Human plan name from `pricing_plans.name`, or the cadence label
    /// when no `pricing_plans` row is linked.
    pub plan_name: String,
    /// `month` for monthly subscriptions, `year` for annual — matches
    /// the `pricing_plans.interval` vocabulary the frontend expects.
    pub interval: String,
    /// Lowercase enum label (`active|past_due|canceled|trialing|unpaid|paused`).
    pub status: String,
    /// Per-row price in cents. Falls back to the public catalog default
    /// when the subscription has no `pricing_plan_id`.
    pub amount_cents: i64,
    /// `subscriptions.created_at` — when the subscription was first recorded.
    pub start_date: DateTime<Utc>,
    /// `subscriptions.current_period_end` while live; `null` once
    /// canceled (no further renewal expected).
    pub next_renewal: Option<DateTime<Utc>>,
    /// `true` when `subscriptions.cancel_at` is set — the subscription
    /// will end at `current_period_end` without further renewal.
    pub cancel_at_period_end: bool,
    pub canceled_at: Option<DateTime<Utc>>,
}

/// OpenAPI wrapper carrying the concrete `PaginatedResponse<SubscriptionRow>`
/// shape so the snapshot doesn't leak the generic into `ApiDoc`.
#[derive(Debug, Serialize, ToSchema)]
pub struct PaginatedSubscriptionsResponse {
    pub data: Vec<SubscriptionRow>,
    pub total: i64,
    pub page: i64,
    pub per_page: i64,
    pub total_pages: i64,
}

/// Aggregate counters for the subscriptions overview KPIs.
///
/// `monthly_count` / `annual_count` mirror what the frontend page binds
/// to today; the additional status counts and `arr_cents` are surfaced
/// for upstream tooling and future KPI cards.
#[derive(Debug, Serialize, ToSchema)]
pub struct SubscriptionStats {
    pub total_active: i64,
    pub monthly_count: i64,
    pub annual_count: i64,
    pub trialing: i64,
    pub past_due: i64,
    pub canceled: i64,
    pub unpaid: i64,
    pub paused: i64,
    pub mrr_cents: i64,
    pub arr_cents: i64,
}

#[utoipa::path(
    get,
    path = "/api/admin/subscriptions",
    tag = "admin-subscriptions",
    operation_id = "admin_subscriptions_list",
    security(("bearer_auth" = [])),
    params(ListSubscriptionsQuery),
    responses(
        (status = 200, description = "Paginated subscriptions", body = PaginatedSubscriptionsResponse),
        (status = 400, description = "Invalid filter value"),
        (status = 401, description = "Unauthenticated"),
        (status = 403, description = "Forbidden")
    )
)]
pub async fn list(
    State(state): State<AppState>,
    privileged: PrivilegedUser,
    Query(q): Query<ListSubscriptionsQuery>,
) -> AppResult<Json<PaginatedResponse<SubscriptionRow>>> {
    privileged.require(&state.policy, PERM_READ)?;

    let page = q.page.unwrap_or(1).max(1);
    let per_page = q.per_page.unwrap_or(15).clamp(1, 100);

    let status = match q.status.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        Some(s) if VALID_STATUSES.contains(&s) => Some(s.to_string()),
        Some(other) => {
            return Err(AppError::BadRequest(format!(
                "invalid status `{other}` (expected one of {})",
                VALID_STATUSES.join(", ")
            )));
        }
        None => None,
    };
    let plan = match q.plan.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        Some(p) if VALID_PLANS.contains(&p) => Some(p.to_string()),
        Some(other) => {
            return Err(AppError::BadRequest(format!(
                "invalid plan `{other}` (expected one of {})",
                VALID_PLANS.join(", ")
            )));
        }
        None => None,
    };
    let search = q
        .search
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string);

    let (rows, total) = db::list_subscriptions_admin(
        &state.db,
        page,
        per_page,
        search.as_deref(),
        status.as_deref(),
        plan.as_deref(),
    )
    .await?;

    // Catalog fallback for rows whose subscription has no
    // `pricing_plan_id` — only fetched when at least one row needs it.
    let need_fallback = rows.iter().any(|r| r.plan_amount_cents.is_none());
    let (default_monthly, default_annual) = if need_fallback {
        db::pricing_monthly_annual_cents(&state.db).await?
    } else {
        (0, 0)
    };

    let data = rows
        .into_iter()
        .map(|r| {
            let plan_label = match r.plan {
                crate::models::SubscriptionPlan::Monthly => "monthly",
                crate::models::SubscriptionPlan::Annual => "annual",
            };
            let interval = match r.plan {
                crate::models::SubscriptionPlan::Monthly => "month",
                crate::models::SubscriptionPlan::Annual => "year",
            };
            let amount_cents = r.plan_amount_cents.unwrap_or(match r.plan {
                crate::models::SubscriptionPlan::Monthly => default_monthly,
                crate::models::SubscriptionPlan::Annual => default_annual,
            });
            let plan_name = r.plan_name.unwrap_or_else(|| {
                match r.plan {
                    crate::models::SubscriptionPlan::Monthly => "Monthly",
                    crate::models::SubscriptionPlan::Annual => "Annual",
                }
                .to_string()
            });
            let next_renewal = if r.status == "canceled" {
                None
            } else {
                Some(r.current_period_end)
            };
            SubscriptionRow {
                id: r.id,
                member_id: r.user_id,
                member_name: r.user_name,
                member_email: r.user_email,
                stripe_subscription_id: r.stripe_subscription_id,
                stripe_customer_id: r.stripe_customer_id,
                plan: plan_label.to_string(),
                plan_name,
                interval: interval.to_string(),
                status: r.status,
                amount_cents,
                start_date: r.created_at,
                next_renewal,
                cancel_at_period_end: r.cancel_at.is_some(),
                canceled_at: r.canceled_at,
            }
        })
        .collect::<Vec<_>>();

    let total_pages = if per_page > 0 {
        (total + per_page - 1) / per_page
    } else {
        0
    };

    Ok(Json(PaginatedResponse {
        data,
        total,
        page,
        per_page,
        total_pages,
    }))
}

#[utoipa::path(
    get,
    path = "/api/admin/subscriptions/stats",
    tag = "admin-subscriptions",
    operation_id = "admin_subscriptions_stats",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Subscription KPIs", body = SubscriptionStats),
        (status = 401, description = "Unauthenticated"),
        (status = 403, description = "Forbidden")
    )
)]
pub async fn stats(
    State(state): State<AppState>,
    privileged: PrivilegedUser,
) -> AppResult<Json<SubscriptionStats>> {
    privileged.require(&state.policy, PERM_READ)?;

    let (mrr_cents, arr_cents, total_active) = db::admin_estimated_mrr_arr_cents(&state.db).await?;
    let monthly_count =
        db::count_subscriptions_by_plan(&state.db, &crate::models::SubscriptionPlan::Monthly)
            .await?;
    let annual_count =
        db::count_subscriptions_by_plan(&state.db, &crate::models::SubscriptionPlan::Annual)
            .await?;
    let trialing = db::count_subscriptions_by_status(&state.db, "trialing").await?;
    let past_due = db::count_subscriptions_by_status(&state.db, "past_due").await?;
    let canceled = db::count_subscriptions_by_status(&state.db, "canceled").await?;
    let unpaid = db::count_subscriptions_by_status(&state.db, "unpaid").await?;
    let paused = db::count_subscriptions_by_status(&state.db, "paused").await?;

    Ok(Json(SubscriptionStats {
        total_active,
        monthly_count,
        annual_count,
        trialing,
        past_due,
        canceled,
        unpaid,
        paused,
        mrr_cents,
        arr_cents,
    }))
}

// ── Helpers ────────────────────────────────────────────────────────────

async fn fetch_user_memberships(
    pool: &sqlx::PgPool,
    user_id: Uuid,
) -> Result<Vec<MembershipRow>, sqlx::Error> {
    let rows = sqlx::query(
        r#"
        SELECT id, user_id, plan_id, granted_by, status, starts_at, ends_at
          FROM memberships
         WHERE user_id = $1
           AND status <> 'expired'
         ORDER BY created_at DESC
        "#,
    )
    .bind(user_id)
    .fetch_all(pool)
    .await?;

    let mut out = Vec::with_capacity(rows.len());
    for r in rows {
        out.push(MembershipRow {
            id: r.try_get("id")?,
            user_id: r.try_get("user_id")?,
            plan_id: r.try_get("plan_id")?,
            granted_by: r.try_get("granted_by")?,
            status: r.try_get("status")?,
            starts_at: r.try_get("starts_at")?,
            ends_at: r.try_get("ends_at")?,
        });
    }
    Ok(out)
}
