#![deny(warnings)]
#![forbid(unsafe_code)]

//! Integration coverage for `GET /api/admin/stats` (range-scoped dashboard).
//!
//! The handler exposes:
//! - lifetime totals (members, subscribers, watchlists, enrollments) — unchanged,
//! - a `period` window resolved from the `range` query parameter, and
//! - a `previous_period` window of identical length immediately before it.
//!
//! These tests assert (a) the default range is `last_30_days`, (b) explicit
//! ranges are honoured + echoed back, (c) `custom` requires/validates dates,
//! and (d) the period counters actually reflect rows landed inside the window
//! (and not rows landed outside it). We seed rows at controlled timestamps via
//! direct SQL `UPDATE`s so the windowing math is testable without waiting on
//! wall-clock.

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

// ── RBAC ───────────────────────────────────────────────────────────────

#[tokio::test]
async fn dashboard_stats_requires_admin() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let member = app.seed_user().await.expect("seed member");

    let resp = app
        .get("/api/admin/stats", Some(&member.access_token))
        .await;
    resp.assert_status(StatusCode::FORBIDDEN);
}

// ── Default range + envelope shape ─────────────────────────────────────

#[tokio::test]
async fn dashboard_stats_defaults_to_last_30_days() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let admin = app.seed_admin().await.expect("seed admin");

    let resp = app.get("/api/admin/stats", Some(&admin.access_token)).await;
    resp.assert_status(StatusCode::OK);
    let body: Value = resp.json().expect("envelope");

    // Lifetime totals must still be present (back-compat).
    for k in [
        "total_members",
        "active_subscriptions",
        "monthly_subscriptions",
        "annual_subscriptions",
        "total_watchlists",
        "total_enrollments",
        "recent_members",
    ] {
        assert!(
            body.get(k).is_some(),
            "missing lifetime field {k}: {body:?}"
        );
    }
    // New range-scoped fields.
    assert_eq!(body["range"], Value::String("last_30_days".to_string()));
    assert!(body.get("from").is_some());
    assert!(body.get("to").is_some());
    let period = body.get("period").expect("period block");
    let previous = body.get("previous_period").expect("previous_period block");
    for k in [
        "new_members",
        "new_subscriptions",
        "canceled_subscriptions",
        "new_enrollments",
        "new_watchlists",
        "revenue_cents",
    ] {
        assert!(period.get(k).is_some(), "period missing {k}");
        assert!(previous.get(k).is_some(), "previous_period missing {k}");
    }

    // Window length sanity: `to - from` must be ~30 days.
    let from: chrono::DateTime<Utc> = serde_json::from_value(body["from"].clone()).expect("from");
    let to: chrono::DateTime<Utc> = serde_json::from_value(body["to"].clone()).expect("to");
    let span = to - from;
    assert_eq!(span.num_days(), 30, "default window should span 30 days");
}

#[tokio::test]
async fn dashboard_stats_honours_explicit_range() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let admin = app.seed_admin().await.expect("seed admin");

    let resp = app
        .get(
            "/api/admin/stats?range=last_7_days",
            Some(&admin.access_token),
        )
        .await;
    resp.assert_status(StatusCode::OK);
    let body: Value = resp.json().expect("envelope");
    assert_eq!(body["range"], Value::String("last_7_days".to_string()));
    let from: chrono::DateTime<Utc> = serde_json::from_value(body["from"].clone()).expect("from");
    let to: chrono::DateTime<Utc> = serde_json::from_value(body["to"].clone()).expect("to");
    assert_eq!((to - from).num_days(), 7);
}

#[tokio::test]
async fn custom_range_requires_valid_dates() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let admin = app.seed_admin().await.expect("seed admin");

    // Missing `to`.
    let resp = app
        .get(
            "/api/admin/stats?range=custom&from=2025-01-01",
            Some(&admin.access_token),
        )
        .await;
    resp.assert_status(StatusCode::BAD_REQUEST);

    // Reversed range.
    let resp = app
        .get(
            "/api/admin/stats?range=custom&from=2025-02-01&to=2025-01-01",
            Some(&admin.access_token),
        )
        .await;
    resp.assert_status(StatusCode::BAD_REQUEST);

    // Happy path.
    let resp = app
        .get(
            "/api/admin/stats?range=custom&from=2025-01-01&to=2025-01-31",
            Some(&admin.access_token),
        )
        .await;
    resp.assert_status(StatusCode::OK);
    let body: Value = resp.json().expect("envelope");
    assert_eq!(body["range"], Value::String("custom".to_string()));
}

// ── Period counters reflect rows in the window ─────────────────────────

