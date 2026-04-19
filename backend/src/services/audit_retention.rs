//! ADM-16: audit-log retention sweeper.
//!
//! `admin_actions` is append-only by design — handlers cannot UPDATE or
//! DELETE rows. To keep the table from growing without bound while still
//! satisfying SOX / ISO 27001 / GDPR retention requirements, this module
//! provides a batched pruner driven by two settings:
//!
//! * `audit.retention_days` — rows older than this many days are eligible
//!   for deletion. Setting this to `0` disables pruning entirely (useful
//!   for compliance modes that demand indefinite retention).
//! * `audit.prune_batch_size` — maximum rows deleted per iteration. Keeps
//!   the lock window short so the worker does not contend with hot
//!   admin-write traffic.
//!
//! The settings live in `app_settings` (migration `072_audit_retention`)
//! so operators can tune retention from the admin UI without redeploying.
//!
//! ## Worker contract
//!
//! [`run_loop`] is intended to be `tokio::spawn`-ed once per process. It
//! reads the settings on every tick (so a UI change takes effect on the
//! next sweep) and exits cleanly when the shutdown receiver fires. The
//! caller owns the `JoinHandle` and is responsible for awaiting it during
//! graceful shutdown.
//!
//! ## Metrics
//!
//! * `audit_pruned_total{kind="rows"}` — counter of rows actually deleted.
//! * `audit_prune_iterations_total` — counter of completed sweep
//!   iterations (excludes early-out for retention=0 / errors).
//! * `audit_prune_errors_total` — counter of failed sweep iterations.
//! * `audit_prune_duration_seconds` — histogram of per-iteration runtime.
//! * `audit_prune_last_success_unixtime` — gauge updated after each
//!   successful iteration; alerts can fire on staleness.

use std::time::Duration;

use sqlx::PgPool;
use tokio::sync::broadcast;
use tokio::time::Instant;

use crate::settings::Cache as SettingsCache;

const KEY_RETENTION_DAYS: &str = "audit.retention_days";
const KEY_PRUNE_BATCH_SIZE: &str = "audit.prune_batch_size";

const DEFAULT_RETENTION_DAYS: i64 = 365;
const DEFAULT_PRUNE_BATCH_SIZE: i64 = 5_000;

/// Hard cap on per-iteration deletions, regardless of what the operator
/// configures. Prevents a fat-fingered "1_000_000_000" from holding the
/// table lock for minutes.
const MAX_BATCH_SIZE: i64 = 100_000;

/// Read the operator-configured retention. Returns `None` when retention
/// is explicitly disabled (key set to `0`).
pub fn current_retention_days(settings: &SettingsCache) -> Option<i64> {
    let raw = get_i64(settings, KEY_RETENTION_DAYS, DEFAULT_RETENTION_DAYS);
    if raw <= 0 {
        None
    } else {
        Some(raw)
    }
}

pub fn current_batch_size(settings: &SettingsCache) -> i64 {
    let raw = get_i64(settings, KEY_PRUNE_BATCH_SIZE, DEFAULT_PRUNE_BATCH_SIZE);
    raw.clamp(1, MAX_BATCH_SIZE)
}

fn get_i64(settings: &SettingsCache, key: &str, default: i64) -> i64 {
    settings
        .get(key)
        .and_then(|r| r.value.as_i64())
        .unwrap_or(default)
}

/// Delete every row in `admin_actions` older than `retention_days` in
/// batches of `batch_size`. Returns the total number of rows removed.
///
/// Implementation note: the inner DELETE uses a CTE to bound the
/// affected row set. `LIMIT` cannot appear directly on `DELETE` in
/// PostgreSQL, but `DELETE … WHERE id IN (SELECT id … LIMIT n)` gives us
/// the same shape with a single planner-friendly query.
pub async fn prune_once(
    pool: &PgPool,
    retention_days: i64,
    batch_size: i64,
) -> Result<u64, sqlx::Error> {
    let mut total_deleted: u64 = 0;
    loop {
        let result = sqlx::query(
            r#"
            DELETE FROM admin_actions
            WHERE id IN (
                SELECT id FROM admin_actions
                WHERE created_at < NOW() - ($1::bigint * INTERVAL '1 day')
                ORDER BY created_at ASC
                LIMIT $2
            )
            "#,
        )
        .bind(retention_days)
        .bind(batch_size)
        .execute(pool)
        .await?;

        let deleted = result.rows_affected();
        total_deleted += deleted;
        if deleted < batch_size as u64 {
            break;
        }
        // Cooperative yield between batches: prevents one sweep from
        // monopolising the connection pool when the backlog is huge.
        tokio::task::yield_now().await;
    }
    Ok(total_deleted)
}

