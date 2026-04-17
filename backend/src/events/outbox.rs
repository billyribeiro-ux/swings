//! Outbox row-level operations: publish, claim, mark-delivered, backoff, dead-letter.
//!
//! Producers write events with [`publish_in_tx`] inside the same [`sqlx::Transaction`]
//! as their domain mutation; a pool of worker tasks later leases rows via
//! [`claim_batch`] (`SELECT … FOR UPDATE SKIP LOCKED`), flips them to `in_flight`,
//! dispatches, and records the result with [`mark_delivered`], [`mark_failed_with_backoff`],
//! or [`mark_dead_letter`].
//!
//! Exponential backoff uses the formula `2^attempts * 1s ± 20% jitter`, per §FDN-04 of
//! the implementation plan. `max_attempts` is loaded from the row itself (default 8) so
//! operators can override per-row for high-value events.

use chrono::{DateTime, Utc};
use rand::Rng;
use serde::{Deserialize, Serialize};
use sqlx::{postgres::PgRow, FromRow, PgPool, Postgres, Row, Transaction};
use std::time::Duration;
use utoipa::ToSchema;
use uuid::Uuid;

/// Lifecycle states for an outbox row. Mirrors the `CHECK` constraint in
/// `019_outbox.sql`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum OutboxStatus {
    /// Awaiting a worker lease.
    Pending,
    /// Leased by a worker (`claim_batch`) and being dispatched.
    InFlight,
    /// Successfully handed off to every matching subscriber.
    Delivered,
    /// Transient failure; will retry until `attempts >= max_attempts`.
    Failed,
    /// Terminal: exceeded `max_attempts`. Requires operator intervention.
    DeadLetter,
}

impl OutboxStatus {
    fn as_str(self) -> &'static str {
        match self {
            OutboxStatus::Pending => "pending",
            OutboxStatus::InFlight => "in_flight",
            OutboxStatus::Delivered => "delivered",
            OutboxStatus::Failed => "failed",
            OutboxStatus::DeadLetter => "dead_letter",
        }
    }

    /// Parse the text stored in the `status` column.
    pub fn from_db(s: &str) -> Option<Self> {
        Some(match s {
            "pending" => OutboxStatus::Pending,
            "in_flight" => OutboxStatus::InFlight,
            "delivered" => OutboxStatus::Delivered,
            "failed" => OutboxStatus::Failed,
            "dead_letter" => OutboxStatus::DeadLetter,
            _ => return None,
        })
    }
}

/// Standard extension headers travelling with every event. Kept deliberately small so
/// the row stays cheap to index / replicate; add free-form per-handler metadata to the
/// `payload` JSON instead.
#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct EventHeaders {
    /// Producer-supplied idempotency key. Subscribers are expected to de-duplicate on
    /// this key when they cannot tolerate at-least-once delivery re-processing.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub idempotency_key: Option<String>,
    /// Request-scoped trace id propagated from the origin span.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub correlation_id: Option<String>,
    /// Tenant scope, for future multi-tenant fan-out.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tenant: Option<String>,
}

/// Producer-side event payload. [`Event::publish`] is the sole public entry point —
/// bypassing the struct into a raw SQL insert would lose the JSON serialization
/// contract enforced here.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Event {
    pub aggregate_type: String,
    pub aggregate_id: String,
    pub event_type: String,
    pub payload: serde_json::Value,
    #[serde(default)]
    pub headers: EventHeaders,
}

/// Row-shaped view used by workers and the admin ops endpoints.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct OutboxRecord {
    pub id: Uuid,
    pub aggregate_type: String,
    pub aggregate_id: String,
    pub event_type: String,
    pub payload: serde_json::Value,
    pub headers: serde_json::Value,
    pub status: OutboxStatus,
    pub attempts: i32,
    pub max_attempts: i32,
    pub next_attempt_at: DateTime<Utc>,
    pub last_error: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl<'r> FromRow<'r, PgRow> for OutboxRecord {
    fn from_row(row: &'r PgRow) -> Result<Self, sqlx::Error> {
        let status_raw: String = row.try_get("status")?;
        let status = OutboxStatus::from_db(&status_raw).ok_or_else(|| {
            sqlx::Error::Decode(format!("unknown outbox status: {status_raw}").into())
        })?;
        Ok(OutboxRecord {
            id: row.try_get("id")?,
            aggregate_type: row.try_get("aggregate_type")?,
            aggregate_id: row.try_get("aggregate_id")?,
            event_type: row.try_get("event_type")?,
            payload: row.try_get("payload")?,
            headers: row.try_get("headers")?,
            status,
            attempts: row.try_get("attempts")?,
            max_attempts: row.try_get("max_attempts")?,
            next_attempt_at: row.try_get("next_attempt_at")?,
            last_error: row.try_get("last_error")?,
            created_at: row.try_get("created_at")?,
            updated_at: row.try_get("updated_at")?,
        })
    }
}

