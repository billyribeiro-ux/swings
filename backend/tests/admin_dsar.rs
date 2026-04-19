#![deny(warnings)]
#![forbid(unsafe_code)]

//! ADM-13 integration coverage for the admin-initiated DSAR surface.
//!
//! Covers:
//!   * RBAC matrix (member / support / admin) for read/export/erase.
//!   * Export composition (single-control, audit + artefact present).
//!   * Erasure dual-control: request → approve → tombstone runs.
//!   * Self-approval refused.
//!   * Pending-erasure deduplication.
//!   * Tombstone scrubs every PII column on `users` and clears
//!     refresh tokens.
//!   * Cancel transitions out of `pending`.
//!   * 404s on unknown user / unknown job.

mod support;

use axum::http::StatusCode;
use serde_json::{json, Value};
use sqlx::Row;
use support::TestApp;
use uuid::Uuid;

// ── RBAC ───────────────────────────────────────────────────────────────

#[tokio::test]
async fn member_cannot_touch_admin_dsar_surface() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let member = app.seed_user().await.expect("seed");

    let list = app
        .get("/api/admin/dsar/jobs", Some(&member.access_token))
        .await;
    list.assert_status(StatusCode::FORBIDDEN);

    let export = app
        .post_json::<Value>(
            "/api/admin/dsar/jobs/export",
            &json!({"target_user_id": member.id, "reason": "x"}),
            Some(&member.access_token),
        )
        .await;
    export.assert_status(StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn support_can_read_but_not_mutate_dsar_jobs() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let support = app.seed_support().await.expect("seed");

    let list = app
        .get("/api/admin/dsar/jobs", Some(&support.access_token))
        .await;
    list.assert_status(StatusCode::OK);

    let export = app
        .post_json::<Value>(
            "/api/admin/dsar/jobs/export",
            &json!({"target_user_id": support.id, "reason": "audit"}),
            Some(&support.access_token),
        )
        .await;
    export.assert_status(StatusCode::FORBIDDEN);

    let req_erase = app
        .post_json::<Value>(
            "/api/admin/dsar/jobs/erase/request",
            &json!({
                "target_user_id": support.id,
                "reason": "right to be forgotten",
            }),
            Some(&support.access_token),
        )
        .await;
    req_erase.assert_status(StatusCode::FORBIDDEN);
}

// ── Export ─────────────────────────────────────────────────────────────

#[tokio::test]
async fn create_export_composes_artefact_and_audits() {
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
                "reason": "Subject access request via support ticket #42",
            }),
            Some(&admin.access_token),
        )
        .await;
    resp.assert_status(StatusCode::CREATED);
    let body: Value = resp.json().expect("body");
    assert_eq!(body["job"]["status"], json!("completed"));
    assert_eq!(body["job"]["kind"], json!("export"));
    assert!(body["job"]["artifact_url"]
        .as_str()
        .expect("artifact")
        .starts_with("data:application/json"));
    assert_eq!(body["export"]["version"], json!(1));
    assert_eq!(
        body["export"]["user"]["id"].as_str().expect("user id"),
        target.id.to_string()
    );

    let audit_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM admin_actions WHERE action = 'admin.dsar.export.completed'",
    )
    .fetch_one(app.db())
    .await
    .expect("audit");
    assert_eq!(audit_count, 1);
}

#[tokio::test]
async fn create_export_unknown_user_is_404() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let admin = app.seed_admin().await.expect("admin");

    let resp = app
        .post_json::<Value>(
            "/api/admin/dsar/jobs/export",
            &json!({
                "target_user_id": Uuid::new_v4(),
                "reason": "ghost",
            }),
            Some(&admin.access_token),
        )
        .await;
    resp.assert_status(StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn create_export_rejects_empty_reason() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let admin = app.seed_admin().await.expect("admin");
    let target = app.seed_user().await.expect("target");

    let resp = app
        .post_json::<Value>(
            "/api/admin/dsar/jobs/export",
            &json!({"target_user_id": target.id, "reason": ""}),
            Some(&admin.access_token),
        )
        .await;
    resp.assert_status(StatusCode::BAD_REQUEST);
}

// ── Erase: dual control ────────────────────────────────────────────────

