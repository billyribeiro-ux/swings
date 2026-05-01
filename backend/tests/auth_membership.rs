#![deny(warnings)]
#![forbid(unsafe_code)]

//! End-to-end integration tests for the auth + membership surface.
//!
//! Covers the gaps identified in the 2026-05-01 audit:
//! - Public registration: success, duplicate email, weak password, bad email
//! - Login gates: banned users, suspended users, expired suspension lifted
//! - Refresh token rotation + reuse detection
//! - RBAC: member hitting admin routes gets 403; unauthenticated gets 401
//! - Logout invalidates refresh token
//! - Password reset: non-existent email returns 200 (no enumeration),
//!   invalid token returns 4xx
//! - Email verification: token is issued on register; valid token marks user
//!   verified in DB

mod support;

use axum::http::StatusCode;
use chrono::{Duration, Utc};
use serde_json::{json, Value};
use support::TestApp;
use swings_api::crypto::hash_token;
use uuid::Uuid;

// ── Registration ─────────────────────────────────────────────────────────────

#[tokio::test]
async fn register_success_returns_tokens_and_member_role() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };

    let email = format!("reg-{}@test.test", Uuid::new_v4().simple());
    let resp = app
        .post_json(
            "/api/auth/register",
            &json!({
                "email": email,
                "password": "password-secure-123",
                "name": "Test User"
            }),
            None,
        )
        .await;

    resp.assert_status(StatusCode::OK);
    let body: Value = resp.json().expect("register body");
    assert_eq!(body["user"]["role"], "member", "new registrant must be 'member'");
    assert!(body["access_token"].as_str().is_some(), "access_token missing");
    assert!(body["refresh_token"].as_str().is_some(), "refresh_token missing");

    // BFF cookies must be set in response headers
    let set_cookies: Vec<String> = resp
        .headers()
        .get_all("set-cookie")
        .iter()
        .filter_map(|v| v.to_str().ok().map(str::to_owned))
        .collect();
    assert!(
        set_cookies.iter().any(|c| c.starts_with("swings_access=")),
        "swings_access cookie missing; got: {set_cookies:?}"
    );
    assert!(
        set_cookies.iter().any(|c| c.starts_with("swings_refresh=")),
        "swings_refresh cookie missing; got: {set_cookies:?}"
    );
}

#[tokio::test]
async fn register_duplicate_email_returns_409() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };

    let email = format!("dup-{}@test.test", Uuid::new_v4().simple());
    let body = json!({
        "email": email,
        "password": "password-secure-123",
        "name": "Test User"
    });

    let r1 = app.post_json("/api/auth/register", &body, None).await;
    r1.assert_status(StatusCode::OK);

    let r2 = app.post_json("/api/auth/register", &body, None).await;
    r2.assert_status(StatusCode::CONFLICT);
}

#[tokio::test]
async fn register_short_password_returns_4xx() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };

    let resp = app
        .post_json(
            "/api/auth/register",
            &json!({
                "email": format!("weak-{}@test.test", Uuid::new_v4().simple()),
                "password": "short",   // < 8 chars
                "name": "Test User"
            }),
            None,
        )
        .await;

    assert!(
        resp.status().is_client_error(),
        "short password should be rejected, got {}",
        resp.status()
    );
}

#[tokio::test]
async fn register_invalid_email_returns_4xx() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };

    let resp = app
        .post_json(
            "/api/auth/register",
            &json!({
                "email": "not-an-email",
                "password": "password-secure-123",
                "name": "Test"
            }),
            None,
        )
        .await;

    assert!(
        resp.status().is_client_error(),
        "invalid email should be rejected, got {}",
        resp.status()
    );
}

// ── Login gates ───────────────────────────────────────────────────────────────

#[tokio::test]
async fn login_banned_user_returns_401() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };

    let user = app.seed_user().await.expect("seed member");

    sqlx::query("UPDATE users SET banned_at = NOW() WHERE id = $1")
        .bind(user.id)
        .execute(app.db())
        .await
        .expect("ban user");

    let resp = app
        .post_json(
            "/api/auth/login",
            &json!({ "email": user.email, "password": user.password }),
            None,
        )
        .await;

    resp.assert_status(StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn login_suspended_user_returns_401() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };

    let user = app.seed_user().await.expect("seed member");

    sqlx::query(
        "UPDATE users SET suspended_at = NOW(), suspended_until = NOW() + INTERVAL '30 days' WHERE id = $1",
    )
    .bind(user.id)
    .execute(app.db())
    .await
    .expect("suspend user");

    let resp = app
        .post_json(
            "/api/auth/login",
            &json!({ "email": user.email, "password": user.password }),
            None,
        )
        .await;

    resp.assert_status(StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn login_expired_suspension_is_lifted_and_succeeds() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };

    let user = app.seed_user().await.expect("seed member");

    // Suspension window is entirely in the past
    sqlx::query(
        "UPDATE users SET suspended_at = NOW() - INTERVAL '2 days', suspended_until = NOW() - INTERVAL '1 day' WHERE id = $1",
    )
    .bind(user.id)
    .execute(app.db())
    .await
    .expect("set expired suspension");

    let resp = app
        .post_json(
            "/api/auth/login",
            &json!({ "email": user.email, "password": user.password }),
            None,
        )
        .await;

    // Expired suspension must be lifted lazily on next login
    resp.assert_status(StatusCode::OK);
}

