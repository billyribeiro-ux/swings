#![deny(warnings)]
#![forbid(unsafe_code)]

//! Integration coverage for the native payment-methods management
//! surface (`/api/member/payment-methods` + the three mutators). Stripe
//! is mocked end-to-end via `wiremock`; the harness primes the test
//! AppState with a `Config::stripe_api_base_url_override` pointing at
//! the mock server URI so every Stripe REST call is intercepted before
//! it leaves the process.
//!
//! These tests cover the seven scenarios called out in the gap doc:
//!
//!   A) list happy path (default flag stamped on the matching card)
//!   B) list 404 when the user has no Stripe customer
//!   C) setup-intent: client_secret threaded through, idempotency-key
//!      forwarded to Stripe
//!   D) set-default happy path + audit row written
//!   E) set-default ownership-rejected (foreign customer → 404)
//!   F) delete happy path + audit row written
//!   G) delete refused when the card is the default and a subscription
//!      is active

mod support;

use axum::http::StatusCode;
use serde_json::Value;
use sqlx::{PgPool, Row};
use support::TestApp;
use uuid::Uuid;
use wiremock::matchers::{header, method as http_method, path as http_path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

// ── Fixtures ───────────────────────────────────────────────────────────

/// Insert a subscription row carrying a real (mock-shaped) Stripe
/// customer id so the member handlers can resolve a customer to operate
/// against. Returns the customer id for assertion convenience.
async fn seed_subscription_with_customer(
    pool: &PgPool,
    user_id: Uuid,
    customer_id: &str,
    status: &str,
) -> Uuid {
    let id = Uuid::new_v4();
    sqlx::query(
        r#"
        INSERT INTO subscriptions (
            id, user_id, stripe_customer_id, stripe_subscription_id,
            plan, status, current_period_start, current_period_end,
            created_at, updated_at
        ) VALUES (
            $1, $2, $3, $4,
            'monthly'::subscription_plan, $5::subscription_status,
            NOW(), NOW() + INTERVAL '30 days',
            NOW(), NOW()
        )
        "#,
    )
    .bind(id)
    .bind(user_id)
    .bind(customer_id)
    .bind(format!("sub_{}", &id.simple().to_string()[..16]))
    .bind(status)
    .execute(pool)
    .await
    .expect("seed subscription");
    id
}

async fn count_audit_rows(pool: &PgPool, action: &str, target_id: &str) -> i64 {
    sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM admin_actions WHERE action = $1 AND target_id = $2",
    )
    .bind(action)
    .bind(target_id)
    .fetch_one(pool)
    .await
    .expect("count audit")
}

// ── A) List happy path ────────────────────────────────────────────────

#[tokio::test]
async fn list_returns_payment_methods_with_default_flag() {
    let server = MockServer::start().await;
    let customer_id = "cus_alice_list";
    let default_pm = "pm_card_default";
    let other_pm = "pm_card_other";

    // Stripe payment_methods list — two cards.
    Mock::given(http_method("GET"))
        .and(http_path(format!(
            "/v1/customers/{customer_id}/payment_methods"
        )))
        .and(query_param("type", "card"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "object": "list",
            "data": [
                {
                    "id": default_pm,
                    "object": "payment_method",
                    "type": "card",
                    "customer": customer_id,
                    "card": {
                        "brand": "visa",
                        "last4": "4242",
                        "exp_month": 12,
                        "exp_year": 2026,
                    }
                },
                {
                    "id": other_pm,
                    "object": "payment_method",
                    "type": "card",
                    "customer": customer_id,
                    "card": {
                        "brand": "mastercard",
                        "last4": "5555",
                        "exp_month": 3,
                        "exp_year": 2027,
                    }
                }
            ]
        })))
        .mount(&server)
        .await;

    // Customer GET — returns the default pointing at `default_pm`.
    Mock::given(http_method("GET"))
        .and(http_path(format!("/v1/customers/{customer_id}")))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": customer_id,
            "object": "customer",
            "invoice_settings": {
                "default_payment_method": default_pm,
            }
        })))
        .mount(&server)
        .await;

    let Some(app) = TestApp::try_new_with_stripe(&server.uri()).await else {
        return;
    };
    let alice = app.seed_user().await.expect("seed alice");
    seed_subscription_with_customer(app.db(), alice.id, customer_id, "active").await;

    let resp = app
        .get("/api/member/payment-methods", Some(&alice.access_token))
        .await;
    resp.assert_status(StatusCode::OK);
    let body: Value = resp.json().expect("body");
    assert_eq!(body["default_payment_method_id"].as_str(), Some(default_pm));
    let data = body["payment_methods"].as_array().expect("array");
    assert_eq!(data.len(), 2);
    let default_row = data
        .iter()
        .find(|r| r["id"] == default_pm)
        .expect("default row");
    assert_eq!(default_row["is_default"], serde_json::json!(true));
    assert_eq!(default_row["brand"], serde_json::json!("visa"));
    assert_eq!(default_row["last4"], serde_json::json!("4242"));
    let other_row = data.iter().find(|r| r["id"] == other_pm).expect("other");
    assert_eq!(other_row["is_default"], serde_json::json!(false));
}

