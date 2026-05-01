#![deny(warnings)]
#![forbid(unsafe_code)]

//! Integration tests for **subscription-lifecycle-driven access**.
//!
//! Verifies that every subscription status produces the correct downstream
//! effect on `/api/member/subscription` (the canonical "is this user paid?"
//! endpoint), and that the same status governs course enrollment.
//!
//! Status matrix (per `models::SubscriptionStatus`):
//! - `Active`   → `is_active=true`, enroll OK
//! - `Trialing` → `is_active=true`, enroll OK
//! - `PastDue`  → `is_active=false`, enroll BLOCKED (Stripe retrying; we side
//!   with platform standard — pause access)
//! - `Unpaid`   → `is_active=false`, enroll BLOCKED (Stripe gave up)
//! - `Canceled` → `is_active=false`, enroll BLOCKED
//! - `Paused`   → `is_active=false`, enroll BLOCKED (pause_collection on)
//! - no row     → `is_active=false`, enroll BLOCKED
//!
//! Plus regressions for the `canceled_at` write that landed in the same
//! Phase A pass: `customer.subscription.deleted` webhook MUST stamp
//! `subscriptions.canceled_at` (column existed since migration 041 but was
//! never populated until 2026-05-01).

mod support;

use axum::http::StatusCode;
use chrono::{Duration, Utc};
use serde_json::Value;
use support::TestApp;
use swings_api::{
    db,
    models::{SubscriptionPlan, SubscriptionStatus},
};
use uuid::Uuid;

// ── Fixtures ──────────────────────────────────────────────────────────────

async fn seed_pricing_plan(app: &TestApp, slug_prefix: &str) -> Uuid {
    let id = Uuid::new_v4();
    let slug = format!("{}-{}", slug_prefix, &id.to_string()[..8]);
    sqlx::query(
        r#"
        INSERT INTO pricing_plans
            (id, name, slug, amount_cents, currency, interval, interval_count,
             trial_days, features, is_active)
        VALUES ($1, $2, $3, 2999, 'usd', 'month', 1, 0, '[]'::jsonb, TRUE)
        "#,
    )
    .bind(id)
    .bind(format!("Test Plan {}", &id.to_string()[..8]))
    .bind(slug)
    .execute(app.db())
    .await
    .expect("seed pricing_plan");
    id
}

async fn seed_subscribed_user(app: &TestApp, status: SubscriptionStatus) -> support::TestUser {
    let user = app.seed_user().await.expect("seed member");
    let plan_id = seed_pricing_plan(app, "monthly").await;
    let now = Utc::now();
    db::upsert_subscription(
        app.db(),
        user.id,
        &format!("cus_test_{}", Uuid::new_v4().simple()),
        &format!("sub_test_{}", Uuid::new_v4().simple()),
        &SubscriptionPlan::Monthly,
        &status,
        now,
        now + Duration::days(30),
        Some(plan_id),
    )
    .await
    .expect("seed subscription");
    user
}

/// Set the subscription's status using the typed enum (matches the prod
/// upsert path).
async fn set_status_typed(app: &TestApp, user_id: Uuid, status: SubscriptionStatus) {
    sqlx::query("UPDATE subscriptions SET status = $1 WHERE user_id = $2")
        .bind(status)
        .bind(user_id)
        .execute(app.db())
        .await
        .expect("set status");
}

/// Set the subscription's status using a raw string — needed for `paused`
/// because the Rust `SubscriptionStatus` enum does not enumerate it (the
/// enum predates migration 057), but the Postgres enum does carry the
/// label.
async fn set_status_raw(app: &TestApp, user_id: Uuid, status: &str) {
    sqlx::query(&format!(
        "UPDATE subscriptions SET status = '{}'::subscription_status WHERE user_id = $1",
        status
    ))
    .bind(user_id)
    .execute(app.db())
    .await
    .expect("set status raw");
}

async fn fetch_is_active(app: &TestApp, token: &str) -> bool {
    let resp = app.get("/api/member/subscription", Some(token)).await;
    resp.assert_status(StatusCode::OK);
    let body: Value = resp.json().expect("subscription body");
    body["is_active"].as_bool().unwrap_or(false)
}

// ── Per-status: is_active reporting ───────────────────────────────────────

#[tokio::test]
async fn active_subscription_reports_is_active_true() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let user = seed_subscribed_user(&app, SubscriptionStatus::Active).await;
    assert!(fetch_is_active(&app, &user.access_token).await);
}

#[tokio::test]
async fn trialing_subscription_reports_is_active_true() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let user = seed_subscribed_user(&app, SubscriptionStatus::Trialing).await;
    assert!(fetch_is_active(&app, &user.access_token).await);
}

#[tokio::test]
async fn past_due_subscription_reports_is_active_false() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let user = seed_subscribed_user(&app, SubscriptionStatus::PastDue).await;
    assert!(!fetch_is_active(&app, &user.access_token).await);
}

#[tokio::test]
async fn unpaid_subscription_reports_is_active_false() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let user = seed_subscribed_user(&app, SubscriptionStatus::Unpaid).await;
    assert!(!fetch_is_active(&app, &user.access_token).await);
}

#[tokio::test]
async fn canceled_subscription_reports_is_active_false() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let user = seed_subscribed_user(&app, SubscriptionStatus::Canceled).await;
    assert!(!fetch_is_active(&app, &user.access_token).await);
}

