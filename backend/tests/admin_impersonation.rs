#![deny(warnings)]
#![forbid(unsafe_code)]

//! ADM-07 integration coverage for the admin impersonation surface.
//!
//! These tests drive real HTTP requests through the in-process Axum
//! router and verify (in this order):
//!
//!   1. RBAC + auth gating on the four /api/admin/security/impersonation
//!      routes (member, support, unauth).
//!   2. Mint validation: empty reason, ttl cap, admin-on-admin refusal,
//!      self-impersonation refusal, missing target.
//!   3. JWT contract: returned token decodes to the expected
//!      `imp_session` / `imp_actor` claims and `sub` == target.
//!   4. Server-side enforcement: impersonation token cannot reach
//!      AdminUser / PrivilegedUser endpoints.
//!   5. Banner contract: every response under impersonation carries the
//!      `X-Impersonation-*` header set.
//!   6. Revocation: revoked tokens are rejected with 401 on subsequent
//!      requests (defence against the stateless-JWT footgun).
//!   7. Audit trail: mint / revoke / exit each write one
//!      `admin_actions` row with the expected metadata.
//!   8. Self-exit endpoint: ends the session and audits against the
//!      real admin actor.
//!
//! Skipped automatically when neither `DATABASE_URL_TEST` nor
//! `DATABASE_URL` is set.

mod support;

use axum::http::StatusCode;
use jsonwebtoken::{decode, DecodingKey, Validation};
use serde_json::{json, Value};
use sqlx::Row;
use support::{test_jwt_secret_current, AssertProblem, TestApp};
use swings_api::extractors::Claims;

// ── RBAC + auth ────────────────────────────────────────────────────────

