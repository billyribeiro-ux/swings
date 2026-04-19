#![deny(warnings)]
#![forbid(unsafe_code)]

//! ADM-20 integration coverage for the idempotency-keys GC worker.
//!
//! Properties asserted:
//!   * `prune_once` deletes expired rows and leaves un-expired rows
//!     intact.
//!   * Empty table is a no-op (zero failures, zero deletions).
//!   * Operator-tunable `idempotency.gc_batch_size` setting is
//!     honoured between iterations.
//!   * `run_loop` exits cleanly on shutdown.
//!
//! The cache table is exercised directly via SQL (rather than driving
//! the middleware) so each test owns its own deterministic seed.

mod support;

use std::time::Duration;

use chrono::Utc;
use sqlx::PgPool;
use support::TestApp;
use tokio::sync::broadcast;
use uuid::Uuid;

async fn seed_row(pool: &PgPool, user_id: Uuid, key: &str, expires_at: chrono::DateTime<Utc>) {
    sqlx::query(
        r#"
        INSERT INTO idempotency_keys
            (user_id, key, method, path, request_hash, status_code,
             response_body, response_headers, in_flight, completed_at,
             expires_at)
        VALUES ($1, $2, 'POST', '/api/admin/orders',
                decode('00', 'hex'), 200, decode('00', 'hex'),
                '{}'::jsonb, FALSE, NOW(), $3)
        "#,
    )
    .bind(user_id)
    .bind(key)
    .bind(expires_at)
    .execute(pool)
    .await
    .expect("seed");
}

async fn count_keys(pool: &PgPool) -> i64 {
    sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM idempotency_keys")
        .fetch_one(pool)
        .await
        .expect("count")
}

#[tokio::test]
async fn prune_once_removes_only_expired_rows() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    sqlx::query("DELETE FROM idempotency_keys")
        .execute(app.db())
        .await
        .expect("clean");

    let admin = app.seed_admin().await.expect("admin");
    seed_row(
        app.db(),
        admin.id,
        "fresh",
        Utc::now() + chrono::Duration::hours(2),
    )
    .await;
    seed_row(
        app.db(),
        admin.id,
        "stale-1",
        Utc::now() - chrono::Duration::hours(1),
    )
    .await;
    seed_row(
        app.db(),
        admin.id,
        "stale-2",
        Utc::now() - chrono::Duration::days(2),
    )
    .await;

    assert_eq!(count_keys(app.db()).await, 3);

    let removed = swings_api::services::idempotency_gc::prune_once(app.db(), app.settings()).await;
    assert_eq!(removed, 2, "two stale rows should have been pruned");
    assert_eq!(count_keys(app.db()).await, 1);

    let surviving: String = sqlx::query_scalar("SELECT key FROM idempotency_keys LIMIT 1")
        .fetch_one(app.db())
        .await
        .expect("row");
    assert_eq!(surviving, "fresh");
}

#[tokio::test]
async fn prune_once_handles_empty_table() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    sqlx::query("DELETE FROM idempotency_keys")
        .execute(app.db())
        .await
        .expect("clean");

    let removed = swings_api::services::idempotency_gc::prune_once(app.db(), app.settings()).await;
    assert_eq!(removed, 0);
}

#[tokio::test]
async fn prune_once_drains_backlog_in_multiple_passes() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    sqlx::query("DELETE FROM idempotency_keys")
        .execute(app.db())
        .await
        .expect("clean");
    let admin = app.seed_admin().await.expect("admin");

    // Force batch_size=2 so 5 expired rows require 3 passes — proves
    // the inner loop drains the backlog rather than stopping after one.
    app.settings()
        .insert_for_tests("idempotency.gc_batch_size", serde_json::json!(2));

    for i in 0..5 {
        seed_row(
            app.db(),
            admin.id,
            &format!("stale-{i}"),
            Utc::now() - chrono::Duration::hours(1),
        )
        .await;
    }
    assert_eq!(count_keys(app.db()).await, 5);

    let removed = swings_api::services::idempotency_gc::prune_once(app.db(), app.settings()).await;
    assert_eq!(removed, 5);
    assert_eq!(count_keys(app.db()).await, 0);
}

#[tokio::test]
async fn prune_pass_respects_explicit_batch_size_cap() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    sqlx::query("DELETE FROM idempotency_keys")
        .execute(app.db())
        .await
        .expect("clean");
    let admin = app.seed_admin().await.expect("admin");

    for i in 0..7 {
        seed_row(
            app.db(),
            admin.id,
            &format!("stale-{i}"),
            Utc::now() - chrono::Duration::hours(1),
        )
        .await;
    }

    // Single pass with batch_size=3 should only take three rows,
    // leaving four behind for subsequent passes.
    let removed = swings_api::services::idempotency_gc::prune_pass(app.db(), 3)
        .await
        .expect("pass");
    assert_eq!(removed, 3);
    assert_eq!(count_keys(app.db()).await, 4);
}

#[tokio::test]
async fn run_loop_exits_on_shutdown() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let (tx, rx) = broadcast::channel::<()>(1);
    let handle = tokio::spawn(swings_api::services::idempotency_gc::run_loop(
        app.db().clone(),
        app.settings().clone(),
        rx,
        Duration::from_secs(60),
    ));
    tokio::time::sleep(Duration::from_millis(50)).await;
    tx.send(()).expect("send shutdown");
    tokio::time::timeout(Duration::from_secs(2), handle)
        .await
        .expect("worker should exit within 2s of shutdown")
        .expect("join");
}
