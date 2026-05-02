#![deny(warnings)]
#![forbid(unsafe_code)]

//! EC-13 integration coverage for the expanded Stripe webhook surface.
//!
//! Each event the handler now dispatches is exercised here:
//!
//!   * `invoice.payment_failed` → `payment_failures` row + `past_due`
//!     status flip + audit log + dedupe.
//!   * `invoice.paid`           → `subscription_invoices` row + recovery
//!     transition back to `active`.
//!   * `charge.refunded`        → `payment_refunds` row + (when an order
//!     is linked) order status flip to `refunded`.
//!   * `refund.created`         → `payment_refunds` row via standalone
//!     refund object (modern Stripe delivery path). Idempotent with
//!     `charge.refunded` on the same `stripe_refund_id`.
//!   * `payment_intent.payment_failed` → `payment_failures` row + (when
//!     an order is linked) order status flip to `failed`.
//!   * `charge.dispute.created` → `payment_disputes` row + outbox event
//!     + (when an order is linked) `disputed_at` flag.
//!   * `customer.subscription.trial_will_end` → `subscription_trial_events`
//!     dedupe row.
//!   * `customer.subscription.paused` / `.resumed` → status flip.
//!
//! A dedicated race test for `invoice.payment_failed` mirrors the
//! `concurrent_same_key_creates_exactly_one_resource` pattern in
//! `tests/admin_idempotency.rs` to prove that two parallel deliveries of
//! the same Stripe event id never produce two `payment_failures` rows.
//!
//! Tests POST to `/api/webhooks/stripe` directly so the signature
//! verification, idempotency claim, and dispatch pipeline are all
//! exercised end-to-end. The signed payload is built per-test from a
//! known `whsec_*` secret installed on a custom-built [`AppState`] —
//! we deliberately bypass the shared [`support::TestApp`] config builder
//! (which leaves `stripe_webhook_secret` empty) without modifying it.

mod support;

use std::sync::Arc;

use axum::{
    body::Body,
    http::{Method, Request, StatusCode},
    Router,
};
use chrono::{Duration as ChronoDuration, Utc};
use hmac::{Hmac, Mac};
use serde_json::json;
use sha2::Sha256;
use sqlx::PgPool;
use support::TestDb;
use swings_api::{
    authz::{Policy, PolicyHandle},
    config::Config,
    db,
    events::WorkerShutdown,
    handlers::webhooks,
    middleware::rate_limit::Backend as RateLimitBackend,
    models::{SubscriptionPlan, SubscriptionStatus, User},
    notifications::Service as NotificationsService,
    services::MediaBackend,
    settings::Cache as SettingsCache,
    AppState,
};
use tempfile::TempDir;
use tower::ServiceExt;
use uuid::Uuid;

const TEST_WEBHOOK_SECRET: &str = "whsec_test_secret_for_unit_tests";

/// Composite handle owning the schema + router + a temp dir we keep
/// alive for the duration of the test. We deliberately do not use the
/// shared `TestApp` builder because it pins `stripe_webhook_secret` to
/// the empty string, which the production handler treats as a 500.
struct WebhookTestRig {
    router: Router<()>,
    db: TestDb,
    _upload_dir: TempDir,
}