#[tokio::test]
async fn unauthenticated_requests_are_401() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let resp = app.get("/api/admin/security/impersonation", None).await;
    resp.assert_status(StatusCode::UNAUTHORIZED);

    let resp = app
        .post_json(
            "/api/admin/security/impersonation",
            &json!({"target_user_id": uuid::Uuid::new_v4(), "reason": "x"}),
            None,
        )
        .await;
    resp.assert_status(StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn member_cannot_reach_impersonation_routes() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let member = app.seed_user().await.expect("seed member");

    let resp = app
        .get(
            "/api/admin/security/impersonation",
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
async fn support_cannot_mint_or_list_impersonation() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    // Support has admin.dashboard.read so it passes PrivilegedUser, but
    // user.impersonate is admin-only per migration 058. The handler's
    // explicit `require()` must reject.
    let support = app.seed_support().await.expect("seed support");

    let list = app
        .get(
            "/api/admin/security/impersonation",
            Some(&support.access_token),
        )
        .await;
    list.assert_status(StatusCode::FORBIDDEN);

    let mint = app
        .post_json(
            "/api/admin/security/impersonation",
            &json!({"target_user_id": uuid::Uuid::new_v4(), "reason": "x"}),
            Some(&support.access_token),
        )
        .await;
    mint.assert_status(StatusCode::FORBIDDEN);
}

// ── Mint validation ────────────────────────────────────────────────────

#[tokio::test]
async fn cannot_impersonate_self() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let admin = app.seed_admin().await.expect("seed admin");

    let resp = app
        .post_json(
            "/api/admin/security/impersonation",
            &json!({"target_user_id": admin.id, "reason": "loop"}),
            Some(&admin.access_token),
        )
        .await;
    resp.assert_status(StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn cannot_impersonate_another_admin() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let admin = app.seed_admin().await.expect("seed admin");
    let other_admin = app.seed_admin().await.expect("seed other admin");

    let resp = app
        .post_json(
            "/api/admin/security/impersonation",
            &json!({"target_user_id": other_admin.id, "reason": "x"}),
            Some(&admin.access_token),
        )
        .await;
    resp.assert_status(StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn cannot_impersonate_missing_user() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let admin = app.seed_admin().await.expect("seed admin");

    let resp = app
        .post_json(
            "/api/admin/security/impersonation",
            &json!({
                "target_user_id": uuid::Uuid::new_v4(),
                "reason": "ghost",
            }),
            Some(&admin.access_token),
        )
        .await;
    resp.assert_status(StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn empty_reason_is_rejected() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let admin = app.seed_admin().await.expect("seed admin");
    let target = app.seed_user().await.expect("seed target");

    let resp = app
        .post_json(
            "/api/admin/security/impersonation",
            &json!({"target_user_id": target.id, "reason": "   "}),
            Some(&admin.access_token),
        )
        .await;
    resp.assert_status(StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn ttl_above_cap_is_rejected() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let admin = app.seed_admin().await.expect("seed admin");
    let target = app.seed_user().await.expect("seed target");

    let resp = app
        .post_json(
            "/api/admin/security/impersonation",
            &json!({
                "target_user_id": target.id,
                "reason": "ok",
                "ttl_minutes": 9999
            }),
            Some(&admin.access_token),
        )
        .await;
    resp.assert_status(StatusCode::BAD_REQUEST);
}

// ── Mint happy path: token shape + audit + banner ──────────────────────

#[tokio::test]
async fn mint_returns_impersonation_token_with_correct_claims() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let admin = app.seed_admin().await.expect("seed admin");
    let target = app.seed_user().await.expect("seed target");

    let resp = app
        .post_json(
            "/api/admin/security/impersonation",
            &json!({
                "target_user_id": target.id,
                "reason": "ticket #42",
                "ttl_minutes": 10
            }),
            Some(&admin.access_token),
        )
        .await;
    resp.assert_status(StatusCode::OK);
    let body: Value = resp.json().expect("mint body");
    let access_token = body["access_token"].as_str().expect("access_token");
    let session_id = body["session"]["id"]
        .as_str()
        .expect("session id")
        .to_string();
    assert_eq!(
        body["session"]["target_user_id"].as_str(),
        Some(target.id.to_string().as_str())
    );
    assert_eq!(
        body["session"]["actor_user_id"].as_str(),
        Some(admin.id.to_string().as_str())
    );
    assert_eq!(body["session"]["reason"].as_str(), Some("ticket #42"));

    // Decode the server-issued JWT through the production Claims
    // struct so the test verifies both signature and field shape.
    let secret = test_jwt_secret_current();
    let decoded = decode::<Claims>(
        access_token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &Validation::default(),
    )
    .expect("jwt verifies under harness secret");
    let claims = decoded.claims;
    assert_eq!(claims.sub, target.id);
    assert_eq!(claims.role, "member");
    assert_eq!(claims.imp_actor, Some(admin.id));
    assert_eq!(claims.imp_actor_role.as_deref(), Some("admin"));
    assert_eq!(claims.imp_session.map(|u| u.to_string()), Some(session_id));
}

#[tokio::test]
async fn mint_writes_audit_row() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let admin = app.seed_admin().await.expect("seed admin");
    let target = app.seed_user().await.expect("seed target");

    let resp = app
        .post_json(
            "/api/admin/security/impersonation",
            &json!({
                "target_user_id": target.id,
                "reason": "audit-check",
            }),
            Some(&admin.access_token),
        )
        .await;
    resp.assert_status(StatusCode::OK);
    let body: Value = resp.json().expect("mint body");
    let session_id = body["session"]["id"]
        .as_str()
        .expect("session id")
        .to_string();

    let row = sqlx::query(
        "SELECT action, target_kind, target_id, metadata
           FROM admin_actions
          WHERE actor_id = $1
            AND action = 'admin.impersonation.start'
          ORDER BY created_at DESC
          LIMIT 1",
    )
    .bind(admin.id)
    .fetch_one(app.db())
    .await
    .expect("audit row");
    let action: String = row.get("action");
    let target_kind: String = row.get("target_kind");
    let target_id: String = row.get("target_id");
    let metadata: Value = row.get("metadata");
    assert_eq!(action, "admin.impersonation.start");
    assert_eq!(target_kind, "impersonation_session");
    assert_eq!(target_id, session_id);
    assert_eq!(
        metadata["target_user_id"].as_str(),
        Some(target.id.to_string().as_str())
    );
    assert_eq!(metadata["reason"].as_str(), Some("audit-check"));
}

// ── Use the impersonation token: banner + admin-gate refusal ───────────

#[tokio::test]
async fn impersonation_token_cannot_reach_admin_endpoints() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let admin = app.seed_admin().await.expect("seed admin");
    let target = app.seed_user().await.expect("seed target");

    let mint = app
        .post_json(
            "/api/admin/security/impersonation",
            &json!({"target_user_id": target.id, "reason": "x"}),
            Some(&admin.access_token),
        )
        .await;
    mint.assert_status(StatusCode::OK);
    let mint_body: Value = mint.json().expect("mint body");
    let imp_token = mint_body["access_token"]
        .as_str()
        .expect("access_token")
        .to_string();

    // Listing impersonation sessions requires PrivilegedUser, which now
    // refuses any token carrying an active imp_session claim.
    let resp = app
        .get("/api/admin/security/impersonation", Some(&imp_token))
        .await;
    resp.assert_status(StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn responses_under_impersonation_carry_banner_headers() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let admin = app.seed_admin().await.expect("seed admin");
    let target = app.seed_user().await.expect("seed target");

    let mint = app
        .post_json(
            "/api/admin/security/impersonation",
            &json!({"target_user_id": target.id, "reason": "banner"}),
            Some(&admin.access_token),
        )
        .await;
    mint.assert_status(StatusCode::OK);
    let mint_body: Value = mint.json().expect("mint body");
    let imp_token = mint_body["access_token"]
        .as_str()
        .expect("token")
        .to_string();
    let session_id = mint_body["session"]["id"]
        .as_str()
        .expect("sid")
        .to_string();

    // Hit /api/auth/me with the impersonation token — that route is
    // AuthUser-gated so the impersonated session reaches it. The
    // response (whatever its status) must carry the banner headers
    // because the banner middleware inspects request.extensions BEFORE
    // running the inner service.
    let resp = app.get("/api/auth/me", Some(&imp_token)).await;

    assert_eq!(
        resp.header("X-Impersonation-Active"),
        Some("true"),
        "banner active header",
    );
    assert_eq!(
        resp.header("X-Impersonator-Id"),
        Some(admin.id.to_string().as_str()),
    );
    assert_eq!(
        resp.header("X-Impersonation-Session"),
        Some(session_id.as_str()),
    );
    assert_eq!(
        resp.header("X-Impersonation-Target"),
        Some(target.id.to_string().as_str()),
    );
    assert_eq!(resp.header("X-Impersonator-Role"), Some("admin"));
}

// ── Revocation kills the live JWT ──────────────────────────────────────

#[tokio::test]
async fn revoking_session_immediately_invalidates_token() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let admin = app.seed_admin().await.expect("seed admin");
    let target = app.seed_user().await.expect("seed target");

    let mint = app
        .post_json(
            "/api/admin/security/impersonation",
            &json!({"target_user_id": target.id, "reason": "kill"}),
            Some(&admin.access_token),
        )
        .await;
    mint.assert_status(StatusCode::OK);
    let mint_body: Value = mint.json().expect("mint body");
    let imp_token = mint_body["access_token"]
        .as_str()
        .expect("token")
        .to_string();
    let session_id = mint_body["session"]["id"]
        .as_str()
        .expect("sid")
        .to_string();

    // Sanity: token is live.
    let pre = app.get("/api/auth/me", Some(&imp_token)).await;
    assert_ne!(pre.status(), StatusCode::UNAUTHORIZED);

    // Admin revokes the session row.
    let revoke = app
        .post_json(
            &format!("/api/admin/security/impersonation/{session_id}/revoke"),
            &json!({"reason": "leak"}),
            Some(&admin.access_token),
        )
        .await;
    revoke.assert_status(StatusCode::OK);

    // The previously-live impersonation token is now refused — the
    // server-side row check fails and AuthUser collapses to 401.
    let post = app.get("/api/auth/me", Some(&imp_token)).await;
    post.assert_status(StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn revoking_writes_audit_row() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let admin = app.seed_admin().await.expect("seed admin");
    let target = app.seed_user().await.expect("seed target");

    let mint = app
        .post_json(
            "/api/admin/security/impersonation",
            &json!({"target_user_id": target.id, "reason": "audit-revoke"}),
            Some(&admin.access_token),
        )
        .await;
    let session_id = mint.json::<Value>().expect("body")["session"]["id"]
        .as_str()
        .expect("sid")
        .to_string();

    let _ = app
        .post_json(
            &format!("/api/admin/security/impersonation/{session_id}/revoke"),
            &json!({"reason": "post-incident"}),
            Some(&admin.access_token),
        )
        .await;

    let row = sqlx::query(
        "SELECT action, metadata
           FROM admin_actions
          WHERE actor_id = $1
            AND action = 'admin.impersonation.revoke'
          ORDER BY created_at DESC
          LIMIT 1",
    )
    .bind(admin.id)
    .fetch_one(app.db())
    .await
    .expect("audit row");
    let action: String = row.get("action");
    let metadata: Value = row.get("metadata");
    assert_eq!(action, "admin.impersonation.revoke");
    assert_eq!(
        metadata["target_user_id"].as_str(),
        Some(target.id.to_string().as_str())
    );
    assert_eq!(metadata["reason"].as_str(), Some("post-incident"));
}

#[tokio::test]
async fn revoking_unknown_session_is_404() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let admin = app.seed_admin().await.expect("seed admin");
    let resp = app
        .post_json(
            &format!(
                "/api/admin/security/impersonation/{}/revoke",
                uuid::Uuid::new_v4()
            ),
            &json!({}),
            Some(&admin.access_token),
        )
        .await;
    resp.assert_status(StatusCode::NOT_FOUND);
}

// ── Self-exit ──────────────────────────────────────────────────────────

#[tokio::test]
async fn self_exit_revokes_session_and_audits() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let admin = app.seed_admin().await.expect("seed admin");
    let target = app.seed_user().await.expect("seed target");

    let mint = app
        .post_json(
            "/api/admin/security/impersonation",
            &json!({"target_user_id": target.id, "reason": "exit-test"}),
            Some(&admin.access_token),
        )
        .await;
    let mint_body: Value = mint.json().expect("body");
    let imp_token = mint_body["access_token"]
        .as_str()
        .expect("token")
        .to_string();
    let session_id = mint_body["session"]["id"]
        .as_str()
        .expect("sid")
        .to_string();

    // Caller uses the impersonation token to end the session.
    let exit = app
        .post_json("/api/auth/impersonation/exit", &json!({}), Some(&imp_token))
        .await;
    exit.assert_status(StatusCode::NO_CONTENT);

    // Token is now invalid.
    let post = app.get("/api/auth/me", Some(&imp_token)).await;
    post.assert_status(StatusCode::UNAUTHORIZED);

    // Audit row recorded against the real admin actor.
    let row = sqlx::query(
        "SELECT actor_id, action, target_id, metadata
           FROM admin_actions
          WHERE action = 'admin.impersonation.exit'
            AND target_id = $1
          ORDER BY created_at DESC
          LIMIT 1",
    )
    .bind(&session_id)
    .fetch_one(app.db())
    .await
    .expect("exit audit row");
    let actor_id: uuid::Uuid = row.get("actor_id");
    let metadata: Value = row.get("metadata");
    assert_eq!(actor_id, admin.id);
    assert_eq!(
        metadata["target_user_id"].as_str(),
        Some(target.id.to_string().as_str())
    );
    assert_eq!(metadata["session_was_live"].as_bool(), Some(true));
}