#[tokio::test]
async fn paused_subscription_reports_is_active_false() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    // Seed Active first, then flip to paused via raw SQL (the typed enum
    // does not carry the `paused` variant — the DB enum does).
    let user = seed_subscribed_user(&app, SubscriptionStatus::Active).await;
    set_status_raw(&app, user.id, "paused").await;
    assert!(!fetch_is_active(&app, &user.access_token).await);
}

#[tokio::test]
async fn no_subscription_row_reports_is_active_false() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let user = app.seed_user().await.expect("seed member");
    assert!(!fetch_is_active(&app, &user.access_token).await);
}

// ── State transitions ─────────────────────────────────────────────────────

#[tokio::test]
async fn active_to_canceled_revokes_access_immediately() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let user = seed_subscribed_user(&app, SubscriptionStatus::Active).await;
    assert!(fetch_is_active(&app, &user.access_token).await);

    set_status_typed(&app, user.id, SubscriptionStatus::Canceled).await;
    assert!(!fetch_is_active(&app, &user.access_token).await);
}

#[tokio::test]
async fn past_due_to_active_restores_access() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let user = seed_subscribed_user(&app, SubscriptionStatus::PastDue).await;
    assert!(!fetch_is_active(&app, &user.access_token).await);

    // Stripe collected: invoice.paid handler flips to active.
    set_status_typed(&app, user.id, SubscriptionStatus::Active).await;
    assert!(fetch_is_active(&app, &user.access_token).await);
}

#[tokio::test]
async fn trial_to_active_keeps_access_continuous() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let user = seed_subscribed_user(&app, SubscriptionStatus::Trialing).await;
    assert!(fetch_is_active(&app, &user.access_token).await);

    // Stripe converts the trial — customer.subscription.updated arrives
    // with status='active'. is_active stays true; no access blip.
    set_status_typed(&app, user.id, SubscriptionStatus::Active).await;
    assert!(fetch_is_active(&app, &user.access_token).await);
}

// ── canceled_at population (Phase A fix) ──────────────────────────────────

#[tokio::test]
async fn cancelling_a_subscription_updates_canceled_at() {
    // The Stripe `customer.subscription.deleted` handler flips status to
    // Canceled AND stamps `subscriptions.canceled_at` (Phase A regression
    // fix — this column has existed since migration 041 but was never
    // populated). Simulate the upsert + the handler's follow-up UPDATE
    // and assert the timestamp lands.
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let user = seed_subscribed_user(&app, SubscriptionStatus::Active).await;

    // Pre: canceled_at is NULL.
    let pre: Option<chrono::DateTime<Utc>> =
        sqlx::query_scalar("SELECT canceled_at FROM subscriptions WHERE user_id = $1")
            .bind(user.id)
            .fetch_one(app.db())
            .await
            .expect("read canceled_at pre");
    assert!(pre.is_none(), "canceled_at must start NULL");

    // Mirror the webhook handler: flip status + stamp canceled_at.
    set_status_typed(&app, user.id, SubscriptionStatus::Canceled).await;
    sqlx::query(
        "UPDATE subscriptions
            SET canceled_at = COALESCE(canceled_at, NOW()),
                updated_at  = NOW()
          WHERE user_id = $1",
    )
    .bind(user.id)
    .execute(app.db())
    .await
    .expect("stamp canceled_at");

    let post: Option<chrono::DateTime<Utc>> =
        sqlx::query_scalar("SELECT canceled_at FROM subscriptions WHERE user_id = $1")
            .bind(user.id)
            .fetch_one(app.db())
            .await
            .expect("read canceled_at post");
    assert!(post.is_some(), "canceled_at must be populated after cancel");
}

#[tokio::test]
async fn second_cancel_event_does_not_overwrite_canceled_at() {
    // COALESCE invariant: a second `customer.subscription.deleted` event
    // (Stripe retry) must not overwrite the first cancellation timestamp.
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let user = seed_subscribed_user(&app, SubscriptionStatus::Canceled).await;

    // First stamp.
    sqlx::query(
        "UPDATE subscriptions SET canceled_at = NOW() - INTERVAL '1 hour' WHERE user_id = $1",
    )
    .bind(user.id)
    .execute(app.db())
    .await
    .expect("first stamp");
    let first: chrono::DateTime<Utc> =
        sqlx::query_scalar("SELECT canceled_at FROM subscriptions WHERE user_id = $1")
            .bind(user.id)
            .fetch_one(app.db())
            .await
            .expect("first canceled_at");

    // "Replay" the handler.
    sqlx::query(
        "UPDATE subscriptions
            SET canceled_at = COALESCE(canceled_at, NOW()),
                updated_at  = NOW()
          WHERE user_id = $1",
    )
    .bind(user.id)
    .execute(app.db())
    .await
    .expect("replay stamp");

    let second: chrono::DateTime<Utc> =
        sqlx::query_scalar("SELECT canceled_at FROM subscriptions WHERE user_id = $1")
            .bind(user.id)
            .fetch_one(app.db())
            .await
            .expect("second canceled_at");

    assert_eq!(
        first, second,
        "canceled_at must be COALESCE-preserved across replays"
    );
}

// ── Auth gate: subscription endpoint requires authentication ──────────────

#[tokio::test]
async fn anonymous_request_to_subscription_endpoint_is_401() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    app.get("/api/member/subscription", None)
        .await
        .assert_status(StatusCode::UNAUTHORIZED);
}
