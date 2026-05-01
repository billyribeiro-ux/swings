#![deny(warnings)]
#![forbid(unsafe_code)]

//! Integration tests for the pricing rollout and grandfather price-protection
//! system implemented in the 2026-05-01 audit session.
//!
//! Tests:
//! - `GET /api/admin/pricing/plans/{id}/rollout-preview` returns accurate counts
//! - Rollout preview correctly counts grandfathered (protected) subscriptions
//! - `POST /api/admin/pricing/subscriptions/{sub_id}/price-protection` toggles flag
//! - Price-protected subscriptions are skipped in `skipped_grandfathered` count
//!   (actual Stripe calls are not made in tests — we assert the skip count
//!   from the preview endpoint and from the DB flag)
//! - RBAC: member cannot access rollout preview or toggle protection

mod support;

use axum::http::StatusCode;
use chrono::{Duration, Utc};
use serde_json::{json, Value};
use support::TestApp;
use swings_api::{
    db,
    models::{SubscriptionPlan, SubscriptionStatus},
};
use uuid::Uuid;

// ── Fixtures ──────────────────────────────────────────────────────────────────

async fn seed_pricing_plan(app: &TestApp, interval: &str) -> Uuid {
    let id = Uuid::new_v4();
    let slug = format!("test-{}-{}", interval, &id.to_string()[..8]);
    sqlx::query(
        r#"
        INSERT INTO pricing_plans
            (id, name, slug, amount_cents, currency, interval, interval_count,
             trial_days, features, is_active)
        VALUES ($1, $2, $3, 2999, 'usd', $4, 1, 0, '[]'::jsonb, TRUE)
        "#,
    )
    .bind(id)
    .bind(format!("Test Plan {}", &id.to_string()[..8]))
    .bind(slug)
    .bind(interval)
    .execute(app.db())
    .await
    .expect("seed pricing_plan");
    id
}

async fn seed_active_subscription(
    app: &TestApp,
    user_id: Uuid,
    pricing_plan_id: Uuid,
) -> Uuid {
    let now = Utc::now();
    let sub = db::upsert_subscription(
        app.db(),
        user_id,
        &format!("cus_test_{}", Uuid::new_v4().simple()),
        &format!("sub_test_{}", Uuid::new_v4().simple()),
        &SubscriptionPlan::Monthly,
        &SubscriptionStatus::Active,
        now,
        now + Duration::days(30),
        Some(pricing_plan_id),
    )
    .await
    .expect("seed subscription");
    sub.id
}

// ── Rollout preview: basic counts ─────────────────────────────────────────────

#[tokio::test]
async fn rollout_preview_returns_total_and_zero_grandfathered_by_default() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let admin = app.seed_admin().await.expect("seed admin");
    let plan_id = seed_pricing_plan(&app, "month").await;

    // Seed 3 linked subscribers
    for _ in 0..3 {
        let member = app.seed_user().await.expect("seed member");
        seed_active_subscription(&app, member.id, plan_id).await;
    }

    let resp = app
        .get(
            &format!("/api/admin/pricing/plans/{plan_id}/rollout-preview"),
            Some(&admin.access_token),
        )
        .await;

    resp.assert_status(StatusCode::OK);
    let body: Value = resp.json().expect("preview body");
    assert_eq!(body["total_in_audience"], 3, "3 active subs in audience");
    assert_eq!(body["would_update"], 3, "all 3 should be updated (none protected)");
    assert_eq!(body["would_skip_grandfathered"], 0, "none grandfathered yet");
}

#[tokio::test]
async fn rollout_preview_reflects_price_protected_subscriptions() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let admin = app.seed_admin().await.expect("seed admin");
    let plan_id = seed_pricing_plan(&app, "month").await;

    // 2 normal + 1 grandfathered
    let m1 = app.seed_user().await.expect("seed member 1");
    let m2 = app.seed_user().await.expect("seed member 2");
    let m3 = app.seed_user().await.expect("seed member 3 (grandfathered)");

    seed_active_subscription(&app, m1.id, plan_id).await;
    seed_active_subscription(&app, m2.id, plan_id).await;
    let protected_sub_id = seed_active_subscription(&app, m3.id, plan_id).await;

    // Enable price protection on m3's subscription
    sqlx::query(
        "UPDATE subscriptions SET price_protection_enabled = TRUE,
         grandfathered_price_cents = 2999, grandfathered_currency = 'usd'
         WHERE id = $1",
    )
    .bind(protected_sub_id)
    .execute(app.db())
    .await
    .expect("enable price protection");

    let resp = app
        .get(
            &format!("/api/admin/pricing/plans/{plan_id}/rollout-preview"),
            Some(&admin.access_token),
        )
        .await;

    resp.assert_status(StatusCode::OK);
    let body: Value = resp.json().expect("preview body");
    assert_eq!(body["total_in_audience"], 3);
    assert_eq!(body["would_update"], 2, "only 2 should be updated");
    assert_eq!(body["would_skip_grandfathered"], 1, "1 grandfathered should be skipped");
}