#[tokio::test]
async fn self_exit_without_impersonation_returns_400() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let member = app.seed_user().await.expect("seed member");
    let resp = app
        .post_json(
            "/api/auth/impersonation/exit",
            &json!({}),
            Some(&member.access_token),
        )
        .await;
    resp.assert_status(StatusCode::BAD_REQUEST);
}

// ── Gap fixes (ADM-07-α) ───────────────────────────────────────────────

/// gap-ratelimit: a single admin must not be able to mint more than
/// `MAX_MINTS_PER_MINUTE` impersonation tokens in a 60s window. Burst
/// the limit and assert the (limit+1)th call returns 429.
#[tokio::test]
async fn mint_rate_limited_per_actor() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let admin = app.seed_admin().await.expect("seed admin");
    let max = swings_api::security::impersonation::MAX_MINTS_PER_MINUTE;

    // Burn through the quota with fresh targets each time so the safety
    // checks pass on every call.
    for _ in 0..max {
        let t = app.seed_user().await.expect("target");
        let resp = app
            .post_json(
                "/api/admin/security/impersonation",
                &json!({"target_user_id": t.id, "reason": "burst"}),
                Some(&admin.access_token),
            )
            .await;
        resp.assert_status(StatusCode::OK);
    }

    let last = app.seed_user().await.expect("over-quota target");
    let blocked = app
        .post_json(
            "/api/admin/security/impersonation",
            &json!({"target_user_id": last.id, "reason": "over-quota"}),
            Some(&admin.access_token),
        )
        .await;
    blocked.assert_status(StatusCode::TOO_MANY_REQUESTS);
}

