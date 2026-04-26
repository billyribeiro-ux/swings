#![deny(warnings)]
#![forbid(unsafe_code)]

//! Phase 4.6 integration coverage for the member self-service surface:
//! `POST /api/member/password`, `DELETE /api/member/account`, and
//! `POST /api/member/coupons/apply`.
//!
//! These endpoints were 404'ing in production before this phase because the
//! frontend `dashboard/account/+page.svelte` calls them but the backend never
//! mounted the routes. The tests here pin the contract end-to-end so a future
//! refactor cannot silently regress the SPA again.
//!
//! Skipped when no test database is configured — same convention as the rest
//! of `backend/tests/`.

mod support;

use argon2::{
    password_hash::{PasswordHash, PasswordVerifier},
    Argon2,
};
use axum::http::StatusCode;
use chrono::{Duration, Utc};
use serde_json::json;
use sha2::{Digest, Sha256};
use support::TestApp;
use uuid::Uuid;

// ── Helpers ────────────────────────────────────────────────────────────

fn sha256_hex(input: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    hasher
        .finalize()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

/// Read the current Argon2 hash for a user. Used to assert mutation /
/// non-mutation of `password_hash` after a change-password attempt.
async fn read_password_hash(pool: &sqlx::PgPool, user_id: Uuid) -> String {
    sqlx::query_scalar::<_, String>("SELECT password_hash FROM users WHERE id = $1")
        .bind(user_id)
        .fetch_one(pool)
        .await
        .expect("read users.password_hash")
}

/// Insert a coupon row directly so each test starts from a known baseline
/// without standing up the full admin flow.
#[allow(clippy::too_many_arguments)]
async fn seed_coupon(
    pool: &sqlx::PgPool,
    code: &str,
    is_active: bool,
    expires_at: Option<chrono::DateTime<Utc>>,
    usage_limit: Option<i32>,
    per_user_limit: i32,
    stripe_coupon_id: Option<&str>,
    created_by: Uuid,
) -> Uuid {
    let id = Uuid::new_v4();
    sqlx::query(
        r#"
        INSERT INTO coupons (
            id, code, description, discount_type, discount_value,
            min_purchase_cents, max_discount_cents, applies_to,
            applicable_plan_ids, applicable_course_ids,
            usage_limit, usage_count, per_user_limit,
            starts_at, expires_at, is_active, stackable, first_purchase_only,
            stripe_coupon_id, created_by, created_at, updated_at
        ) VALUES (
            $1, $2, NULL, 'percentage'::discount_type, 10,
            NULL, NULL, 'all',
            '{}', '{}',
            $3, 0, $4,
            NULL, $5, $6, false, false,
            $7, $8, NOW(), NOW()
        )
        "#,
    )
    .bind(id)
    .bind(code)
    .bind(usage_limit)
    .bind(per_user_limit)
    .bind(expires_at)
    .bind(is_active)
    .bind(stripe_coupon_id)
    .bind(created_by)
    .execute(pool)
    .await
    .expect("seed coupon");
    id
}

/// Insert a row into `subscriptions` with no Stripe twin so the
/// coupon-apply path can locate "an active subscription" without
/// hitting Stripe. The empty `stripe_subscription_id` short-circuits
/// the Stripe call inside the handler.
async fn seed_local_subscription(pool: &sqlx::PgPool, user_id: Uuid) -> Uuid {
    let id = Uuid::new_v4();
    sqlx::query(
        r#"
        INSERT INTO subscriptions (
            id, user_id, stripe_subscription_id, stripe_customer_id,
            plan, status, current_period_start, current_period_end,
            cancel_at_period_end, created_at, updated_at
        ) VALUES (
            $1, $2, '', '',
            'monthly'::subscription_plan, 'active'::subscription_status,
            NOW(), NOW() + INTERVAL '30 days',
            false, NOW(), NOW()
        )
        "#,
    )
    .bind(id)
    .bind(user_id)
    .execute(pool)
    .await
    .expect("seed subscription");
    id
}

// ── Password change ────────────────────────────────────────────────────

#[tokio::test]
async fn password_change_succeeds_with_correct_current_password() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let user = app.seed_user().await.expect("seed");
    let original_hash = read_password_hash(app.db(), user.id).await;

    let resp = app
        .post_json(
            "/api/member/password",
            &json!({
                "current_password": user.password,
                "new_password": "brand-new-pw-123",
            }),
            Some(&user.access_token),
        )
        .await;
    resp.assert_status(StatusCode::OK);

    // New cookies (access + refresh) should be set so the calling session
    // keeps working.
    let cookie_lines: Vec<&str> = resp
        .headers()
        .get_all(axum::http::header::SET_COOKIE)
        .into_iter()
        .filter_map(|v| v.to_str().ok())
        .collect();
    assert!(
        cookie_lines.iter().any(|l| l.starts_with("swings_access=")),
        "expected swings_access Set-Cookie, got {cookie_lines:?}"
    );
    assert!(
        cookie_lines.iter().any(|l| l.starts_with("swings_refresh=")),
        "expected swings_refresh Set-Cookie, got {cookie_lines:?}"
    );

    // The persisted Argon2 hash must have changed.
    let updated_hash = read_password_hash(app.db(), user.id).await;
    assert_ne!(original_hash, updated_hash, "password_hash unchanged");

    // And the new hash must verify against the new password.
    let parsed = PasswordHash::new(&updated_hash).expect("parse new hash");
    assert!(
        Argon2::default()
            .verify_password(b"brand-new-pw-123", &parsed)
            .is_ok(),
        "new password should verify against the persisted hash"
    );
}

