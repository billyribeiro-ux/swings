#![deny(warnings)]
#![forbid(unsafe_code)]

//! ADM-05 integration coverage for `handlers::admin_security` and the
//! adjacent audit-log writes wired into `handlers::admin`.
//!
//! Every test here drives a real request through the in-process Axum
//! router, so authentication, RBAC, audit-row creation, and the database
//! schema are exercised end-to-end against the per-test ephemeral schema
//! provisioned by `TestApp`.
//!
//! Skipped automatically when neither `DATABASE_URL_TEST` nor
//! `DATABASE_URL` is set — the existing harness convention.

mod support;

use axum::http::StatusCode;
use serde_json::{json, Value};
use sqlx::Row;
use support::{AssertProblem, TestApp, TestRole};
use uuid::Uuid;

// ── Member lifecycle: suspend / reactivate ─────────────────────────────

#[tokio::test]
async fn suspend_then_reactivate_writes_audit_and_blocks_login() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };

    let admin = app.seed_admin().await.expect("seed admin");
    let target = app.seed_user().await.expect("seed target member");
    let pre_password = target.password.clone();

    // --- suspend ---
    let resp = app
        .post_json(
            &format!("/api/admin/members/{}/suspend", target.id),
            &json!({ "reason": "spam-policy-violation" }),
            Some(&admin.access_token),
        )
        .await;
    resp.assert_status(StatusCode::OK);
    let body: Value = resp.json().expect("suspend response body");
    assert_eq!(
        body["id"].as_str(),
        Some(target.id.to_string().as_str()),
        "response echoes target id"
    );
    assert!(
        body["suspended_at"].is_string(),
        "response includes suspended_at timestamp"
    );

    // --- audit row landed ---
    let audit_row =
        sqlx::query("SELECT action, target_kind, target_id, metadata FROM admin_actions WHERE actor_id = $1 ORDER BY created_at DESC LIMIT 1")
            .bind(admin.id)
            .fetch_one(app.db())
            .await
            .expect("audit row");
    let action: String = audit_row.get("action");
    let target_kind: String = audit_row.get("target_kind");
    let target_id: String = audit_row.get("target_id");
    let metadata: Value = audit_row.get("metadata");
    assert_eq!(action, "user.suspend");
    assert_eq!(target_kind, "user");
    assert_eq!(target_id, target.id.to_string());
    assert_eq!(metadata["reason"], "spam-policy-violation");

    // --- login fails with 401 (suspended) ---
    let login = app
        .post_json(
            "/api/auth/login",
            &json!({ "email": target.email, "password": pre_password.clone() }),
            None,
        )
        .await;
    login.assert_problem(AssertProblem {
        status: StatusCode::UNAUTHORIZED,
        type_suffix: "unauthorized",
        title: "Unauthorized",
    });

    // --- failed_login_attempts row landed with reason="suspended" ---
    let failed_reason: String = sqlx::query_scalar(
        "SELECT reason FROM failed_login_attempts WHERE email = $1 ORDER BY occurred_at DESC LIMIT 1",
    )
    .bind(target.email.to_lowercase())
    .fetch_one(app.db())
    .await
    .expect("failed login row");
    assert_eq!(failed_reason, "suspended");

    // --- reactivate ---
    let resp = app
        .post_json(
            &format!("/api/admin/members/{}/reactivate", target.id),
            &json!({}),
            Some(&admin.access_token),
        )
        .await;
    resp.assert_status(StatusCode::OK);

    // --- login now succeeds ---
    let login = app
        .post_json(
            "/api/auth/login",
            &json!({ "email": target.email, "password": pre_password }),
            None,
        )
        .await;
    login.assert_status(StatusCode::OK);
}

#[tokio::test]
async fn suspend_admin_account_is_rejected_409() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };

    let admin_actor = app.seed_admin().await.expect("seed actor admin");
    let admin_target = app.seed_admin().await.expect("seed target admin");

    let resp = app
        .post_json(
            &format!("/api/admin/members/{}/suspend", admin_target.id),
            &json!({ "reason": "self-test" }),
            Some(&admin_actor.access_token),
        )
        .await;
    resp.assert_status(StatusCode::CONFLICT);
}

