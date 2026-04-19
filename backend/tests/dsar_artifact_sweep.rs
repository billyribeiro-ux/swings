#![deny(warnings)]
#![forbid(unsafe_code)]

//! ADM-19 integration coverage for the DSAR artefact TTL sweep worker.
//!
//! Properties asserted:
//!   * `prune_once` deletes the on-disk artefact and NULLs the
//!     pointer columns when `artifact_expires_at < now()`.
//!   * Rows whose TTL has not elapsed are left untouched.
//!   * Rows with no `artifact_storage_key` (e.g. inline-mode jobs)
//!     are skipped.
//!   * Missing on-disk file is treated as success (idempotent retries).
//!   * `run_loop` exits cleanly on shutdown.
//!
//! The sweep is exercised end-to-end against the local-storage
//! backend that `TestApp::media_backend()` exposes, so we cover the
//! full path: claim → `MediaBackend::delete` → DB UPDATE.

mod support;

use std::time::Duration;

use serde_json::json;
use sqlx::Row;
use support::TestApp;
use tokio::sync::broadcast;
use uuid::Uuid;

#[tokio::test]
async fn prune_once_deletes_expired_artifact_and_nulls_columns() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let admin = app.seed_admin().await.expect("admin");
    let target = app.seed_user().await.expect("target");

    // Drive the existing async pipeline so we get a realistic row +
    // file on disk, then backdate the TTL.
    let resp = app
        .post_json::<serde_json::Value>(
            "/api/admin/dsar/jobs/export",
            &json!({
                "target_user_id": target.id,
                "reason":         "Sweep — TTL test",
                "async":          true,
            }),
            Some(&admin.access_token),
        )
        .await;
    let job_id: Uuid = resp.json::<serde_json::Value>().expect("body")["job"]["id"]
        .as_str()
        .expect("id")
        .parse()
        .expect("uuid");

    swings_api::services::dsar_worker::run_iteration_for_tests(app.db(), &app.media_backend())
        .await;

    let path = app.upload_dir().join(format!("dsar/{job_id}.json"));
    assert!(path.exists(), "fixture artefact missing");

    sqlx::query(
        "UPDATE dsar_jobs SET artifact_expires_at = NOW() - INTERVAL '1 hour' WHERE id = $1",
    )
    .bind(job_id)
    .execute(app.db())
    .await
    .expect("backdate");

    let scrubbed =
        swings_api::services::dsar_artifact_sweep::prune_once(app.db(), &app.media_backend()).await;
    assert_eq!(scrubbed, 1, "exactly one row should have been swept");

    assert!(
        !path.exists(),
        "artefact file should be removed by the sweep"
    );

    let row = sqlx::query(
        r#"
        SELECT artifact_url, artifact_storage_key, artifact_kind,
               artifact_expires_at, status
          FROM dsar_jobs WHERE id = $1
        "#,
    )
    .bind(job_id)
    .fetch_one(app.db())
    .await
    .expect("row");
    let url: Option<String> = row.try_get("artifact_url").unwrap();
    let key: Option<String> = row.try_get("artifact_storage_key").unwrap();
    let kind: Option<String> = row.try_get("artifact_kind").unwrap();
    let exp: Option<chrono::DateTime<chrono::Utc>> = row.try_get("artifact_expires_at").unwrap();
    let status: String = row.try_get("status").unwrap();
    assert!(url.is_none() && key.is_none() && kind.is_none() && exp.is_none());
    assert_eq!(status, "completed", "sweep must not change job status");
}

#[tokio::test]
async fn prune_once_leaves_unexpired_rows_untouched() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let admin = app.seed_admin().await.expect("admin");
    let target = app.seed_user().await.expect("target");

    let resp = app
        .post_json::<serde_json::Value>(
            "/api/admin/dsar/jobs/export",
            &json!({
                "target_user_id": target.id,
                "reason":         "Sweep — fresh row",
                "async":          true,
            }),
            Some(&admin.access_token),
        )
        .await;
    let job_id: Uuid = resp.json::<serde_json::Value>().expect("body")["job"]["id"]
        .as_str()
        .expect("id")
        .parse()
        .expect("uuid");

    swings_api::services::dsar_worker::run_iteration_for_tests(app.db(), &app.media_backend())
        .await;

    let scrubbed =
        swings_api::services::dsar_artifact_sweep::prune_once(app.db(), &app.media_backend()).await;
    assert_eq!(scrubbed, 0, "fresh artefact must not be swept");

    let key: Option<String> =
        sqlx::query_scalar("SELECT artifact_storage_key FROM dsar_jobs WHERE id = $1")
            .bind(job_id)
            .fetch_one(app.db())
            .await
            .expect("row");
    assert!(key.is_some(), "storage key must remain on un-expired row");
}