/// Long-lived worker. Sleeps `interval` between iterations and exits
/// when the broadcast `shutdown` fires. Defensive: a transient DB error
/// is logged + counted, and the loop continues on the next tick.
pub async fn run_loop(
    pool: PgPool,
    settings: SettingsCache,
    mut shutdown: broadcast::Receiver<()>,
    interval: Duration,
) {
    tracing::info!(
        interval_secs = interval.as_secs(),
        "audit-retention worker started"
    );
    let mut ticker = tokio::time::interval(interval);
    // Drop the immediate first tick — the boot sequence has enough
    // pressure on the DB; let the first sweep happen one interval in.
    ticker.tick().await;
    loop {
        tokio::select! {
            _ = ticker.tick() => {
                run_iteration(&pool, &settings).await;
            }
            _ = shutdown.recv() => {
                tracing::info!("audit-retention worker received shutdown");
                return;
            }
        }
    }
}

async fn run_iteration(pool: &PgPool, settings: &SettingsCache) {
    let Some(retention_days) = current_retention_days(settings) else {
        // Pruning explicitly disabled — record the iteration so the
        // gauge stays fresh and alerts don't false-positive.
        update_last_success_gauge();
        return;
    };
    let batch_size = current_batch_size(settings);
    let started = Instant::now();
    match prune_once(pool, retention_days, batch_size).await {
        Ok(deleted) => {
            metrics::counter!("audit_pruned_total", "kind" => "rows").increment(deleted);
            metrics::counter!("audit_prune_iterations_total").increment(1);
            metrics::histogram!("audit_prune_duration_seconds")
                .record(started.elapsed().as_secs_f64());
            update_last_success_gauge();
            if deleted > 0 {
                tracing::info!(
                    deleted,
                    retention_days,
                    batch_size,
                    "audit-retention sweep completed"
                );
            }
        }
        Err(err) => {
            metrics::counter!("audit_prune_errors_total").increment(1);
            tracing::error!(error = %err, "audit-retention sweep failed");
        }
    }
}

fn update_last_success_gauge() {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as f64)
        .unwrap_or(0.0);
    metrics::gauge!("audit_prune_last_success_unixtime").set(now);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn current_retention_days_zero_disables() {
        let cache = SettingsCache::new();
        cache.insert_for_tests(KEY_RETENTION_DAYS, serde_json::json!(0));
        assert_eq!(current_retention_days(&cache), None);
    }

    #[test]
    fn current_retention_days_negative_disables() {
        let cache = SettingsCache::new();
        cache.insert_for_tests(KEY_RETENTION_DAYS, serde_json::json!(-7));
        assert_eq!(current_retention_days(&cache), None);
    }

    #[test]
    fn current_retention_days_uses_default_when_missing() {
        let cache = SettingsCache::new();
        assert_eq!(current_retention_days(&cache), Some(DEFAULT_RETENTION_DAYS));
    }

    #[test]
    fn current_batch_size_clamps_to_max() {
        let cache = SettingsCache::new();
        cache.insert_for_tests(KEY_PRUNE_BATCH_SIZE, serde_json::json!(MAX_BATCH_SIZE * 10));
        assert_eq!(current_batch_size(&cache), MAX_BATCH_SIZE);
    }

    #[test]
    fn current_batch_size_clamps_to_min() {
        let cache = SettingsCache::new();
        cache.insert_for_tests(KEY_PRUNE_BATCH_SIZE, serde_json::json!(0));
        assert_eq!(current_batch_size(&cache), 1);
    }
}