#[tokio::test]
async fn password_change_fails_with_wrong_current_password() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let user = app.seed_user().await.expect("seed");
    let original_hash = read_password_hash(app.db(), user.id).await;

    let resp = app
        .post_json(
            "/api/member/password",
            &json!({
                "current_password": "definitely-not-the-password",
                "new_password": "brand-new-pw-123",
            }),
            Some(&user.access_token),
        )
        .await;
    resp.assert_status(StatusCode::UNAUTHORIZED);

    let after = read_password_hash(app.db(), user.id).await;
    assert_eq!(
        original_hash, after,
        "wrong current_password must not mutate password_hash"
    );
}

#[tokio::test]
async fn password_change_revokes_other_sessions() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let user = app.seed_user().await.expect("seed");

    // Mint a *second* refresh token row for the same user — modelling a
    // second logged-in browser tab. We seed via direct SQL because the
    // login HTTP path would also rate-limit and clutter the test.
    let second_refresh = Uuid::new_v4().to_string();
    let second_hash = sha256_hex(&second_refresh);
    let expires_at = Utc::now() + Duration::days(30);
    sqlx::query(
        "INSERT INTO refresh_tokens (id, user_id, token_hash, expires_at, family_id, used)
         VALUES ($1, $2, $3, $4, $5, FALSE)",
    )
    .bind(Uuid::new_v4())
    .bind(user.id)
    .bind(&second_hash)
    .bind(expires_at)
    .bind(Uuid::new_v4())
    .execute(app.db())
    .await
    .expect("seed second refresh token");

    // Sanity check the second token works pre-change.
    let pre = app
        .post_json(
            "/api/auth/refresh",
            &json!({ "refresh_token": second_refresh }),
            None,
        )
        .await;
    pre.assert_status(StatusCode::OK);

    // Now mint a *third* refresh row to model the calling tab; the
    // refresh above consumed the second-tab token, so we need a fresh
    // token to revoke.
    let other_refresh = Uuid::new_v4().to_string();
    let other_hash = sha256_hex(&other_refresh);
    sqlx::query(
        "INSERT INTO refresh_tokens (id, user_id, token_hash, expires_at, family_id, used)
         VALUES ($1, $2, $3, $4, $5, FALSE)",
    )
    .bind(Uuid::new_v4())
    .bind(user.id)
    .bind(&other_hash)
    .bind(Utc::now() + Duration::days(30))
    .bind(Uuid::new_v4())
    .execute(app.db())
    .await
    .expect("seed other refresh token");

    // Change the password from the first session.
    let resp = app
        .post_json(
            "/api/member/password",
            &json!({
                "current_password": user.password,
                "new_password": "rotated-pw-456",
            }),
            Some(&user.access_token),
        )
        .await;
    resp.assert_status(StatusCode::OK);

    // The "other tab"'s refresh token must now be invalid.
    let post = app
        .post_json(
            "/api/auth/refresh",
            &json!({ "refresh_token": other_refresh }),
            None,
        )
        .await;
    assert_eq!(
        post.status(),
        StatusCode::UNAUTHORIZED,
        "other-session refresh token must be revoked after password change"
    );
}

#[tokio::test]
async fn password_change_anonymous_returns_401() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let resp = app
        .post_json(
            "/api/member/password",
            &json!({
                "current_password": "x",
                "new_password": "y-very-long-pw",
            }),
            None,
        )
        .await;
    resp.assert_status(StatusCode::UNAUTHORIZED);
}

// ── Account delete ─────────────────────────────────────────────────────

#[tokio::test]
async fn account_delete_succeeds_and_clears_cookies() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let user = app.seed_user().await.expect("seed");

    let resp = app
        .delete("/api/member/account", Some(&user.access_token))
        .await;
    resp.assert_status(StatusCode::OK);

    // Both auth cookies should be cleared (Max-Age=0 / empty value).
    let cookie_lines: Vec<&str> = resp
        .headers()
        .get_all(axum::http::header::SET_COOKIE)
        .into_iter()
        .filter_map(|v| v.to_str().ok())
        .collect();
    let access_clear = cookie_lines
        .iter()
        .find(|l| l.starts_with("swings_access="))
        .expect("swings_access deletion cookie");
    let refresh_clear = cookie_lines
        .iter()
        .find(|l| l.starts_with("swings_refresh="))
        .expect("swings_refresh deletion cookie");
    assert!(
        access_clear.contains("Max-Age=0"),
        "access cookie should be cleared; got {access_clear:?}"
    );
    assert!(
        refresh_clear.contains("Max-Age=0"),
        "refresh cookie should be cleared; got {refresh_clear:?}"
    );

    // The user row must be gone (hard-delete mirrors `db::delete_user`).
    let still_there: Option<(Uuid,)> = sqlx::query_as("SELECT id FROM users WHERE id = $1")
        .bind(user.id)
        .fetch_optional(app.db())
        .await
        .expect("query users");
    assert!(still_there.is_none(), "user row should be deleted");
}