/// gap-logout: hitting `/api/auth/logout` while authenticated by an
/// impersonation token must end the impersonation session, NOT delete
/// the target user's refresh tokens.
#[tokio::test]
async fn logout_under_impersonation_ends_session_not_user_refresh_tokens() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let admin = app.seed_admin().await.expect("seed admin");
    let target = app.seed_user().await.expect("seed target");

    // Sanity: the target has a refresh token from `seed_user`.
    let rt_before: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM refresh_tokens WHERE user_id = $1")
            .bind(target.id)
            .fetch_one(app.db())
            .await
            .expect("count rt");
    assert!(
        rt_before > 0,
        "test prerequisite: seeded user should have at least one refresh token"
    );

    let mint = app
        .post_json(
            "/api/admin/security/impersonation",
            &json!({"target_user_id": target.id, "reason": "logout-test"}),
            Some(&admin.access_token),
        )
        .await;
    let mint_body: Value = mint.json().expect("mint body");
    let imp_token = mint_body["access_token"]
        .as_str()
        .expect("token")
        .to_string();
    let session_id = mint_body["session"]["id"]
        .as_str()
        .expect("sid")
        .to_string();

    // Logout under impersonation.
    let resp = app
        .post_json("/api/auth/logout", &json!({}), Some(&imp_token))
        .await;
    resp.assert_status(StatusCode::OK);
    let body: Value = resp.json().expect("logout body");
    assert_eq!(body["impersonation_ended"].as_bool(), Some(true));

    // Target's refresh tokens are untouched.
    let rt_after: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM refresh_tokens WHERE user_id = $1")
            .bind(target.id)
            .fetch_one(app.db())
            .await
            .expect("count rt after");
    assert_eq!(
        rt_after, rt_before,
        "logout-under-impersonation must not delete target refresh tokens"
    );

    // Session row is revoked.
    let revoked_at: Option<chrono::DateTime<chrono::Utc>> =
        sqlx::query_scalar("SELECT revoked_at FROM impersonation_sessions WHERE id = $1::uuid")
            .bind(&session_id)
            .fetch_one(app.db())
            .await
            .expect("session row");
    assert!(revoked_at.is_some(), "session should be revoked by logout");

    // The impersonation token is now invalid.
    let post = app.get("/api/auth/me", Some(&imp_token)).await;
    post.assert_status(StatusCode::UNAUTHORIZED);
}