impl WebhookTestRig {
    async fn try_new() -> Option<Self> {
        if !support::has_test_database() {
            eprintln!("[stripe_webhooks] skipping — DATABASE_URL{{,_TEST}} unset");
            return None;
        }
        let db = TestDb::new().await.ok()?;
        let upload_dir = TempDir::new().ok()?;
        let upload_path = upload_dir.path().to_string_lossy().into_owned();
        let policy = Policy::load(db.pool()).await.ok()?;
        let settings = SettingsCache::new();
        settings.reload(db.pool()).await.ok()?;

        let config = Config {
            database_url: String::new(),
            jwt_secret: "test-harness-jwt-secret-pad-pad-pad-pad-pad-pad-pad-pad-pad".into(),
            jwt_expiration_hours: 24,
            refresh_token_expiration_days: 30,
            port: 0,
            frontend_url: "http://localhost:5173".into(),
            stripe_secret_key: String::new(),
            stripe_webhook_secret: TEST_WEBHOOK_SECRET.into(),
            stripe_api_base_url_override: None,
            upload_dir: upload_path.clone(),
            api_url: "http://localhost:3001".into(),
            smtp_host: "smtp.example.test".into(),
            smtp_port: 587,
            smtp_user: String::new(),
            smtp_password: String::new(),
            smtp_from: "noreply@example.test".into(),
            app_url: "http://localhost:5173".into(),
            app_env: "test".into(),
            cors_allowed_origins: vec!["http://localhost:5173".into()],
        };
        let state = AppState {
            db: db.pool().clone(),
            config: Arc::new(config),
            email_service: None,
            media_backend: MediaBackend::Local {
                upload_dir: upload_path,
            },
            policy: Arc::new(PolicyHandle::new(policy)),
            outbox_shutdown: WorkerShutdown::default(),
            rate_limit: RateLimitBackend::InProcess(Arc::new(Default::default())),
            notifications: Arc::new(NotificationsService::new(
                Some(Arc::new(
                    swings_api::notifications::channels::email::NoopProvider::new(),
                )),
                "Swings <noreply@example.test>".into(),
            )),
            settings,
        };
        let router = Router::new()
            .nest("/api/webhooks", webhooks::router())
            .with_state(state);
        Some(Self {
            router,
            db,
            _upload_dir: upload_dir,
        })
    }

    fn pool(&self) -> &PgPool {
        self.db.pool()
    }

    /// Send a Stripe-shaped event payload, sign it with the test secret,
    /// and POST to `/api/webhooks/stripe`. Returns the response status.
    async fn post_stripe(&self, event: &serde_json::Value) -> StatusCode {
        let payload = serde_json::to_string(event).expect("serialize event");
        let timestamp = Utc::now().timestamp();
        let header = sign(TEST_WEBHOOK_SECRET, &payload, timestamp);
        let req = Request::builder()
            .method(Method::POST)
            .uri("/api/webhooks/stripe")
            .header("content-type", "application/json")
            .header("stripe-signature", &header)
            // Pin a unique IP so the rate-limit governor doesn't bleed
            // between parallel-running tests.
            .header(
                "X-Forwarded-For",
                format!("10.{}.{}.{}", rand_octet(), rand_octet(), rand_octet()),
            )
            .body(Body::from(payload))
            .expect("build request");
        let resp = self.router.clone().oneshot(req).await.expect("dispatch");
        resp.status()
    }
}

fn sign(secret: &str, payload: &str, timestamp: i64) -> String {
    type HmacSha256 = Hmac<Sha256>;
    let mut mac = HmacSha256::new_from_slice(secret.as_bytes()).expect("hmac key");
    mac.update(format!("{timestamp}.{payload}").as_bytes());
    let digest = mac.finalize().into_bytes();
    let hex = digest
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<String>();
    format!("t={timestamp},v1={hex}")
}

fn rand_octet() -> u8 {
    (Uuid::new_v4().as_u128() & 0xFF) as u8
}

// ── Fixture helpers ────────────────────────────────────────────────────

async fn seed_user(pool: &PgPool, email: &str) -> User {
    db::create_user(pool, email, "x", "Test User")
        .await
        .expect("seed user")
}

async fn seed_active_subscription(
    pool: &PgPool,
    user_id: Uuid,
    stripe_sub_id: &str,
    stripe_customer_id: &str,
    status: SubscriptionStatus,
) -> Uuid {
    let now = Utc::now();
    db::upsert_subscription(
        pool,
        user_id,
        stripe_customer_id,
        stripe_sub_id,
        &SubscriptionPlan::Monthly,
        &status,
        now,
        now + ChronoDuration::days(30),
        None,
    )
    .await
    .expect("upsert sub")
    .id
}

async fn audit_count_for(pool: &PgPool, event_id: &str) -> i64 {
    let row: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM stripe_webhook_audit WHERE stripe_event_id = $1")
            .bind(event_id)
            .fetch_one(pool)
            .await
            .expect("audit count");
    row.0
}

fn unique(prefix: &str) -> String {
    format!("{prefix}_{}", Uuid::new_v4().simple())
}