// ── RBAC ─────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn member_cannot_access_admin_subscriptions() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let member = app.seed_user().await.expect("seed member");
    app.get("/api/admin/subscriptions", Some(&member.access_token))
        .await
        .assert_status(StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn member_cannot_access_admin_members() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let member = app.seed_user().await.expect("seed member");
    app.get("/api/admin/members", Some(&member.access_token))
        .await
        .assert_status(StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn member_cannot_access_admin_audit_log() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let member = app.seed_user().await.expect("seed member");
    app.get("/api/admin/audit", Some(&member.access_token))
        .await
        .assert_status(StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn member_cannot_access_admin_pricing() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let member = app.seed_user().await.expect("seed member");
    app.get("/api/admin/pricing/plans", Some(&member.access_token))
        .await
        .assert_status(StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn unauthenticated_request_to_admin_route_returns_401() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    app.get("/api/admin/subscriptions", None)
        .await
        .assert_status(StatusCode::UNAUTHORIZED);
}

// ── Refresh token rotation ────────────────────────────────────────────────────

#[tokio::test]
async fn refresh_token_rotation_returns_new_pair() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };

    let user = app.seed_user().await.expect("seed member");

    let resp = app
        .post_json(
            "/api/auth/refresh",
            &json!({ "refresh_token": user.refresh_token }),
            None,
        )
        .await;

    resp.assert_status(StatusCode::OK);
    let body: Value = resp.json().expect("refresh body");
    let new_access = body["access_token"].as_str().expect("new access_token");
    let new_refresh = body["refresh_token"].as_str().expect("new refresh_token");

    assert_ne!(new_access, user.access_token, "access token must rotate");
    assert_ne!(new_refresh, user.refresh_token, "refresh token must rotate");

    // New access token must authenticate
    app.get("/api/auth/me", Some(new_access))
        .await
        .assert_status(StatusCode::OK);
}

#[tokio::test]
async fn used_refresh_token_cannot_be_reused() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };

    let user = app.seed_user().await.expect("seed member");

    // First rotation — succeeds
    let r1 = app
        .post_json(
            "/api/auth/refresh",
            &json!({ "refresh_token": user.refresh_token }),
            None,
        )
        .await;
    r1.assert_status(StatusCode::OK);

    // Replay the now-spent token — must be rejected
    let r2 = app
        .post_json(
            "/api/auth/refresh",
            &json!({ "refresh_token": user.refresh_token }),
            None,
        )
        .await;
    assert!(
        r2.status() == StatusCode::UNAUTHORIZED || r2.status() == StatusCode::FORBIDDEN,
        "reused refresh token must be rejected, got {}",
        r2.status()
    );
}

// ── Logout ────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn logout_prevents_refresh_token_reuse() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };

    let user = app.seed_user().await.expect("seed member");

    app.post_json("/api/auth/logout", &json!({}), Some(&user.access_token))
        .await
        .assert_status(StatusCode::OK);

    let refresh = app
        .post_json(
            "/api/auth/refresh",
            &json!({ "refresh_token": user.refresh_token }),
            None,
        )
        .await;
    assert!(
        refresh.status() == StatusCode::UNAUTHORIZED || refresh.status() == StatusCode::FORBIDDEN,
        "refresh after logout must fail, got {}",
        refresh.status()
    );
}

// ── Password reset ────────────────────────────────────────────────────────────

#[tokio::test]
async fn forgot_password_always_returns_200_no_enumeration() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };

    // Non-existent email must not reveal account existence
    app.post_json(
        "/api/auth/forgot-password",
        &json!({ "email": "nobody@nowhere.test" }),
        None,
    )
    .await
    .assert_status(StatusCode::OK);
}

#[tokio::test]
async fn reset_password_with_invalid_token_returns_4xx() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };

    let resp = app
        .post_json(
            "/api/auth/reset-password",
            &json!({
                "token": Uuid::new_v4().to_string(),
                "new_password": "brand-new-secure-pw"
            }),
            None,
        )
        .await;

    assert!(
        resp.status().is_client_error(),
        "invalid reset token must be rejected, got {}",
        resp.status()
    );
}

// ── Email verification ────────────────────────────────────────────────────────

