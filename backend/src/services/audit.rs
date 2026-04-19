//! ADM-01: append-only admin / support audit log writer.
//!
//! Every privileged action that mutates a user, subscription, order, role,
//! coupon, course enrollment, consent record, or impersonation token MUST
//! be persisted via [`record_admin_action`]. The function is the single
//! supported writer for the `admin_actions` table introduced in migration
//! `055_admin_actions.sql`.
//!
//! ## Design
//!
//! * **Append-only.** Callers never UPDATE or DELETE; retention is owned
//!   by a future scheduled job. The schema does not grant the application
//!   role any mutating privilege beyond INSERT.
//! * **Generic.** `target_kind` carries the resource type as a free-text
//!   tag (`"user"`, `"subscription"`, `"order"`, `"role"`, …) so new
//!   domains adopt the table without further schema work.
//! * **Failure-tolerant by default.** [`record_admin_action`] returns the
//!   underlying [`AppError`] so the caller can decide whether a logging
//!   failure should fail the request. The convenience wrapper
//!   [`record_admin_action_best_effort`] swallows + logs errors and is the
//!   right call for handlers where the audit row is observability rather
//!   than authorisation evidence.
//! * **Ergonomic call sites.** Use [`AdminAction`] to construct the row
//!   inline; the builder pattern keeps verbose handler bodies readable.

use std::net::IpAddr;

use serde_json::Value as JsonValue;
use sqlx::PgPool;
use uuid::Uuid;

use crate::error::{AppError, AppResult};
use crate::extractors::ClientInfo;
use crate::models::UserRole;

/// Structured payload for a single audit row.
///
/// Construct with [`AdminAction::new`] and fill in the optional fields with
/// the builder methods. All `with_*` methods return `Self` so the call
/// fits on one statement at the handler call site.
#[derive(Debug, Clone)]
pub struct AdminAction {
    /// User id of the admin / support agent that initiated the action.
    pub actor_id: Uuid,
    /// Role the actor presented at the time of the action — captured at
    /// write time so a later role change does not retroactively alter the
    /// audit trail.
    pub actor_role: UserRole,
    /// Dot-delimited verb describing the action, e.g. `"user.suspend"` or
    /// `"subscription.cancel"`. Conventionally the same key used to
    /// authorise the request via the FDN-07 policy engine.
    pub action: &'static str,
    /// Resource type the action targets — `"user"`, `"subscription"`,
    /// `"order"`, `"role"`, `"coupon"`, etc. Free-text by design.
    pub target_kind: &'static str,
    /// Optional resource identifier. Stored as TEXT so non-UUID
    /// identifiers (e.g. Stripe ids) remain expressible.
    pub target_id: Option<String>,
    /// Source IP captured from the request. `None` for background work.
    pub ip_address: Option<IpAddr>,
    /// Verbatim `User-Agent` header captured from the request, truncated
    /// to a sane upper bound by the writer.
    pub user_agent: Option<String>,
    /// Structured JSON payload — reason text, before/after diffs, etc.
    pub metadata: JsonValue,
}

impl AdminAction {
    /// Build the minimum-viable action row. Optional context lands via
    /// [`Self::with_target_id`], [`Self::with_metadata`], and
    /// [`Self::with_client`].
    #[must_use]
    pub fn new(
        actor_id: Uuid,
        actor_role: UserRole,
        action: &'static str,
        target_kind: &'static str,
    ) -> Self {
        Self {
            actor_id,
            actor_role,
            action,
            target_kind,
            target_id: None,
            ip_address: None,
            user_agent: None,
            metadata: JsonValue::Object(Default::default()),
        }
    }

    /// Attach an arbitrary identifier for the resource the action targets.
    #[must_use]
    pub fn with_target_id<T: ToString>(mut self, id: T) -> Self {
        self.target_id = Some(id.to_string());
        self
    }

    /// Attach a structured JSON payload (reason, diff, etc.).
    #[must_use]
    pub fn with_metadata(mut self, metadata: JsonValue) -> Self {
        self.metadata = metadata;
        self
    }

    /// Pull source IP + UA off a [`ClientInfo`] extracted from the request.
    #[must_use]
    pub fn with_client(mut self, client: &ClientInfo) -> Self {
        self.ip_address = client.ip;
        self.user_agent = client.user_agent.clone();
        self
    }
}

