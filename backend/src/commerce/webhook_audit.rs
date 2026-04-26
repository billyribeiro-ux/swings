//! EC-13: webhook-side audit log.
//!
//! `services::audit::record_admin_action` is the canonical writer for
//! human-driven mutations, but its `actor_id UUID NOT NULL REFERENCES
//! users(id)` FK rules out reusing it for webhook-driven mutations
//! (Stripe is not a `users` row and we will not weaken that FK). This
//! module wraps the dedicated `stripe_webhook_audit` table introduced by
//! migration 077; the schema mirrors the relevant columns of
//! `admin_actions` so an operator can grep both tables uniformly.
//!
//! All inserts are idempotent on `(stripe_event_id, target_kind,
//! target_id)` so a Stripe retry that races past the entry-point
//! idempotency guard never bloats the audit log.

use serde_json::Value as JsonValue;
use sqlx::PgPool;

use crate::error::AppResult;

/// Insert a webhook audit row. `target_id`/`target_kind` are nullable so
/// "global" events (e.g. a webhook with no resolvable user) still produce
/// an audit trail.
pub async fn record_webhook_audit(
    pool: &PgPool,
    stripe_event_id: &str,
    event_type: &str,
    target_kind: Option<&str>,
    target_id: Option<&str>,
    metadata: JsonValue,
) -> AppResult<()> {
    sqlx::query(
        r#"
        INSERT INTO stripe_webhook_audit
            (stripe_event_id, event_type, target_kind, target_id, metadata)
        VALUES ($1, $2, $3, $4, $5)
        ON CONFLICT (stripe_event_id, target_kind, target_id) DO NOTHING
        "#,
    )
    .bind(stripe_event_id)
    .bind(event_type)
    .bind(target_kind)
    .bind(target_id)
    .bind(metadata)
    .execute(pool)
    .await?;
    Ok(())
}

/// Best-effort variant: log + swallow the error so webhook handlers never
/// roll back a successful state mutation just because the audit row
/// failed to land. Mirrors the
/// [`crate::services::audit::record_admin_action_best_effort`] contract.
pub async fn record_webhook_audit_best_effort(
    pool: &PgPool,
    stripe_event_id: &str,
    event_type: &str,
    target_kind: Option<&str>,
    target_id: Option<&str>,
    metadata: JsonValue,
) {
    if let Err(e) = record_webhook_audit(
        pool,
        stripe_event_id,
        event_type,
        target_kind,
        target_id,
        metadata,
    )
    .await
    {
        tracing::error!(
            error = %e,
            stripe_event_id,
            event_type,
            "failed to record stripe_webhook_audit; continuing"
        );
    }
}