#[tokio::test]
async fn erase_request_then_approve_runs_tombstone() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let requester = app.seed_admin().await.expect("requester");
    let approver = app.seed_admin().await.expect("approver");
    let target = app.seed_user().await.expect("target");
    let original_email = target.email.clone();

    // Step 1: request.
    let req = app
        .post_json::<Value>(
            "/api/admin/dsar/jobs/erase/request",
            &json!({
                "target_user_id": target.id,
                "reason": "GDPR Art. 17 request from subject 2026-04-19",
            }),
            Some(&requester.access_token),
        )
        .await;
    req.assert_status(StatusCode::CREATED);
    let req_body: Value = req.json().expect("body");
    let job_id = Uuid::parse_str(req_body["id"].as_str().expect("id")).expect("uuid");
    assert_eq!(req_body["status"], json!("pending"));
    assert_eq!(req_body["kind"], json!("erase"));

    // Step 2: a *different* admin approves.
    let approve = app
        .post_json::<Value>(
            &format!("/api/admin/dsar/jobs/{job_id}/erase/approve"),
            &json!({"approval_reason": "verified ticket #42, sub identity confirmed"}),
            Some(&approver.access_token),
        )
        .await;
    approve.assert_status(StatusCode::OK);
    let approve_body: Value = approve.json().expect("body");
    assert_eq!(approve_body["job"]["status"], json!("completed"));
    let summary = &approve_body["summary"];
    assert!(summary["placeholder_email"]
        .as_str()
        .expect("placeholder")
        .starts_with("erased-"));

    // Tombstone applied to users row.
    let row = sqlx::query(
        "SELECT email, name, password_hash, erased_at, erasure_job_id FROM users WHERE id = $1",
    )
    .bind(target.id)
    .fetch_one(app.db())
    .await
    .expect("user row");
    let new_email: String = row.try_get("email").expect("email");
    let new_name: String = row.try_get("name").expect("name");
    let pw_hash: String = row.try_get("password_hash").expect("pw");
    let erased_at: Option<chrono::DateTime<chrono::Utc>> =
        row.try_get("erased_at").expect("erased_at");
    let job_back: Option<Uuid> = row.try_get("erasure_job_id").expect("erasure_job_id");

    assert_ne!(new_email, original_email, "PII email must be overwritten");
    assert!(new_email.starts_with("erased-"));
    assert_eq!(new_name, "");
    assert!(pw_hash.starts_with("$ERASED$"));
    assert!(erased_at.is_some());
    assert_eq!(job_back, Some(job_id));

    // Audit chain: requested + approved + completed.
    let audit_actions: Vec<String> = sqlx::query_scalar(
        "SELECT action FROM admin_actions
          WHERE target_kind = 'user' AND target_id = $1
          ORDER BY created_at",
    )
    .bind(target.id.to_string())
    .fetch_all(app.db())
    .await
    .expect("audit");
    assert!(audit_actions.contains(&"admin.dsar.erase.requested".to_string()));
    assert!(audit_actions.contains(&"admin.dsar.erase.approved".to_string()));
    assert!(audit_actions.contains(&"admin.dsar.erase.completed".to_string()));
}

#[tokio::test]
async fn self_approval_is_forbidden() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let admin = app.seed_admin().await.expect("admin");
    let target = app.seed_user().await.expect("target");

    let req = app
        .post_json::<Value>(
            "/api/admin/dsar/jobs/erase/request",
            &json!({
                "target_user_id": target.id,
                "reason": "subject right to erasure 2026-04-19",
            }),
            Some(&admin.access_token),
        )
        .await;
    req.assert_status(StatusCode::CREATED);
    let body: Value = req.json().expect("body");
    let job_id = body["id"].as_str().expect("id");

    let approve = app
        .post_json::<Value>(
            &format!("/api/admin/dsar/jobs/{job_id}/erase/approve"),
            &json!({"approval_reason": "self-approval should not be allowed"}),
            Some(&admin.access_token),
        )
        .await;
    approve.assert_status(StatusCode::FORBIDDEN);

    // Target user row must NOT be tombstoned.
    let erased_at: Option<chrono::DateTime<chrono::Utc>> =
        sqlx::query_scalar("SELECT erased_at FROM users WHERE id = $1")
            .bind(target.id)
            .fetch_one(app.db())
            .await
            .expect("erased_at");
    assert!(erased_at.is_none());
}

#[tokio::test]
async fn duplicate_pending_erase_is_409() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let admin = app.seed_admin().await.expect("admin");
    let target = app.seed_user().await.expect("target");

    let first = app
        .post_json::<Value>(
            "/api/admin/dsar/jobs/erase/request",
            &json!({
                "target_user_id": target.id,
                "reason": "first attempt for subject erasure",
            }),
            Some(&admin.access_token),
        )
        .await;
    first.assert_status(StatusCode::CREATED);

    let second = app
        .post_json::<Value>(
            "/api/admin/dsar/jobs/erase/request",
            &json!({
                "target_user_id": target.id,
                "reason": "second attempt should collide",
            }),
            Some(&admin.access_token),
        )
        .await;
    second.assert_status(StatusCode::CONFLICT);
}