// ── RBAC enforcement ───────────────────────────────────────────────────

#[tokio::test]
async fn member_cannot_call_admin_security_endpoints() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };

    let member = app.seed_user().await.expect("seed member");
    let target = app.seed_user().await.expect("seed target");

    let resp = app
        .post_json(
            &format!("/api/admin/members/{}/suspend", target.id),
            &json!({}),
            Some(&member.access_token),
        )
        .await;
    resp.assert_problem(AssertProblem {
        status: StatusCode::FORBIDDEN,
        type_suffix: "forbidden",
        title: "Forbidden",
    });
}

#[tokio::test]
async fn unauthenticated_admin_security_call_is_401() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };

    let target = app.seed_user().await.expect("seed target");
    let resp = app
        .post_json(
            &format!("/api/admin/members/{}/suspend", target.id),
            &json!({}),
            None,
        )
        .await;
    resp.assert_status(StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn support_can_suspend_but_cannot_ban() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };

    let support = app.seed_support().await.expect("seed support");
    let target = app.seed_user().await.expect("seed target");

    // suspend → 200 (support has user.suspend per 058)
    let resp = app
        .post_json(
            &format!("/api/admin/members/{}/suspend", target.id),
            &json!({ "reason": "support test" }),
            Some(&support.access_token),
        )
        .await;
    resp.assert_status(StatusCode::OK);

    // ban → 403 (support does NOT have user.ban)
    let resp = app
        .post_json(
            &format!("/api/admin/members/{}/ban", target.id),
            &json!({ "reason": "support test" }),
            Some(&support.access_token),
        )
        .await;
    resp.assert_problem(AssertProblem {
        status: StatusCode::FORBIDDEN,
        type_suffix: "forbidden",
        title: "Forbidden",
    });
}

// ── Sessions: list + force-logout ──────────────────────────────────────

#[tokio::test]
async fn list_and_revoke_member_sessions() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };

    let admin = app.seed_admin().await.expect("seed admin");
    let target = app.seed_user().await.expect("seed target");

    // The target is seeded with one active refresh token by the harness.
    let resp = app
        .get(
            &format!("/api/admin/members/{}/sessions", target.id),
            Some(&admin.access_token),
        )
        .await;
    resp.assert_status(StatusCode::OK);
    let body: Value = resp.json().expect("sessions body");
    assert_eq!(body["user_id"], json!(target.id));
    assert_eq!(body["total"], json!(1));
    assert!(
        body["active_sessions"]
            .as_array()
            .is_some_and(|a| a.len() == 1),
        "exactly one active session present"
    );

    // Force logout — DELETE on the collection.
    let resp = app
        .delete(
            &format!("/api/admin/members/{}/sessions", target.id),
            Some(&admin.access_token),
        )
        .await;
    resp.assert_status(StatusCode::OK);
    let body: Value = resp.json().expect("force-logout body");
    assert_eq!(body["revoked_count"], json!(1));

    // List again — empty.
    let resp = app
        .get(
            &format!("/api/admin/members/{}/sessions", target.id),
            Some(&admin.access_token),
        )
        .await;
    let body: Value = resp.json().expect("post-revoke body");
    assert_eq!(body["total"], json!(0));

    // Audit row recorded.
    let action: String = sqlx::query_scalar(
        "SELECT action FROM admin_actions WHERE actor_id = $1 AND target_kind = 'user' AND action LIKE 'user.session.%' ORDER BY created_at DESC LIMIT 1",
    )
    .bind(admin.id)
    .fetch_one(app.db())
    .await
    .expect("session audit row");
    assert_eq!(action, "user.session.revoke_all");
}

