//! ADM-05: privileged admin / support handlers for the member lifecycle,
//! session management, and security telemetry.
//!
//! All endpoints land under `/api/admin/security/*` and `/api/admin/members/*`
//! (the latter merged into the existing `admin::router()` in `main.rs`).
//!
//! Authorisation contract — every handler:
//!   1. Goes through [`PrivilegedUser`], proving the caller carries
//!      `admin.dashboard.read` (admin always; support per the FDN-07 seed).
//!   2. Calls `admin.require(&state.policy, "<perm>")` to enforce the
//!      per-action permission introduced in `058_admin_lifecycle_perms.sql`.
//!   3. Writes one row to `admin_actions` via [`services::audit`] before
//!      returning so the action is recoverable from the audit log alone.
//!
//! See `BACKEND-AUDIT-REPORT.md` (ADM-05 section) for the full design rationale.

use axum::{
    extract::{Path, Query, State},
    routing::{get, post},
    Json, Router,
};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::FromRow;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{
    db,
    error::{AppError, AppResult},
    extractors::{ClientInfo, PrivilegedUser},
    models::{PaginationParams, UserResponse, UserRole},
    notifications::send::{send_notification, Recipient, SendOptions},
    services::audit::{record_admin_action, AdminAction},
    AppState,
};

/// Cap on per-page items for any admin list endpoint defined in this module.
/// Mirrors `PaginationParams::per_page` upper bound used elsewhere.
const MAX_PER_PAGE: i64 = 100;

// ── Router ──────────────────────────────────────────────────────────────

pub fn router() -> Router<AppState> {
    Router::new()
        // Member lifecycle.
        .route("/members/{id}/suspend", post(suspend_member))
        .route("/members/{id}/reactivate", post(reactivate_member))
        .route("/members/{id}/ban", post(ban_member))
        .route(
            "/members/{id}/force-password-reset",
            post(force_password_reset),
        )
        .route("/members/{id}/verify-email", post(mark_email_verified))
        // Sessions / refresh tokens.
        .route("/members/{id}/sessions", get(list_sessions))
        .route("/members/{id}/sessions", axum::routing::delete(force_logout))
        .route(
            "/members/{id}/sessions/{session_id}",
            axum::routing::delete(revoke_session),
        )
        // Security telemetry.
        .route("/security/audit-log", get(list_audit_log))
        .route("/security/failed-logins", get(list_failed_logins))
}