#[tokio::test]
async fn account_delete_records_audit_action() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let user = app.seed_user().await.expect("seed");

    let resp = app
        .delete("/api/member/account", Some(&user.access_token))
        .await;
    resp.assert_status(StatusCode::OK);

    let action_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM admin_actions WHERE actor_id = $1 AND action = $2",
    )
    .bind(user.id)
    .bind("member.account.delete")
    .fetch_one(app.db())
    .await
    .expect("count audit rows");
    assert_eq!(
        action_count, 1,
        "expected exactly one member.account.delete audit row"
    );
}

#[tokio::test]
async fn account_delete_anonymous_returns_401() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let resp = app.delete("/api/member/account", None).await;
    resp.assert_status(StatusCode::UNAUTHORIZED);
}

// ── Coupon apply ───────────────────────────────────────────────────────

#[tokio::test]
async fn coupon_apply_404s_for_unknown_code() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let user = app.seed_user().await.expect("seed");
    seed_local_subscription(app.db(), user.id).await;

    let resp = app
        .post_json(
            "/api/member/coupons/apply",
            &json!({ "code": "NOSUCH-CODE-XYZ" }),
            Some(&user.access_token),
        )
        .await;
    resp.assert_status(StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn coupon_apply_400s_for_expired_coupon() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let user = app.seed_user().await.expect("seed");
    seed_local_subscription(app.db(), user.id).await;

    let yesterday = Utc::now() - Duration::days(1);
    seed_coupon(
        app.db(),
        "EXPIRED10",
        true,
        Some(yesterday),
        None,
        1,
        None,
        user.id,
    )
    .await;

    let resp = app
        .post_json(
            "/api/member/coupons/apply",
            &json!({ "code": "EXPIRED10" }),
            Some(&user.access_token),
        )
        .await;
    resp.assert_status(StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn coupon_apply_400s_for_inactive_coupon() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let user = app.seed_user().await.expect("seed");
    seed_local_subscription(app.db(), user.id).await;

    seed_coupon(
        app.db(),
        "INACTIVE10",
        false,
        None,
        None,
        1,
        None,
        user.id,
    )
    .await;

    let resp = app
        .post_json(
            "/api/member/coupons/apply",
            &json!({ "code": "INACTIVE10" }),
            Some(&user.access_token),
        )
        .await;
    resp.assert_status(StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn coupon_apply_succeeds_for_valid_coupon() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let user = app.seed_user().await.expect("seed");
    seed_local_subscription(app.db(), user.id).await;

    let coupon_id =
        seed_coupon(app.db(), "WELCOME10", true, None, None, 1, None, user.id).await;

    let resp = app
        .post_json(
            "/api/member/coupons/apply",
            &json!({ "code": "WELCOME10" }),
            Some(&user.access_token),
        )
        .await;
    resp.assert_status(StatusCode::OK);

    let body: serde_json::Value = resp.json().expect("json");
    assert_eq!(body["ok"], json!(true));
    assert_eq!(body["coupon_id"], json!(coupon_id.to_string()));

    // Redemption row must be present and usage_count bumped.
    let usages: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM coupon_usages WHERE coupon_id = $1 AND user_id = $2",
    )
    .bind(coupon_id)
    .bind(user.id)
    .fetch_one(app.db())
    .await
    .expect("count usages");
    assert_eq!(usages, 1);
    let count_on_coupon: i32 =
        sqlx::query_scalar("SELECT usage_count FROM coupons WHERE id = $1")
            .bind(coupon_id)
            .fetch_one(app.db())
            .await
            .expect("read usage_count");
    assert_eq!(count_on_coupon, 1);
}

#[tokio::test]
async fn coupon_apply_409s_when_already_redeemed() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let user = app.seed_user().await.expect("seed");
    seed_local_subscription(app.db(), user.id).await;

    seed_coupon(app.db(), "ONCE10", true, None, None, 1, None, user.id).await;

    let first = app
        .post_json(
            "/api/member/coupons/apply",
            &json!({ "code": "ONCE10" }),
            Some(&user.access_token),
        )
        .await;
    first.assert_status(StatusCode::OK);

    let second = app
        .post_json(
            "/api/member/coupons/apply",
            &json!({ "code": "ONCE10" }),
            Some(&user.access_token),
        )
        .await;
    second.assert_status(StatusCode::CONFLICT);
}

#[tokio::test]
async fn coupon_apply_anonymous_returns_401() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let resp = app
        .post_json(
            "/api/member/coupons/apply",
            &json!({ "code": "ANY" }),
            None,
        )
        .await;
    resp.assert_status(StatusCode::UNAUTHORIZED);
}
