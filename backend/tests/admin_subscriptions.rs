#![deny(warnings)]
#![forbid(unsafe_code)]

//! ADM-11 integration coverage for the manual subscription operations
//! admin surface.

mod support;

use axum::http::StatusCode;
use chrono::{DateTime, Duration, Utc};
use serde_json::{json, Value};
use sqlx::Row;
use support::{AssertProblem, TestApp};
use swings_api::{
    db,
    models::{SubscriptionPlan, SubscriptionStatus},
};
use uuid::Uuid;

// ── Fixtures ───────────────────────────────────────────────────────────

async fn seed_membership_plan(app: &TestApp) -> Uuid {
    let id = Uuid::new_v4();
    sqlx::query(
        r#"
        INSERT INTO membership_plans (id, slug, name, description, default_duration_days, is_active)
        VALUES ($1, $2, $3, '', 30, TRUE)
        "#,
    )
    .bind(id)
    .bind(format!("plan-{}", &id.to_string()[..8]))
    .bind("Test Plan")
    .execute(app.db())
    .await
    .expect("seed membership_plan");
    id
}

async fn seed_subscription_for(app: &TestApp, user_id: Uuid) -> Uuid {
    let now = Utc::now();
    let sub = db::upsert_subscription(
        app.db(),
        user_id,
        &format!("cus_test_{}", Uuid::new_v4()),
        &format!("sub_test_{}", Uuid::new_v4()),
        &SubscriptionPlan::Monthly,
        &SubscriptionStatus::Active,
        now,
        now + Duration::days(30),
    )
    .await
    .expect("upsert subscription");
    sub.id
}

// ── RBAC gates ─────────────────────────────────────────────────────────

#[tokio::test]
async fn member_cannot_touch_admin_subscription_surface() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let member = app.seed_user().await.expect("seed member");

    let resp = app
        .get(
            &format!("/api/admin/subscriptions/by-user/{}", member.id),
            Some(&member.access_token),
        )
        .await;
    resp.assert_status(StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn support_can_read_but_not_mutate_subscriptions() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let admin = app.seed_admin().await.expect("seed admin");
    let support = app.seed_support().await.expect("seed support");

    // GET works for support — `admin.subscription.read` is seeded.
    let read = app
        .get(
            &format!("/api/admin/subscriptions/by-user/{}", admin.id),
            Some(&support.access_token),
        )
        .await;
    read.assert_status(StatusCode::OK);

    // Mutations must be denied.
    let plan_id = seed_membership_plan(&app).await;
    let comp = app
        .post_json::<Value>(
            "/api/admin/subscriptions/comp",
            &json!({"user_id": admin.id, "plan_id": plan_id, "duration_days": 30}),
            Some(&support.access_token),
        )
        .await;
    comp.assert_status(StatusCode::FORBIDDEN);
}

// ── Comp / gift ────────────────────────────────────────────────────────

#[tokio::test]
async fn comp_grant_creates_membership_and_audits() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let admin = app.seed_admin().await.expect("seed admin");
    let recipient = app.seed_user().await.expect("seed recipient");
    let plan_id = seed_membership_plan(&app).await;

    let resp = app
        .post_json::<Value>(
            "/api/admin/subscriptions/comp",
            &json!({
                "user_id":       recipient.id,
                "plan_id":       plan_id,
                "duration_days": 60,
                "notes":         "VIP customer"
            }),
            Some(&admin.access_token),
        )
        .await;
    resp.assert_status(StatusCode::CREATED);
    let body: Value = resp.json().expect("body");
    let mid = body["membership_id"].as_str().expect("membership_id");
    assert!(body["ends_at"].is_string(), "ends_at should be present");

    // Membership row exists with the right shape.
    let row = sqlx::query(
        "SELECT user_id, plan_id, granted_by, status FROM memberships WHERE id = $1::uuid",
    )
    .bind(mid)
    .fetch_one(app.db())
    .await
    .expect("membership row");
    let user_id: Uuid = row.try_get("user_id").expect("user_id");
    assert_eq!(user_id, recipient.id);
    let plan: Uuid = row.try_get("plan_id").expect("plan_id");
    assert_eq!(plan, plan_id);
    let granted_by: String = row.try_get("granted_by").expect("granted_by");
    assert_eq!(granted_by, "manual");

    // Audit row landed.
    let audit_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM admin_actions WHERE action = 'admin.subscription.comp' AND target_kind = 'membership'",
    )
    .fetch_one(app.db())
    .await
    .expect("audit count");
    assert_eq!(audit_count, 1);
}