// ── DTOs ────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize, ToSchema)]
pub struct LifecycleRequest {
    /// Optional free-text reason captured in the audit log + the `users`
    /// row (truncated to 512 chars by the DB CHECK constraint).
    #[serde(default)]
    pub reason: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ForcePasswordResetResponse {
    pub user_id: Uuid,
    pub reset_url_dispatched: bool,
}

#[derive(Debug, Serialize, ToSchema, FromRow)]
pub struct SessionRow {
    pub id: Uuid,
    pub user_id: Uuid,
    pub family_id: Uuid,
    pub used: bool,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct SessionsResponse {
    pub user_id: Uuid,
    pub active_sessions: Vec<SessionRow>,
    pub total: i64,
}

#[derive(Debug, Serialize, ToSchema, FromRow)]
pub struct AuditLogRow {
    pub id: Uuid,
    pub actor_id: Uuid,
    pub actor_role: UserRole,
    pub action: String,
    pub target_kind: String,
    pub target_id: Option<String>,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub metadata: serde_json::Value,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct AuditLogFilter {
    pub actor_id: Option<Uuid>,
    pub action: Option<String>,
    pub target_kind: Option<String>,
    pub target_id: Option<String>,
    pub page: Option<i64>,
    pub per_page: Option<i64>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct AuditLogResponse {
    pub data: Vec<AuditLogRow>,
    pub page: i64,
    pub per_page: i64,
    pub total: i64,
    pub total_pages: i64,
}

#[derive(Debug, Serialize, ToSchema, FromRow)]
pub struct FailedLoginRow {
    pub id: Uuid,
    pub email: String,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub reason: String,
    pub occurred_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct FailedLoginFilter {
    pub email: Option<String>,
    pub ip: Option<String>,
    pub since_hours: Option<i64>,
    pub page: Option<i64>,
    pub per_page: Option<i64>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct FailedLoginResponse {
    pub data: Vec<FailedLoginRow>,
    pub page: i64,
    pub per_page: i64,
    pub total: i64,
    pub total_pages: i64,
}

// ── Member lifecycle handlers ──────────────────────────────────────────

#[utoipa::path(
    post,
    path = "/api/admin/members/{id}/suspend",
    tag = "admin-security",
    security(("bearer_auth" = [])),
    request_body = LifecycleRequest,
    responses(
        (status = 200, description = "Member suspended", body = UserResponse),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Member not found"),
        (status = 409, description = "Cannot suspend an admin")
    )
)]
async fn suspend_member(
    State(state): State<AppState>,
    admin: PrivilegedUser,
    client: ClientInfo,
    Path(target_id): Path<Uuid>,
    Json(req): Json<LifecycleRequest>,
) -> AppResult<Json<UserResponse>> {
    admin.require(&state.policy, "user.suspend")?;

    let target = db::find_user_by_id(&state.db, target_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Member not found".to_string()))?;

    if matches!(target.role, UserRole::Admin) {
        return Err(AppError::Conflict(
            "Refusing to suspend an admin account".to_string(),
        ));
    }

    let reason = req.reason.as_deref().map(str::trim).filter(|s| !s.is_empty());
    let updated = sqlx::query_as::<_, crate::models::User>(
        r#"
        UPDATE users
           SET suspended_at      = NOW(),
               suspension_reason = $1,
               banned_at         = NULL,
               ban_reason        = NULL,
               updated_at        = NOW()
         WHERE id = $2
         RETURNING *
        "#,
    )
    .bind(reason)
    .bind(target_id)
    .fetch_one(&state.db)
    .await?;

    // Refresh tokens are revoked unconditionally — a suspended user must
    // not retain an active session after the action lands.
    db::delete_user_refresh_tokens(&state.db, target_id).await?;

    record_admin_action(
        &state.db,
        AdminAction::new(admin.user_id, admin.role, "user.suspend", "user")
            .with_target_id(target_id)
            .with_client(&client)
            .with_metadata(serde_json::json!({
                "reason": reason,
                "previous_state": lifecycle_state(&target),
            })),
    )
    .await?;

    Ok(Json(updated.into()))
}

#[utoipa::path(
    post,
    path = "/api/admin/members/{id}/reactivate",
    tag = "admin-security",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Member reactivated", body = UserResponse),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Member not found")
    )
)]
async fn reactivate_member(
    State(state): State<AppState>,
    admin: PrivilegedUser,
    client: ClientInfo,
    Path(target_id): Path<Uuid>,
) -> AppResult<Json<UserResponse>> {
    admin.require(&state.policy, "user.reactivate")?;

    let target = db::find_user_by_id(&state.db, target_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Member not found".to_string()))?;

    let updated = sqlx::query_as::<_, crate::models::User>(
        r#"
        UPDATE users
           SET suspended_at      = NULL,
               suspension_reason = NULL,
               banned_at         = NULL,
               ban_reason        = NULL,
               updated_at        = NOW()
         WHERE id = $1
         RETURNING *
        "#,
    )
    .bind(target_id)
    .fetch_one(&state.db)
    .await?;

    record_admin_action(
        &state.db,
        AdminAction::new(admin.user_id, admin.role, "user.reactivate", "user")
            .with_target_id(target_id)
            .with_client(&client)
            .with_metadata(serde_json::json!({
                "previous_state": lifecycle_state(&target),
            })),
    )
    .await?;

    Ok(Json(updated.into()))
}

#[utoipa::path(
    post,
    path = "/api/admin/members/{id}/ban",
    tag = "admin-security",
    security(("bearer_auth" = [])),
    request_body = LifecycleRequest,
    responses(
        (status = 200, description = "Member banned", body = UserResponse),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Member not found"),
        (status = 409, description = "Cannot ban an admin")
    )
)]
async fn ban_member(
    State(state): State<AppState>,
    admin: PrivilegedUser,
    client: ClientInfo,
    Path(target_id): Path<Uuid>,
    Json(req): Json<LifecycleRequest>,
) -> AppResult<Json<UserResponse>> {
    // Hard ban is admin-only by the seed in 058 — `support` has
    // `user.suspend` / `user.reactivate` but NOT `user.ban`.
    admin.require(&state.policy, "user.ban")?;

    let target = db::find_user_by_id(&state.db, target_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Member not found".to_string()))?;

    if matches!(target.role, UserRole::Admin) {
        return Err(AppError::Conflict(
            "Refusing to ban an admin account".to_string(),
        ));
    }

    let reason = req.reason.as_deref().map(str::trim).filter(|s| !s.is_empty());
    let updated = sqlx::query_as::<_, crate::models::User>(
        r#"
        UPDATE users
           SET banned_at         = NOW(),
               ban_reason        = $1,
               suspended_at      = NULL,
               suspension_reason = NULL,
               updated_at        = NOW()
         WHERE id = $2
         RETURNING *
        "#,
    )
    .bind(reason)
    .bind(target_id)
    .fetch_one(&state.db)
    .await?;

    db::delete_user_refresh_tokens(&state.db, target_id).await?;

    record_admin_action(
        &state.db,
        AdminAction::new(admin.user_id, admin.role, "user.ban", "user")
            .with_target_id(target_id)
            .with_client(&client)
            .with_metadata(serde_json::json!({
                "reason": reason,
                "previous_state": lifecycle_state(&target),
            })),
    )
    .await?;

    Ok(Json(updated.into()))
}

#[utoipa::path(
    post,
    path = "/api/admin/members/{id}/force-password-reset",
    tag = "admin-security",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Reset link dispatched", body = ForcePasswordResetResponse),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Member not found")
    )
)]
async fn force_password_reset(
    State(state): State<AppState>,
    admin: PrivilegedUser,
    client: ClientInfo,
    Path(target_id): Path<Uuid>,
) -> AppResult<Json<ForcePasswordResetResponse>> {
    admin.require(&state.policy, "user.force_password_reset")?;

    let target = db::find_user_by_id(&state.db, target_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Member not found".to_string()))?;

    let raw_token = Uuid::new_v4().to_string();
    let token_hash = sha256_hex(&raw_token);
    let expires_at = Utc::now() + Duration::hours(1);
    db::create_password_reset_token(&state.db, target_id, &token_hash, expires_at).await?;

    // All existing refresh tokens are revoked: the operator initiated this
    // because the account state is suspect, so any cached session must die.
    db::delete_user_refresh_tokens(&state.db, target_id).await?;

    let reset_url = format!(
        "{}/admin/reset-password?token={}",
        state.config.frontend_url, raw_token
    );
    let ctx = serde_json::json!({
        "name": target.name,
        "reset_url": reset_url,
        "app_url": state.config.app_url,
        "year": Utc::now().format("%Y").to_string(),
        "initiated_by": "support",
    });

    let dispatched = send_notification(
        &state.db,
        "user.password_reset",
        &Recipient::User {
            user_id: target.id,
            email: target.email.clone(),
        },
        ctx,
        SendOptions::default(),
    )
    .await
    .is_ok();

    record_admin_action(
        &state.db,
        AdminAction::new(
            admin.user_id,
            admin.role,
            "user.force_password_reset",
            "user",
        )
        .with_target_id(target_id)
        .with_client(&client)
        .with_metadata(serde_json::json!({ "dispatched": dispatched })),
    )
    .await?;

    Ok(Json(ForcePasswordResetResponse {
        user_id: target_id,
        reset_url_dispatched: dispatched,
    }))
}

#[utoipa::path(
    post,
    path = "/api/admin/members/{id}/verify-email",
    tag = "admin-security",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Email marked verified", body = UserResponse),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Member not found")
    )
)]
async fn mark_email_verified(
    State(state): State<AppState>,
    admin: PrivilegedUser,
    client: ClientInfo,
    Path(target_id): Path<Uuid>,
) -> AppResult<Json<UserResponse>> {
    admin.require(&state.policy, "user.email.verify")?;

    let target = db::find_user_by_id(&state.db, target_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Member not found".to_string()))?;

    if target.email_verified_at.is_some() {
        // Idempotent: already verified is a 200 no-op so retries don't
        // pollute the audit log with duplicate write rows.
        return Ok(Json(target.into()));
    }

    let updated = sqlx::query_as::<_, crate::models::User>(
        "UPDATE users SET email_verified_at = NOW(), updated_at = NOW() WHERE id = $1 RETURNING *",
    )
    .bind(target_id)
    .fetch_one(&state.db)
    .await?;

    record_admin_action(
        &state.db,
        AdminAction::new(admin.user_id, admin.role, "user.email.verify", "user")
            .with_target_id(target_id)
            .with_client(&client),
    )
    .await?;

    Ok(Json(updated.into()))
}

// ── Session viewer / force logout ──────────────────────────────────────

#[utoipa::path(
    get,
    path = "/api/admin/members/{id}/sessions",
    tag = "admin-security",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Active sessions for the member", body = SessionsResponse),
        (status = 403, description = "Forbidden")
    )
)]
async fn list_sessions(
    State(state): State<AppState>,
    admin: PrivilegedUser,
    Path(user_id): Path<Uuid>,
) -> AppResult<Json<SessionsResponse>> {
    admin.require(&state.policy, "user.session.read")?;

    let rows = sqlx::query_as::<_, SessionRow>(
        r#"SELECT id, user_id, family_id, used, created_at, expires_at
             FROM refresh_tokens
            WHERE user_id = $1 AND expires_at > NOW() AND used = FALSE
            ORDER BY created_at DESC"#,
    )
    .bind(user_id)
    .fetch_all(&state.db)
    .await?;

    let total = rows.len() as i64;
    Ok(Json(SessionsResponse {
        user_id,
        active_sessions: rows,
        total,
    }))
}

