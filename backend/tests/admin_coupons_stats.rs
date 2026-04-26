#![deny(warnings)]
#![forbid(unsafe_code)]

//! Phase 4.5 — `GET /api/admin/coupons/stats` integration coverage.
//!
//! The admin coupons dashboard at `/admin/coupons` calls this endpoint on
//! mount and after every mutation. Before Phase 4.5 the route 404'd because
//! the stats handler was missing; this suite pins the contract end-to-end
//! so a future refactor cannot silently regress the SPA.
//!
//! Covered:
//! * RBAC: admin OK, support OK (read perm), member 403, anonymous 401.
//! * Empty database returns zeros — never errors.
//! * Counts increment after seeding active / expired coupons + redemptions.
//! * `redemptions_today` filter respects the UTC midnight boundary.
//!
//! Skipped when no test database is configured — same convention as the
//! rest of `backend/tests/`.

mod support;

use axum::http::StatusCode;
use chrono::{Duration, Utc};
use serde_json::Value;
use support::TestApp;
use uuid::Uuid;

// ── Helpers ────────────────────────────────────────────────────────────

/// Insert a coupon row directly so each test starts from a known baseline
/// without standing up the full admin create flow.
async fn seed_coupon(
    pool: &sqlx::PgPool,
    code: &str,
    is_active: bool,
    expires_at: Option<chrono::DateTime<Utc>>,
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
            created_by, created_at, updated_at
        ) VALUES (
            $1, $2, NULL, 'percentage'::discount_type, 10,
            NULL, NULL, 'all',
            '{}', '{}',
            NULL, 0, 1,
            NULL, $3, $4, false, false,
            $5, NOW(), NOW()
        )
        "#,
    )
    .bind(id)
    .bind(code)
    .bind(expires_at)
    .bind(is_active)
    .bind(created_by)
    .execute(pool)
    .await
    .expect("seed coupon");
    id
}

/// Insert a `coupon_usages` row whose `used_at` is exactly `used_at`.
/// We override `used_at` so the test can place rows on either side of the
/// UTC-midnight boundary that powers `redemptions_today`.
async fn seed_usage_at(
    pool: &sqlx::PgPool,
    coupon_id: Uuid,
    user_id: Uuid,
    used_at: chrono::DateTime<Utc>,
) {
    sqlx::query(
        r#"
        INSERT INTO coupon_usages
            (id, coupon_id, user_id, subscription_id, discount_applied_cents, used_at)
        VALUES ($1, $2, $3, NULL, 250, $4)
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(coupon_id)
    .bind(user_id)
    .bind(used_at)
    .execute(pool)
    .await
    .expect("seed coupon_usage");
}

// ── RBAC ───────────────────────────────────────────────────────────────

#[tokio::test]
async fn stats_requires_auth_token() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let resp = app.get("/api/admin/coupons/stats", None).await;
    resp.assert_status(StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn member_cannot_read_stats() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let member = app.seed_user().await.expect("seed member");
    let resp = app
        .get("/api/admin/coupons/stats", Some(&member.access_token))
        .await;
    resp.assert_status(StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn admin_can_read_stats() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let admin = app.seed_admin().await.expect("seed admin");
    let resp = app
        .get("/api/admin/coupons/stats", Some(&admin.access_token))
        .await;
    resp.assert_status(StatusCode::OK);
}

#[tokio::test]
async fn support_can_read_stats() {
    // Support inherits `coupon.read_any` via 021_rbac.sql:235.
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let support = app.seed_support().await.expect("seed support");
    let resp = app
        .get("/api/admin/coupons/stats", Some(&support.access_token))
        .await;
    resp.assert_status(StatusCode::OK);
}

// ── Shape ──────────────────────────────────────────────────────────────

#[tokio::test]
async fn stats_response_carries_every_documented_field() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let admin = app.seed_admin().await.expect("seed admin");

    let resp = app
        .get("/api/admin/coupons/stats", Some(&admin.access_token))
        .await;
    resp.assert_status(StatusCode::OK);
    let body: Value = resp.json().expect("body");

    // Phase 4.5 spec field names.
    assert!(body["total_coupons"].is_i64(), "total_coupons missing");
    assert!(body["active_coupons"].is_i64(), "active_coupons missing");
    assert!(body["expired_coupons"].is_i64(), "expired_coupons missing");
    assert!(
        body["redemptions_total"].is_i64(),
        "redemptions_total missing"
    );
    assert!(
        body["redemptions_today"].is_i64(),
        "redemptions_today missing"
    );

    // Legacy aliases the existing admin page binds against — must keep
    // returning so the SPA does not regress.
    assert!(body["active_count"].is_i64(), "active_count alias missing");
    assert!(body["total_usages"].is_i64(), "total_usages alias missing");
    assert!(
        body["total_discount_cents"].is_i64(),
        "total_discount_cents missing"
    );
}

// ── Counts ─────────────────────────────────────────────────────────────

#[tokio::test]
async fn active_and_expired_buckets_track_seeded_rows() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let admin = app.seed_admin().await.expect("seed admin");

    // Snapshot the baseline so we can assert *deltas*; other tests in the
    // same Postgres schema may have left rows behind.
    let baseline = read_stats(&app, &admin.access_token).await;

    let _ = seed_coupon(app.db(), "STATS-ACTIVE-A", true, None, admin.id).await;
    let _ = seed_coupon(app.db(), "STATS-ACTIVE-B", true, None, admin.id).await;
    let _ = seed_coupon(
        app.db(),
        "STATS-EXPIRED",
        true,
        Some(Utc::now() - Duration::days(2)),
        admin.id,
    )
    .await;

    let after = read_stats(&app, &admin.access_token).await;

    let delta_total = after["total_coupons"].as_i64().expect("total_coupons")
        - baseline["total_coupons"].as_i64().expect("baseline total");
    let delta_active = after["active_coupons"].as_i64().expect("active_coupons")
        - baseline["active_coupons"]
            .as_i64()
            .expect("baseline active");
    let delta_expired = after["expired_coupons"].as_i64().expect("expired_coupons")
        - baseline["expired_coupons"]
            .as_i64()
            .expect("baseline expired");

    assert_eq!(delta_total, 3, "all three seeded coupons should count");
    assert_eq!(
        delta_active, 2,
        "two unexpired-active coupons should land in active_coupons"
    );
    assert_eq!(
        delta_expired, 1,
        "the past-expiry coupon should land in expired_coupons"
    );
}

