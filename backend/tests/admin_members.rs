#![deny(warnings)]
#![forbid(unsafe_code)]

//! ADM-10 integration coverage for the admin members search + manual-create
//! surface.

mod support;

use axum::http::StatusCode;
use serde_json::{json, Value};
use sqlx::Row;
use support::{AssertProblem, TestApp};

// ── RBAC gates ─────────────────────────────────────────────────────────

#[tokio::test]
async fn search_requires_admin_member_read() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    // Plain member must be locked out.
    let member = app.seed_user().await.expect("seed member");
    let resp = app
        .get(
            "/api/admin/members/search?q=any",
            Some(&member.access_token),
        )
        .await;
    resp.assert_status(StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn support_can_search_but_not_create() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let support = app.seed_support().await.expect("seed support");

    // Migration 066 grants `admin.member.read` to support.
    let search = app
        .get("/api/admin/members/search", Some(&support.access_token))
        .await;
    search.assert_status(StatusCode::OK);

    let create = app
        .post_json::<Value>(
            "/api/admin/members",
            &json!({
                "email": "no-create-by-support@example.com",
                "name":  "Nope",
                "role":  "member"
            }),
            Some(&support.access_token),
        )
        .await;
    create.assert_status(StatusCode::FORBIDDEN);
}

// ── Search ─────────────────────────────────────────────────────────────

#[tokio::test]
async fn search_returns_paginated_envelope() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let admin = app.seed_admin().await.expect("seed admin");

    let resp = app
        .get("/api/admin/members/search", Some(&admin.access_token))
        .await;
    resp.assert_status(StatusCode::OK);
    let body: Value = resp.json().expect("envelope");

    // Required pagination keys.
    for k in ["data", "total", "page", "per_page", "total_pages"] {
        assert!(body.get(k).is_some(), "missing key {k}: {body:?}");
    }
    assert!(body["data"].is_array());
    // Default page size from the handler.
    assert_eq!(body["per_page"], json!(25));
}

#[tokio::test]
async fn search_filters_by_substring_and_role() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let admin = app.seed_admin().await.expect("seed admin");

    // Create three members with predictable shapes.
    for body in [
        json!({"email": "alice.cooper@example.com", "name": "Alice Cooper", "role": "member"}),
        json!({"email": "bob.dylan@example.com",    "name": "Bob Dylan",    "role": "author"}),
        json!({"email": "carol.king@example.com",   "name": "Carol King",   "role": "member"}),
    ] {
        app.post_json::<Value>("/api/admin/members", &body, Some(&admin.access_token))
            .await
            .assert_status(StatusCode::CREATED);
    }

    // Substring across email + name (case-insensitive).
    let by_name = app
        .get(
            "/api/admin/members/search?q=COOPER",
            Some(&admin.access_token),
        )
        .await;
    by_name.assert_status(StatusCode::OK);
    let body: Value = by_name.json().expect("body");
    let emails: Vec<&str> = body["data"]
        .as_array()
        .expect("array")
        .iter()
        .map(|r| r["email"].as_str().expect("email"))
        .collect();
    assert!(emails.contains(&"alice.cooper@example.com"));
    assert!(!emails.contains(&"bob.dylan@example.com"));

    // Role filter must narrow to author rows.
    let by_role = app
        .get(
            "/api/admin/members/search?role=author",
            Some(&admin.access_token),
        )
        .await;
    by_role.assert_status(StatusCode::OK);
    let body: Value = by_role.json().expect("body");
    for row in body["data"].as_array().expect("array") {
        assert_eq!(row["role"], json!("author"));
    }
}

#[tokio::test]
async fn search_unknown_role_filter_is_400() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let admin = app.seed_admin().await.expect("seed admin");

    let resp = app
        .get(
            "/api/admin/members/search?role=superadmin",
            Some(&admin.access_token),
        )
        .await;
    resp.assert_problem(AssertProblem {
        status: StatusCode::BAD_REQUEST,
        type_suffix: "bad-request",
        title: "Bad Request",
    });
}

#[tokio::test]
async fn search_status_filter_targets_lifecycle_state() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let admin = app.seed_admin().await.expect("seed admin");

    // Seed two members; suspend one directly.
    for body in [
        json!({"email": "active.dude@example.com",   "name": "Active Dude",   "role": "member", "email_verified": true}),
        json!({"email": "suspended.dude@example.com","name": "Suspended Dude","role": "member", "email_verified": true}),
    ] {
        app.post_json::<Value>("/api/admin/members", &body, Some(&admin.access_token))
            .await
            .assert_status(StatusCode::CREATED);
    }
    sqlx::query(
        "UPDATE users SET suspended_at = NOW(), suspension_reason = 'test' WHERE email = $1",
    )
    .bind("suspended.dude@example.com")
    .execute(app.db())
    .await
    .expect("suspend");

    let suspended = app
        .get(
            "/api/admin/members/search?status=suspended",
            Some(&admin.access_token),
        )
        .await;
    suspended.assert_status(StatusCode::OK);
    let body: Value = suspended.json().expect("body");
    let emails: Vec<&str> = body["data"]
        .as_array()
        .expect("array")
        .iter()
        .map(|r| r["email"].as_str().expect("email"))
        .collect();
    assert!(emails.contains(&"suspended.dude@example.com"));
    assert!(!emails.contains(&"active.dude@example.com"));
}

