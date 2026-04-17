#![deny(warnings)]
#![forbid(unsafe_code)]

//! FDN-09 integration tests for the Resend delivery-status webhook.
//!
//! Gated on `DATABASE_URL` just like the other Postgres-backed tests — the
//! suite skips gracefully in sandboxed sandboxes where no DB is reachable.

use std::time::Duration;

use hmac::{Hmac, Mac};
use sha2::Sha256;
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use swings_api::notifications::webhooks::resend::{process_event, ResendWebhookEnvelope};
use uuid::Uuid;

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

async fn seed_delivery(pool: &PgPool, provider_id: &str, recipient: &str) -> Uuid {
    let row: (Uuid,) = sqlx::query_as(
        r#"
        INSERT INTO notification_deliveries
            (anonymous_email, template_key, channel, provider_id, status, subject, rendered_body)
        VALUES ($1, 'user.welcome', 'email', $2, 'sent', 'Hi', '<p>hi</p>')
        RETURNING id
        "#,
    )
    .bind(recipient)
    .bind(provider_id)
    .fetch_one(pool)
    .await
    .expect("seed");
    row.0
}

async fn cleanup(pool: &PgPool, email: &str, provider_id: &str, event_id: &str) {
    let _ = sqlx::query("DELETE FROM notification_deliveries WHERE provider_id = $1")
        .bind(provider_id)
        .execute(pool)
        .await;
    let _ = sqlx::query("DELETE FROM notification_suppression WHERE email = $1")
        .bind(email)
        .execute(pool)
        .await;
    let _ = sqlx::query(
        "DELETE FROM processed_webhook_events WHERE source = 'resend' AND event_id = $1",
    )
    .bind(event_id)
    .execute(pool)
    .await;
}