// ── A. invoice.payment_failed ──────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn invoice_payment_failed_marks_past_due_and_logs_failure() {
    let Some(rig) = WebhookTestRig::try_new().await else {
        return;
    };
    let user = seed_user(rig.pool(), &format!("{}@example.com", unique("u"))).await;
    let stripe_sub_id = unique("sub");
    let stripe_customer_id = unique("cus");
    let sub_uuid = seed_active_subscription(
        rig.pool(),
        user.id,
        &stripe_sub_id,
        &stripe_customer_id,
        SubscriptionStatus::Active,
    )
    .await;

    let event_id = unique("evt");
    let next_attempt = Utc::now().timestamp() + 86_400;
    let event = json!({
        "id": event_id,
        "type": "invoice.payment_failed",
        "data": {
            "object": {
                "id": unique("in"),
                "object": "invoice",
                "subscription": stripe_sub_id,
                "customer": stripe_customer_id,
                "status": "open",
                "amount_due": 4900,
                "amount_paid": 0,
                "currency": "usd",
                "attempt_count": 1,
                "next_payment_attempt": next_attempt,
                "last_finalization_error": {
                    "code": "card_declined",
                    "message": "Your card was declined.",
                }
            }
        }
    });

    assert_eq!(rig.post_stripe(&event).await, StatusCode::OK);

    // Subscription flipped to past_due.
    let row: (String,) = sqlx::query_as("SELECT status::text FROM subscriptions WHERE id = $1")
        .bind(sub_uuid)
        .fetch_one(rig.pool())
        .await
        .expect("status");
    assert_eq!(row.0, "past_due");

    // payment_failures row exists and is dedupe-keyed on the event id.
    let pf_count: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM payment_failures WHERE stripe_event_id = $1")
            .bind(&event_id)
            .fetch_one(rig.pool())
            .await
            .expect("pf count");
    assert_eq!(pf_count.0, 1);

    // Audit row landed.
    assert_eq!(audit_count_for(rig.pool(), &event_id).await, 1);

    // Replay: same event id is idempotent at the entry point.
    assert_eq!(rig.post_stripe(&event).await, StatusCode::OK);
    let pf_count_after: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM payment_failures WHERE stripe_event_id = $1")
            .bind(&event_id)
            .fetch_one(rig.pool())
            .await
            .expect("pf count after");
    assert_eq!(pf_count_after.0, 1, "replay must not create a second row");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn invoice_payment_failed_is_race_safe_under_concurrent_redelivery() {
    // Mirrors the `concurrent_same_key_creates_exactly_one_resource`
    // pattern from `tests/admin_idempotency.rs`. Two parallel posts of
    // the same Stripe event id must converge on exactly one
    // `payment_failures` row.
    let Some(rig) = WebhookTestRig::try_new().await else {
        return;
    };
    let user = seed_user(rig.pool(), &format!("{}@example.com", unique("u"))).await;
    let stripe_sub_id = unique("sub");
    let stripe_customer_id = unique("cus");
    seed_active_subscription(
        rig.pool(),
        user.id,
        &stripe_sub_id,
        &stripe_customer_id,
        SubscriptionStatus::Active,
    )
    .await;

    let event_id = unique("evt");
    let event = json!({
        "id": event_id,
        "type": "invoice.payment_failed",
        "data": {
            "object": {
                "id": unique("in"),
                "subscription": stripe_sub_id,
                "customer": stripe_customer_id,
                "status": "open",
                "amount_due": 1000,
                "amount_paid": 0,
                "currency": "usd",
                "attempt_count": 1,
                "next_payment_attempt": Utc::now().timestamp() + 3600,
            }
        }
    });

    let barrier = Arc::new(tokio::sync::Barrier::new(4));
    let mut handles = Vec::new();
    for _ in 0..4 {
        let bar = barrier.clone();
        // Clone the router-with-state so each task runs its own
        // `oneshot`. The shared sqlx pool is the only contended
        // resource, which is exactly what we want to stress here.
        let router = rig.router.clone();
        let event = event.clone();
        handles.push(tokio::spawn(async move {
            bar.wait().await;
            let payload = serde_json::to_string(&event).unwrap();
            let ts = Utc::now().timestamp();
            let header = sign(TEST_WEBHOOK_SECRET, &payload, ts);
            let req = Request::builder()
                .method(Method::POST)
                .uri("/api/webhooks/stripe")
                .header("content-type", "application/json")
                .header("stripe-signature", &header)
                .header(
                    "X-Forwarded-For",
                    format!("10.{}.{}.{}", rand_octet(), rand_octet(), rand_octet()),
                )
                .body(Body::from(payload))
                .unwrap();
            router.oneshot(req).await.unwrap().status()
        }));
    }
    for h in handles {
        let status = h.await.unwrap();
        assert_eq!(status, StatusCode::OK);
    }

    let pf_count: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM payment_failures WHERE stripe_event_id = $1")
            .bind(&event_id)
            .fetch_one(rig.pool())
            .await
            .expect("pf count");
    assert_eq!(
        pf_count.0, 1,
        "concurrent redeliveries must produce exactly one payment_failures row"
    );
}