#[utoipa::path(
    delete,
    path = "/api/admin/members/{id}/sessions",
    tag = "admin-security",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "All sessions revoked"),
        (status = 403, description = "Forbidden")
    )
)]
async fn force_logout(
    State(state): State<AppState>,
    admin: PrivilegedUser,
    client: ClientInfo,
    Path(user_id): Path<Uuid>,
) -> AppResult<Json<serde_json::Value>> {
    admin.require(&state.policy, "user.session.revoke")?;

    let res = sqlx::query("DELETE FROM refresh_tokens WHERE user_id = $1")
        .bind(user_id)
        .execute(&state.db)
        .await?;

    record_admin_action(
        &state.db,
        AdminAction::new(admin.user_id, admin.role, "user.session.revoke_all", "user")
            .with_target_id(user_id)
            .with_client(&client)
            .with_metadata(serde_json::json!({ "revoked_count": res.rows_affected() })),
    )
    .await?;

    Ok(Json(serde_json::json!({
        "user_id": user_id,
        "revoked_count": res.rows_affected(),
    })))
}

#[utoipa::path(
    delete,
    path = "/api/admin/members/{id}/sessions/{session_id}",
    tag = "admin-security",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Session revoked"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Session not found")
    )
)]
async fn revoke_session(
    State(state): State<AppState>,
    admin: PrivilegedUser,
    client: ClientInfo,
    Path((user_id, session_id)): Path<(Uuid, Uuid)>,
) -> AppResult<Json<serde_json::Value>> {
    admin.require(&state.policy, "user.session.revoke")?;

    // Lookup first so the response distinguishes "no such session" from
    // "session belongs to another user" — both 404 to the client to avoid
    // probing, but logged distinctly server-side.
    let row: Option<(Uuid,)> =
        sqlx::query_as("SELECT user_id FROM refresh_tokens WHERE id = $1")
            .bind(session_id)
            .fetch_optional(&state.db)
            .await?;
    match row {
        Some((owner,)) if owner == user_id => {}
        Some((other_owner,)) => {
            tracing::warn!(
                target_user = %user_id,
                actual_owner = %other_owner,
                session_id = %session_id,
                "session revoke target/owner mismatch; returning 404"
            );
            return Err(AppError::NotFound("Session not found".to_string()));
        }
        None => return Err(AppError::NotFound("Session not found".to_string())),
    }

    sqlx::query("DELETE FROM refresh_tokens WHERE id = $1")
        .bind(session_id)
        .execute(&state.db)
        .await?;

    record_admin_action(
        &state.db,
        AdminAction::new(admin.user_id, admin.role, "user.session.revoke_one", "session")
            .with_target_id(session_id)
            .with_client(&client)
            .with_metadata(serde_json::json!({ "user_id": user_id })),
    )
    .await?;

    Ok(Json(serde_json::json!({
        "session_id": session_id,
        "user_id": user_id,
        "revoked": true,
    })))
}

