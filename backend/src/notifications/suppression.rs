//! Suppression list — per-email deny list maintained by provider bounce
//! webhooks and the public unsubscribe-all path.
//!
//! The contract is simple: before any send, if the recipient address appears
//! in `notification_suppression`, the send is skipped and the delivery row is
//! recorded with `status='suppressed'`. Ops re-enables an address by deleting
//! the row through the admin API.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, PgExecutor};
use utoipa::ToSchema;

use crate::error::AppError;

/// Reason codes accepted in the `reason` column. Not an enum — new values
/// may be added by webhook handlers without a schema migration.
pub const REASON_BOUNCE_HARD: &str = "bounce_hard";
pub const REASON_COMPLAINT: &str = "complaint";
pub const REASON_UNSUBSCRIBE_ALL: &str = "user_unsubscribe_all";

/// Row shape for `notification_suppression`.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct Suppression {
    pub email: String,
    pub reason: String,
    pub suppressed_at: DateTime<Utc>,
}

/// Is this e-mail currently suppressed?
pub async fn is_suppressed<'e, E: PgExecutor<'e>>(
    executor: E,
    email: &str,
) -> Result<bool, AppError> {
    let row: Option<(String,)> =
        sqlx::query_as("SELECT email FROM notification_suppression WHERE email = $1 LIMIT 1")
            .bind(email)
            .fetch_optional(executor)
            .await?;
    Ok(row.is_some())
}

/// Add (or replace) a suppression entry.
///
/// Bounce / complaint webhooks call this with a concrete reason; the
/// unsubscribe-all path uses [`REASON_UNSUBSCRIBE_ALL`].
pub async fn suppress<'e, E: PgExecutor<'e>>(
    executor: E,
    email: &str,
    reason: &str,
) -> Result<Suppression, AppError> {
    let row = sqlx::query_as::<_, Suppression>(
        r#"
        INSERT INTO notification_suppression (email, reason, suppressed_at)
        VALUES ($1, $2, NOW())
        ON CONFLICT (email) DO UPDATE SET
            reason = EXCLUDED.reason,
            suppressed_at = NOW()
        RETURNING email, reason, suppressed_at
        "#,
    )
    .bind(email)
    .bind(reason)
    .fetch_one(executor)
    .await?;
    Ok(row)
}

/// Remove an e-mail from the suppression list. Returns `true` if a row was
/// removed, `false` if the address was not suppressed.
pub async fn unsuppress<'e, E: PgExecutor<'e>>(executor: E, email: &str) -> Result<bool, AppError> {
    let rows = sqlx::query("DELETE FROM notification_suppression WHERE email = $1")
        .bind(email)
        .execute(executor)
        .await?;
    Ok(rows.rows_affected() > 0)
}

/// Paginated list for the admin ops endpoint.
pub async fn list<'e, E: PgExecutor<'e>>(
    executor: E,
    limit: i64,
    offset: i64,
) -> Result<Vec<Suppression>, AppError> {
    let rows = sqlx::query_as::<_, Suppression>(
        r#"
        SELECT email, reason, suppressed_at
        FROM notification_suppression
        ORDER BY suppressed_at DESC
        LIMIT $1 OFFSET $2
        "#,
    )
    .bind(limit)
    .bind(offset)
    .fetch_all(executor)
    .await?;
    Ok(rows)
}

/// Row count for pagination.
pub async fn count<'e, E: PgExecutor<'e>>(executor: E) -> Result<i64, AppError> {
    let n: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM notification_suppression")
        .fetch_one(executor)
        .await?;
    Ok(n)
}
