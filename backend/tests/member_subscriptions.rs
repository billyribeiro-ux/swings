#![deny(warnings)]
#![forbid(unsafe_code)]

//! Phase 5 integration coverage for the member-facing subscriptions
//! surface: list (history), detail, and the four mutators
//! (cancel / resume / pause / unpause). Plan-switch happy-path requires
//! a live Stripe sandbox so it's covered at the unit-test boundary; here
//! we only assert ownership-404 on the switch endpoint.

mod support;

use axum::http::StatusCode;
use chrono::{DateTime, Duration, Utc};
use serde_json::Value;
use sqlx::PgPool;
use support::TestApp;
use uuid::Uuid;

// ── Fixtures ───────────────────────────────────────────────────────────

/// Insert a local-only subscription (no Stripe twin) so the mutators can
/// run without a real Stripe sandbox. Every field is set so the row
/// decodes cleanly via `Subscription` (FromRow). Returns the new id.
async fn seed_subscription(pool: &PgPool, user_id: Uuid, plan: &str, status: &str) -> Uuid {
    let id = Uuid::new_v4();
    sqlx::query(
        r#"
        INSERT INTO subscriptions (
            id, user_id, stripe_customer_id, stripe_subscription_id,
            plan, status, current_period_start, current_period_end,
            created_at, updated_at
        ) VALUES (
            $1, $2, '', '',
            $3::subscription_plan, $4::subscription_status,
            NOW(), NOW() + INTERVAL '30 days',
            NOW(), NOW()
        )
        "#,
    )
    .bind(id)
    .bind(user_id)
    .bind(plan)
    .bind(status)
    .execute(pool)
    .await
    .expect("seed subscription");
    id
}

/// Read the `paused_at` column for assertions against the pause endpoint.
async fn read_paused_at(pool: &PgPool, sub_id: Uuid) -> Option<DateTime<Utc>> {
    sqlx::query_scalar::<_, Option<DateTime<Utc>>>(
        "SELECT paused_at FROM subscriptions WHERE id = $1",
    )
    .bind(sub_id)
    .fetch_one(pool)
    .await
    .expect("read paused_at")
}

async fn read_cancel_at(pool: &PgPool, sub_id: Uuid) -> Option<DateTime<Utc>> {
    sqlx::query_scalar::<_, Option<DateTime<Utc>>>(
        "SELECT cancel_at FROM subscriptions WHERE id = $1",
    )
    .bind(sub_id)
    .fetch_one(pool)
    .await
    .expect("read cancel_at")
}

// ── List ───────────────────────────────────────────────────────────────

#[tokio::test]
async fn list_returns_full_history_including_canceled_rows() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let alice = app.seed_user().await.expect("seed alice");
    seed_subscription(app.db(), alice.id, "monthly", "active").await;
    seed_subscription(app.db(), alice.id, "annual", "canceled").await;
    seed_subscription(app.db(), alice.id, "monthly", "paused").await;

    let resp = app
        .get("/api/member/subscriptions", Some(&alice.access_token))
        .await;
    resp.assert_status(StatusCode::OK);
    let body: Value = resp.json().expect("body");
    assert_eq!(body["total"], serde_json::json!(3));
    let data = body["data"].as_array().expect("data");
    assert_eq!(data.len(), 3);

    let statuses: Vec<&str> = data
        .iter()
        .map(|r| r["status"].as_str().expect("status"))
        .collect();
    assert!(statuses.contains(&"active"));
    assert!(statuses.contains(&"canceled"));
    assert!(statuses.contains(&"paused"));
}

#[tokio::test]
async fn list_returns_empty_array_for_user_with_no_subscriptions() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let alice = app.seed_user().await.expect("seed alice");
    let resp = app
        .get("/api/member/subscriptions", Some(&alice.access_token))
        .await;
    resp.assert_status(StatusCode::OK);
    let body: Value = resp.json().expect("body");
    assert_eq!(body["total"], serde_json::json!(0));
    assert_eq!(
        body["data"].as_array().expect("data array").len(),
        0,
        "expected empty data array"
    );
}

// ── Detail / ownership ────────────────────────────────────────────────

