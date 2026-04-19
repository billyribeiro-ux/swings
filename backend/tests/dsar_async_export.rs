#![deny(warnings)]
#![forbid(unsafe_code)]

//! ADM-17 integration coverage for the asynchronous DSAR export pipeline.
//!
//! Covers:
//!   * `POST /jobs/export` with `async=true` returns 202 and creates a
//!     `pending` row with no inline payload.
//!   * Worker claims `pending` rows, composes the JSON envelope,
//!     persists to the local upload dir, and stamps the row with
//!     `completed`, `artifact_kind=local`, a storage key, and a TTL.
//!   * `GET /jobs/{id}/artifact` streams the JSON to the operator
//!     with the right `Content-Type` and a download disposition.
//!   * Pre-completion artefact requests are rejected (409).
//!   * Inline jobs cannot be streamed via the artefact route (400).

mod support;

use axum::http::StatusCode;
use serde_json::{json, Value};
use sqlx::Row;
use support::TestApp;
use uuid::Uuid;

#[tokio::test]
async fn async_export_queues_pending_job_and_returns_202() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let admin = app.seed_admin().await.expect("admin");
    let target = app.seed_user().await.expect("target");

    let resp = app
        .post_json::<Value>(
            "/api/admin/dsar/jobs/export",
            &json!({
                "target_user_id": target.id,
                "reason":         "Async export — large account",
                "async":          true,
            }),
            Some(&admin.access_token),
        )
        .await;
    resp.assert_status(StatusCode::ACCEPTED);
    let body: Value = resp.json().expect("body");
    assert_eq!(body["job"]["status"], json!("pending"));
    assert!(body["export"].is_null());
    assert!(body["job"]["artifact_url"].is_null());

    let audit_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM admin_actions WHERE action = 'admin.dsar.export.queued'",
    )
    .fetch_one(app.db())
    .await
    .expect("audit");
    assert_eq!(audit_count, 1);
}

#[tokio::test]
async fn worker_completes_pending_export_and_writes_local_artifact() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let admin = app.seed_admin().await.expect("admin");
    let target = app.seed_user().await.expect("target");

    let resp = app
        .post_json::<Value>(
            "/api/admin/dsar/jobs/export",
            &json!({
                "target_user_id": target.id,
                "reason":         "Async export — worker drain",
                "async":          true,
            }),
            Some(&admin.access_token),
        )
        .await;
    resp.assert_status(StatusCode::ACCEPTED);
    let body: Value = resp.json().expect("body");
    let job_id: Uuid = body["job"]["id"]
        .as_str()
        .expect("job id")
        .parse()
        .expect("uuid");

    swings_api::services::dsar_worker::run_iteration_for_tests(app.db(), &app.media_backend())
        .await;

    let row = sqlx::query(
        r#"
        SELECT status, artifact_kind, artifact_storage_key, artifact_url,
               artifact_expires_at, completed_at
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
    let expires_at: Option<chrono::DateTime<chrono::Utc>> =
        row.try_get("artifact_expires_at").unwrap();
    let completed_at: Option<chrono::DateTime<chrono::Utc>> = row.try_get("completed_at").unwrap();
    assert_eq!(status, "completed");
    assert_eq!(kind.as_deref(), Some("local"));
    assert_eq!(key.as_deref(), Some(format!("dsar/{job_id}.json").as_str()));
    assert_eq!(
        url.as_deref(),
        Some(format!("/api/admin/dsar/jobs/{job_id}/artifact").as_str())
    );
    assert!(expires_at.is_some());
    assert!(completed_at.is_some());

    let path = app.upload_dir().join(format!("dsar/{job_id}.json"));
    let bytes = tokio::fs::read(&path).await.expect("artefact on disk");
    let parsed: Value = serde_json::from_slice(&bytes).expect("json");
    assert_eq!(parsed["version"], json!(1));
    assert_eq!(
        parsed["user"]["id"].as_str().unwrap(),
        target.id.to_string()
    );
}

