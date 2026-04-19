//! ADM-20: garbage collector for the `idempotency_keys` cache.
//!
//! The Idempotency-Key middleware (`middleware::idempotency`) writes
//! one row per admin POST and stamps `expires_at = now() + 24h`.
//! Without active pruning the table grows unbounded — a busy admin
//! plane will accumulate millions of rows over a year and drag down
//! the index hit rate on every claim.
//!
//! ## Strategy
//!
//! Each iteration:
//!   1. Reads the operator-tunable `idempotency.gc_batch_size` from
//!      [`SettingsCache`] (default 1000, hard cap 10 000 to bound the
//!      lock window).
//!   2. Repeatedly issues a bounded `DELETE … WHERE ctid = ANY (
//!      SELECT ctid … LIMIT $1 FOR UPDATE SKIP LOCKED)` until a pass
//!      removes fewer than `batch_size` rows or the iteration cap
//!      [`MAX_PASSES_PER_TICK`] is hit. The CTE-style delete keeps
//!      lock duration predictable even when the backlog is huge.
//!   3. Reports the row count of the table as a gauge so SRE can
//!      alarm on runaway growth without sampling `pg_stat_user_tables`.
//!
//! ## Why this is a separate worker (not a cron / pg_cron job)
//!
//! * The same shutdown signal that drains in-flight requests also
//!   drains this loop, so a deploy never leaks a half-finished prune.
//! * Settings live in `app_settings`, the same source of truth as the
//!   live admin policy — tuning batch size from the admin UI takes
//!   effect on the next tick with no SQL access required.
//! * Metrics emit through the same Prometheus exporter as the rest of
//!   the admin plane.

use std::time::{Duration, Instant};

use sqlx::PgPool;
use tokio::sync::broadcast;

use crate::settings::Cache as SettingsCache;

const KEY_GC_BATCH_SIZE: &str = "idempotency.gc_batch_size";
const DEFAULT_BATCH_SIZE: i64 = 1_000;
/// Hard ceiling regardless of operator input. 10 000 deletes against
/// the (user_id, key) PK plus the two secondary indexes runs in
/// ~50ms on managed Postgres; larger batches start to compete with
/// admin POSTs for `idempotency_keys` row locks.
const MAX_BATCH_SIZE: i64 = 10_000;
/// Bound on the number of `prune_pass` invocations per iteration so a
/// massive backlog can't monopolise a worker tick. The next tick picks
/// up where this one left off.
const MAX_PASSES_PER_TICK: usize = 20;

pub fn current_batch_size(settings: &SettingsCache) -> i64 {
    settings
        .get(KEY_GC_BATCH_SIZE)
        .and_then(|r| r.value.as_i64())
        .unwrap_or(DEFAULT_BATCH_SIZE)
        .clamp(1, MAX_BATCH_SIZE)
}

/// Single bounded DELETE pass. Returns the number of rows actually
/// removed. Uses `ctid = ANY(...)` so Postgres can use the heap-tid
/// rather than re-walking `expires_at_idx` for the second time.
pub async fn prune_pass(pool: &PgPool, batch_size: i64) -> Result<u64, sqlx::Error> {
    let result = sqlx::query(
        r#"
        DELETE FROM idempotency_keys
         WHERE ctid IN (
            SELECT ctid
              FROM idempotency_keys
             WHERE expires_at < NOW()
             ORDER BY expires_at ASC
             LIMIT $1
             FOR UPDATE SKIP LOCKED
         )
        "#,
    )
    .bind(batch_size)
    .execute(pool)
    .await?;
    Ok(result.rows_affected())
}

/// Drain expired rows for one worker tick. Returns the total number
/// of rows removed across all passes. Exposed so integration tests
/// can drive the GC deterministically without a sleeping loop.
pub async fn prune_once(pool: &PgPool, settings: &SettingsCache) -> u64 {
    let batch_size = current_batch_size(settings);
    let started = Instant::now();
    let mut total: u64 = 0;
    for _ in 0..MAX_PASSES_PER_TICK {
        match prune_pass(pool, batch_size).await {
            Ok(removed) => {
                total += removed;
                if (removed as i64) < batch_size {
                    break;
                }
            }
            Err(err) => {
                metrics::counter!("idempotency_keys_prune_failed_total").increment(1);
                tracing::error!(error = %err, "idempotency-gc prune pass failed");
                break;
            }
        }
    }
    metrics::counter!("idempotency_keys_pruned_total").increment(total);
    metrics::histogram!("idempotency_keys_prune_duration_seconds")
        .record(started.elapsed().as_secs_f64());
    metrics::gauge!("idempotency_keys_prune_last_success_unixtime")
        .set(chrono::Utc::now().timestamp() as f64);
    if let Some(rows) = sample_table_rows(pool).await {
        metrics::gauge!("idempotency_keys_table_rows").set(rows as f64);
    }
    if total > 0 {
        tracing::info!(removed = total, "idempotency-gc tick complete");
    }
    total
}

/// Best-effort sample of the cache table's current row count. Uses
/// `pg_class.reltuples` so this stays cheap even if the table grows
/// to millions of rows; the precision (tied to the latest `ANALYZE`)
/// is fine for an SLO gauge.
async fn sample_table_rows(pool: &PgPool) -> Option<i64> {
    sqlx::query_scalar::<_, f32>(
        "SELECT reltuples FROM pg_class WHERE relname = 'idempotency_keys'",
    )
    .fetch_optional(pool)
    .await
    .ok()
    .flatten()
    .map(|f| f as i64)
}

/// Long-lived worker. Spawn once per process from `main.rs`. Sleeps
/// `interval` between iterations and exits when the broadcast
/// `shutdown` fires.
pub async fn run_loop(
    pool: PgPool,
    settings: SettingsCache,
    mut shutdown: broadcast::Receiver<()>,
    interval: Duration,
) {
    tracing::info!(
        interval_secs = interval.as_secs(),
        "idempotency-gc worker started"
    );
    let mut ticker = tokio::time::interval(interval);
    ticker.tick().await; // discard the immediate first tick
    loop {
        tokio::select! {
            _ = ticker.tick() => {
                let _ = prune_once(&pool, &settings).await;
            }
            _ = shutdown.recv() => {
                tracing::info!("idempotency-gc worker received shutdown");
                return;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn batch_size_default_when_missing() {
        let cache = SettingsCache::new();
        assert_eq!(current_batch_size(&cache), DEFAULT_BATCH_SIZE);
    }

    #[test]
    fn batch_size_clamps_above_ceiling() {
        let cache = SettingsCache::new();
        cache.insert_for_tests(KEY_GC_BATCH_SIZE, serde_json::json!(50_000));
        assert_eq!(current_batch_size(&cache), MAX_BATCH_SIZE);
    }

    #[test]
    fn batch_size_clamps_below_floor() {
        let cache = SettingsCache::new();
        cache.insert_for_tests(KEY_GC_BATCH_SIZE, serde_json::json!(0));
        assert_eq!(current_batch_size(&cache), 1);
    }

    #[test]
    fn batch_size_honours_operator_value_within_bounds() {
        let cache = SettingsCache::new();
        cache.insert_for_tests(KEY_GC_BATCH_SIZE, serde_json::json!(2_500));
        assert_eq!(current_batch_size(&cache), 2_500);
    }
}
