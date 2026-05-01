//! ADM-17: async DSAR export worker.
//!
//! Picks up `dsar_jobs` rows with `kind='export' AND status='pending'`,
//! composes the JSON envelope using the same builder the synchronous
//! handler uses, persists the artefact via the configured
//! [`MediaBackend`], and updates the row to `completed` with the
//! storage key + TTL.
//!
//! The worker uses `FOR UPDATE SKIP LOCKED` so multiple replicas can
//! safely run the loop without coordinating leases. Each iteration
//! claims at most [`MAX_BATCH`] rows; oversized batches would extend
//! the lock window and block concurrent admin reads.
//!
//! ## Storage layout
//!
//! * `MediaBackend::R2`     — `dsar/{job_id}.json`. Returned URL is a
//!   short-lived presigned GET so a stolen URL cannot outlive its TTL.
//! * `MediaBackend::Local`  — `{upload_dir}/dsar/{job_id}.json`. The
//!   admin streamer (`/api/admin/dsar/jobs/{id}/artifact`) serves the
//!   file with RBAC, so the URL stored on the row is the streamer
//!   route, not a filesystem path.
//!
//! Either way `dsar_jobs.artifact_storage_key` keeps the canonical
//! object key so the streamer can re-presign on demand and the TTL
//! sweep can delete the underlying object.
//!
//! ## Failure model
//!
//! * Composer error → row goes to `status='failed'` with the message
//!   captured in `failure_reason`. Operators can re-queue manually.
//! * Storage error  → same as above.
//! * Worker panic   → tokio runtime catches it; the row remains in
//!   `composing` and the operator must intervene. A future enhancement
//!   could add a stuck-row reaper.
//!
//! ## Metrics
//!
//! * `dsar_export_claimed_total`  — rows the worker claimed.
//! * `dsar_export_completed_total` — rows that finished successfully.
//! * `dsar_export_failed_total`    — rows that errored.
//! * `dsar_export_duration_seconds` — histogram of compose+upload time.

use std::time::Duration;

use bytes::Bytes;
use sqlx::PgPool;
use tokio::sync::broadcast;
use tokio::time::Instant;
use uuid::Uuid;

use crate::services::{audit, dsar_admin, MediaBackend};

const MAX_BATCH: i64 = 5;
const ARTIFACT_TTL_SECS: u64 = 60 * 60 * 24; // 24h — long enough for an
                                             // operator to download + retry
                                             // a flaky network.

#[derive(Debug, sqlx::FromRow)]
struct ClaimedJob {
    id: Uuid,
    target_user_id: Uuid,
    requested_by: Uuid,
}

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
        "dsar-export worker started"
    );
    let mut ticker = tokio::time::interval(interval);
    ticker.tick().await; // discard the immediate first tick
    loop {
        tokio::select! {
            _ = ticker.tick() => {
                run_iteration(&pool, &media_backend).await;
            }
            _ = shutdown.recv() => {
                tracing::info!("dsar-export worker received shutdown");
                return;
            }
        }
    }
}

/// Drain one batch of pending exports. Exposed for integration tests
/// so a fixture can call it deterministically without spinning the
/// background ticker. Production callers should prefer [`run_loop`].
#[doc(hidden)]
pub async fn run_iteration_for_tests(pool: &PgPool, media_backend: &MediaBackend) {
    run_iteration(pool, media_backend).await;
}

async fn run_iteration(pool: &PgPool, media_backend: &MediaBackend) {
    let claimed = match claim_batch(pool).await {
        Ok(c) => c,
        Err(err) => {
            metrics::counter!("dsar_export_failed_total", "stage" => "claim").increment(1);
            tracing::error!(error = %err, "dsar-export claim failed");
            return;
        }
    };
    if claimed.is_empty() {
        return;
    }
    metrics::counter!("dsar_export_claimed_total").increment(claimed.len() as u64);
    for job in claimed {
        process_job(pool, media_backend, job).await;
    }
}

async fn claim_batch(pool: &PgPool) -> Result<Vec<ClaimedJob>, sqlx::Error> {
    sqlx::query_as::<_, ClaimedJob>(
        r#"
        UPDATE dsar_jobs
        SET status = 'composing',
            updated_at = NOW()
        WHERE id IN (
            SELECT id FROM dsar_jobs
            WHERE kind = 'export' AND status = 'pending'
            ORDER BY created_at ASC
            LIMIT $1
            FOR UPDATE SKIP LOCKED
        )
        RETURNING id, target_user_id, requested_by
        "#,
    )
    .bind(MAX_BATCH)
    .fetch_all(pool)
    .await
}