// ── B) List 404 when no customer ──────────────────────────────────────

#[tokio::test]
async fn list_returns_404_when_user_has_no_stripe_customer() {
    let server = MockServer::start().await;
    let Some(app) = TestApp::try_new_with_stripe(&server.uri()).await else {
        return;
    };
    let alice = app.seed_user().await.expect("seed alice");
    // Deliberately no subscription seed → resolve_member_stripe_customer
    // should 404 before hitting Stripe.
    let resp = app
        .get("/api/member/payment-methods", Some(&alice.access_token))
        .await;
    resp.assert_status(StatusCode::NOT_FOUND);
}

// ── C) SetupIntent: client_secret + idempotency-key forwarding ───────

#[tokio::test]
async fn setup_intent_returns_client_secret_and_forwards_idempotency_key() {
    let server = MockServer::start().await;
    let customer_id = "cus_alice_setup";
    let idem_key = "idem-setup-intent-1";

    Mock::given(http_method("POST"))
        .and(http_path("/v1/setup_intents"))
        .and(header("Idempotency-Key", idem_key))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": "seti_test_123",
            "object": "setup_intent",
            "client_secret": "seti_test_123_secret_xyz",
        })))
        .mount(&server)
        .await;

    let Some(app) = TestApp::try_new_with_stripe(&server.uri()).await else {
        return;
    };
    let alice = app.seed_user().await.expect("seed alice");
    seed_subscription_with_customer(app.db(), alice.id, customer_id, "active").await;

    let resp = app
        .post_json_with_idempotency_key(
            "/api/member/payment-methods/setup-intent",
            &serde_json::json!({}),
            Some(&alice.access_token),
            idem_key,
        )
        .await;
    resp.assert_status(StatusCode::OK);
    let body: Value = resp.json().expect("body");
    assert_eq!(
        body["client_secret"].as_str(),
        Some("seti_test_123_secret_xyz")
    );

    // Verify the request actually carried the idempotency-key header to
    // Stripe (the mock matcher above already enforces it but we double-
    // check via received_requests for clarity).
    let received = server.received_requests().await.expect("received");
    let setup_req = received
        .iter()
        .find(|r| r.url.path() == "/v1/setup_intents")
        .expect("setup intent request");
    assert_eq!(
        setup_req
            .headers
            .get("idempotency-key")
            .map(|v| v.to_str().unwrap_or("")),
        Some(idem_key)
    );
    // And carried the customer id in the form body.
    let body = String::from_utf8_lossy(&setup_req.body);
    assert!(
        body.contains(&urlencoding::encode(customer_id).into_owned()),
        "customer id missing from form body: {body}"
    );
}

// ── D) Set-default happy path ────────────────────────────────────────

#[tokio::test]
async fn set_default_happy_path_writes_audit_row() {
    let server = MockServer::start().await;
    let customer_id = "cus_alice_setdef";
    let pm_id = "pm_card_setdef";

    // Ownership probe.
    Mock::given(http_method("GET"))
        .and(http_path(format!("/v1/payment_methods/{pm_id}")))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": pm_id,
            "object": "payment_method",
            "customer": customer_id,
            "card": {
                "brand": "visa",
                "last4": "4242",
                "exp_month": 1,
                "exp_year": 2030,
            }
        })))
        .mount(&server)
        .await;

    // Set-default mutation on the customer.
    Mock::given(http_method("POST"))
        .and(http_path(format!("/v1/customers/{customer_id}")))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": customer_id,
            "invoice_settings": { "default_payment_method": pm_id }
        })))
        .mount(&server)
        .await;

    let Some(app) = TestApp::try_new_with_stripe(&server.uri()).await else {
        return;
    };
    let alice = app.seed_user().await.expect("seed alice");
    seed_subscription_with_customer(app.db(), alice.id, customer_id, "active").await;

    let resp = app
        .post_json(
            &format!("/api/member/payment-methods/{pm_id}/set-default"),
            &serde_json::json!({}),
            Some(&alice.access_token),
        )
        .await;
    resp.assert_status(StatusCode::OK);
    let body: Value = resp.json().expect("body");
    assert_eq!(body["default_payment_method_id"].as_str(), Some(pm_id));

    let count = count_audit_rows(app.db(), "member.payment_method.set_default", pm_id).await;
    assert_eq!(count, 1, "expected exactly one audit row");
}

// ── E) Set-default ownership rejected ─────────────────────────────────

