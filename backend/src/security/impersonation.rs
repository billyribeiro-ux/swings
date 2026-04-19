//! ADM-07: server-side state for admin impersonation tokens.
//!
//! This module owns the SQL-side contract for the
//! `impersonation_sessions` table introduced by migration
//! `060_impersonation_sessions.sql`.
//!
//! Why a server-side row exists at all
//! -----------------------------------
//! Access JWTs in this codebase are stateless HS256 blobs. Without a
//! companion DB row there is no way to **revoke** an issued impersonation
//! token before its `exp`, and a leaked admin secret would allow
//! arbitrarily long impersonation. Every impersonation JWT carries the
//! row id in its `imp_session` claim, and [`AuthUser::from_request_parts`]
//! defers to [`lookup_active`] on every request — flipping `revoked_at`
//! immediately invalidates the live token.
//!
//! Defence in depth
//! ----------------
//! * `expires_at` is enforced **both** in the JWT (`exp`) and in this
//!   table; an attacker who could forge a long `exp` is still capped by
//!   the row.
//! * `actor_role` and `actor_user_id` are recorded at mint time, so
//!   audit reconstruction does not depend on the user's current role.
//! * Mutations always write an `admin_actions` row via the calling
//!   handler — the table is never the only source of truth for what
//!   happened.

use std::net::IpAddr;

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{
    error::{AppError, AppResult},
    models::UserRole,
};

/// Maximum impersonation session lifetime accepted at mint time.
///
/// Keep this short: the longer a session lives, the larger the audit
/// surface. 60 minutes is enough for almost every helpdesk task and
/// matches the convention used by Auth0 / Okta default impersonation
/// flows. Operators who need longer must mint a fresh session — that
/// re-records the `reason` in the audit log on each renewal.
pub const MAX_TTL_MINUTES: i64 = 60;

/// Default TTL when the caller does not specify `ttl_minutes`.
pub const DEFAULT_TTL_MINUTES: i64 = 30;

/// Materialised impersonation-session row as returned by the admin CRUD
/// endpoints and consumed by [`crate::extractors::AuthUser`].
#[derive(Debug, Clone, Serialize, ToSchema, sqlx::FromRow)]
pub struct ImpersonationSession {
    pub id: Uuid,
    pub actor_user_id: Uuid,
    pub actor_role: UserRole,
    pub target_user_id: Uuid,
    pub reason: String,
    pub issued_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub revoked_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub revoked_by: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub revoke_reason: Option<String>,
    /// Mint-time IP in canonical text form. Stored as Postgres `inet`,
    /// projected via `host(...)` so callers don't need the `ipnetwork`
    /// sqlx feature.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ip_address: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_agent: Option<String>,
}

impl ImpersonationSession {
    /// Is this row currently honoured by [`crate::extractors::AuthUser`]?
    /// Centralised so the read path and the response shape never drift.
    #[must_use]
    pub fn is_active(&self, now: DateTime<Utc>) -> bool {
        self.revoked_at.is_none() && self.expires_at > now
    }
}

/// Inputs accepted by `POST /api/admin/security/impersonation`.
#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateImpersonationInput {
    /// User to impersonate. Must exist; must not be the caller; must
    /// not itself be an admin (defence-in-depth — `impersonator` audit
    /// trail across admins would be a privilege-escalation footgun).
    pub target_user_id: Uuid,
    /// Required free-text justification (1..=500 chars). Surfaced in
    /// the audit log so post-incident review can answer "why".
    pub reason: String,
    /// Requested TTL in minutes. Capped at [`MAX_TTL_MINUTES`].
    /// Defaults to [`DEFAULT_TTL_MINUTES`] when omitted.
    #[serde(default)]
    pub ttl_minutes: Option<i64>,
}

/// Validate user-supplied input before any DB or token work happens.
pub fn validate_input(input: &CreateImpersonationInput) -> AppResult<()> {
    let trimmed = input.reason.trim();
    if trimmed.is_empty() {
        return Err(AppError::BadRequest(
            "Impersonation reason must not be empty.".into(),
        ));
    }
    if trimmed.len() > 500 {
        return Err(AppError::BadRequest(
            "Impersonation reason exceeds 500 characters.".into(),
        ));
    }
    if let Some(ttl) = input.ttl_minutes {
        if ttl < 1 {
            return Err(AppError::BadRequest(
                "ttl_minutes must be >= 1.".into(),
            ));
        }
        if ttl > MAX_TTL_MINUTES {
            return Err(AppError::BadRequest(format!(
                "ttl_minutes must not exceed {MAX_TTL_MINUTES}.",
            )));
        }
    }
    Ok(())
}