#[tokio::test]
async fn artifact_endpoint_streams_completed_export() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let admin = app.seed_admin().await.expect("admin");
    let target = app.seed_user().await.expect("target");

    let resp = app
        .post_json::<Value>(
            "/api/admin/dsar/jobs/export",
            &json!({
                "target_user_id": target.id,
                "reason":         "Async export — streamer",
                "async":          true,
            }),
            Some(&admin.access_token),
        )
        .await;
    let job_id: Uuid = resp.json::<Value>().expect("body")["job"]["id"]
        .as_str()
        .expect("id")
        .parse()
        .expect("uuid");

    swings_api::services::dsar_worker::run_iteration_for_tests(app.db(), &app.media_backend())
        .await;

    let stream = app
        .get(
            &format!("/api/admin/dsar/jobs/{job_id}/artifact"),
            Some(&admin.access_token),
        )
        .await;
    stream.assert_status(StatusCode::OK);
    let ct = stream
        .header("content-type")
        .expect("content-type")
        .to_string();
    assert!(ct.starts_with("application/json"), "got {ct}");
    let cd = stream
        .header("content-disposition")
        .expect("content-disposition")
        .to_string();
    assert!(cd.contains(&format!("dsar-{job_id}.json")), "got {cd}");

    let parsed: Value = stream.json().expect("json");
    assert_eq!(
        parsed["user"]["id"].as_str().unwrap(),
        target.id.to_string()
    );

    let download_audits: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM admin_actions WHERE action = 'admin.dsar.export.downloaded'",
    )
    .fetch_one(app.db())
    .await
    .expect("count");
    assert_eq!(download_audits, 1);
}

#[tokio::test]
async fn artifact_endpoint_rejects_pending_job() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let admin = app.seed_admin().await.expect("admin");
    let target = app.seed_user().await.expect("target");

    let resp = app
        .post_json::<Value>(
            "/api/admin/dsar/jobs/export",
            &json!({
                "target_user_id": target.id,
                "reason":         "Pending streamer",
                "async":          true,
            }),
            Some(&admin.access_token),
        )
        .await;
    let job_id: Uuid = resp.json::<Value>().expect("body")["job"]["id"]
        .as_str()
        .expect("id")
        .parse()
        .expect("uuid");

    let stream = app
        .get(
            &format!("/api/admin/dsar/jobs/{job_id}/artifact"),
            Some(&admin.access_token),
        )
        .await;
    stream.assert_status(StatusCode::CONFLICT);
}

#[tokio::test]
async fn artifact_endpoint_refuses_inline_artefact() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let admin = app.seed_admin().await.expect("admin");
    let target = app.seed_user().await.expect("target");

    let resp = app
        .post_json::<Value>(
            "/api/admin/dsar/jobs/export",
            &json!({
                "target_user_id": target.id,
                "reason":         "Inline + streamer attempt",
            }),
            Some(&admin.access_token),
        )
        .await;
    let job_id: Uuid = resp.json::<Value>().expect("body")["job"]["id"]
        .as_str()
        .expect("id")
        .parse()
        .expect("uuid");

    let stream = app
        .get(
            &format!("/api/admin/dsar/jobs/{job_id}/artifact"),
            Some(&admin.access_token),
        )
        .await;
    stream.assert_status(StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn member_cannot_stream_artifact() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let admin = app.seed_admin().await.expect("admin");
    let member = app.seed_user().await.expect("member");

    let resp = app
        .post_json::<Value>(
            "/api/admin/dsar/jobs/export",
            &json!({
                "target_user_id": member.id,
                "reason":         "Async export — RBAC test",
                "async":          true,
            }),
            Some(&admin.access_token),
        )
        .await;
    let job_id: Uuid = resp.json::<Value>().expect("body")["job"]["id"]
        .as_str()
        .expect("id")
        .parse()
        .expect("uuid");

    swings_api::services::dsar_worker::run_iteration_for_tests(app.db(), &app.media_backend())
        .await;

    let stream = app
        .get(
            &format!("/api/admin/dsar/jobs/{job_id}/artifact"),
            Some(&member.access_token),
        )
        .await;
    stream.assert_status(StatusCode::FORBIDDEN);
}