/// Errors raised by outbox row-level operations. Kept narrow so `Dispatcher` /
/// `Worker` can distinguish infrastructure failures from DB drift.
#[derive(Debug, thiserror::Error)]
pub enum OutboxError {
    #[error("database error: {0}")]
    Database(#[from] sqlx::Error),
    #[error("payload serialization failed: {0}")]
    Serialize(#[from] serde_json::Error),
}

/// Insert an outbox row inside the caller's transaction. The row is committed
/// atomically with whatever domain mutation precedes or follows it — that is the
/// entire point of the outbox pattern.
///
/// Returns the new row id so the caller can correlate back in logs / tests.
pub async fn publish_in_tx(
    tx: &mut Transaction<'_, Postgres>,
    event: &Event,
) -> Result<Uuid, OutboxError> {
    let headers_json = serde_json::to_value(&event.headers)?;
    let row = sqlx::query(
        r#"
        INSERT INTO outbox_events (aggregate_type, aggregate_id, event_type, payload, headers)
        VALUES ($1, $2, $3, $4, $5)
        RETURNING id
        "#,
    )
    .bind(&event.aggregate_type)
    .bind(&event.aggregate_id)
    .bind(&event.event_type)
    .bind(&event.payload)
    .bind(&headers_json)
    .fetch_one(&mut **tx)
    .await?;
    let id: Uuid = row.try_get("id")?;
    Ok(id)
}

/// Atomically lease up to `limit` rows that are ready to dispatch. Rows returned
/// here are flipped to `in_flight` and visible to no other worker thanks to
/// `FOR UPDATE SKIP LOCKED`. Uses a CTE so the lock + update + return is one
/// round-trip per worker tick.
pub async fn claim_batch(pool: &PgPool, limit: i64) -> Result<Vec<OutboxRecord>, OutboxError> {
    let rows = sqlx::query_as::<_, OutboxRecord>(
        r#"
        WITH cte AS (
            SELECT id
            FROM outbox_events
            WHERE status IN ('pending', 'in_flight')
              AND next_attempt_at <= NOW()
            ORDER BY next_attempt_at ASC
            FOR UPDATE SKIP LOCKED
            LIMIT $1
        )
        UPDATE outbox_events oe
        SET status = 'in_flight',
            updated_at = NOW()
        FROM cte
        WHERE oe.id = cte.id
        RETURNING oe.id, oe.aggregate_type, oe.aggregate_id, oe.event_type,
                  oe.payload, oe.headers, oe.status, oe.attempts, oe.max_attempts,
                  oe.next_attempt_at, oe.last_error, oe.created_at, oe.updated_at
        "#,
    )
    .bind(limit)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

/// Final success state — the row is settled and will not be re-dispatched.
pub async fn mark_delivered(pool: &PgPool, id: Uuid) -> Result<(), OutboxError> {
    sqlx::query(
        r#"
        UPDATE outbox_events
        SET status = 'delivered',
            last_error = NULL,
            updated_at = NOW()
        WHERE id = $1
        "#,
    )
    .bind(id)
    .execute(pool)
    .await?;
    Ok(())
}

/// Transient failure — record the cause, bump `attempts`, and schedule the next
/// attempt with exponential backoff + jitter. The caller decides whether the row
/// graduates to `dead_letter` by comparing `attempts` to `max_attempts` (see
/// [`Worker`](super::Worker)).
pub async fn mark_failed_with_backoff(
    pool: &PgPool,
    id: Uuid,
    attempts_so_far: i32,
    last_error: &str,
) -> Result<DateTime<Utc>, OutboxError> {
    let delay = backoff_delay(attempts_so_far);
    let now = Utc::now();
    let next_attempt_at =
        now + chrono::Duration::from_std(delay).unwrap_or(chrono::Duration::seconds(60));
    sqlx::query(
        r#"
        UPDATE outbox_events
        SET status = 'failed',
            attempts = attempts + 1,
            next_attempt_at = $2,
            last_error = $3,
            updated_at = NOW()
        WHERE id = $1
        "#,
    )
    .bind(id)
    .bind(next_attempt_at)
    .bind(last_error)
    .execute(pool)
    .await?;
    Ok(next_attempt_at)
}

/// Terminal failure — worker has exhausted `max_attempts`. Operators can
/// reset via `POST /api/admin/outbox/{id}/retry`.
pub async fn mark_dead_letter(
    pool: &PgPool,
    id: Uuid,
    last_error: &str,
) -> Result<(), OutboxError> {
    sqlx::query(
        r#"
        UPDATE outbox_events
        SET status = 'dead_letter',
            attempts = attempts + 1,
            last_error = $2,
            updated_at = NOW()
        WHERE id = $1
        "#,
    )
    .bind(id)
    .bind(last_error)
    .execute(pool)
    .await?;
    Ok(())
}

/// Exponential backoff with ±20% jitter. Purely arithmetic — no I/O — so it's
/// covered by an inline unit test.
///
/// Formula: `base = 2^attempts * 1s`, clamped to avoid silly numbers past
/// `attempts ≈ 20`. Jitter is a multiplicative factor uniformly drawn from
/// `[0.8, 1.2]`.
pub fn backoff_delay(attempts_so_far: i32) -> Duration {
    let clamped = attempts_so_far.clamp(0, 20) as u32;
    // `2^clamped` — `clamped` is already bounded by the `clamp` above, so a
    // direct shift is safe. `checked_shl` returns None for shifts >= 64, which
    // we map to u64::MAX before the `.min(60*60)` cap below.
    let base_secs: u64 = 1u64.checked_shl(clamped).unwrap_or(u64::MAX);
    // Cap at ~1 hour so a DLQ-eligible row does not linger for days.
    let capped_secs = base_secs.min(60 * 60);
    let mut rng = rand::thread_rng();
    let jitter: f64 = rng.gen_range(0.8..=1.2);
    let jittered = (capped_secs as f64 * jitter).max(0.1);
    Duration::from_millis((jittered * 1000.0) as u64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backoff_is_bounded_by_jitter() {
        // attempts=0 → 1s * [0.8,1.2] = [800ms, 1200ms].
        for _ in 0..200 {
            let d = backoff_delay(0).as_millis();
            assert!(
                (800..=1200).contains(&(d as u64)),
                "base 1s delay out of [800,1200]ms band: {d}",
            );
        }
    }

    #[test]
    fn backoff_grows_monotonically_on_median() {
        // The jittered distributions overlap for consecutive `attempts`, but their
        // means differ by 2×. Sample many draws and compare means — that is what the
        // worker effectively experiences in aggregate.
        fn mean_ms(n: u32, samples: usize) -> f64 {
            let total: u64 = (0..samples)
                .map(|_| backoff_delay(n as i32).as_millis() as u64)
                .sum();
            total as f64 / samples as f64
        }
        let m0 = mean_ms(0, 500);
        let m1 = mean_ms(1, 500);
        let m2 = mean_ms(2, 500);
        assert!(m1 > m0 * 1.5, "m1={m1} not materially > m0={m0}");
        assert!(m2 > m1 * 1.5, "m2={m2} not materially > m1={m1}");
    }

    #[test]
    fn backoff_caps_at_one_hour() {
        // `attempts=20` would otherwise be 2^20 ≈ 12 days.
        let d = backoff_delay(20).as_secs();
        // With the 1h cap and ±20% jitter: upper bound ≈ 3600*1.2 = 4320s.
        assert!(d <= 4320, "cap leaked: {d}s");
    }

    #[test]
    fn outbox_status_round_trip() {
        for s in [
            OutboxStatus::Pending,
            OutboxStatus::InFlight,
            OutboxStatus::Delivered,
            OutboxStatus::Failed,
            OutboxStatus::DeadLetter,
        ] {
            assert_eq!(OutboxStatus::from_db(s.as_str()), Some(s));
        }
        assert_eq!(OutboxStatus::from_db("garbage"), None);
    }
}