#[tokio::test]
async fn detail_returns_404_for_foreign_subscription() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let alice = app.seed_user().await.expect("seed alice");
    let bob = app.seed_user().await.expect("seed bob");
    let bobs_sub = seed_subscription(app.db(), bob.id, "monthly", "active").await;

    let resp = app
        .get(
            &format!("/api/member/subscriptions/{bobs_sub}"),
            Some(&alice.access_token),
        )
        .await;
    resp.assert_status(StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn detail_returns_subscription_for_owner() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let alice = app.seed_user().await.expect("seed alice");
    let sub_id = seed_subscription(app.db(), alice.id, "monthly", "active").await;

    let resp = app
        .get(
            &format!("/api/member/subscriptions/{sub_id}"),
            Some(&alice.access_token),
        )
        .await;
    resp.assert_status(StatusCode::OK);
    let body: Value = resp.json().expect("body");
    assert_eq!(
        body["subscription"]["id"],
        serde_json::json!(sub_id.to_string())
    );
    assert!(body["invoices"].is_array());
    assert!(body["related_orders"].is_array());
}

// ── Cancel / resume ───────────────────────────────────────────────────

#[tokio::test]
async fn cancel_sets_cancel_at_period_end_for_owner() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let alice = app.seed_user().await.expect("seed alice");
    let sub_id = seed_subscription(app.db(), alice.id, "monthly", "active").await;

    let resp = app
        .post_json::<Value>(
            &format!("/api/member/subscriptions/{sub_id}/cancel"),
            &serde_json::json!({}),
            Some(&alice.access_token),
        )
        .await;
    resp.assert_status(StatusCode::OK);

    let cancel_at = read_cancel_at(app.db(), sub_id).await;
    assert!(
        cancel_at.is_some(),
        "cancel_at should be populated after cancel"
    );

    // Resume clears it.
    let resume = app
        .post_json::<Value>(
            &format!("/api/member/subscriptions/{sub_id}/resume"),
            &serde_json::json!({}),
            Some(&alice.access_token),
        )
        .await;
    resume.assert_status(StatusCode::OK);
    let after_resume = read_cancel_at(app.db(), sub_id).await;
    assert!(
        after_resume.is_none(),
        "cancel_at should be cleared after resume"
    );
}

#[tokio::test]
async fn cancel_returns_404_for_foreign_subscription() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let alice = app.seed_user().await.expect("seed alice");
    let bob = app.seed_user().await.expect("seed bob");
    let bobs_sub = seed_subscription(app.db(), bob.id, "monthly", "active").await;

    let resp = app
        .post_json::<Value>(
            &format!("/api/member/subscriptions/{bobs_sub}/cancel"),
            &serde_json::json!({}),
            Some(&alice.access_token),
        )
        .await;
    resp.assert_status(StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn cancel_writes_audit_row() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let alice = app.seed_user().await.expect("seed alice");
    let sub_id = seed_subscription(app.db(), alice.id, "monthly", "active").await;

    app.post_json::<Value>(
        &format!("/api/member/subscriptions/{sub_id}/cancel"),
        &serde_json::json!({}),
        Some(&alice.access_token),
    )
    .await
    .assert_status(StatusCode::OK);

    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM admin_actions WHERE action = 'member.subscription.cancel'",
    )
    .fetch_one(app.db())
    .await
    .expect("count audit rows");
    assert_eq!(count, 1, "expected exactly one cancel audit row");
}

// ── Pause / unpause ───────────────────────────────────────────────────

#[tokio::test]
async fn pause_writes_paused_at_with_optional_resume_timestamp() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let alice = app.seed_user().await.expect("seed alice");
    let sub_id = seed_subscription(app.db(), alice.id, "monthly", "active").await;

    assert!(read_paused_at(app.db(), sub_id).await.is_none());

    let resume_at =
        (Utc::now() + Duration::days(14)).to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
    let resp = app
        .post_json::<Value>(
            &format!("/api/member/subscriptions/{sub_id}/pause"),
            &serde_json::json!({ "resume_at": resume_at }),
            Some(&alice.access_token),
        )
        .await;
    resp.assert_status(StatusCode::OK);
    let paused_at = read_paused_at(app.db(), sub_id).await;
    assert!(paused_at.is_some(), "paused_at should be populated");

    let unpause = app
        .post_json::<Value>(
            &format!("/api/member/subscriptions/{sub_id}/unpause"),
            &serde_json::json!({}),
            Some(&alice.access_token),
        )
        .await;
    unpause.assert_status(StatusCode::OK);
    let after = read_paused_at(app.db(), sub_id).await;
    assert!(after.is_none(), "paused_at should be cleared after unpause");
}