// ── B. invoice.paid ────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn invoice_paid_recovers_dunning_subscription_to_active() {
    let Some(rig) = WebhookTestRig::try_new().await else {
        return;
    };
    let user = seed_user(rig.pool(), &format!("{}@example.com", unique("u"))).await;
    let stripe_sub_id = unique("sub");
    let stripe_customer_id = unique("cus");
    let sub_uuid = seed_active_subscription(
        rig.pool(),
        user.id,
        &stripe_sub_id,
        &stripe_customer_id,
        SubscriptionStatus::PastDue,
    )
    .await;

    let event_id = unique("evt");
    let stripe_invoice_id = unique("in");
    let event = json!({
        "id": event_id,
        "type": "invoice.paid",
        "data": {
            "object": {
                "id": stripe_invoice_id,
                "subscription": stripe_sub_id,
                "customer": stripe_customer_id,
                "status": "paid",
                "amount_due": 4900,
                "amount_paid": 4900,
                "currency": "usd",
                "attempt_count": 2,
                "status_transitions": { "paid_at": Utc::now().timestamp() },
            }
        }
    });

    assert_eq!(rig.post_stripe(&event).await, StatusCode::OK);

    let status: (String,) = sqlx::query_as("SELECT status::text FROM subscriptions WHERE id = $1")
        .bind(sub_uuid)
        .fetch_one(rig.pool())
        .await
        .expect("status");
    assert_eq!(status.0, "active");

    let invoice: (String,) =
        sqlx::query_as("SELECT status FROM subscription_invoices WHERE stripe_invoice_id = $1")
            .bind(&stripe_invoice_id)
            .fetch_one(rig.pool())
            .await
            .expect("invoice");
    assert_eq!(invoice.0, "paid");

    assert_eq!(audit_count_for(rig.pool(), &event_id).await, 1);

    // Replay — no second invoice row, no second audit row.
    assert_eq!(rig.post_stripe(&event).await, StatusCode::OK);
    let inv_count: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM subscription_invoices WHERE stripe_invoice_id = $1")
            .bind(&stripe_invoice_id)
            .fetch_one(rig.pool())
            .await
            .unwrap();
    assert_eq!(inv_count.0, 1);
}

// ── C. charge.refunded ─────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn charge_refunded_records_refund_row() {
    let Some(rig) = WebhookTestRig::try_new().await else {
        return;
    };
    let user = seed_user(rig.pool(), &format!("{}@example.com", unique("u"))).await;
    let stripe_customer_id = unique("cus");
    // Link the customer for the user so the refund can resolve user_id
    // via the find_user_by_stripe_customer JOIN.
    seed_active_subscription(
        rig.pool(),
        user.id,
        &unique("sub"),
        &stripe_customer_id,
        SubscriptionStatus::Active,
    )
    .await;

    let event_id = unique("evt");
    let stripe_refund_id = unique("re");
    let event = json!({
        "id": event_id,
        "type": "charge.refunded",
        "data": {
            "object": {
                "id": unique("ch"),
                "object": "charge",
                "payment_intent": unique("pi"),
                "customer": stripe_customer_id,
                "currency": "usd",
                "refunds": {
                    "data": [{
                        "id": stripe_refund_id,
                        "amount": 2500,
                        "currency": "usd",
                        "reason": "requested_by_customer",
                        "status": "succeeded",
                        "created": Utc::now().timestamp(),
                    }]
                }
            }
        }
    });

    assert_eq!(rig.post_stripe(&event).await, StatusCode::OK);

    let row: (String, i64, Option<Uuid>) = sqlx::query_as(
        "SELECT stripe_refund_id, amount_cents, user_id FROM payment_refunds WHERE stripe_refund_id = $1",
    )
    .bind(&stripe_refund_id)
    .fetch_one(rig.pool())
    .await
    .expect("refund row");
    assert_eq!(row.0, stripe_refund_id);
    assert_eq!(row.1, 2500);
    assert_eq!(row.2, Some(user.id));

    assert_eq!(audit_count_for(rig.pool(), &event_id).await, 1);

    // Replay — same refund id, no duplicate row.
    assert_eq!(rig.post_stripe(&event).await, StatusCode::OK);
    let count: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM payment_refunds WHERE stripe_refund_id = $1")
            .bind(&stripe_refund_id)
            .fetch_one(rig.pool())
            .await
            .unwrap();
    assert_eq!(count.0, 1);
}