// ── Audit log viewer ───────────────────────────────────────────────────

#[utoipa::path(
    get,
    path = "/api/admin/security/audit-log",
    tag = "admin-security",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Filtered admin_actions feed", body = AuditLogResponse),
        (status = 403, description = "Forbidden")
    )
)]
async fn list_audit_log(
    State(state): State<AppState>,
    admin: PrivilegedUser,
    Query(filter): Query<AuditLogFilter>,
) -> AppResult<Json<AuditLogResponse>> {
    admin.require(&state.policy, "admin.audit.read")?;

    let per_page = clamp_per_page(filter.per_page);
    let page = filter.page.unwrap_or(1).max(1);
    let offset = (page - 1) * per_page;

    // Hand-rolled query to thread the optional filters without sqlx-macros
    // (the rest of `db.rs` follows the same `query_as` + bind pattern).
    let rows = sqlx::query_as::<_, AuditLogRow>(
        r#"
        SELECT id,
               actor_id,
               actor_role,
               action,
               target_kind,
               target_id,
               COALESCE(host(ip_address), NULL) AS ip_address,
               user_agent,
               metadata,
               created_at
          FROM admin_actions
         WHERE ($1::uuid IS NULL OR actor_id   = $1::uuid)
           AND ($2::text IS NULL OR action     = $2::text)
           AND ($3::text IS NULL OR target_kind= $3::text)
           AND ($4::text IS NULL OR target_id  = $4::text)
         ORDER BY created_at DESC
         LIMIT $5 OFFSET $6
        "#,
    )
    .bind(filter.actor_id)
    .bind(filter.action.as_deref())
    .bind(filter.target_kind.as_deref())
    .bind(filter.target_id.as_deref())
    .bind(per_page)
    .bind(offset)
    .fetch_all(&state.db)
    .await?;

    let total: (i64,) = sqlx::query_as(
        r#"
        SELECT COUNT(*)
          FROM admin_actions
         WHERE ($1::uuid IS NULL OR actor_id   = $1::uuid)
           AND ($2::text IS NULL OR action     = $2::text)
           AND ($3::text IS NULL OR target_kind= $3::text)
           AND ($4::text IS NULL OR target_id  = $4::text)
        "#,
    )
    .bind(filter.actor_id)
    .bind(filter.action.as_deref())
    .bind(filter.target_kind.as_deref())
    .bind(filter.target_id.as_deref())
    .fetch_one(&state.db)
    .await?;

    let total_pages = ((total.0 as f64) / (per_page as f64)).ceil() as i64;

    Ok(Json(AuditLogResponse {
        data: rows,
        page,
        per_page,
        total: total.0,
        total_pages,
    }))
}