#[tokio::test]
async fn erase_unknown_user_is_404() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let admin = app.seed_admin().await.expect("admin");

    let resp = app
        .post_json::<Value>(
            "/api/admin/dsar/jobs/erase/request",
            &json!({
                "target_user_id": Uuid::new_v4(),
                "reason": "subject right to erasure 2026-04-19",
            }),
            Some(&admin.access_token),
        )
        .await;
    resp.assert_status(StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn erase_already_tombstoned_user_is_409() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let requester = app.seed_admin().await.expect("requester");
    let approver = app.seed_admin().await.expect("approver");
    let target = app.seed_user().await.expect("target");

    let req = app
        .post_json::<Value>(
            "/api/admin/dsar/jobs/erase/request",
            &json!({
                "target_user_id": target.id,
                "reason": "first erasure for subject",
            }),
            Some(&requester.access_token),
        )
        .await;
    req.assert_status(StatusCode::CREATED);
    let job_id = req.json::<Value>().expect("body")["id"]
        .as_str()
        .expect("id")
        .to_string();

    app.post_json::<Value>(
        &format!("/api/admin/dsar/jobs/{job_id}/erase/approve"),
        &json!({"approval_reason": "approved by second admin"}),
        Some(&approver.access_token),
    )
    .await
    .assert_status(StatusCode::OK);

    // Trying again must 409.
    let again = app
        .post_json::<Value>(
            "/api/admin/dsar/jobs/erase/request",
            &json!({
                "target_user_id": target.id,
                "reason": "should fail because user already tombstoned",
            }),
            Some(&requester.access_token),
        )
        .await;
    again.assert_status(StatusCode::CONFLICT);
}

// ── Cancel + read ──────────────────────────────────────────────────────

#[tokio::test]
async fn cancel_pending_erase_transitions_to_cancelled() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let admin = app.seed_admin().await.expect("admin");
    let target = app.seed_user().await.expect("target");

    let req = app
        .post_json::<Value>(
            "/api/admin/dsar/jobs/erase/request",
            &json!({
                "target_user_id": target.id,
                "reason": "subject erasure to be cancelled",
            }),
            Some(&admin.access_token),
        )
        .await;
    req.assert_status(StatusCode::CREATED);
    let job_id = req.json::<Value>().expect("body")["id"]
        .as_str()
        .expect("id")
        .to_string();

    let cancel = app
        .post_json::<Value>(
            &format!("/api/admin/dsar/jobs/{job_id}/cancel"),
            &json!({"reason": "wrong target"}),
            Some(&admin.access_token),
        )
        .await;
    cancel.assert_status(StatusCode::OK);
    let body: Value = cancel.json().expect("body");
    assert_eq!(body["status"], json!("cancelled"));
    assert_eq!(body["failure_reason"], json!("wrong target"));

    // Read returns the cancelled state.
    let read = app
        .get(
            &format!("/api/admin/dsar/jobs/{job_id}"),
            Some(&admin.access_token),
        )
        .await;
    read.assert_status(StatusCode::OK);
    assert_eq!(
        read.json::<Value>().expect("body")["status"],
        json!("cancelled")
    );
}

#[tokio::test]
async fn cancel_completed_job_is_409() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let admin = app.seed_admin().await.expect("admin");
    let target = app.seed_user().await.expect("target");

    let resp = app
        .post_json::<Value>(
            "/api/admin/dsar/jobs/export",
            &json!({"target_user_id": target.id, "reason": "audit"}),
            Some(&admin.access_token),
        )
        .await;
    resp.assert_status(StatusCode::CREATED);
    let job_id = resp.json::<Value>().expect("body")["job"]["id"]
        .as_str()
        .expect("id")
        .to_string();

    let cancel = app
        .post_json::<Value>(
            &format!("/api/admin/dsar/jobs/{job_id}/cancel"),
            &json!({}),
            Some(&admin.access_token),
        )
        .await;
    cancel.assert_status(StatusCode::CONFLICT);
}