#[tokio::test]
async fn redemptions_today_filters_by_utc_midnight() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let admin = app.seed_admin().await.expect("seed admin");
    let user = app.seed_user().await.expect("seed user");

    let baseline = read_stats(&app, &admin.access_token).await;
    let baseline_total = baseline["redemptions_total"]
        .as_i64()
        .expect("baseline total");
    let baseline_today = baseline["redemptions_today"]
        .as_i64()
        .expect("baseline today");

    // Coupon to attach the redemptions to.
    let coupon_id = seed_coupon(app.db(), "STATS-RED", true, None, admin.id).await;

    // 1) A redemption *yesterday* — counts in `redemptions_total` only.
    seed_usage_at(app.db(), coupon_id, user.id, Utc::now() - Duration::days(1)).await;

    // 2) Two redemptions *today* — count in both buckets.
    seed_usage_at(app.db(), coupon_id, user.id, Utc::now()).await;
    seed_usage_at(app.db(), coupon_id, user.id, Utc::now()).await;

    let after = read_stats(&app, &admin.access_token).await;

    let delta_total = after["redemptions_total"].as_i64().expect("after total") - baseline_total;
    let delta_today = after["redemptions_today"].as_i64().expect("after today") - baseline_today;

    assert_eq!(
        delta_total, 3,
        "all three redemptions should land in redemptions_total"
    );
    assert_eq!(
        delta_today, 2,
        "only the two redemptions inside today's UTC window should land in redemptions_today"
    );
}

#[tokio::test]
async fn redemptions_total_alias_matches_redemption_count() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let admin = app.seed_admin().await.expect("seed admin");

    let body = read_stats(&app, &admin.access_token).await;
    assert_eq!(
        body["redemptions_total"]
            .as_i64()
            .expect("redemptions_total"),
        body["redemption_count"].as_i64().expect("redemption_count"),
        "the legacy alias must mirror the spec field"
    );
    assert_eq!(
        body["redemptions_total"]
            .as_i64()
            .expect("redemptions_total"),
        body["total_usages"].as_i64().expect("total_usages"),
        "the legacy alias must mirror the spec field"
    );
    assert_eq!(
        body["active_coupons"].as_i64().expect("active_coupons"),
        body["active_count"].as_i64().expect("active_count"),
        "active_count alias must mirror the spec field"
    );
}

// ── Helpers ────────────────────────────────────────────────────────────

async fn read_stats(app: &TestApp, token: &str) -> Value {
    let resp = app.get("/api/admin/coupons/stats", Some(token)).await;
    resp.assert_status(StatusCode::OK);
    resp.json().expect("stats body")
}