// ── Create ─────────────────────────────────────────────────────────────

#[tokio::test]
async fn create_member_without_password_marks_requires_setup() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let admin = app.seed_admin().await.expect("seed admin");

    let resp = app
        .post_json::<Value>(
            "/api/admin/members",
            &json!({
                "email": "no-pass@example.com",
                "name":  "No Pass",
                "role":  "member"
            }),
            Some(&admin.access_token),
        )
        .await;
    resp.assert_status(StatusCode::CREATED);
    let body: Value = resp.json().expect("body");
    assert_eq!(body["requires_password_setup"], json!(true));
    assert_eq!(body["user"]["email"], json!("no-pass@example.com"));
    assert_eq!(body["user"]["role"], json!("member"));

    // Audit row landed.
    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM admin_actions WHERE action = 'admin.member.create' AND target_kind = 'user'"
    )
    .fetch_one(app.db())
    .await
    .expect("count");
    assert!(count >= 1);
}

#[tokio::test]
async fn create_member_with_temp_password_logs_in() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let admin = app.seed_admin().await.expect("seed admin");

    let resp = app
        .post_json::<Value>(
            "/api/admin/members",
            &json!({
                "email":         "with-pass@example.com",
                "name":          "With Pass",
                "role":          "author",
                "temp_password": "Sup3rSecretSeed!",
                "email_verified": true
            }),
            Some(&admin.access_token),
        )
        .await;
    resp.assert_status(StatusCode::CREATED);
    let body: Value = resp.json().expect("body");
    assert_eq!(body["requires_password_setup"], json!(false));

    // Verify the temp password actually authenticates by hitting the
    // public login endpoint — proves the hash format matches the
    // self-serve registration path.
    let login = app
        .post_json::<Value>(
            "/api/auth/login",
            &json!({
                "email":    "with-pass@example.com",
                "password": "Sup3rSecretSeed!"
            }),
            None,
        )
        .await;
    login.assert_status(StatusCode::OK);
}

#[tokio::test]
async fn can_create_admin_via_manual_create_when_actor_has_role_manage() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let admin = app.seed_admin().await.expect("seed admin");

    let resp = app
        .post_json::<Value>(
            "/api/admin/members",
            &json!({
                "email": "new.admin@example.com",
                "name":  "New Admin",
                "role":  "admin",
                "email_verified": true
            }),
            Some(&admin.access_token),
        )
        .await;
    resp.assert_status(StatusCode::CREATED);
    let body: Value = resp.json().expect("body");
    assert_eq!(body["user"]["role"], json!("admin"));
    assert_eq!(body["user"]["email"], json!("new.admin@example.com"));
}

#[tokio::test]
async fn duplicate_email_returns_409() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let admin = app.seed_admin().await.expect("seed admin");

    let body = json!({
        "email": "dup@example.com",
        "name":  "Dup",
        "role":  "member"
    });
    let first = app
        .post_json::<Value>("/api/admin/members", &body, Some(&admin.access_token))
        .await;
    first.assert_status(StatusCode::CREATED);

    let second = app
        .post_json::<Value>("/api/admin/members", &body, Some(&admin.access_token))
        .await;
    second.assert_status(StatusCode::CONFLICT);
}

#[tokio::test]
async fn invalid_email_short_password_rejected() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let admin = app.seed_admin().await.expect("seed admin");

    let bad_email = app
        .post_json::<Value>(
            "/api/admin/members",
            &json!({
                "email": "not-an-email",
                "name":  "Whatever",
                "role":  "member"
            }),
            Some(&admin.access_token),
        )
        .await;
    bad_email.assert_status(StatusCode::BAD_REQUEST);

    let short_pw = app
        .post_json::<Value>(
            "/api/admin/members",
            &json!({
                "email": "another@example.com",
                "name":  "Short PW",
                "role":  "member",
                "temp_password": "abc"
            }),
            Some(&admin.access_token),
        )
        .await;
    short_pw.assert_status(StatusCode::BAD_REQUEST);
}

// ── Email verification flag round-trips into the DB ────────────────────

#[tokio::test]
async fn email_verified_flag_persists_on_create() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let admin = app.seed_admin().await.expect("seed admin");

    let resp = app
        .post_json::<Value>(
            "/api/admin/members",
            &json!({
                "email": "verified@example.com",
                "name":  "Verified",
                "role":  "member",
                "email_verified": true
            }),
            Some(&admin.access_token),
        )
        .await;
    resp.assert_status(StatusCode::CREATED);

    let row = sqlx::query("SELECT email_verified_at FROM users WHERE email = $1")
        .bind("verified@example.com")
        .fetch_one(app.db())
        .await
        .expect("row");
    let verified_at: Option<chrono::DateTime<chrono::Utc>> =
        row.try_get("email_verified_at").expect("col");
    assert!(verified_at.is_some(), "email_verified_at should be set");
}