/// Resolved TTL for a fresh session — clamped to [`MAX_TTL_MINUTES`].
#[must_use]
pub fn resolve_ttl(input_ttl: Option<i64>) -> Duration {
    let minutes = input_ttl
        .unwrap_or(DEFAULT_TTL_MINUTES)
        .clamp(1, MAX_TTL_MINUTES);
    Duration::minutes(minutes)
}

/// Insert a new active session. Caller is responsible for ensuring the
/// caller has the `user.impersonate` permission and that the target
/// passes the safety checks in [`assert_target_safe`].
pub async fn create(
    pool: &PgPool,
    actor_user_id: Uuid,
    actor_role: UserRole,
    target_user_id: Uuid,
    reason: &str,
    ttl: Duration,
    client_ip: Option<IpAddr>,
    user_agent: Option<&str>,
) -> AppResult<ImpersonationSession> {
    let now = Utc::now();
    let expires_at = now + ttl;
    let trimmed_reason = reason.trim();

    let ip_text = client_ip.map(|ip| ip.to_string());
    let row = sqlx::query_as::<_, ImpersonationSession>(
        r#"
        INSERT INTO impersonation_sessions (
            actor_user_id, actor_role, target_user_id,
            reason, issued_at, expires_at,
            ip_address, user_agent
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7::inet, $8)
        RETURNING
            id, actor_user_id, actor_role, target_user_id,
            reason, issued_at, expires_at,
            revoked_at, revoked_by, revoke_reason,
            host(ip_address) AS ip_address,
            user_agent
        "#,
    )
    .bind(actor_user_id)
    .bind(actor_role)
    .bind(target_user_id)
    .bind(trimmed_reason)
    .bind(now)
    .bind(expires_at)
    .bind(ip_text)
    .bind(user_agent)
    .fetch_one(pool)
    .await?;

    Ok(row)
}