#[tokio::test]
async fn comp_grant_open_ended_when_no_duration() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let admin = app.seed_admin().await.expect("seed admin");
    let recipient = app.seed_user().await.expect("seed recipient");
    let plan_id = seed_membership_plan(&app).await;

    let resp = app
        .post_json::<Value>(
            "/api/admin/subscriptions/comp",
            &json!({
                "user_id": recipient.id,
                "plan_id": plan_id
            }),
            Some(&admin.access_token),
        )
        .await;
    resp.assert_status(StatusCode::CREATED);
    let body: Value = resp.json().expect("body");
    assert!(
        body["ends_at"].is_null(),
        "ends_at should be null for open-ended"
    );
}

#[tokio::test]
async fn comp_grant_unknown_user_is_404() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let admin = app.seed_admin().await.expect("seed admin");
    let plan_id = seed_membership_plan(&app).await;

    let resp = app
        .post_json::<Value>(
            "/api/admin/subscriptions/comp",
            &json!({
                "user_id": Uuid::new_v4(),
                "plan_id": plan_id,
                "duration_days": 7
            }),
            Some(&admin.access_token),
        )
        .await;
    resp.assert_status(StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn comp_grant_unknown_plan_is_404() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let admin = app.seed_admin().await.expect("seed admin");
    let recipient = app.seed_user().await.expect("seed recipient");

    let resp = app
        .post_json::<Value>(
            "/api/admin/subscriptions/comp",
            &json!({
                "user_id": recipient.id,
                "plan_id": Uuid::new_v4(),
                "duration_days": 7
            }),
            Some(&admin.access_token),
        )
        .await;
    resp.assert_status(StatusCode::NOT_FOUND);
}

// ── Extend ─────────────────────────────────────────────────────────────

#[tokio::test]
async fn extend_pushes_period_end_and_logs_change() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let admin = app.seed_admin().await.expect("seed admin");
    let user = app.seed_user().await.expect("seed user");
    let sub_id = seed_subscription_for(&app, user.id).await;

    let before: DateTime<Utc> =
        sqlx::query_scalar("SELECT current_period_end FROM subscriptions WHERE id = $1")
            .bind(sub_id)
            .fetch_one(app.db())
            .await
            .expect("before");

    let resp = app
        .post_json::<Value>(
            &format!("/api/admin/subscriptions/{sub_id}/extend"),
            &json!({"days": 14, "notes": "comp on top of paid"}),
            Some(&admin.access_token),
        )
        .await;
    resp.assert_status(StatusCode::OK);
    let body: Value = resp.json().expect("body");
    assert_eq!(body["subscription_id"], json!(sub_id));

    let after: DateTime<Utc> =
        sqlx::query_scalar("SELECT current_period_end FROM subscriptions WHERE id = $1")
            .bind(sub_id)
            .fetch_one(app.db())
            .await
            .expect("after");
    let delta = after - before;
    assert_eq!(delta.num_days(), 14);

    // change-log row landed with the new kind.
    let change_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM subscription_changes WHERE subscription_id = $1 AND kind = 'manual_extend'",
    )
    .bind(sub_id)
    .fetch_one(app.db())
    .await
    .expect("change count");
    assert_eq!(change_count, 1);
}

#[tokio::test]
async fn extend_rejects_zero_or_excessive_days() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let admin = app.seed_admin().await.expect("seed admin");
    let user = app.seed_user().await.expect("seed user");
    let sub_id = seed_subscription_for(&app, user.id).await;

    let zero = app
        .post_json::<Value>(
            &format!("/api/admin/subscriptions/{sub_id}/extend"),
            &json!({"days": 0}),
            Some(&admin.access_token),
        )
        .await;
    zero.assert_status(StatusCode::BAD_REQUEST);

    let too_many = app
        .post_json::<Value>(
            &format!("/api/admin/subscriptions/{sub_id}/extend"),
            &json!({"days": 9999}),
            Some(&admin.access_token),
        )
        .await;
    too_many.assert_status(StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn extend_unknown_subscription_is_404() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let admin = app.seed_admin().await.expect("seed admin");

    let resp = app
        .post_json::<Value>(
            &format!("/api/admin/subscriptions/{}/extend", Uuid::new_v4()),
            &json!({"days": 7}),
            Some(&admin.access_token),
        )
        .await;
    resp.assert_status(StatusCode::NOT_FOUND);
}