#[tokio::test]
async fn register_issues_email_verification_token_in_db() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };

    let email = format!("verify-{}@test.test", Uuid::new_v4().simple());
    let resp = app
        .post_json(
            "/api/auth/register",
            &json!({
                "email": email,
                "password": "password-secure-123",
                "name": "Verify Test"
            }),
            None,
        )
        .await;
    resp.assert_status(StatusCode::OK);
    let body: Value = resp.json().expect("register body");
    let user_id: Uuid = body["user"]["id"]
        .as_str()
        .and_then(|s| s.parse().ok())
        .expect("user id in response");

    // A verification token row must exist for this user
    let count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM email_verification_tokens WHERE user_id = $1")
            .bind(user_id)
            .fetch_one(app.db())
            .await
            .expect("count verification tokens");

    assert!(count > 0, "email verification token must be issued on register");
}

#[tokio::test]
async fn verify_email_with_valid_token_marks_user_verified() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };

    let member = app.seed_user().await.expect("seed member");

    // Seed a known raw token directly: compute the hash ourselves using the
    // same function the handler uses, insert it, then call the endpoint with
    // the raw token. This roundtrip proves the handler finds and accepts the
    // token without relying on email delivery.
    let raw_token = Uuid::new_v4().to_string();
    let token_hash = hash_token(&raw_token);
    let expires_at = Utc::now() + Duration::hours(24);

    sqlx::query(
        "INSERT INTO email_verification_tokens (id, user_id, token_hash, expires_at)
         VALUES ($1, $2, $3, $4)",
    )
    .bind(Uuid::new_v4())
    .bind(member.id)
    .bind(&token_hash)
    .bind(expires_at)
    .execute(app.db())
    .await
    .expect("seed verification token");

    let resp = app
        .post_json(
            "/api/auth/verify-email",
            &json!({ "token": raw_token }),
            None,
        )
        .await;

    resp.assert_status(StatusCode::OK);
    let body: Value = resp.json().expect("verify-email body");
    assert_eq!(body["verified"], true);

    // email_verified_at must now be set
    let verified_at: Option<chrono::DateTime<Utc>> = sqlx::query_scalar(
        "SELECT email_verified_at FROM users WHERE id = $1",
    )
    .bind(member.id)
    .fetch_one(app.db())
    .await
    .expect("fetch email_verified_at");

    assert!(
        verified_at.is_some(),
        "email_verified_at must be set after successful verification"
    );

    // token must be marked used (used_at IS NOT NULL)
    let used_at: Option<chrono::DateTime<Utc>> = sqlx::query_scalar(
        "SELECT used_at FROM email_verification_tokens WHERE token_hash = $1",
    )
    .bind(&token_hash)
    .fetch_one(app.db())
    .await
    .expect("fetch used_at");

    assert!(used_at.is_some(), "token must be marked used after consumption");
}

#[tokio::test]
async fn verify_email_with_expired_token_returns_4xx() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };

    let member = app.seed_user().await.expect("seed member");

    let raw_token = Uuid::new_v4().to_string();
    let token_hash = hash_token(&raw_token);
    // expires_at is in the past — the handler must reject it
    let expired_at = Utc::now() - Duration::seconds(1);

    sqlx::query(
        "INSERT INTO email_verification_tokens (id, user_id, token_hash, expires_at)
         VALUES ($1, $2, $3, $4)",
    )
    .bind(Uuid::new_v4())
    .bind(member.id)
    .bind(&token_hash)
    .bind(expired_at)
    .execute(app.db())
    .await
    .expect("seed expired verification token");

    let resp = app
        .post_json(
            "/api/auth/verify-email",
            &json!({ "token": raw_token }),
            None,
        )
        .await;

    assert!(
        resp.status().is_client_error(),
        "expired token must be rejected, got {}",
        resp.status()
    );
}

#[tokio::test]
async fn verify_email_token_cannot_be_reused() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };

    let member = app.seed_user().await.expect("seed member");

    let raw_token = Uuid::new_v4().to_string();
    let token_hash = hash_token(&raw_token);
    let expires_at = Utc::now() + Duration::hours(24);

    sqlx::query(
        "INSERT INTO email_verification_tokens (id, user_id, token_hash, expires_at)
         VALUES ($1, $2, $3, $4)",
    )
    .bind(Uuid::new_v4())
    .bind(member.id)
    .bind(&token_hash)
    .bind(expires_at)
    .execute(app.db())
    .await
    .expect("seed verification token");

    // First use — must succeed
    app.post_json(
        "/api/auth/verify-email",
        &json!({ "token": raw_token }),
        None,
    )
    .await
    .assert_status(StatusCode::OK);

    // Second use of the same token — must be rejected
    let resp = app
        .post_json(
            "/api/auth/verify-email",
            &json!({ "token": raw_token }),
            None,
        )
        .await;

    assert!(
        resp.status().is_client_error(),
        "reused verification token must be rejected, got {}",
        resp.status()
    );
}
