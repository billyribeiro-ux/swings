//! ADM-11: manual subscription operations admin surface.
//!
//! Mounted at `/api/admin/subscriptions`. Three operator workflows:
//!
//!   * `POST /comp`               — mint a `memberships` row (no Stripe).
//!   * `POST /{id}/extend`        — push `current_period_end` by N days.
//!   * `POST /{id}/billing-cycle` — override `billing_cycle_anchor`.
//!   * `GET  /by-user/{user_id}`  — read a user's subscription + active
//!                                  memberships in one round-trip.
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
    extract::{Path, State},
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use sqlx::Row;
use utoipa::ToSchema;
use uuid::Uuid;
use validator::Validate;

use crate::{
    db,
    error::{AppError, AppResult},
    extractors::{ClientInfo, PrivilegedUser},
    models::{Subscription, SubscriptionStatus, SubscriptionStatusResponse},
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
        .route("/comp", post(comp_grant))
        .route("/by-user/{user_id}", get(by_user))
        .route("/{id}/extend", post(extend_period))
        .route("/{id}/billing-cycle", post(override_billing_cycle))
}

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