#[tokio::test]
async fn list_filters_by_kind_and_status() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let admin = app.seed_admin().await.expect("admin");
    let user_a = app.seed_user().await.expect("a");
    let user_b = app.seed_user().await.expect("b");

    app.post_json::<Value>(
        "/api/admin/dsar/jobs/export",
        &json!({"target_user_id": user_a.id, "reason": "audit a"}),
        Some(&admin.access_token),
    )
    .await
    .assert_status(StatusCode::CREATED);

    app.post_json::<Value>(
        "/api/admin/dsar/jobs/erase/request",
        &json!({
            "target_user_id": user_b.id,
            "reason": "subject erasure for b account",
        }),
        Some(&admin.access_token),
    )
    .await
    .assert_status(StatusCode::CREATED);

    let exports = app
        .get(
            "/api/admin/dsar/jobs?kind=export",
            Some(&admin.access_token),
        )
        .await;
    exports.assert_status(StatusCode::OK);
    assert_eq!(exports.json::<Value>().expect("body")["total"], json!(1));

    let pending = app
        .get(
            "/api/admin/dsar/jobs?status=pending",
            Some(&admin.access_token),
        )
        .await;
    pending.assert_status(StatusCode::OK);
    assert_eq!(pending.json::<Value>().expect("body")["total"], json!(1));

    let bogus = app
        .get(
            "/api/admin/dsar/jobs?status=fictional",
            Some(&admin.access_token),
        )
        .await;
    bogus.assert_status(StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn read_unknown_job_is_404() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let admin = app.seed_admin().await.expect("admin");

    let resp = app
        .get(
            &format!("/api/admin/dsar/jobs/{}", Uuid::new_v4()),
            Some(&admin.access_token),
        )
        .await;
    resp.assert_status(StatusCode::NOT_FOUND);
}

// ── Tombstone parity check ─────────────────────────────────────────────

#[tokio::test]
async fn tombstone_clears_pii_columns_and_drops_refresh_tokens() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let requester = app.seed_admin().await.expect("requester");
    let approver = app.seed_admin().await.expect("approver");
    let target = app.seed_user().await.expect("target");

    // Pre-populate PII fields so the tombstone has work to do.
    sqlx::query(
        r#"
        UPDATE users
           SET name='Alice Doe', avatar_url='https://x/a.png', bio='hi',
               position='trader', website_url='https://alice.example',
               twitter_url='https://t/a', linkedin_url='https://l/a',
               youtube_url='https://y/a', instagram_url='https://i/a',
               suspension_reason='because', ban_reason='because2',
               email_verified_at=NOW()
         WHERE id = $1
        "#,
    )
    .bind(target.id)
    .execute(app.db())
    .await
    .expect("populate PII");

    // Refresh-token row already exists from seed; verify ≥1.
    let pre_tokens: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM refresh_tokens WHERE user_id = $1")
            .bind(target.id)
            .fetch_one(app.db())
            .await
            .expect("pre tokens");
    assert!(pre_tokens >= 1);

    // Run the dual-control flow.
    let req = app
        .post_json::<Value>(
            "/api/admin/dsar/jobs/erase/request",
            &json!({
                "target_user_id": target.id,
                "reason": "subject erasure full PII parity check",
            }),
            Some(&requester.access_token),
        )
        .await;
    req.assert_status(StatusCode::CREATED);
    let job_id = req.json::<Value>().expect("body")["id"]
        .as_str()
        .expect("id")
        .to_string();

    app.post_json::<Value>(
        &format!("/api/admin/dsar/jobs/{job_id}/erase/approve"),
        &json!({"approval_reason": "approved by second admin per policy"}),
        Some(&approver.access_token),
    )
    .await
    .assert_status(StatusCode::OK);

    // Confirm every PII column we populated is now scrubbed.
    let row = sqlx::query(
        r#"
        SELECT name, avatar_url, bio, position, website_url, twitter_url,
               linkedin_url, youtube_url, instagram_url, suspension_reason,
               ban_reason, email_verified_at
          FROM users WHERE id = $1
        "#,
    )
    .bind(target.id)
    .fetch_one(app.db())
    .await
    .expect("post user");
    let name: String = row.try_get("name").expect("name");
    assert_eq!(name, "");
    for col in [
        "avatar_url",
        "bio",
        "position",
        "website_url",
        "twitter_url",
        "linkedin_url",
        "youtube_url",
        "instagram_url",
        "suspension_reason",
        "ban_reason",
    ] {
        let v: Option<String> = row.try_get(col).expect("nullable text");
        assert!(
            v.is_none(),
            "column {col} should be NULL post-tombstone, got {v:?}"
        );
    }
    let verified: Option<chrono::DateTime<chrono::Utc>> =
        row.try_get("email_verified_at").expect("verified");
    assert!(verified.is_none());

    // Refresh tokens cleared.
    let post_tokens: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM refresh_tokens WHERE user_id = $1")
            .bind(target.id)
            .fetch_one(app.db())
            .await
            .expect("post tokens");
    assert_eq!(post_tokens, 0);
}