// ── Billing-cycle override ─────────────────────────────────────────────

#[tokio::test]
async fn cycle_override_sets_anchor_and_audits() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let admin = app.seed_admin().await.expect("seed admin");
    let user = app.seed_user().await.expect("seed user");
    let sub_id = seed_subscription_for(&app, user.id).await;

    let new_anchor = Utc::now() + Duration::days(45);
    let resp = app
        .post_json::<Value>(
            &format!("/api/admin/subscriptions/{sub_id}/billing-cycle"),
            &json!({"anchor": new_anchor, "notes": "align with calendar quarter"}),
            Some(&admin.access_token),
        )
        .await;
    resp.assert_status(StatusCode::OK);
    let body: Value = resp.json().expect("body");
    assert!(
        body["previous_anchor"].is_null(),
        "fresh subscription has no anchor"
    );

    let anchor_db: Option<DateTime<Utc>> =
        sqlx::query_scalar("SELECT billing_cycle_anchor FROM subscriptions WHERE id = $1")
            .bind(sub_id)
            .fetch_one(app.db())
            .await
            .expect("anchor");
    assert!(anchor_db.is_some());

    let change_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM subscription_changes WHERE subscription_id = $1 AND kind = 'cycle_override'",
    )
    .bind(sub_id)
    .fetch_one(app.db())
    .await
    .expect("change count");
    assert_eq!(change_count, 1);
}

#[tokio::test]
async fn cycle_override_rejects_past_anchor() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let admin = app.seed_admin().await.expect("seed admin");
    let user = app.seed_user().await.expect("seed user");
    let sub_id = seed_subscription_for(&app, user.id).await;

    let past = Utc::now() - Duration::days(1);
    let resp = app
        .post_json::<Value>(
            &format!("/api/admin/subscriptions/{sub_id}/billing-cycle"),
            &json!({"anchor": past}),
            Some(&admin.access_token),
        )
        .await;
    resp.assert_problem(AssertProblem {
        status: StatusCode::BAD_REQUEST,
        type_suffix: "bad-request",
        title: "Bad Request",
    });
}

// ── by-user roll-up ────────────────────────────────────────────────────

#[tokio::test]
async fn by_user_returns_subscription_and_memberships() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let admin = app.seed_admin().await.expect("seed admin");
    let user = app.seed_user().await.expect("seed user");
    let sub_id = seed_subscription_for(&app, user.id).await;

    // Mint a comp on top so the rollup has both shapes.
    let plan_id = seed_membership_plan(&app).await;
    app.post_json::<Value>(
        "/api/admin/subscriptions/comp",
        &json!({"user_id": user.id, "plan_id": plan_id, "duration_days": 30}),
        Some(&admin.access_token),
    )
    .await
    .assert_status(StatusCode::CREATED);

    let resp = app
        .get(
            &format!("/api/admin/subscriptions/by-user/{}", user.id),
            Some(&admin.access_token),
        )
        .await;
    resp.assert_status(StatusCode::OK);
    let body: Value = resp.json().expect("body");
    assert_eq!(body["subscription"]["is_active"], json!(true));
    assert_eq!(
        body["subscription"]["subscription"]["id"],
        json!(sub_id),
        "expected the seeded subscription to surface"
    );
    let memberships = body["memberships"].as_array().expect("memberships array");
    assert_eq!(memberships.len(), 1);
    assert_eq!(memberships[0]["granted_by"], json!("manual"));
    assert_eq!(memberships[0]["status"], json!("active"));
}

#[tokio::test]
async fn by_user_unknown_member_is_404() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let admin = app.seed_admin().await.expect("seed admin");

    let resp = app
        .get(
            &format!("/api/admin/subscriptions/by-user/{}", Uuid::new_v4()),
            Some(&admin.access_token),
        )
        .await;
    resp.assert_status(StatusCode::NOT_FOUND);
}
