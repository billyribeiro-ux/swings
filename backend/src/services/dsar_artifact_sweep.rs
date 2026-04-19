//! ADM-19: TTL sweep for asynchronous DSAR export artefacts.
//!
//! When [`crate::services::dsar_worker`] composes a DSAR export, it
//! stamps `dsar_jobs.artifact_expires_at` with `now() + 24h`. The
//! presigned URL stops working at that TTL — but the underlying R2
//! object (or local file) lingers forever unless something deletes
//! it. This worker is that something.
//!
//! ## What it does
//!
//! Each iteration:
//!   1. Selects up to [`MAX_BATCH`] rows where `artifact_expires_at < now()`
//!      AND `artifact_storage_key IS NOT NULL`. Uses `FOR UPDATE SKIP LOCKED`
//!      so multiple replicas can run safely.
//!   2. For each row, calls [`MediaBackend::delete`] (idempotent on
//!      `NotFound`).
//!   3. NULLs `artifact_url`, `artifact_storage_key`, `artifact_kind`,
//!      `artifact_expires_at` so the row no longer advertises a dead URL.
//!      The job stays `completed`, the audit record stays in
//!      `admin_actions`, and the request reason / approver chain stays
//!      visible — only the artefact pointer is scrubbed.
//!
//! ## Why a separate worker
//!
//! The compose worker [`dsar_worker`] is sized for low latency
//! (`DSAR_WORKER_INTERVAL_SECS=30` default). Cleanup is tolerant of
//! hour-scale lag, so this loop ticks at `DSAR_SWEEP_INTERVAL_SECS=3600`
//! by default. Splitting them keeps a slow R2 delete from blocking
//! fresh exports.
//!
//! ## Failure model
//!
//! Storage error → counted in `dsar_artifacts_sweep_failed_total` and
//! left for the next tick. The DB row is **not** updated until the
//! object is gone, so retries are automatic and idempotent.

use std::time::Duration;

use sqlx::{PgPool, Row};
use tokio::sync::broadcast;
use tokio::time::Instant;
use uuid::Uuid;

use crate::services::MediaBackend;

const MAX_BATCH: i64 = 50;

/// Long-lived worker. Spawn once per process from `main.rs`. Sleeps
/// `interval` between iterations and exits when the broadcast
/// `shutdown` fires.
pub async fn run_loop(
    pool: PgPool,
    media_backend: MediaBackend,
    mut shutdown: broadcast::Receiver<()>,
    interval: Duration,
) {
    tracing::info!(
        interval_secs = interval.as_secs(),
        backend = if media_backend.is_r2() { "r2" } else { "local" },
        "dsar-artifact-sweep worker started"
    );
    let mut ticker = tokio::time::interval(interval);
    ticker.tick().await; // discard the immediate first tick
    loop {
        tokio::select! {
            _ = ticker.tick() => {
                let _ = prune_once(&pool, &media_backend).await;
            }
            _ = shutdown.recv() => {
                tracing::info!("dsar-artifact-sweep worker received shutdown");
                return;
            }
        }
    }
}

/// Drain one batch of expired artefacts. Returns the number of rows
/// scrubbed. Exposed for integration tests so a fixture can drive
/// the sweep deterministically.
pub async fn prune_once(pool: &PgPool, media_backend: &MediaBackend) -> u64 {
    let started = Instant::now();
    let claimed = match claim_batch(pool).await {
        Ok(c) => c,
        Err(err) => {
            metrics::counter!("dsar_artifacts_sweep_failed_total", "stage" => "claim").increment(1);
            tracing::error!(error = %err, "dsar-artifact-sweep claim failed");
            return 0;
        }
    };
    if claimed.is_empty() {
        record_metrics(0, started);
        return 0;
    }

    let mut scrubbed: u64 = 0;
    for (id, key) in claimed {
        match media_backend.delete(&key).await {
            Ok(()) => match clear_artifact_columns(pool, id).await {
                Ok(()) => {
                    scrubbed += 1;
                    tracing::info!(job_id = %id, key = %key, "dsar artefact swept");
                }
                Err(err) => {
                    metrics::counter!(
                        "dsar_artifacts_sweep_failed_total",
                        "stage" => "db_clear"
                    )
                    .increment(1);
                    tracing::error!(job_id = %id, error = %err, "dsar artefact db-clear failed");
                }
            },
            Err(err) => {
                metrics::counter!(
                    "dsar_artifacts_sweep_failed_total",
                    "stage" => "delete"
                )
                .increment(1);
                tracing::warn!(
                    job_id = %id,
                    key = %key,
                    error = %err,
                    "dsar artefact delete failed; will retry next tick"
                );
            }
        }
    }
    record_metrics(scrubbed, started);
    scrubbed
}

fn record_metrics(scrubbed: u64, started: Instant) {
    metrics::counter!("dsar_artifacts_swept_total").increment(scrubbed);
    metrics::histogram!("dsar_artifacts_sweep_duration_seconds")
        .record(started.elapsed().as_secs_f64());
    metrics::gauge!("dsar_artifacts_sweep_last_success_unixtime")
        .set(chrono::Utc::now().timestamp() as f64);
}

async fn claim_batch(pool: &PgPool) -> Result<Vec<(Uuid, String)>, sqlx::Error> {
    // No status update needed — the sweep is idempotent and the
    // claim window is the duration of the in-process loop iteration.
    // We fetch the keys once and rely on the per-row `clear_artifact_columns`
    // CAS-style update (`WHERE artifact_storage_key IS NOT NULL`) to
    // serialise against another replica that might have raced ahead.
    let rows = sqlx::query(
        r#"
        SELECT id, artifact_storage_key
          FROM dsar_jobs
         WHERE artifact_expires_at IS NOT NULL
           AND artifact_expires_at < NOW()
           AND artifact_storage_key IS NOT NULL
         ORDER BY artifact_expires_at ASC
         LIMIT $1
         FOR UPDATE SKIP LOCKED
        "#,
    )
    .bind(MAX_BATCH)
    .fetch_all(pool)
    .await?;

    let mut out = Vec::with_capacity(rows.len());
    for row in rows {
        let id: Uuid = row.try_get("id")?;
        let key: String = row.try_get("artifact_storage_key")?;
        out.push((id, key));
    }
    Ok(out)
}

async fn clear_artifact_columns(pool: &PgPool, id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        UPDATE dsar_jobs
        SET artifact_url = NULL,
            artifact_storage_key = NULL,
            artifact_kind = NULL,
            artifact_expires_at = NULL,
            updated_at = NOW()
        WHERE id = $1
          AND artifact_storage_key IS NOT NULL
        "#,
    )
    .bind(id)
    .execute(pool)
    .await
    .map(|_| ())
}

#[cfg(test)]
mod tests {
    use super::*;

    // Compile-time guards on `MAX_BATCH` so a future tuning typo
    // can't ship a value that holds row locks for too long or
    // wastes the round-trip on a tiny batch. `const { assert!(..) }`
    // makes clippy happy (the assertion is a constant value) and
    // catches the regression at build-time rather than test-time.
    const _: () = assert!(MAX_BATCH <= 100, "lock window must stay short");
    const _: () = assert!(MAX_BATCH >= 10, "batch must be useful");

    #[test]
    fn batch_size_keeps_lock_window_short() {
        // Re-asserts the const-time invariants so the regression
        // also surfaces in test output, not just in compile errors.
        const { assert!(MAX_BATCH <= 100, "lock window must stay short") };
        const { assert!(MAX_BATCH >= 10, "batch must be useful") };
    }
}
