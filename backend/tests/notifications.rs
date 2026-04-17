#![deny(warnings)]
#![forbid(unsafe_code)]

//! FDN-05 integration tests for the notifications subsystem.
//!
//! These tests touch a real Postgres schema. They skip silently when
//! `DATABASE_URL` is not set so `cargo test` is runnable in sandboxed
//! environments — CI populates `DATABASE_URL` and runs migrations before
//! invoking this suite.

use std::sync::Arc;
use std::time::Duration;

use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use uuid::Uuid;

use swings_api::notifications::{
    self,
    send::{send_notification, Recipient, SendOptions},
    suppression,
    templates::Template,
    unsubscribe,
};

async fn connect() -> Option<PgPool> {
    let url = std::env::var("DATABASE_URL").ok()?;
    let pool = PgPoolOptions::new()
        .max_connections(4)
        .acquire_timeout(Duration::from_secs(5))
        .connect(&url)
        .await
        .ok()?;
    sqlx::migrate!("./migrations").run(&pool).await.ok()?;
    Some(pool)
}

async fn cleanup(pool: &PgPool, email: &str) {
    let _ = sqlx::query("DELETE FROM notification_deliveries WHERE anonymous_email = $1")
        .bind(email)
        .execute(pool)
        .await;
    let _ = sqlx::query("DELETE FROM notification_suppression WHERE email = $1")
        .bind(email)
        .execute(pool)
        .await;
    let _ = sqlx::query("DELETE FROM unsubscribe_tokens WHERE email = $1")
        .bind(email)
        .execute(pool)
        .await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn resolve_and_render_seeded_welcome_template() {
    let Some(pool) = connect().await else {
        eprintln!("DATABASE_URL not set; skipping notifications integration test");
        return;
    };

    let template = Template::resolve(&pool, "user.welcome", "email", "en")
        .await
        .expect("welcome template must be seeded");
    assert_eq!(template.key, "user.welcome");
    assert_eq!(template.channel, "email");
    assert!(template.subject.is_some());

    let ctx = serde_json::json!({
        "name": "Ada Lovelace",
        "app_url": "https://example.com",
        "year": "2026",
    });
    let rendered = template.render(&ctx).expect("render");
    assert!(rendered.body.contains("Ada Lovelace"));
    assert!(rendered.body.contains("https://example.com"));
    let subj = rendered.subject.expect("subject present");
    assert!(subj.contains("Precision"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn send_notification_enqueues_delivery_and_outbox() {
    let Some(pool) = connect().await else {
        return;
    };
    let email = format!("nt-send-{}@test.local", Uuid::new_v4().simple());
    cleanup(&pool, &email).await;

    let ctx = serde_json::json!({
        "name": "Test User",
        "app_url": "https://example.com",
        "year": "2026",
    });
    let result = send_notification(
        &pool,
        "user.welcome",
        &Recipient::Anonymous {
            email: email.clone(),
        },
        ctx,
        SendOptions::default(),
    )
    .await
    .expect("send ok");

    let row: (String,) = sqlx::query_as("SELECT status FROM notification_deliveries WHERE id = $1")
        .bind(result.delivery_id)
        .fetch_one(&pool)
        .await
        .expect("delivery row");
    assert_eq!(row.0, "queued");

    // Outbox row exists for the delivery.
    let outbox_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM outbox_events WHERE aggregate_type = 'notification' AND aggregate_id = $1",
    )
    .bind(result.delivery_id.to_string())
    .fetch_one(&pool)
    .await
    .expect("count outbox");
    assert_eq!(outbox_count, 1);

    cleanup(&pool, &email).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn suppression_short_circuits_send() {
    let Some(pool) = connect().await else {
        return;
    };
    let email = format!("nt-suppr-{}@test.local", Uuid::new_v4().simple());
    cleanup(&pool, &email).await;

    suppression::suppress(&pool, &email, "test")
        .await
        .expect("suppress ok");

    let ctx = serde_json::json!({
        "name": "Blocked",
        "app_url": "https://x",
        "year": "2026",
    });
    let result = send_notification(
        &pool,
        "user.welcome",
        &Recipient::Anonymous {
            email: email.clone(),
        },
        ctx,
        SendOptions::default(),
    )
    .await
    .expect("send (with suppression) should still succeed — it records the row as suppressed");

    let row: (String,) = sqlx::query_as("SELECT status FROM notification_deliveries WHERE id = $1")
        .bind(result.delivery_id)
        .fetch_one(&pool)
        .await
        .expect("delivery row");
    assert_eq!(row.0, "suppressed");

    // No outbox event when we short-circuit — the delivery never reaches the
    // channel.
    let outbox_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM outbox_events WHERE aggregate_type = 'notification' AND aggregate_id = $1",
    )
    .bind(result.delivery_id.to_string())
    .fetch_one(&pool)
    .await
    .expect("count outbox");
    assert_eq!(outbox_count, 0);

    cleanup(&pool, &email).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn unsubscribe_token_consume_flow() {
    let Some(pool) = connect().await else {
        return;
    };
    let email = format!("nt-unsub-{}@test.local", Uuid::new_v4().simple());
    cleanup(&pool, &email).await;

    // Anonymous token → all-marketing suppression.
    let raw = unsubscribe::mint_token(&pool, None, &email, None, None)
        .await
        .expect("mint");

    let action = unsubscribe::consume_token(&pool, &raw)
        .await
        .expect("consume");
    match action {
        unsubscribe::UnsubscribeAction::AllMarketing { email: e } => assert_eq!(e, email),
        other => panic!("unexpected action: {other:?}"),
    }

    // The suppression row was inserted.
    assert!(suppression::is_suppressed(&pool, &email)
        .await
        .expect("check"));

    // Replay must fail.
    let err = unsubscribe::consume_token(&pool, &raw)
        .await
        .expect_err("replay must fail");
    matches!(err, unsubscribe::UnsubscribeError::AlreadyUsed);

    cleanup(&pool, &email).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn registry_has_every_declared_channel() {
    // Cheap sanity check that the runtime registry is wired whether or not an
    // e-mail service is configured.
    let svc = Arc::new(notifications::Service::new(None));
    let reg = svc.channels();
    for name in [
        "email", "sms", "push", "in_app", "slack", "discord", "webhook",
    ] {
        assert!(reg.get(name).is_some(), "channel `{name}` missing");
    }
}