/// gap-refresh: a refresh attempt that carries an impersonation
/// bearer token must be refused with 403, even when the body's
/// refresh_token is otherwise valid.
#[tokio::test]
async fn refresh_with_impersonation_bearer_is_forbidden() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let admin = app.seed_admin().await.expect("seed admin");
    let target = app.seed_user().await.expect("seed target");

    let mint = app
        .post_json(
            "/api/admin/security/impersonation",
            &json!({"target_user_id": target.id, "reason": "refresh-test"}),
            Some(&admin.access_token),
        )
        .await;
    let mint_body: Value = mint.json().expect("mint body");
    let imp_token = mint_body["access_token"]
        .as_str()
        .expect("token")
        .to_string();

    // Use the target's *real* refresh token, but present the
    // impersonation token in Authorization. The endpoint must fail
    // closed before consulting the refresh-token row.
    let resp = app
        .post_json(
            "/api/auth/refresh",
            &json!({"refresh_token": target.refresh_token}),
            Some(&imp_token),
        )
        .await;
    resp.assert_status(StatusCode::FORBIDDEN);
}

/// gap-pagination: list endpoint accepts `?limit=N` and returns a
/// `next_cursor` when the page is full.
#[tokio::test]
async fn list_supports_cursor_pagination() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let admin = app.seed_admin().await.expect("seed admin");

    // Seed 3 active sessions then list with limit=2.
    for _ in 0..3 {
        let t = app.seed_user().await.expect("target");
        let resp = app
            .post_json(
                "/api/admin/security/impersonation",
                &json!({"target_user_id": t.id, "reason": "pg"}),
                Some(&admin.access_token),
            )
            .await;
        resp.assert_status(StatusCode::OK);
    }

    let page1 = app
        .get(
            "/api/admin/security/impersonation?limit=2",
            Some(&admin.access_token),
        )
        .await;
    page1.assert_status(StatusCode::OK);
    let body1: Value = page1.json().expect("page1");
    assert_eq!(body1["data"].as_array().map(|a| a.len()), Some(2));
    let cursor = body1["next_cursor"]
        .as_str()
        .expect("next_cursor present when page is full")
        .to_string();

    let qs = url::form_urlencoded::Serializer::new(String::new())
        .append_pair("limit", "2")
        .append_pair("after", &cursor)
        .finish();
    let page2 = app
        .get(
            &format!("/api/admin/security/impersonation?{qs}"),
            Some(&admin.access_token),
        )
        .await;
    page2.assert_status(StatusCode::OK);
    let body2: Value = page2.json().expect("page2");
    let len2 = body2["data"].as_array().map(|a| a.len()).unwrap_or(0);
    assert!(
        len2 >= 1,
        "second page must contain at least the remaining session"
    );
}