#[tokio::test]
async fn set_default_returns_404_when_pm_belongs_to_someone_else() {
    let server = MockServer::start().await;
    let customer_id = "cus_alice_ownership";
    let pm_id = "pm_card_foreign";

    Mock::given(http_method("GET"))
        .and(http_path(format!("/v1/payment_methods/{pm_id}")))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": pm_id,
            "object": "payment_method",
            "customer": "cus_someone_else",
            "card": {
                "brand": "visa",
                "last4": "0000",
                "exp_month": 1,
                "exp_year": 2030,
            }
        })))
        .mount(&server)
        .await;

    let Some(app) = TestApp::try_new_with_stripe(&server.uri()).await else {
        return;
    };
    let alice = app.seed_user().await.expect("seed alice");
    seed_subscription_with_customer(app.db(), alice.id, customer_id, "active").await;

    let resp = app
        .post_json(
            &format!("/api/member/payment-methods/{pm_id}/set-default"),
            &serde_json::json!({}),
            Some(&alice.access_token),
        )
        .await;
    resp.assert_status(StatusCode::NOT_FOUND);

    // No audit row should have been written.
    let count = count_audit_rows(app.db(), "member.payment_method.set_default", pm_id).await;
    assert_eq!(count, 0);
}

// ── F) Delete happy path ─────────────────────────────────────────────

#[tokio::test]
async fn delete_happy_path_writes_audit_row() {
    let server = MockServer::start().await;
    let customer_id = "cus_alice_delete";
    let pm_id = "pm_card_delete";

    // Ownership probe + customer GET (for default check) + detach POST.
    Mock::given(http_method("GET"))
        .and(http_path(format!("/v1/payment_methods/{pm_id}")))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": pm_id,
            "object": "payment_method",
            "customer": customer_id,
            "card": {
                "brand": "amex",
                "last4": "1111",
                "exp_month": 6,
                "exp_year": 2028,
            }
        })))
        .mount(&server)
        .await;

    // Customer has a different default — so removing this PM is allowed
    // even with an active subscription.
    Mock::given(http_method("GET"))
        .and(http_path(format!("/v1/customers/{customer_id}")))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": customer_id,
            "invoice_settings": { "default_payment_method": "pm_card_other_default" }
        })))
        .mount(&server)
        .await;

    Mock::given(http_method("POST"))
        .and(http_path(format!("/v1/payment_methods/{pm_id}/detach")))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": pm_id,
            "object": "payment_method",
            "customer": null,
        })))
        .mount(&server)
        .await;

    let Some(app) = TestApp::try_new_with_stripe(&server.uri()).await else {
        return;
    };
    let alice = app.seed_user().await.expect("seed alice");
    seed_subscription_with_customer(app.db(), alice.id, customer_id, "active").await;

    let resp = app
        .delete(
            &format!("/api/member/payment-methods/{pm_id}"),
            Some(&alice.access_token),
        )
        .await;
    resp.assert_status(StatusCode::OK);
    let body: Value = resp.json().expect("body");
    assert_eq!(body["deleted"], serde_json::json!(true));

    let count = count_audit_rows(app.db(), "member.payment_method.delete", pm_id).await;
    assert_eq!(count, 1);

    // Cross-check the audit metadata mirrors the customer id and the
    // was_default flag (false here — we removed a non-default card).
    let row = sqlx::query(
        "SELECT metadata FROM admin_actions
          WHERE action = 'member.payment_method.delete' AND target_id = $1",
    )
    .bind(pm_id)
    .fetch_one(app.db())
    .await
    .expect("audit row");
    let metadata: Value = row.get("metadata");
    assert_eq!(metadata["was_default"], serde_json::json!(false));
    assert_eq!(metadata["self_service"], serde_json::json!(true));
}

// ── G) Delete refuses default-with-active-sub ─────────────────────────

#[tokio::test]
async fn delete_refuses_when_default_and_subscription_active() {
    let server = MockServer::start().await;
    let customer_id = "cus_alice_refuse";
    let pm_id = "pm_card_default_active";

    Mock::given(http_method("GET"))
        .and(http_path(format!("/v1/payment_methods/{pm_id}")))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": pm_id,
            "object": "payment_method",
            "customer": customer_id,
            "card": {
                "brand": "visa",
                "last4": "4242",
                "exp_month": 1,
                "exp_year": 2030,
            }
        })))
        .mount(&server)
        .await;

    // Customer reports this PM as the default.
    Mock::given(http_method("GET"))
        .and(http_path(format!("/v1/customers/{customer_id}")))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": customer_id,
            "invoice_settings": { "default_payment_method": pm_id }
        })))
        .mount(&server)
        .await;

    let Some(app) = TestApp::try_new_with_stripe(&server.uri()).await else {
        return;
    };
    let alice = app.seed_user().await.expect("seed alice");
    seed_subscription_with_customer(app.db(), alice.id, customer_id, "active").await;

    let resp = app
        .delete(
            &format!("/api/member/payment-methods/{pm_id}"),
            Some(&alice.access_token),
        )
        .await;
    resp.assert_status(StatusCode::BAD_REQUEST);
    let body: Value = resp.json().expect("body");
    let detail = body["detail"].as_str().unwrap_or("");
    assert!(
        detail.contains("default payment method"),
        "expected default-payment-method message, got: {detail}"
    );

    // No detach mock was registered — the wiremock would log an
    // unmatched request if the handler had reached Stripe. The 400
    // assertion above plus the no-audit assertion below cover both
    // safety properties (no Stripe call, no audit row).
    let count = count_audit_rows(app.db(), "member.payment_method.delete", pm_id).await;
    assert_eq!(count, 0);
}