/// Insert an [`AdminAction`] row. Returns the generated id.
///
/// Errors propagate as [`AppError::Database`] so the caller can decide
/// whether a logging failure should fail the request. Most call sites
/// should prefer [`record_admin_action_best_effort`] — the audit row is
/// observability, not authorisation evidence.
pub async fn record_admin_action(pool: &PgPool, entry: AdminAction) -> AppResult<Uuid> {
    let user_agent_truncated = entry.user_agent.as_deref().map(truncate_user_agent);
    let ip_text = entry.ip_address.map(|ip| ip.to_string());

    let id: (Uuid,) = sqlx::query_as(
        r#"
        INSERT INTO admin_actions
            (actor_id, actor_role, action, target_kind, target_id, ip_address, user_agent, metadata)
        VALUES
            ($1, $2::user_role, $3, $4, $5, $6::inet, $7, $8)
        RETURNING id
        "#,
    )
    .bind(entry.actor_id)
    .bind(entry.actor_role.as_str())
    .bind(entry.action)
    .bind(entry.target_kind)
    .bind(&entry.target_id)
    .bind(&ip_text)
    .bind(&user_agent_truncated)
    .bind(&entry.metadata)
    .fetch_one(pool)
    .await
    .map_err(AppError::from)?;

    tracing::info!(
        admin_action.id = %id.0,
        admin_action.actor_id = %entry.actor_id,
        admin_action.action = entry.action,
        admin_action.target_kind = entry.target_kind,
        admin_action.target_id = entry.target_id.as_deref().unwrap_or(""),
        "admin action recorded"
    );

    Ok(id.0)
}

/// Best-effort variant: log the failure and continue.
///
/// Use this when the audit row is observability rather than authorisation
/// evidence. A failed insert here does not fail the user-facing request.
pub async fn record_admin_action_best_effort(pool: &PgPool, entry: AdminAction) -> Option<Uuid> {
    let action = entry.action;
    let target_kind = entry.target_kind;
    let actor_id = entry.actor_id;
    match record_admin_action(pool, entry).await {
        Ok(id) => Some(id),
        Err(e) => {
            tracing::error!(
                error = %e,
                actor_id = %actor_id,
                action,
                target_kind,
                "failed to record admin_action; continuing"
            );
            None
        }
    }
}

/// Cap the stored UA at 1 KiB. Some bot UAs run to many kilobytes; we
/// neither need nor want to persist them in their entirety.
fn truncate_user_agent(ua: &str) -> String {
    const MAX: usize = 1024;
    if ua.len() <= MAX {
        ua.to_string()
    } else {
        let mut s = ua.as_bytes()[..MAX].to_vec();
        // Defensive: ensure we never split a multi-byte character.
        while !std::str::from_utf8(&s).is_ok() && !s.is_empty() {
            s.pop();
        }
        String::from_utf8(s).unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truncate_user_agent_caps_long_strings() {
        let ua = "x".repeat(2000);
        let truncated = truncate_user_agent(&ua);
        assert_eq!(truncated.len(), 1024);
    }

    #[test]
    fn truncate_user_agent_passes_short_strings_unchanged() {
        let ua = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7)";
        assert_eq!(truncate_user_agent(ua), ua);
    }

    #[test]
    fn truncate_user_agent_preserves_utf8_boundary() {
        // Every char is 4 bytes; if the truncation boundary lands mid-char
        // the helper must back off until the slice is valid.
        let ua = "🦀".repeat(300); // 1200 bytes
        let truncated = truncate_user_agent(&ua);
        assert!(truncated.is_char_boundary(truncated.len()));
        assert!(std::str::from_utf8(truncated.as_bytes()).is_ok());
        assert!(truncated.len() <= 1024);
    }

    #[test]
    fn admin_action_builder_threads_optionals() {
        let actor = Uuid::new_v4();
        let target = Uuid::new_v4();
        let action = AdminAction::new(actor, UserRole::Admin, "user.suspend", "user")
            .with_target_id(target)
            .with_metadata(serde_json::json!({"reason": "spam"}));

        assert_eq!(action.actor_id, actor);
        assert_eq!(action.actor_role, UserRole::Admin);
        assert_eq!(action.action, "user.suspend");
        assert_eq!(action.target_kind, "user");
        assert_eq!(action.target_id.as_deref(), Some(target.to_string().as_str()));
        assert_eq!(action.metadata["reason"], "spam");
        assert!(action.ip_address.is_none());
        assert!(action.user_agent.is_none());
    }
}