/// Per-request hot-path lookup. Used by [`crate::extractors::AuthUser`]
/// to decide whether an incoming impersonation token is still honoured.
/// Returns `Ok(None)` when the row is missing, revoked, or expired —
/// the caller is expected to translate that into `AppError::Unauthorized`
/// because the JWT itself is no longer valid.
pub async fn lookup_active(
    pool: &PgPool,
    id: Uuid,
) -> Result<Option<ImpersonationSession>, sqlx::Error> {
    let row = sqlx::query_as::<_, ImpersonationSession>(
        r#"
        SELECT
            id, actor_user_id, actor_role, target_user_id,
            reason, issued_at, expires_at,
            revoked_at, revoked_by, revoke_reason,
            host(ip_address) AS ip_address,
            user_agent
        FROM impersonation_sessions
        WHERE id = $1
          AND revoked_at IS NULL
          AND expires_at > NOW()
        "#,
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;
    Ok(row)
}

/// Lookup a row regardless of revoked/expired state. Used by handlers
/// returning the session to the admin UI.
pub async fn get(pool: &PgPool, id: Uuid) -> AppResult<Option<ImpersonationSession>> {
    let row = sqlx::query_as::<_, ImpersonationSession>(
        r#"
        SELECT
            id, actor_user_id, actor_role, target_user_id,
            reason, issued_at, expires_at,
            revoked_at, revoked_by, revoke_reason,
            host(ip_address) AS ip_address,
            user_agent
        FROM impersonation_sessions
        WHERE id = $1
        "#,
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;
    Ok(row)
}

/// List currently-honoured sessions, newest first. Filters at the DB
/// level so the `idx_impersonation_sessions_active` partial index is
/// usable.
pub async fn list_active(pool: &PgPool) -> AppResult<Vec<ImpersonationSession>> {
    let rows = sqlx::query_as::<_, ImpersonationSession>(
        r#"
        SELECT
            id, actor_user_id, actor_role, target_user_id,
            reason, issued_at, expires_at,
            revoked_at, revoked_by, revoke_reason,
            host(ip_address) AS ip_address,
            user_agent
        FROM impersonation_sessions
        WHERE revoked_at IS NULL
          AND expires_at > NOW()
        ORDER BY issued_at DESC
        LIMIT 200
        "#,
    )
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

/// Mark a session revoked. Idempotent: re-revoking an already-revoked
/// row returns `Ok(None)` so the handler can surface a 404.
pub async fn revoke(
    pool: &PgPool,
    id: Uuid,
    revoked_by: Uuid,
    reason: Option<&str>,
) -> AppResult<Option<ImpersonationSession>> {
    let row = sqlx::query_as::<_, ImpersonationSession>(
        r#"
        UPDATE impersonation_sessions
           SET revoked_at    = NOW(),
               revoked_by    = $2,
               revoke_reason = $3
         WHERE id = $1
           AND revoked_at IS NULL
        RETURNING
            id, actor_user_id, actor_role, target_user_id,
            reason, issued_at, expires_at,
            revoked_at, revoked_by, revoke_reason,
            host(ip_address) AS ip_address,
            user_agent
        "#,
    )
    .bind(id)
    .bind(revoked_by)
    .bind(reason.map(str::trim).filter(|s| !s.is_empty()))
    .fetch_optional(pool)
    .await?;
    Ok(row)
}

/// Defence-in-depth: an admin must not impersonate themselves and must
/// not impersonate another admin (would let an admin "act as" another
/// admin and confuse audit attribution). Support / member targets are
/// fine.
pub async fn assert_target_safe(
    pool: &PgPool,
    actor_user_id: Uuid,
    target_user_id: Uuid,
) -> AppResult<UserRole> {
    if actor_user_id == target_user_id {
        return Err(AppError::BadRequest(
            "Cannot impersonate yourself.".into(),
        ));
    }

    let row: Option<(UserRole,)> =
        sqlx::query_as("SELECT role FROM users WHERE id = $1")
            .bind(target_user_id)
            .fetch_optional(pool)
            .await?;

    let target_role = row
        .ok_or_else(|| AppError::NotFound("Target user not found.".into()))?
        .0;

    if matches!(target_role, UserRole::Admin) {
        return Err(AppError::BadRequest(
            "Cannot impersonate another admin.".into(),
        ));
    }
    Ok(target_role)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_rejects_empty_reason() {
        let input = CreateImpersonationInput {
            target_user_id: Uuid::new_v4(),
            reason: "   ".into(),
            ttl_minutes: None,
        };
        assert!(matches!(
            validate_input(&input),
            Err(AppError::BadRequest(_))
        ));
    }

    #[test]
    fn validate_rejects_too_long_reason() {
        let input = CreateImpersonationInput {
            target_user_id: Uuid::new_v4(),
            reason: "x".repeat(501),
            ttl_minutes: None,
        };
        assert!(matches!(
            validate_input(&input),
            Err(AppError::BadRequest(_))
        ));
    }

    #[test]
    fn validate_rejects_ttl_zero_or_negative() {
        let input = CreateImpersonationInput {
            target_user_id: Uuid::new_v4(),
            reason: "ok".into(),
            ttl_minutes: Some(0),
        };
        assert!(matches!(
            validate_input(&input),
            Err(AppError::BadRequest(_))
        ));
    }

    #[test]
    fn validate_rejects_ttl_above_cap() {
        let input = CreateImpersonationInput {
            target_user_id: Uuid::new_v4(),
            reason: "ok".into(),
            ttl_minutes: Some(MAX_TTL_MINUTES + 1),
        };
        assert!(matches!(
            validate_input(&input),
            Err(AppError::BadRequest(_))
        ));
    }

    #[test]
    fn validate_accepts_default_and_clamped_inputs() {
        let input = CreateImpersonationInput {
            target_user_id: Uuid::new_v4(),
            reason: " helpdesk #1234 ".into(),
            ttl_minutes: Some(MAX_TTL_MINUTES),
        };
        assert!(validate_input(&input).is_ok());
    }

    #[test]
    fn resolve_ttl_uses_default_when_unset() {
        assert_eq!(
            resolve_ttl(None),
            Duration::minutes(DEFAULT_TTL_MINUTES)
        );
    }

    #[test]
    fn resolve_ttl_clamps_to_max() {
        assert_eq!(
            resolve_ttl(Some(MAX_TTL_MINUTES + 999)),
            Duration::minutes(MAX_TTL_MINUTES)
        );
    }

    #[test]
    fn resolve_ttl_floors_at_one() {
        assert_eq!(resolve_ttl(Some(0)), Duration::minutes(1));
    }

    #[test]
    fn is_active_respects_expires_at_and_revoked_at() {
        let base = ImpersonationSession {
            id: Uuid::new_v4(),
            actor_user_id: Uuid::new_v4(),
            actor_role: UserRole::Admin,
            target_user_id: Uuid::new_v4(),
            reason: "x".into(),
            issued_at: Utc::now() - Duration::minutes(5),
            expires_at: Utc::now() + Duration::minutes(15),
            revoked_at: None,
            revoked_by: None,
            revoke_reason: None,
            ip_address: None,
            user_agent: None,
        };
        assert!(base.is_active(Utc::now()));

        let mut expired = base.clone();
        expired.expires_at = Utc::now() - Duration::minutes(1);
        assert!(!expired.is_active(Utc::now()));

        let mut revoked = base.clone();
        revoked.revoked_at = Some(Utc::now());
        assert!(!revoked.is_active(Utc::now()));
    }
}