fn envelope(
    event_type: &str,
    event_id: &str,
    provider_id: &str,
    to: &str,
) -> ResendWebhookEnvelope {
    let raw = serde_json::json!({
        "type": event_type,
        "id": event_id,
        "created_at": "2026-04-17T12:00:00Z",
        "data": {
            "email_id": provider_id,
            "to": [to],
        }
    });
    serde_json::from_value(raw).expect("envelope")
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn delivered_event_flips_status() {
    let Some(pool) = connect().await else {
        return;
    };
    let provider_id = format!("re_test_{}", Uuid::new_v4().simple());
    let email = format!("resend-wh-{}@test.local", Uuid::new_v4().simple());
    let event_id = format!("evt_{}", Uuid::new_v4().simple());
    cleanup(&pool, &email, &provider_id, &event_id).await;

    let delivery_id = seed_delivery(&pool, &provider_id, &email).await;
    let env = envelope("email.delivered", &event_id, &provider_id, &email);

    let outcome = process_event(&pool, &env).await.expect("process ok");
    assert!(matches!(
        outcome,
        swings_api::notifications::webhooks::resend::WebhookOutcome::Updated
    ));

    let status: (String,) =
        sqlx::query_as("SELECT status FROM notification_deliveries WHERE id = $1")
            .bind(delivery_id)
            .fetch_one(&pool)
            .await
            .expect("row");
    assert_eq!(status.0, "delivered");

    // Replay the same event — should be deduped.
    let outcome2 = process_event(&pool, &env).await.expect("process dup ok");
    assert!(matches!(
        outcome2,
        swings_api::notifications::webhooks::resend::WebhookOutcome::Duplicate
    ));

    cleanup(&pool, &email, &provider_id, &event_id).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn hard_bounce_suppresses_recipient() {
    let Some(pool) = connect().await else {
        return;
    };
    let provider_id = format!("re_test_{}", Uuid::new_v4().simple());
    let email = format!("resend-bounce-{}@test.local", Uuid::new_v4().simple());
    let event_id = format!("evt_{}", Uuid::new_v4().simple());
    cleanup(&pool, &email, &provider_id, &event_id).await;

    let _delivery_id = seed_delivery(&pool, &provider_id, &email).await;
    let raw = serde_json::json!({
        "type": "email.bounced",
        "id": &event_id,
        "data": {
            "email_id": &provider_id,
            "to": [&email],
            "bounce_type": "hard"
        }
    });
    let env: ResendWebhookEnvelope = serde_json::from_value(raw).expect("envelope");

    process_event(&pool, &env).await.expect("process ok");

    let suppressed = swings_api::notifications::suppression::is_suppressed(&pool, &email)
        .await
        .expect("check");
    assert!(suppressed, "hard bounce must insert a suppression row");

    cleanup(&pool, &email, &provider_id, &event_id).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn complaint_suppresses_recipient() {
    let Some(pool) = connect().await else {
        return;
    };
    let provider_id = format!("re_test_{}", Uuid::new_v4().simple());
    let email = format!("resend-complaint-{}@test.local", Uuid::new_v4().simple());
    let event_id = format!("evt_{}", Uuid::new_v4().simple());
    cleanup(&pool, &email, &provider_id, &event_id).await;

    let _delivery_id = seed_delivery(&pool, &provider_id, &email).await;
    let env = envelope("email.complained", &event_id, &provider_id, &email);

    process_event(&pool, &env).await.expect("process ok");

    let suppressed = swings_api::notifications::suppression::is_suppressed(&pool, &email)
        .await
        .expect("check");
    assert!(suppressed, "complaint must suppress");

    cleanup(&pool, &email, &provider_id, &event_id).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn clicked_does_not_downgrade_to_opened() {
    let Some(pool) = connect().await else {
        return;
    };
    let provider_id = format!("re_test_{}", Uuid::new_v4().simple());
    let email = format!("resend-click-{}@test.local", Uuid::new_v4().simple());
    let event_click = format!("evt_click_{}", Uuid::new_v4().simple());
    let event_open = format!("evt_open_{}", Uuid::new_v4().simple());
    cleanup(&pool, &email, &provider_id, &event_click).await;
    cleanup(&pool, &email, &provider_id, &event_open).await;

    let delivery_id = seed_delivery(&pool, &provider_id, &email).await;

    // Out-of-order delivery: the click event lands first, then the (stale)
    // open event. The second call must NOT downgrade the row to `opened`.
    let click = envelope("email.clicked", &event_click, &provider_id, &email);
    process_event(&pool, &click).await.expect("click ok");
    let open = envelope("email.opened", &event_open, &provider_id, &email);
    process_event(&pool, &open).await.expect("open ok");

    let status: (String,) =
        sqlx::query_as("SELECT status FROM notification_deliveries WHERE id = $1")
            .bind(delivery_id)
            .fetch_one(&pool)
            .await
            .expect("row");
    assert_eq!(status.0, "clicked", "opened must not downgrade clicked");

    cleanup(&pool, &email, &provider_id, &event_click).await;
    cleanup(&pool, &email, &provider_id, &event_open).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn unknown_provider_id_is_noop() {
    let Some(pool) = connect().await else {
        return;
    };
    let event_id = format!("evt_{}", Uuid::new_v4().simple());
    let env = envelope(
        "email.delivered",
        &event_id,
        "re_unknown_id",
        "nowhere@test.local",
    );

    let outcome = process_event(&pool, &env).await.expect("process ok");
    assert!(matches!(
        outcome,
        swings_api::notifications::webhooks::resend::WebhookOutcome::UnknownDelivery
    ));

    let _ = sqlx::query(
        "DELETE FROM processed_webhook_events WHERE source = 'resend' AND event_id = $1",
    )
    .bind(&event_id)
    .execute(&pool)
    .await;
}

fn sign_svix(body: &[u8], secret: &[u8], timestamp: i64, svix_id: &str) -> String {
    let prefix = if svix_id.is_empty() {
        format!("{timestamp}.")
    } else {
        format!("{svix_id}.{timestamp}.")
    };
    let mut mac = Hmac::<Sha256>::new_from_slice(secret).expect("hmac");
    mac.update(prefix.as_bytes());
    mac.update(body);
    let digest = mac.finalize().into_bytes();
    let hex: String = digest.iter().map(|b| format!("{b:02x}")).collect();
    format!("v1,{hex}")
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn signature_round_trip_accepts_hex_format() {
    use swings_api::notifications::webhooks::resend::verify_signature;

    let secret = b"whsec_test";
    let body = br#"{"type":"email.delivered","id":"evt_1","data":{"email_id":"x","to":["a@b"]}}"#;
    let ts = chrono::Utc::now().timestamp();
    let svix_id = "msg_01";
    let sig = sign_svix(body, secret, ts, svix_id);
    assert!(verify_signature(body, secret, svix_id, ts, &sig));
}