// ── Price-protection toggle endpoint ─────────────────────────────────────────

#[tokio::test]
async fn toggle_price_protection_enables_and_disables() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let admin = app.seed_admin().await.expect("seed admin");
    let plan_id = seed_pricing_plan(&app, "month").await;
    let member = app.seed_user().await.expect("seed member");
    let sub_id = seed_active_subscription(&app, member.id, plan_id).await;

    // Enable protection
    let enable_resp = app
        .post_json(
            &format!("/api/admin/pricing/subscriptions/{sub_id}/price-protection"),
            &json!({
                "enabled": true,
                "grandfathered_price_cents": 2999,
                "grandfathered_currency": "usd"
            }),
            Some(&admin.access_token),
        )
        .await;
    enable_resp.assert_status(StatusCode::OK);
    let enable_body: Value = enable_resp.json().expect("enable body");
    assert_eq!(enable_body["price_protection_enabled"], true);

    // Verify DB state
    let protected: bool = sqlx::query_scalar(
        "SELECT price_protection_enabled FROM subscriptions WHERE id = $1",
    )
    .bind(sub_id)
    .fetch_one(app.db())
    .await
    .expect("fetch protection flag");
    assert!(protected, "price_protection_enabled must be true after enabling");

    // Disable protection
    let disable_resp = app
        .post_json(
            &format!("/api/admin/pricing/subscriptions/{sub_id}/price-protection"),
            &json!({ "enabled": false }),
            Some(&admin.access_token),
        )
        .await;
    disable_resp.assert_status(StatusCode::OK);
    let disable_body: Value = disable_resp.json().expect("disable body");
    assert_eq!(disable_body["price_protection_enabled"], false);

    // Verify DB state again
    let still_protected: bool = sqlx::query_scalar(
        "SELECT price_protection_enabled FROM subscriptions WHERE id = $1",
    )
    .bind(sub_id)
    .fetch_one(app.db())
    .await
    .expect("fetch protection flag after disable");
    assert!(!still_protected, "price_protection_enabled must be false after disabling");
}

#[tokio::test]
async fn toggle_price_protection_unknown_subscription_returns_404() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let admin = app.seed_admin().await.expect("seed admin");

    let resp = app
        .post_json(
            &format!("/api/admin/pricing/subscriptions/{}/price-protection", Uuid::new_v4()),
            &json!({ "enabled": true }),
            Some(&admin.access_token),
        )
        .await;
    resp.assert_status(StatusCode::NOT_FOUND);
}

// ── RBAC gates ────────────────────────────────────────────────────────────────

#[tokio::test]
async fn member_cannot_access_rollout_preview() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let member = app.seed_user().await.expect("seed member");
    let plan_id = seed_pricing_plan(&app, "month").await;

    app.get(
        &format!("/api/admin/pricing/plans/{plan_id}/rollout-preview"),
        Some(&member.access_token),
    )
    .await
    .assert_status(StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn member_cannot_toggle_price_protection() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let member = app.seed_user().await.expect("seed member");
    let plan_id = seed_pricing_plan(&app, "month").await;
    let sub_id = seed_active_subscription(&app, member.id, plan_id).await;

    app.post_json(
        &format!("/api/admin/pricing/subscriptions/{sub_id}/price-protection"),
        &json!({ "enabled": true }),
        Some(&member.access_token),
    )
    .await
    .assert_status(StatusCode::FORBIDDEN);
}

// ── Rollout preview: plan not found ──────────────────────────────────────────

#[tokio::test]
async fn rollout_preview_unknown_plan_returns_404() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let admin = app.seed_admin().await.expect("seed admin");

    app.get(
        &format!("/api/admin/pricing/plans/{}/rollout-preview", Uuid::new_v4()),
        Some(&admin.access_token),
    )
    .await
    .assert_status(StatusCode::NOT_FOUND);
}