#[tokio::test]
async fn pause_returns_404_for_foreign_subscription() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let alice = app.seed_user().await.expect("seed alice");
    let bob = app.seed_user().await.expect("seed bob");
    let bobs_sub = seed_subscription(app.db(), bob.id, "monthly", "active").await;

    let resp = app
        .post_json::<Value>(
            &format!("/api/member/subscriptions/{bobs_sub}/pause"),
            &serde_json::json!({}),
            Some(&alice.access_token),
        )
        .await;
    resp.assert_status(StatusCode::NOT_FOUND);
}

// ── Switch plan ───────────────────────────────────────────────────────

#[tokio::test]
async fn switch_plan_returns_404_for_foreign_subscription() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let alice = app.seed_user().await.expect("seed alice");
    let bob = app.seed_user().await.expect("seed bob");
    let bobs_sub = seed_subscription(app.db(), bob.id, "monthly", "active").await;

    let resp = app
        .post_json::<Value>(
            &format!("/api/member/subscriptions/{bobs_sub}/switch-plan"),
            &serde_json::json!({ "pricing_plan_id": Uuid::new_v4() }),
            Some(&alice.access_token),
        )
        .await;
    resp.assert_status(StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn switch_plan_returns_404_for_unknown_plan() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let alice = app.seed_user().await.expect("seed alice");
    let sub_id = seed_subscription(app.db(), alice.id, "monthly", "active").await;

    let resp = app
        .post_json::<Value>(
            &format!("/api/member/subscriptions/{sub_id}/switch-plan"),
            &serde_json::json!({ "pricing_plan_id": Uuid::new_v4() }),
            Some(&alice.access_token),
        )
        .await;
    resp.assert_status(StatusCode::NOT_FOUND);
}

// ── Coupon redemptions ────────────────────────────────────────────────

#[tokio::test]
async fn redeemed_coupons_returns_only_callers_history() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let alice = app.seed_user().await.expect("seed alice");
    let bob = app.seed_user().await.expect("seed bob");

    // Seed a coupon owned by alice (created_by = alice) so the FK
    // constraint is satisfied; reuse for both users' redemptions.
    let coupon_id = Uuid::new_v4();
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
            $1, 'WELCOME10', NULL, 'percentage'::discount_type, 10,
            NULL, NULL, 'all',
            '{}', '{}',
            NULL, 0, 99,
            NULL, NULL, true, false, false,
            NULL, $2, NOW(), NOW()
        )
        "#,
    )
    .bind(coupon_id)
    .bind(alice.id)
    .execute(app.db())
    .await
    .expect("seed coupon");

    sqlx::query(
        r#"
        INSERT INTO coupon_usages
            (id, coupon_id, user_id, subscription_id, discount_applied_cents, used_at)
        VALUES ($1, $2, $3, NULL, 500, NOW())
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(coupon_id)
    .bind(alice.id)
    .execute(app.db())
    .await
    .expect("seed alice usage");

    sqlx::query(
        r#"
        INSERT INTO coupon_usages
            (id, coupon_id, user_id, subscription_id, discount_applied_cents, used_at)
        VALUES ($1, $2, $3, NULL, 700, NOW())
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(coupon_id)
    .bind(bob.id)
    .execute(app.db())
    .await
    .expect("seed bob usage");

    let resp = app
        .get("/api/member/coupons/redeemed", Some(&alice.access_token))
        .await;
    resp.assert_status(StatusCode::OK);
    let body: Value = resp.json().expect("body");
    let arr = body.as_array().expect("array");
    assert_eq!(arr.len(), 1, "alice should see exactly her own redemption");
    assert_eq!(arr[0]["coupon_code"], serde_json::json!("WELCOME10"));
    assert_eq!(arr[0]["discount_applied_cents"], serde_json::json!(500));
}