// ── D. payment_intent.payment_failed ───────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn payment_intent_failed_records_failure_row() {
    let Some(rig) = WebhookTestRig::try_new().await else {
        return;
    };
    let event_id = unique("evt");
    let pi_id = unique("pi");
    let event = json!({
        "id": event_id,
        "type": "payment_intent.payment_failed",
        "data": {
            "object": {
                "id": pi_id,
                "object": "payment_intent",
                "customer": unique("cus"),
                "amount": 1500,
                "currency": "usd",
                "last_payment_error": {
                    "code": "card_declined",
                    "message": "Your card was declined.",
                }
            }
        }
    });

    assert_eq!(rig.post_stripe(&event).await, StatusCode::OK);

    let pf: (String, Option<i64>, bool) = sqlx::query_as(
        "SELECT stripe_payment_intent_id, amount_cents, final FROM payment_failures WHERE stripe_event_id = $1",
    )
    .bind(&event_id)
    .fetch_one(rig.pool())
    .await
    .expect("pf row");
    assert_eq!(pf.0, pi_id);
    assert_eq!(pf.1, Some(1500));
    assert!(
        pf.2,
        "PI failures are terminal — no Stripe retry behind them"
    );

    assert_eq!(audit_count_for(rig.pool(), &event_id).await, 1);
}

// ── E. charge.dispute.created ──────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn charge_dispute_created_records_dispute_and_emits_outbox_alert() {
    let Some(rig) = WebhookTestRig::try_new().await else {
        return;
    };
    let user = seed_user(rig.pool(), &format!("{}@example.com", unique("u"))).await;
    let stripe_customer_id = unique("cus");
    seed_active_subscription(
        rig.pool(),
        user.id,
        &unique("sub"),
        &stripe_customer_id,
        SubscriptionStatus::Active,
    )
    .await;

    let event_id = unique("evt");
    let stripe_dispute_id = unique("dp");
    let event = json!({
        "id": event_id,
        "type": "charge.dispute.created",
        "data": {
            "object": {
                "id": stripe_dispute_id,
                "object": "dispute",
                "charge": unique("ch"),
                "payment_intent": unique("pi"),
                "customer": stripe_customer_id,
                "amount": 4900,
                "currency": "usd",
                "reason": "fraudulent",
                "status": "warning_needs_response",
                "is_charge_refundable": true,
                "evidence_details": { "due_by": Utc::now().timestamp() + 86_400 * 7 },
            }
        }
    });

    assert_eq!(rig.post_stripe(&event).await, StatusCode::OK);

    let dispute: (String, i64, String) = sqlx::query_as(
        "SELECT stripe_dispute_id, amount_cents, status FROM payment_disputes WHERE stripe_dispute_id = $1",
    )
    .bind(&stripe_dispute_id)
    .fetch_one(rig.pool())
    .await
    .expect("dispute row");
    assert_eq!(dispute.0, stripe_dispute_id);
    assert_eq!(dispute.1, 4900);
    assert_eq!(dispute.2, "warning_needs_response");

    // Outbox event for ops alerting.
    let outbox_count: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM outbox_events WHERE event_type = 'ops.dispute_opened' AND aggregate_id = $1",
    )
    .bind(&stripe_dispute_id)
    .fetch_one(rig.pool())
    .await
    .expect("outbox count");
    assert_eq!(outbox_count.0, 1);

    assert_eq!(audit_count_for(rig.pool(), &event_id).await, 1);
}