// ── Failed login viewer ────────────────────────────────────────────────

#[utoipa::path(
    get,
    path = "/api/admin/security/failed-logins",
    tag = "admin-security",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Filtered failed login attempts", body = FailedLoginResponse),
        (status = 403, description = "Forbidden")
    )
)]
async fn list_failed_logins(
    State(state): State<AppState>,
    admin: PrivilegedUser,
    Query(filter): Query<FailedLoginFilter>,
) -> AppResult<Json<FailedLoginResponse>> {
    admin.require(&state.policy, "admin.security.read")?;

    let per_page = clamp_per_page(filter.per_page);
    let page = filter.page.unwrap_or(1).max(1);
    let offset = (page - 1) * per_page;

    let since_hours = filter.since_hours.unwrap_or(24).clamp(1, 24 * 30);
    let since_cutoff = Utc::now() - Duration::hours(since_hours);
    let email_filter = filter.email.as_deref().map(str::to_lowercase);

    let rows = sqlx::query_as::<_, FailedLoginRow>(
        r#"
        SELECT id,
               email,
               COALESCE(host(ip_address), NULL) AS ip_address,
               user_agent,
               reason,
               occurred_at
          FROM failed_login_attempts
         WHERE occurred_at >= $1
           AND ($2::text IS NULL OR lower(email) = $2::text)
           AND ($3::text IS NULL OR host(ip_address) = $3::text)
         ORDER BY occurred_at DESC
         LIMIT $4 OFFSET $5
        "#,
    )
    .bind(since_cutoff)
    .bind(email_filter.as_deref())
    .bind(filter.ip.as_deref())
    .bind(per_page)
    .bind(offset)
    .fetch_all(&state.db)
    .await?;

    let total: (i64,) = sqlx::query_as(
        r#"
        SELECT COUNT(*)
          FROM failed_login_attempts
         WHERE occurred_at >= $1
           AND ($2::text IS NULL OR lower(email) = $2::text)
           AND ($3::text IS NULL OR host(ip_address) = $3::text)
        "#,
    )
    .bind(since_cutoff)
    .bind(email_filter.as_deref())
    .bind(filter.ip.as_deref())
    .fetch_one(&state.db)
    .await?;

    let total_pages = ((total.0 as f64) / (per_page as f64)).ceil() as i64;

    Ok(Json(FailedLoginResponse {
        data: rows,
        page,
        per_page,
        total: total.0,
        total_pages,
    }))
}