#[tokio::test]
async fn revoke_specific_session_is_owner_scoped() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };

    let admin = app.seed_admin().await.expect("seed admin");
    let alice = app.seed_user().await.expect("seed alice");
    let bob = app.seed_user().await.expect("seed bob");

    // Pick alice's session id.
    let alice_session: Uuid =
        sqlx::query_scalar("SELECT id FROM refresh_tokens WHERE user_id = $1 LIMIT 1")
            .bind(alice.id)
            .fetch_one(app.db())
            .await
            .expect("alice session id");

    // Trying to revoke alice's session via bob's URL → 404.
    let resp = app
        .delete(
            &format!("/api/admin/members/{}/sessions/{}", bob.id, alice_session),
            Some(&admin.access_token),
        )
        .await;
    resp.assert_status(StatusCode::NOT_FOUND);

    // Through alice's URL → 200.
    let resp = app
        .delete(
            &format!("/api/admin/members/{}/sessions/{}", alice.id, alice_session),
            Some(&admin.access_token),
        )
        .await;
    resp.assert_status(StatusCode::OK);
}

// ── Audit-log viewer ───────────────────────────────────────────────────

#[tokio::test]
async fn audit_log_viewer_returns_recent_actions() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };

    let admin = app.seed_admin().await.expect("seed admin");
    let target = app.seed_user().await.expect("seed target");

    // Generate two distinct audit actions.
    let _ = app
        .post_json(
            &format!("/api/admin/members/{}/suspend", target.id),
            &json!({ "reason": "audit-test" }),
            Some(&admin.access_token),
        )
        .await;
    let _ = app
        .post_json(
            &format!("/api/admin/members/{}/reactivate", target.id),
            &json!({}),
            Some(&admin.access_token),
        )
        .await;

    let resp = app
        .get(
            "/api/admin/security/audit-log?per_page=10",
            Some(&admin.access_token),
        )
        .await;
    resp.assert_status(StatusCode::OK);
    let body: Value = resp.json().expect("audit-log body");
    assert!(body["total"].as_i64().unwrap_or(0) >= 2);
    let empty: Vec<Value> = Vec::new();
    let actions: Vec<&str> = body["data"]
        .as_array()
        .unwrap_or(&empty)
        .iter()
        .filter_map(|r| r["action"].as_str())
        .collect();
    assert!(actions.contains(&"user.suspend"));
    assert!(actions.contains(&"user.reactivate"));
}

#[tokio::test]
async fn audit_log_filter_by_action_works() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };

    let admin = app.seed_admin().await.expect("seed admin");
    let target = app.seed_user().await.expect("seed target");

    let _ = app
        .post_json(
            &format!("/api/admin/members/{}/suspend", target.id),
            &json!({ "reason": "filter-test" }),
            Some(&admin.access_token),
        )
        .await;
    let _ = app
        .post_json(
            &format!("/api/admin/members/{}/reactivate", target.id),
            &json!({}),
            Some(&admin.access_token),
        )
        .await;

    let resp = app
        .get(
            "/api/admin/security/audit-log?action=user.suspend",
            Some(&admin.access_token),
        )
        .await;
    let body: Value = resp.json().expect("audit-log body");
    assert!(body["total"].as_i64().unwrap_or(0) >= 1);
    let empty: Vec<Value> = Vec::new();
    for row in body["data"].as_array().unwrap_or(&empty) {
        assert_eq!(row["action"], "user.suspend");
    }
}

// ── Failed-login viewer ────────────────────────────────────────────────

#[tokio::test]
async fn failed_login_viewer_lists_attempts() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };

    let admin = app.seed_admin().await.expect("seed admin");

    // Drive a couple of failed logins via the public endpoint so the
    // pipeline (handler → ClientInfo → db::record_failed_login) is exercised.
    let _ = app
        .post_json(
            "/api/auth/login",
            &json!({ "email": "ghost@example.test", "password": "nope" }),
            None,
        )
        .await;
    let _ = app
        .post_json(
            "/api/auth/login",
            &json!({ "email": "ghost@example.test", "password": "still-nope" }),
            None,
        )
        .await;

    let resp = app
        .get(
            "/api/admin/security/failed-logins?since_hours=1",
            Some(&admin.access_token),
        )
        .await;
    resp.assert_status(StatusCode::OK);
    let body: Value = resp.json().expect("failed-logins body");
    assert!(
        body["total"].as_i64().unwrap_or(0) >= 2,
        "should see at least the two ghost-account attempts"
    );
    let row = &body["data"][0];
    assert_eq!(row["reason"], "unknown_email");
    assert_eq!(row["email"], "ghost@example.test");
}