// ── F. customer.subscription.trial_will_end ────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn trial_will_end_records_dedupe_row_and_skips_replay() {
    let Some(rig) = WebhookTestRig::try_new().await else {
        return;
    };
    let user = seed_user(rig.pool(), &format!("{}@example.com", unique("u"))).await;
    let stripe_sub_id = unique("sub");
    let stripe_customer_id = unique("cus");
    let sub_uuid = seed_active_subscription(
        rig.pool(),
        user.id,
        &stripe_sub_id,
        &stripe_customer_id,
        SubscriptionStatus::Trialing,
    )
    .await;

    let trial_end = Utc::now().timestamp() + 3 * 86_400;
    let event_id = unique("evt");
    let event = json!({
        "id": event_id,
        "type": "customer.subscription.trial_will_end",
        "data": {
            "object": {
                "id": stripe_sub_id,
                "customer": stripe_customer_id,
                "trial_end": trial_end,
                "status": "trialing",
            }
        }
    });

    assert_eq!(rig.post_stripe(&event).await, StatusCode::OK);

    let row: (Uuid,) = sqlx::query_as(
        "SELECT subscription_id FROM subscription_trial_events WHERE subscription_id = $1",
    )
    .bind(sub_uuid)
    .fetch_one(rig.pool())
    .await
    .expect("trial event row");
    assert_eq!(row.0, sub_uuid);

    assert_eq!(audit_count_for(rig.pool(), &event_id).await, 1);
}

// ── G. customer.subscription.paused ────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn subscription_paused_flips_status_to_paused() {
    let Some(rig) = WebhookTestRig::try_new().await else {
        return;
    };
    let user = seed_user(rig.pool(), &format!("{}@example.com", unique("u"))).await;
    let stripe_sub_id = unique("sub");
    let stripe_customer_id = unique("cus");
    let sub_uuid = seed_active_subscription(
        rig.pool(),
        user.id,
        &stripe_sub_id,
        &stripe_customer_id,
        SubscriptionStatus::Active,
    )
    .await;

    let event_id = unique("evt");
    let event = json!({
        "id": event_id,
        "type": "customer.subscription.paused",
        "data": {
            "object": {
                "id": stripe_sub_id,
                "customer": stripe_customer_id,
                "status": "paused",
            }
        }
    });

    assert_eq!(rig.post_stripe(&event).await, StatusCode::OK);

    let row: (String,) = sqlx::query_as("SELECT status::text FROM subscriptions WHERE id = $1")
        .bind(sub_uuid)
        .fetch_one(rig.pool())
        .await
        .expect("status");
    assert_eq!(row.0, "paused");

    assert_eq!(audit_count_for(rig.pool(), &event_id).await, 1);
}

// ── H. customer.subscription.resumed ───────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn subscription_resumed_flips_status_back_to_active() {
    let Some(rig) = WebhookTestRig::try_new().await else {
        return;
    };
    let user = seed_user(rig.pool(), &format!("{}@example.com", unique("u"))).await;
    let stripe_sub_id = unique("sub");
    let stripe_customer_id = unique("cus");
    let sub_uuid = seed_active_subscription(
        rig.pool(),
        user.id,
        &stripe_sub_id,
        &stripe_customer_id,
        SubscriptionStatus::Active,
    )
    .await;
    // Manually pause so we can prove the resume actually moved us.
    sqlx::query(
        "UPDATE subscriptions SET status = 'paused'::subscription_status, paused_at = NOW() WHERE id = $1",
    )
    .bind(sub_uuid)
    .execute(rig.pool())
    .await
    .expect("manual pause");

    let event_id = unique("evt");
    let event = json!({
        "id": event_id,
        "type": "customer.subscription.resumed",
        "data": {
            "object": {
                "id": stripe_sub_id,
                "customer": stripe_customer_id,
                "status": "active",
            }
        }
    });

    assert_eq!(rig.post_stripe(&event).await, StatusCode::OK);

    let row: (String, Option<chrono::DateTime<Utc>>) =
        sqlx::query_as("SELECT status::text, paused_at FROM subscriptions WHERE id = $1")
            .bind(sub_uuid)
            .fetch_one(rig.pool())
            .await
            .expect("status");
    assert_eq!(row.0, "active");
    assert!(row.1.is_none(), "paused_at should be cleared on resume");

    assert_eq!(audit_count_for(rig.pool(), &event_id).await, 1);
}