// Touch this re-export so unused-import lints stay quiet when the only
// caller of `PaginationParams` lives in a sibling module.
#[allow(dead_code)]
fn _silence_unused_pagination_import(_: PaginationParams) {}

// ── Helpers ────────────────────────────────────────────────────────────

fn lifecycle_state(u: &crate::models::User) -> serde_json::Value {
    serde_json::json!({
        "suspended_at":      u.suspended_at,
        "suspension_reason": u.suspension_reason,
        "banned_at":         u.banned_at,
        "ban_reason":        u.ban_reason,
        "email_verified_at": u.email_verified_at,
    })
}

fn clamp_per_page(per_page: Option<i64>) -> i64 {
    per_page.unwrap_or(25).clamp(1, MAX_PER_PAGE)
}

fn sha256_hex(input: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    hasher
        .finalize()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clamp_per_page_caps_at_max() {
        assert_eq!(clamp_per_page(Some(1_000)), MAX_PER_PAGE);
        assert_eq!(clamp_per_page(Some(0)), 1);
        assert_eq!(clamp_per_page(None), 25);
        assert_eq!(clamp_per_page(Some(50)), 50);
    }

    #[test]
    fn lifecycle_state_serializes_all_fields() {
        let user = crate::models::User {
            id: Uuid::new_v4(),
            email: "x@y.test".into(),
            password_hash: "_".into(),
            name: "X".into(),
            role: UserRole::Member,
            avatar_url: None,
            bio: None,
            position: None,
            website_url: None,
            twitter_url: None,
            linkedin_url: None,
            youtube_url: None,
            instagram_url: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            suspended_at: Some(Utc::now()),
            suspension_reason: Some("spam".into()),
            banned_at: None,
            ban_reason: None,
            email_verified_at: None,
        };
        let json = lifecycle_state(&user);
        assert!(json["suspended_at"].is_string());
        assert_eq!(json["suspension_reason"], "spam");
        assert!(json["banned_at"].is_null());
    }

    #[test]
    fn sha256_hex_matches_known_vector() {
        // sha256("abc") = ba7816bf...
        let h = sha256_hex("abc");
        assert!(h.starts_with("ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"));
    }
}