#[tokio::test]
async fn prune_once_skips_rows_with_no_storage_key() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let admin = app.seed_admin().await.expect("admin");
    let target = app.seed_user().await.expect("target");

    // Inline (synchronous) export — no storage key, just a data: URI
    // in artifact_url. Then backdate the TTL to prove the WHERE
    // clause's `artifact_storage_key IS NOT NULL` guard works.
    let resp = app
        .post_json::<serde_json::Value>(
            "/api/admin/dsar/jobs/export",
            &json!({
                "target_user_id": target.id,
                "reason":         "Inline + backdated",
            }),
            Some(&admin.access_token),
        )
        .await;
    let job_id: Uuid = resp.json::<serde_json::Value>().expect("body")["job"]["id"]
        .as_str()
        .expect("id")
        .parse()
        .expect("uuid");

    sqlx::query(
        "UPDATE dsar_jobs SET artifact_expires_at = NOW() - INTERVAL '1 day' WHERE id = $1",
    )
    .bind(job_id)
    .execute(app.db())
    .await
    .expect("backdate");

    let scrubbed =
        swings_api::services::dsar_artifact_sweep::prune_once(app.db(), &app.media_backend()).await;
    assert_eq!(scrubbed, 0, "rows without a storage key must be skipped");

    let url: Option<String> =
        sqlx::query_scalar("SELECT artifact_url FROM dsar_jobs WHERE id = $1")
            .bind(job_id)
            .fetch_one(app.db())
            .await
            .expect("row");
    assert!(
        url.as_deref()
            .map(|u| u.starts_with("data:"))
            .unwrap_or(false),
        "inline data: URI must remain"
    );
}

#[tokio::test]
async fn prune_once_is_idempotent_when_file_already_gone() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let admin = app.seed_admin().await.expect("admin");
    let target = app.seed_user().await.expect("target");

    let resp = app
        .post_json::<serde_json::Value>(
            "/api/admin/dsar/jobs/export",
            &json!({
                "target_user_id": target.id,
                "reason":         "Idempotency — manual file delete",
                "async":          true,
            }),
            Some(&admin.access_token),
        )
        .await;
    let job_id: Uuid = resp.json::<serde_json::Value>().expect("body")["job"]["id"]
        .as_str()
        .expect("id")
        .parse()
        .expect("uuid");

    swings_api::services::dsar_worker::run_iteration_for_tests(app.db(), &app.media_backend())
        .await;

    // Manually rip the file out from under the sweep, leaving the
    // pointer columns intact + TTL backdated. The sweep must still
    // succeed — `MediaBackend::delete` swallows `NotFound`.
    let path = app.upload_dir().join(format!("dsar/{job_id}.json"));
    tokio::fs::remove_file(&path).await.expect("rm");
    sqlx::query(
        "UPDATE dsar_jobs SET artifact_expires_at = NOW() - INTERVAL '1 hour' WHERE id = $1",
    )
    .bind(job_id)
    .execute(app.db())
    .await
    .expect("backdate");

    let scrubbed =
        swings_api::services::dsar_artifact_sweep::prune_once(app.db(), &app.media_backend()).await;
    assert_eq!(scrubbed, 1, "missing file should not block the sweep");

    let key: Option<String> =
        sqlx::query_scalar("SELECT artifact_storage_key FROM dsar_jobs WHERE id = $1")
            .bind(job_id)
            .fetch_one(app.db())
            .await
            .expect("row");
    assert!(key.is_none());
}

#[tokio::test]
async fn run_loop_exits_on_shutdown() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };

    let (tx, rx) = broadcast::channel::<()>(1);
    let pool = app.db().clone();
    let backend = app.media_backend();
    let handle = tokio::spawn(swings_api::services::dsar_artifact_sweep::run_loop(
        pool,
        backend,
        rx,
        Duration::from_secs(60),
    ));

    tokio::time::sleep(Duration::from_millis(50)).await;
    tx.send(()).expect("send shutdown");

    tokio::time::timeout(Duration::from_secs(2), handle)
        .await
        .expect("worker should exit within 2s of shutdown")
        .expect("join");
}