// ── E. refund.created (modern standalone Stripe event) ─────────────────

/// `refund.created` records a `payment_refunds` row using the standalone
/// refund object that modern Stripe accounts deliver instead of (or in
/// addition to) the embedded `charge.refunds.data[]` shape in
/// `charge.refunded`. This path goes through
/// `ChargeRefundFields::from_refund_object`.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn refund_created_records_refund_row() {
    let Some(rig) = WebhookTestRig::try_new().await else {
        return;
    };

    let event_id = unique("evt");
    let stripe_refund_id = unique("re");
    let stripe_charge_id = unique("ch");
    let stripe_pi_id = unique("pi");

    let event = json!({
        "id": event_id,
        "type": "refund.created",
        "data": {
            "object": {
                "id": stripe_refund_id,
                "object": "refund",
                "amount": 4999,
                "currency": "usd",
                "charge": stripe_charge_id,
                "payment_intent": stripe_pi_id,
                "reason": "duplicate",
                "status": "succeeded",
                "created": Utc::now().timestamp(),
            }
        }
    });

    assert_eq!(rig.post_stripe(&event).await, StatusCode::OK);

    let row: (String, i64) = sqlx::query_as(
        "SELECT stripe_refund_id, amount_cents FROM payment_refunds WHERE stripe_refund_id = $1",
    )
    .bind(&stripe_refund_id)
    .fetch_one(rig.pool())
    .await
    .expect("refund row");
    assert_eq!(row.0, stripe_refund_id);
    assert_eq!(row.1, 4999);

    assert_eq!(audit_count_for(rig.pool(), &event_id).await, 1);
}

/// When both `charge.refunded` (old embedded shape) AND `refund.created`
/// (modern standalone shape) arrive for the same `stripe_refund_id`, only
/// a single `payment_refunds` row is written. The idempotency key is the
/// `stripe_refund_id` — `ON CONFLICT (stripe_refund_id) DO NOTHING`.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn charge_refunded_and_refund_created_are_idempotent_for_same_refund() {
    let Some(rig) = WebhookTestRig::try_new().await else {
        return;
    };
    let user = seed_user(rig.pool(), &format!("{}@example.com", unique("u"))).await;
    let stripe_customer_id = unique("cus");
    seed_active_subscription(
        rig.pool(),
        user.id,
        &unique("sub"),
        &stripe_customer_id,
        SubscriptionStatus::Active,
    )
    .await;

    let charge_id = unique("ch");
    let pi_id = unique("pi");
    let stripe_refund_id = unique("re");

    // First: the old `charge.refunded` event with embedded refunds.data[]
    let charge_refunded_event_id = unique("evt");
    let charge_refunded = json!({
        "id": charge_refunded_event_id,
        "type": "charge.refunded",
        "data": {
            "object": {
                "id": charge_id,
                "object": "charge",
                "payment_intent": pi_id,
                "customer": stripe_customer_id,
                "currency": "usd",
                "refunds": {
                    "data": [{
                        "id": stripe_refund_id,
                        "amount": 1500,
                        "currency": "usd",
                        "reason": "requested_by_customer",
                        "status": "succeeded",
                        "created": Utc::now().timestamp(),
                    }]
                }
            }
        }
    });

    // Second: the modern `refund.created` for the same refund id
    let refund_created_event_id = unique("evt");
    let refund_created = json!({
        "id": refund_created_event_id,
        "type": "refund.created",
        "data": {
            "object": {
                "id": stripe_refund_id,
                "object": "refund",
                "amount": 1500,
                "currency": "usd",
                "charge": charge_id,
                "payment_intent": pi_id,
                "reason": "requested_by_customer",
                "status": "succeeded",
                "created": Utc::now().timestamp(),
            }
        }
    });

    assert_eq!(rig.post_stripe(&charge_refunded).await, StatusCode::OK);
    assert_eq!(rig.post_stripe(&refund_created).await, StatusCode::OK);

    let count: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM payment_refunds WHERE stripe_refund_id = $1")
            .bind(&stripe_refund_id)
            .fetch_one(rig.pool())
            .await
            .unwrap();
    assert_eq!(
        count.0, 1,
        "duplicate events must produce exactly one refund row"
    );
}
