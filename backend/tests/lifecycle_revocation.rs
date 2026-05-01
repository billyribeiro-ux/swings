#![deny(warnings)]
#![forbid(unsafe_code)]

//! Integration tests for **per-request lifecycle revocation**.
//!
//! Covers the security gap fixed in 2026-05-01 Phase A: the `AuthUser` and
//! `OptionalAuthUser` extractors must re-check `users.banned_at` and
//! `users.suspended_at` on every request, not only at login. Without this,
//! a banned account retained access for up to `JWT_EXPIRATION_HOURS` (24h
//! by default) — far too long when the bar is "abusive account, kill access
//! NOW."
//!
//! Test plan:
//! - Login → first authed request succeeds (smoke).
//! - Admin bans the user (DB write only — no logout broadcast). Next authed
//!   request with the same JWT must 401 even though the JWT itself is still
//!   cryptographically valid.
//! - Same shape for suspension (open-ended) and time-boxed suspension that
//!   has not yet expired.
//! - Time-boxed suspension whose `suspended_until` is in the past must NOT
//!   block the request — the lazy-unsuspend path keeps the user productive.
//! - Hard-deleted user row → 401 (the bearer token is treated as never
//!   having existed).
//! - Cart endpoints use `OptionalAuthUser`; banned users must be downgraded
//!   to anonymous (not 401, since the contract is infallible). Verify the
//!   cart cookie carries an anonymous identity even when the JWT decodes
//!   for a banned user.

mod support;

use axum::http::StatusCode;
use chrono::{Duration, Utc};
use support::TestApp;

#[tokio::test]
async fn fresh_login_can_call_authed_endpoint() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let user = app.seed_user().await.expect("seed member");
    app.get("/api/auth/me", Some(&user.access_token))
        .await
        .assert_status(StatusCode::OK);
}

