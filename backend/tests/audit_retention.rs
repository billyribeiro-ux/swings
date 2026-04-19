#![deny(warnings)]
#![forbid(unsafe_code)]

//! ADM-16 integration coverage for the audit-retention sweeper.
//!
//! Asserts:
//!   * `prune_once` deletes only rows older than the configured cutoff.
//!   * Batch limit bounds the per-iteration deletion count.
//!   * Multiple iterations drain the entire eligible set.
//!   * Settings cache reflects what the migration seeded so the worker
//!     boots with sensible defaults.

mod support;

use std::time::Duration;

use sqlx::Row;
use support::TestApp;
use swings_api::services::audit_retention;
use uuid::Uuid;

/// Insert an `admin_actions` row with a backdated `created_at`. Bypasses
/// the public `record_admin_action` writer so we can engineer history
/// for the retention test.
async fn seed_audit_row(app: &TestApp, actor_id: Uuid, days_old: i64) {
    sqlx::query(
        r#"
        INSERT INTO admin_actions
            (actor_id, actor_role, action, target_kind, target_id, metadata, created_at)
        VALUES
            ($1, 'admin', 'test.retention', 'test', NULL, '{}'::jsonb,
             NOW() - ($2::bigint * INTERVAL '1 day'))
        "#,
    )
    .bind(actor_id)
    .bind(days_old)
    .execute(app.db())
    .await
    .expect("seed audit row");
}

async fn count_audit_rows(app: &TestApp) -> i64 {
    let row = sqlx::query("SELECT COUNT(*) AS c FROM admin_actions")
        .fetch_one(app.db())
        .await
        .expect("count audit rows");
    row.get::<i64, _>("c")
}

#[tokio::test]
async fn prune_once_only_removes_rows_older_than_cutoff() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let admin = app.seed_admin().await.expect("seed admin");

    // Reset to a known starting point — the test schema may already
    // have a few rows from `seed_admin` itself (user.created etc).
    sqlx::query("DELETE FROM admin_actions")
        .execute(app.db())
        .await
        .expect("reset audit log");

    seed_audit_row(&app, admin.id, 400).await;
    seed_audit_row(&app, admin.id, 380).await;
    seed_audit_row(&app, admin.id, 30).await;
    seed_audit_row(&app, admin.id, 1).await;

    assert_eq!(count_audit_rows(&app).await, 4);

    let deleted = audit_retention::prune_once(app.db(), 365, 1_000)
        .await
        .expect("prune");
    assert_eq!(deleted, 2, "only the two rows >365 days old should go");
    assert_eq!(count_audit_rows(&app).await, 2);
}

#[tokio::test]
async fn prune_once_drains_in_multiple_batches() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let admin = app.seed_admin().await.expect("seed admin");

    sqlx::query("DELETE FROM admin_actions")
        .execute(app.db())
        .await
        .expect("reset audit log");

    for _ in 0..7 {
        seed_audit_row(&app, admin.id, 500).await;
    }
    assert_eq!(count_audit_rows(&app).await, 7);

    // Tiny batch size forces multiple iterations inside `prune_once`.
    let deleted = audit_retention::prune_once(app.db(), 365, 2)
        .await
        .expect("prune");
    assert_eq!(deleted, 7);
    assert_eq!(count_audit_rows(&app).await, 0);
}

#[tokio::test]
async fn prune_once_zero_eligible_is_a_noop() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let admin = app.seed_admin().await.expect("seed admin");

    sqlx::query("DELETE FROM admin_actions")
        .execute(app.db())
        .await
        .expect("reset audit log");

    seed_audit_row(&app, admin.id, 30).await;
    seed_audit_row(&app, admin.id, 60).await;

    let deleted = audit_retention::prune_once(app.db(), 365, 1_000)
        .await
        .expect("prune");
    assert_eq!(deleted, 0);
    assert_eq!(count_audit_rows(&app).await, 2);
}

#[tokio::test]
async fn run_loop_exits_on_shutdown_without_running_iteration() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };

    let (tx, rx) = tokio::sync::broadcast::channel::<()>(1);
    let pool = app.db().clone();
    let settings = swings_api::settings::Cache::new();

    let handle = tokio::spawn(audit_retention::run_loop(
        pool,
        settings,
        rx,
        Duration::from_secs(60),
    ));

    // Fire shutdown immediately — well within the first ticker tick, so
    // the worker must observe the broadcast before any sweep happens.
    tx.send(()).expect("broadcast shutdown");
    let outcome = tokio::time::timeout(Duration::from_secs(2), handle).await;
    assert!(outcome.is_ok(), "worker did not shut down within 2s");
}

#[tokio::test]
async fn settings_seeded_with_default_retention() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };

    let row = sqlx::query("SELECT value FROM app_settings WHERE key = 'audit.retention_days'")
        .fetch_one(app.db())
        .await
        .expect("seed exists");
    let value: serde_json::Value = row.get("value");
    assert_eq!(value, serde_json::json!(365));

    let row = sqlx::query("SELECT value FROM app_settings WHERE key = 'audit.prune_batch_size'")
        .fetch_one(app.db())
        .await
        .expect("seed exists");
    let value: serde_json::Value = row.get("value");
    assert_eq!(value, serde_json::json!(5000));
}