async fn process_job(pool: &PgPool, media_backend: &MediaBackend, job: ClaimedJob) {
    let started = Instant::now();
    match compose_and_persist(pool, media_backend, &job).await {
        Ok((kind, key, url, expires_at)) => {
            let _ = sqlx::query(
                r#"
                UPDATE dsar_jobs
                SET status = 'completed',
                    artifact_kind = $2,
                    artifact_storage_key = $3,
                    artifact_url = $4,
                    artifact_expires_at = $5,
                    completed_at = NOW(),
                    updated_at = NOW()
                WHERE id = $1
                "#,
            )
            .bind(job.id)
            .bind(&kind)
            .bind(&key)
            .bind(&url)
            .bind(expires_at)
            .execute(pool)
            .await
            .map_err(|err| {
                tracing::error!(job_id = %job.id, error = %err, "persist completion failed");
            });
            audit::record_admin_action_best_effort(
                pool,
                audit::AdminAction {
                    actor_id: job.requested_by,
                    actor_role: crate::models::UserRole::Admin,
                    action: "admin.dsar.export.completed_async",
                    target_kind: "user",
                    target_id: Some(job.target_user_id.to_string()),
                    ip_address: None,
                    user_agent: None,
                    metadata: serde_json::json!({
                        "job_id":  job.id,
                        "key":     key,
                        "kind":    kind,
                    }),
                },
            )
            .await;
            metrics::counter!("dsar_export_completed_total").increment(1);
            metrics::histogram!("dsar_export_duration_seconds")
                .record(started.elapsed().as_secs_f64());
            metrics::gauge!("dsar_export_last_success_unixtime")
                .set(chrono::Utc::now().timestamp() as f64);
            tracing::info!(
                job_id = %job.id,
                target = %job.target_user_id,
                "dsar export completed"
            );
        }
        Err(err) => {
            tracing::error!(
                job_id = %job.id,
                target = %job.target_user_id,
                error = %err,
                "dsar export failed"
            );
            let _ = sqlx::query(
                r#"
                UPDATE dsar_jobs
                SET status = 'failed',
                    failure_reason = $2,
                    updated_at = NOW()
                WHERE id = $1
                "#,
            )
            .bind(job.id)
            .bind(err.to_string())
            .execute(pool)
            .await;
            metrics::counter!("dsar_export_failed_total", "stage" => "process").increment(1);
        }
    }
}

async fn compose_and_persist(
    pool: &PgPool,
    media_backend: &MediaBackend,
    job: &ClaimedJob,
) -> Result<(String, String, String, chrono::DateTime<chrono::Utc>), WorkerError> {
    let export = dsar_admin::build_admin_export(pool, job.target_user_id)
        .await
        .map_err(|e| WorkerError::Compose(e.to_string()))?
        .ok_or_else(|| WorkerError::Compose(format!("user {} vanished", job.target_user_id)))?;

    let payload = serde_json::to_vec_pretty(&export)
        .map_err(|e| WorkerError::Compose(format!("serialize: {e}")))?;

    let key = format!("dsar/{}.json", job.id);
    let expires_at = chrono::Utc::now() + chrono::Duration::seconds(ARTIFACT_TTL_SECS as i64);

    match media_backend {
        MediaBackend::R2(r2) => {
            r2.upload(&key, Bytes::from(payload), "application/json")
                .await
                .map_err(|e| WorkerError::Storage(e.to_string()))?;
            let presigned = r2
                .presign_get(&key, Duration::from_secs(ARTIFACT_TTL_SECS))
                .await
                .map_err(|e| WorkerError::Storage(e.to_string()))?;
            Ok(("r2".to_string(), key, presigned, expires_at))
        }
        MediaBackend::Local { upload_dir } => {
            let dir = std::path::Path::new(upload_dir).join("dsar");
            tokio::fs::create_dir_all(&dir)
                .await
                .map_err(|e| WorkerError::Storage(format!("mkdir: {e}")))?;
            let path = dir.join(format!("{}.json", job.id));
            tokio::fs::write(&path, &payload)
                .await
                .map_err(|e| WorkerError::Storage(format!("write: {e}")))?;
            // Local mode: URL is the streamer route. Access goes through
            // the privileged downloader so RBAC still applies.
            let url = format!("/api/admin/dsar/jobs/{}/artifact", job.id);
            Ok(("local".to_string(), key, url, expires_at))
        }
    }
}

#[derive(Debug, thiserror::Error)]
enum WorkerError {
    #[error("compose: {0}")]
    Compose(String),
    #[error("storage: {0}")]
    Storage(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn artifact_ttl_is_24h() {
        assert_eq!(ARTIFACT_TTL_SECS, 86_400);
    }

    // Compile-time guard on `MAX_BATCH` — see the same idiom in
    // `dsar_artifact_sweep.rs` for the rationale.
    const _: () = assert!(MAX_BATCH <= 10, "lock window must stay short");

    #[test]
    fn max_batch_is_modest() {
        const { assert!(MAX_BATCH <= 10, "lock window must stay short") };
    }
}