// ── Email verification (idempotent) ────────────────────────────────────

#[tokio::test]
async fn mark_email_verified_is_idempotent_and_audited_once() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };

    let admin = app.seed_admin().await.expect("seed admin");
    let target = app.seed_user().await.expect("seed target");

    let first = app
        .post_json(
            &format!("/api/admin/members/{}/verify-email", target.id),
            &json!({}),
            Some(&admin.access_token),
        )
        .await;
    first.assert_status(StatusCode::OK);
    let body: Value = first.json().expect("verify body");
    assert!(body["email_verified_at"].is_string());

    let second = app
        .post_json(
            &format!("/api/admin/members/{}/verify-email", target.id),
            &json!({}),
            Some(&admin.access_token),
        )
        .await;
    second.assert_status(StatusCode::OK);

    // Audit table should only contain ONE `user.email.verify` row.
    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*)::bigint FROM admin_actions WHERE action = 'user.email.verify' AND target_id = $1",
    )
    .bind(target.id.to_string())
    .fetch_one(app.db())
    .await
    .expect("count");
    assert_eq!(count, 1, "second call must be a no-op (no duplicate audit)");
}

// ── Force password reset ──────────────────────────────────────────────

#[tokio::test]
async fn force_password_reset_creates_token_revokes_sessions_and_audits() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };

    let admin = app.seed_admin().await.expect("seed admin");
    let target = app.seed_user().await.expect("seed target");

    let resp = app
        .post_json(
            &format!("/api/admin/members/{}/force-password-reset", target.id),
            &json!({}),
            Some(&admin.access_token),
        )
        .await;
    resp.assert_status(StatusCode::OK);
    let body: Value = resp.json().expect("force-reset body");
    assert_eq!(body["user_id"], json!(target.id));

    // Refresh tokens for the target are gone.
    let count: i64 =
        sqlx::query_scalar("SELECT COUNT(*)::bigint FROM refresh_tokens WHERE user_id = $1")
            .bind(target.id)
            .fetch_one(app.db())
            .await
            .expect("count tokens");
    assert_eq!(count, 0);

    // A password_reset_tokens row exists.
    let token_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*)::bigint FROM password_reset_tokens WHERE user_id = $1 AND used = FALSE",
    )
    .bind(target.id)
    .fetch_one(app.db())
    .await
    .expect("count reset tokens");
    assert_eq!(token_count, 1);

    // Audit row recorded.
    let action: String = sqlx::query_scalar(
        "SELECT action FROM admin_actions WHERE target_id = $1 ORDER BY created_at DESC LIMIT 1",
    )
    .bind(target.id.to_string())
    .fetch_one(app.db())
    .await
    .expect("audit row");
    assert_eq!(action, "user.force_password_reset");
}

// ── Admin actions on existing routes (admin.rs) ────────────────────────

#[tokio::test]
async fn delete_member_writes_audit_row() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };

    let admin = app.seed_admin().await.expect("seed admin");
    let target = app
        .seed_user_with_role(TestRole::Member)
        .await
        .expect("seed target");

    let resp = app
        .delete(
            &format!("/api/admin/members/{}", target.id),
            Some(&admin.access_token),
        )
        .await;
    resp.assert_status(StatusCode::OK);

    let action: String = sqlx::query_scalar(
        "SELECT action FROM admin_actions WHERE target_id = $1 AND target_kind = 'user' ORDER BY created_at DESC LIMIT 1",
    )
    .bind(target.id.to_string())
    .fetch_one(app.db())
    .await
    .expect("audit row");
    assert_eq!(action, "user.delete");
}
