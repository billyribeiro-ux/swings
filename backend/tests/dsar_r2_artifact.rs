#![deny(warnings)]
#![forbid(unsafe_code)]

//! ADM-17 + ADM-19 integration coverage for the **R2 (S3-compatible)
//! transport** of the asynchronous DSAR export pipeline.
//!
//! The local-disk path is exercised by `dsar_async_export.rs` and
//! `dsar_artifact_sweep.rs`; this binary exists so the production
//! transport — `R2Storage::upload`, `presign_get`, `delete_object`
//! plus the worker code paths that wrap them — also gets a real
//! end-to-end probe.
//!
//! ## How to run locally
//!
//! Start any S3-compatible emulator and export the test env vars,
//! then `cargo test --test dsar_r2_artifact`. Example with MinIO:
//!
//! ```bash
//! docker run -d --rm --name swings-minio -p 9000:9000 \
//!   -e MINIO_ROOT_USER=minioadmin -e MINIO_ROOT_PASSWORD=minioadmin \
//!   minio/minio server /data
//!
//! export R2_TEST_ENDPOINT=http://localhost:9000
//! export R2_TEST_ACCESS_KEY=minioadmin
//! export R2_TEST_SECRET_KEY=minioadmin
//! cargo test --manifest-path backend/Cargo.toml --test dsar_r2_artifact
//! ```
//!
//! Without `R2_TEST_ENDPOINT` set, every test in this binary
//! returns early so CI runs that don't have Docker available stay
//! green. The `[skip] no R2 emulator configured` line in the test
//! output makes the elision visible.

mod support;

use serde_json::json;
use sqlx::Row;
use support::{TestApp, TestUser};
use uuid::Uuid;

/// Boilerplate the every R2 test needs: an admin, a target user,
/// and a `MediaBackend::R2` pointed at the configured emulator with
/// a fresh bucket. Returns `None` when the emulator is not running
/// — callers convert that into an early return.
async fn r2_setup() -> Option<(
    TestApp,
    TestUser,
    TestUser,
    swings_api::services::MediaBackend,
)> {
    let app = TestApp::try_new().await?;
    let backend = match app.try_media_backend_r2().await {
        Some(b) => b,
        None => {
            eprintln!("[skip] no R2 emulator configured (set R2_TEST_ENDPOINT to enable)");
            return None;
        }
    };
    let admin = app.seed_admin().await.expect("admin");
    let target = app.seed_user().await.expect("target");
    Some((app, admin, target, backend))
}

#[tokio::test]
async fn worker_uploads_artifact_to_r2_and_records_metadata() {
    let Some((app, admin, target, backend)) = r2_setup().await else {
        return;
    };

    let resp = app
        .post_json::<serde_json::Value>(
            "/api/admin/dsar/jobs/export",
            &json!({
                "target_user_id": target.id,
                "reason":         "R2 integration smoke test",
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

    swings_api::services::dsar_worker::run_iteration_for_tests(app.db(), &backend).await;

    let row = sqlx::query(
        r#"
        SELECT status, artifact_kind, artifact_storage_key,
               artifact_url, artifact_expires_at
          FROM dsar_jobs WHERE id = $1
        "#,
    )
    .bind(job_id)
    .fetch_one(app.db())
    .await
    .expect("row");
    let status: String = row.try_get("status").unwrap();
    let kind: Option<String> = row.try_get("artifact_kind").unwrap();
    let key: Option<String> = row.try_get("artifact_storage_key").unwrap();
    let url: Option<String> = row.try_get("artifact_url").unwrap();
    let exp: Option<chrono::DateTime<chrono::Utc>> = row.try_get("artifact_expires_at").unwrap();

    assert_eq!(status, "completed");
    assert_eq!(kind.as_deref(), Some("r2"));
    assert!(
        key.as_deref().is_some_and(|k| k.starts_with("dsar/")),
        "storage key must live under the dsar/ prefix; got {key:?}"
    );
    let url = url.expect("R2 path must populate artifact_url with a presigned link");
    assert!(
        url.contains("X-Amz-Signature") || url.contains("X-Amz-Credential"),
        "artifact_url must look like a presigned S3 URL; got {url}"
    );
    let exp = exp.expect("artifact_expires_at must be set on the R2 path");
    assert!(
        exp > chrono::Utc::now(),
        "TTL must be in the future right after compose"
    );

    // Probe the bucket directly. The worker should have written the
    // exact key it stored on the row.
    let r2 = match backend {
        swings_api::services::MediaBackend::R2(ref r2) => r2,
        _ => unreachable!("setup guaranteed R2 backend"),
    };
    assert!(
        r2.object_exists(key.as_deref().unwrap())
            .await
            .expect("head"),
        "object missing in bucket after a successful compose"
    );
}

#[tokio::test]
async fn ttl_sweep_deletes_r2_object_and_clears_columns() {
    let Some((app, admin, target, backend)) = r2_setup().await else {
        return;
    };

    let resp = app
        .post_json::<serde_json::Value>(
            "/api/admin/dsar/jobs/export",
            &json!({
                "target_user_id": target.id,
                "reason":         "R2 sweep test — backdate TTL",
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

    swings_api::services::dsar_worker::run_iteration_for_tests(app.db(), &backend).await;
    let key: String =
        sqlx::query_scalar("SELECT artifact_storage_key FROM dsar_jobs WHERE id = $1")
            .bind(job_id)
            .fetch_one(app.db())
            .await
            .expect("row");

    let r2 = match backend {
        swings_api::services::MediaBackend::R2(ref r2) => r2.clone(),
        _ => unreachable!(),
    };
    assert!(
        r2.object_exists(&key).await.expect("head"),
        "object should exist before sweep"
    );

    sqlx::query(
        "UPDATE dsar_jobs SET artifact_expires_at = NOW() - INTERVAL '1 hour' WHERE id = $1",
    )
    .bind(job_id)
    .execute(app.db())
    .await
    .expect("backdate");

    let scrubbed = swings_api::services::dsar_artifact_sweep::prune_once(app.db(), &backend).await;
    assert_eq!(scrubbed, 1, "exactly one R2 row should have been swept");

    assert!(
        !r2.object_exists(&key).await.expect("head"),
        "R2 object must be deleted by the sweep"
    );

    let url: Option<String> =
        sqlx::query_scalar("SELECT artifact_url FROM dsar_jobs WHERE id = $1")
            .bind(job_id)
            .fetch_one(app.db())
            .await
            .expect("row");
    assert!(url.is_none(), "presigned URL must be NULLed after sweep");
}

#[tokio::test]
async fn r2_delete_is_idempotent_for_missing_key() {
    let Some((_app, _admin, _target, backend)) = r2_setup().await else {
        return;
    };
    // Calling delete on a key that was never uploaded must succeed —
    // this is what makes the sweep safe to retry after a partial
    // failure (worker crashed mid-update, second tick re-attempts).
    backend
        .delete("dsar/never-existed.json")
        .await
        .expect("idempotent delete must succeed for a missing key");
}