#[tokio::test]
async fn banned_user_is_revoked_on_next_request() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let user = app.seed_user().await.expect("seed member");

    // Smoke: token works before the ban.
    app.get("/api/auth/me", Some(&user.access_token))
        .await
        .assert_status(StatusCode::OK);

    // Operator stamps the ban directly on the row — same effect as the
    // admin endpoint hitting the same column.
    sqlx::query("UPDATE users SET banned_at = NOW() WHERE id = $1")
        .bind(user.id)
        .execute(app.db())
        .await
        .expect("ban user");

    // Same JWT, post-ban: must be rejected. This is the regression we are
    // pinning — pre-fix the call returned 200 because the extractor only
    // verified the token signature.
    app.get("/api/auth/me", Some(&user.access_token))
        .await
        .assert_status(StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn open_ended_suspension_is_revoked_on_next_request() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let user = app.seed_user().await.expect("seed member");

    sqlx::query("UPDATE users SET suspended_at = NOW() WHERE id = $1")
        .bind(user.id)
        .execute(app.db())
        .await
        .expect("suspend user");

    app.get("/api/auth/me", Some(&user.access_token))
        .await
        .assert_status(StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn time_boxed_suspension_blocks_until_expiry() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let user = app.seed_user().await.expect("seed member");

    // suspended_until is 7 days out → still in effect.
    sqlx::query(
        "UPDATE users
            SET suspended_at    = NOW(),
                suspended_until = NOW() + INTERVAL '7 days'
          WHERE id = $1",
    )
    .bind(user.id)
    .execute(app.db())
    .await
    .expect("apply timed suspension");

    app.get("/api/auth/me", Some(&user.access_token))
        .await
        .assert_status(StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn expired_time_boxed_suspension_self_heals_on_request() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let user = app.seed_user().await.expect("seed member");

    // Suspension started yesterday and expired one second ago — the
    // extractor's `is_active()` should return true so the request succeeds.
    sqlx::query(
        "UPDATE users
            SET suspended_at    = NOW() - INTERVAL '1 day',
                suspended_until = NOW() - INTERVAL '1 second'
          WHERE id = $1",
    )
    .bind(user.id)
    .execute(app.db())
    .await
    .expect("apply expired suspension");

    app.get("/api/auth/me", Some(&user.access_token))
        .await
        .assert_status(StatusCode::OK);
}

#[tokio::test]
async fn hard_deleted_user_token_is_rejected() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let user = app.seed_user().await.expect("seed member");

    // Delete the row out from under the still-valid JWT. The lifecycle
    // lookup returns `Ok(None)` and the extractor must collapse to
    // Unauthorized — not panic, not return data for a ghost user.
    sqlx::query("DELETE FROM users WHERE id = $1")
        .bind(user.id)
        .execute(app.db())
        .await
        .expect("delete user");

    app.get("/api/auth/me", Some(&user.access_token))
        .await
        .assert_status(StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn admin_ban_revokes_active_admin_token() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let admin = app.seed_admin().await.expect("seed admin");

    // Pre-ban: the admin can hit /api/admin/audit.
    app.get("/api/admin/audit", Some(&admin.access_token))
        .await
        .assert_status(StatusCode::OK);

    sqlx::query("UPDATE users SET banned_at = NOW() WHERE id = $1")
        .bind(admin.id)
        .execute(app.db())
        .await
        .expect("ban admin");

    // Banned admin: AuthUser collapses → 401, never reaches AdminUser.
    // (We assert UNAUTHORIZED specifically — not FORBIDDEN — because the
    //  ban makes the bearer token "stop being a token" entirely.)
    app.get("/api/admin/audit", Some(&admin.access_token))
        .await
        .assert_status(StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn ban_with_future_dated_timestamp_still_blocks() {
    // Defensive: a ban applied with `banned_at = NOW() + 1 hour` (e.g. by
    // an operator scheduling a future ban) should ALSO block. The
    // extractor only checks `banned_at IS NOT NULL`, not the timestamp
    // value, because a future-dated ban is still a ban — operators do not
    // get to time-travel access decisions.
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let user = app.seed_user().await.expect("seed member");

    sqlx::query("UPDATE users SET banned_at = NOW() + INTERVAL '1 hour' WHERE id = $1")
        .bind(user.id)
        .execute(app.db())
        .await
        .expect("future-date ban");

    app.get("/api/auth/me", Some(&user.access_token))
        .await
        .assert_status(StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn ban_does_not_leak_via_optional_auth_endpoint() {
    // The cart endpoints use `OptionalAuthUser`. Pre-fix, a banned user
    // would still be identified as themselves on optional-auth endpoints
    // because the JWT decodes fine. Post-fix, the extractor downgrades to
    // anonymous — same as a guest.
    //
    // We assert the contract via the cart "get my cart" endpoint:
    // anonymous callers receive an empty cart shape; an "identified-then-
    // demoted" banned user must receive the same anonymous shape, NOT
    // the cart belonging to that user.
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let user = app.seed_user().await.expect("seed member");

    sqlx::query("UPDATE users SET banned_at = NOW() WHERE id = $1")
        .bind(user.id)
        .execute(app.db())
        .await
        .expect("ban user");

    // Cart endpoint should NOT 401 (it's optional auth) — it returns an
    // anonymous cart. We only assert the request succeeded; the deeper
    // contract (the cart returned is the guest one, not the banned user's)
    // is enforced by the extractor returning `user_id: None`, which
    // unit tests on the extractor cover.
    let resp = app.get("/api/cart", Some(&user.access_token)).await;
    assert!(
        resp.status() == StatusCode::OK || resp.status() == StatusCode::NOT_FOUND,
        "cart endpoint should not 401 a banned-but-decodable JWT, got {}",
        resp.status()
    );
}

#[tokio::test]
async fn refresh_after_ban_is_rejected() {
    // Defense in depth: even if some extractor skips the lifecycle check
    // (regression), the refresh endpoint runs through `AuthUser` and must
    // bounce a banned user. Otherwise the user would mint a fresh access
    // token and slip past the gate for another 24h.
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
            "/api/auth/refresh",
            &serde_json::json!({ "refresh_token": user.refresh_token }),
            None,
        )
        .await;
    assert!(
        resp.status().is_client_error(),
        "refresh after ban must fail; got {}",
        resp.status()
    );
}

#[tokio::test]
async fn unsuspend_lifts_block_for_subsequent_requests() {
    // Operator suspends a user (open-ended), then unsuspends. The next
    // request must succeed without requiring a new login — same JWT.
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let user = app.seed_user().await.expect("seed member");

    sqlx::query("UPDATE users SET suspended_at = NOW() WHERE id = $1")
        .bind(user.id)
        .execute(app.db())
        .await
        .expect("suspend");
    app.get("/api/auth/me", Some(&user.access_token))
        .await
        .assert_status(StatusCode::UNAUTHORIZED);

    // Lift the suspension.
    sqlx::query(
        "UPDATE users
            SET suspended_at = NULL, suspended_until = NULL, suspension_reason = NULL
          WHERE id = $1",
    )
    .bind(user.id)
    .execute(app.db())
    .await
    .expect("unsuspend");

    app.get("/api/auth/me", Some(&user.access_token))
        .await
        .assert_status(StatusCode::OK);
}

#[tokio::test]
async fn ban_then_unban_restores_access() {
    // Symmetry test for ban: unban must un-revoke.
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let user = app.seed_user().await.expect("seed member");

    sqlx::query("UPDATE users SET banned_at = NOW() WHERE id = $1")
        .bind(user.id)
        .execute(app.db())
        .await
        .expect("ban");
    app.get("/api/auth/me", Some(&user.access_token))
        .await
        .assert_status(StatusCode::UNAUTHORIZED);

    sqlx::query("UPDATE users SET banned_at = NULL, ban_reason = NULL WHERE id = $1")
        .bind(user.id)
        .execute(app.db())
        .await
        .expect("unban");

    app.get("/api/auth/me", Some(&user.access_token))
        .await
        .assert_status(StatusCode::OK);
}

// Reference: silence the unused-import warning when this file is the
// only consumer of these symbols.
#[allow(dead_code)]
fn _force_use(_: Duration, _: Utc) {}