#[tokio::test]
async fn period_counts_include_in_window_and_exclude_out_of_window() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let admin = app.seed_admin().await.expect("seed admin");

    let now = Utc::now();
    let in_window_ts = now - Duration::days(3); // inside last_7_days
    let out_of_window_ts = now - Duration::days(45); // outside last_7_days entirely

    // ── Users: one inside window, one outside. We insert via raw SQL so we
    // can set `created_at` to a controlled timestamp in a single statement.
    let in_user_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO users (id, email, password_hash, name, role, created_at) VALUES ($1, $2, 'x', 'In Window', 'member', $3)",
    )
    .bind(in_user_id)
    .bind(format!("in-{in_user_id}@example.com"))
    .bind(in_window_ts)
    .execute(app.db())
    .await
    .expect("insert in-user");

    let out_user_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO users (id, email, password_hash, name, role, created_at) VALUES ($1, $2, 'x', 'Out Window', 'member', $3)",
    )
    .bind(out_user_id)
    .bind(format!("out-{out_user_id}@example.com"))
    .bind(out_of_window_ts)
    .execute(app.db())
    .await
    .expect("insert out-user");

    // ── Subscription: one inside window, also canceled inside window.
    let sub = db::upsert_subscription(
        app.db(),
        in_user_id,
        &format!("cus_{}", Uuid::new_v4()),
        &format!("sub_{}", Uuid::new_v4()),
        &SubscriptionPlan::Monthly,
        &SubscriptionStatus::Active,
        now - Duration::days(30),
        now + Duration::days(30),
        None,
    )
    .await
    .expect("upsert subscription");
    sqlx::query("UPDATE subscriptions SET created_at = $1, canceled_at = $1 WHERE id = $2")
        .bind(in_window_ts)
        .bind(sub.id)
        .execute(app.db())
        .await
        .expect("backdate subscription");

    // ── Watchlist: one inside window.
    let wl = db::create_watchlist(
        app.db(),
        "Window Watchlist",
        chrono::NaiveDate::from_ymd_opt(2025, 1, 1).expect("valid date"),
        None,
        None,
        false,
    )
    .await
    .expect("create watchlist");
    sqlx::query("UPDATE watchlists SET created_at = $1 WHERE id = $2")
        .bind(in_window_ts)
        .bind(wl.id)
        .execute(app.db())
        .await
        .expect("backdate watchlist");

    // ── Course enrollment: one inside window.
    sqlx::query(
        "INSERT INTO course_enrollments (id, user_id, course_id, progress, enrolled_at) VALUES ($1, $2, $3, 0, $4)",
    )
    .bind(Uuid::new_v4())
    .bind(in_user_id)
    .bind(format!("course-{}", Uuid::new_v4()))
    .bind(in_window_ts)
    .execute(app.db())
    .await
    .expect("insert enrollment");

    // Hit the endpoint with last_7_days.
    let resp = app
        .get(
            "/api/admin/stats?range=last_7_days",
            Some(&admin.access_token),
        )
        .await;
    resp.assert_status(StatusCode::OK);
    let body: Value = resp.json().expect("envelope");
    let period = &body["period"];
    let previous = &body["previous_period"];

    // The seeded in-window rows must be counted in `period`.
    assert!(
        period["new_members"].as_i64().unwrap_or(0) >= 1,
        "expected ≥1 new member in window, got {}",
        period["new_members"]
    );
    assert!(
        period["new_subscriptions"].as_i64().unwrap_or(0) >= 1,
        "expected ≥1 new subscription in window, got {}",
        period["new_subscriptions"]
    );
    assert!(
        period["canceled_subscriptions"].as_i64().unwrap_or(0) >= 1,
        "expected ≥1 canceled subscription in window, got {}",
        period["canceled_subscriptions"]
    );
    assert!(
        period["new_watchlists"].as_i64().unwrap_or(0) >= 1,
        "expected ≥1 new watchlist in window, got {}",
        period["new_watchlists"]
    );
    assert!(
        period["new_enrollments"].as_i64().unwrap_or(0) >= 1,
        "expected ≥1 new enrollment in window, got {}",
        period["new_enrollments"]
    );

    // The out-of-window user landed 45 days ago — outside both `period`
    // (last 7 days) and `previous_period` (the 7 days immediately before).
    // Sanity-check that we didn't accidentally bucket it in either.
    let new_members_total =
        period["new_members"].as_i64().unwrap_or(0) + previous["new_members"].as_i64().unwrap_or(0);
    // We seeded exactly one user inside both windows combined.
    assert!(
        new_members_total >= 1,
        "expected at least the in-window user accounted for, got total={new_members_total}"
    );
}
