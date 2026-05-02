#![deny(warnings)]
#![forbid(unsafe_code)]

//! Integration coverage for the member-facing **switch-plan** endpoints
//! end-to-end against a `wiremock::MockServer` standing in for Stripe.
//!
//! The previous integration suite (`member_subscriptions.rs`) covered only
//! the unhappy paths (ownership 404 / unknown plan 404). This binary closes
//! the gap and exercises the full Stripe round-trip:
//!
//! * `GET /api/member/subscriptions/{id}/switch-plan/preview` ↔
//!   `GET /v1/subscriptions/{id}` + `GET /v1/invoices/upcoming`
//! * `POST /api/member/subscriptions/{id}/switch-plan` ↔
//!   `GET /v1/subscriptions/{id}` + `POST /v1/subscriptions/{id}`
//!
//! The harness pattern mirrors `tests/resend_provider.rs`: stand up a
//! `MockServer`, point the production code at it via a per-`AppState`
//! base-URL override (plumbed through `Config::stripe_api_base_url_override`),
//! and assert against `MockServer::received_requests()` to verify the
//! request shape on the wire.

mod support;

use axum::http::StatusCode;
use serde_json::Value;
use sqlx::PgPool;
use support::TestApp;
use uuid::Uuid;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

// ── Fixtures ───────────────────────────────────────────────────────────

/// Insert a subscription with a real `stripe_subscription_id` so the
/// switch-plan handler does NOT short-circuit the Stripe round-trip.
async fn seed_stripe_subscription(
    pool: &PgPool,
    user_id: Uuid,
    stripe_sub_id: &str,
    pricing_plan_id: Option<Uuid>,
) -> Uuid {
    let id = Uuid::new_v4();
    sqlx::query(
        r#"
        INSERT INTO subscriptions (
            id, user_id, stripe_customer_id, stripe_subscription_id,
            plan, status, pricing_plan_id,
            current_period_start, current_period_end,
            created_at, updated_at
        ) VALUES (
            $1, $2, 'cus_test_fixture', $3,
            'monthly'::subscription_plan, 'active'::subscription_status, $4,
            NOW(), NOW() + INTERVAL '30 days',
            NOW(), NOW()
        )
        "#,
    )
    .bind(id)
    .bind(user_id)
    .bind(stripe_sub_id)
    .bind(pricing_plan_id)
    .execute(pool)
    .await
    .expect("seed subscription with stripe_subscription_id");
    id
}

/// Insert a `pricing_plans` row carrying `stripe_price_id` so the
/// switch-plan handler can resolve a Stripe target price.
async fn seed_pricing_plan(
    pool: &PgPool,
    slug: &str,
    interval: &str,
    stripe_price_id: &str,
) -> Uuid {
    let id = Uuid::new_v4();
    sqlx::query(
        r#"
        INSERT INTO pricing_plans (
            id, name, slug, description,
            stripe_price_id, stripe_product_id,
            amount_cents, currency, interval, interval_count,
            trial_days, features, highlight_text, is_popular, is_active,
            sort_order, created_at, updated_at
        ) VALUES (
            $1, $2, $3, '',
            $4, NULL,
            4900, 'usd', $5, 1,
            0, '[]'::jsonb, '', false, true,
            1, NOW(), NOW()
        )
        "#,
    )
    .bind(id)
    .bind(format!("Plan {slug}"))
    .bind(slug)
    .bind(stripe_price_id)
    .bind(interval)
    .execute(pool)
    .await
    .expect("seed pricing plan");
    id
}

/// Realistic Stripe `/v1/subscriptions/{id}?expand[]=items.data` JSON
/// shape. Only `items.data[0].id` is read by the handler — the surrounding
/// envelope is included so a future Stripe version pin that adds typed
/// fields stays compatible.
fn subscription_envelope(stripe_sub_id: &str, item_id: &str) -> Value {
    serde_json::json!({
        "id": stripe_sub_id,
        "object": "subscription",
        "items": {
            "object": "list",
            "data": [
                {
                    "id": item_id,
                    "object": "subscription_item",
                    "price": { "id": "price_existing_old", "object": "price" }
                }
            ]
        }
    })
}

/// Realistic Stripe `/v1/invoices/upcoming` response with two proration
/// line items (one credit, one charge) plus a non-proration line.
fn upcoming_invoice_with_prorations() -> Value {
    serde_json::json!({
        "object": "invoice",
        "amount_due": 1234,
        "total": 4900,
        "currency": "usd",
        "lines": {
            "object": "list",
            "data": [
                {
                    "amount": -2466,
                    "proration": true,
                    "description": "Unused time on Monthly plan"
                },
                {
                    "amount": 3700,
                    "proration": true,
                    "description": "Remaining time on Annual plan"
                },
                {
                    "amount": 4900,
                    "proration": false,
                    "description": "Annual plan recurring charge"
                }
            ]
        }
    })
}

// ── Test A — preview happy path ────────────────────────────────────────

#[tokio::test]
async fn preview_switch_plan_returns_proration_breakdown() {
    let server = MockServer::start().await;
    let Some(app) = TestApp::try_new_with_stripe(&server.uri()).await else {
        return;
    };

    // Mock 1: handler first calls `retrieve_subscription` to discover the
    // line-item id Stripe assigned. We assert path-only here — the query
    // string carries the urlencoded `expand[]=items.data` parameter.
    Mock::given(method("GET"))
        .and(path("/v1/subscriptions/sub_test_preview"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(subscription_envelope(
                "sub_test_preview",
                "si_test_item_preview",
            )),
        )
        .expect(1)
        .mount(&server)
        .await;

    // Mock 2: the proration preview itself.
    Mock::given(method("GET"))
        .and(path("/v1/invoices/upcoming"))
        .respond_with(ResponseTemplate::new(200).set_body_json(upcoming_invoice_with_prorations()))
        .expect(1)
        .mount(&server)
        .await;

    let alice = app.seed_user().await.expect("seed alice");
    let from_plan = seed_pricing_plan(app.db(), "monthly-from", "month", "price_old").await;
    let target_plan = seed_pricing_plan(app.db(), "annual-to", "year", "price_test_target").await;
    let sub_id =
        seed_stripe_subscription(app.db(), alice.id, "sub_test_preview", Some(from_plan)).await;

    let resp = app
        .get(
            &format!(
                "/api/member/subscriptions/{sub_id}/switch-plan/preview?pricing_plan_id={target_plan}"
            ),
            Some(&alice.access_token),
        )
        .await;
    resp.assert_status(StatusCode::OK);
    let body: Value = resp.json().expect("preview body");

    // proration_credit_cents = sum of |negative proration line amounts|.
    assert_eq!(body["proration_credit_cents"], serde_json::json!(2466));
    // proration_charge_cents = sum of positive proration line amounts.
    assert_eq!(body["proration_charge_cents"], serde_json::json!(3700));
    assert_eq!(body["immediate_total_cents"], serde_json::json!(1234));
    assert_eq!(body["next_invoice_total_cents"], serde_json::json!(4900));
    assert_eq!(body["currency"], serde_json::json!("usd"));

    // Verify wiremock saw the `subscription=sub_test_preview` query and
    // the encoded `subscription_items[0][price]=price_test_target` pair.
    let received = server.received_requests().await.expect("received requests");
    let upcoming = received
        .iter()
        .find(|r| r.url.path() == "/v1/invoices/upcoming")
        .expect("upcoming-invoice request was issued");
    let qs = upcoming.url.query().unwrap_or_default();
    assert!(
        qs.contains("subscription=sub_test_preview"),
        "missing subscription= in query: {qs}"
    );
    assert!(
        qs.contains("price_test_target"),
        "target price id should appear in subscription_items[0][price]: {qs}"
    );
    // `subscription_items[0][id]` is bracket-encoded.
    assert!(
        qs.contains("si_test_item_preview"),
        "item id should appear in subscription_items[0][id]: {qs}"
    );
}

// ── Test B — switch-plan happy path with prorate=true ──────────────────

#[tokio::test]
async fn switch_plan_happy_path_persists_and_audits() {
    let server = MockServer::start().await;
    let Some(app) = TestApp::try_new_with_stripe(&server.uri()).await else {
        return;
    };

    Mock::given(method("GET"))
        .and(path("/v1/subscriptions/sub_test_swap"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(subscription_envelope("sub_test_swap", "si_test_item_swap")),
        )
        .expect(1)
        .mount(&server)
        .await;

    // Stripe's POST /v1/subscriptions/{id} returns the updated subscription
    // — handler discards the body but Stripe still ships a typed envelope.
    Mock::given(method("POST"))
        .and(path("/v1/subscriptions/sub_test_swap"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": "sub_test_swap",
            "object": "subscription",
            "status": "active"
        })))
        .expect(1)
        .mount(&server)
        .await;

    let alice = app.seed_user().await.expect("seed alice");
    let from_plan = seed_pricing_plan(app.db(), "monthly-from-b", "month", "price_old_b").await;
    let target_plan =
        seed_pricing_plan(app.db(), "annual-to-b", "year", "price_test_target_b").await;
    let sub_id =
        seed_stripe_subscription(app.db(), alice.id, "sub_test_swap", Some(from_plan)).await;

    let idempotency_key = format!("idem-switch-{}", Uuid::new_v4());
    let resp = app
        .post_json_with_idempotency_key(
            &format!("/api/member/subscriptions/{sub_id}/switch-plan"),
            &serde_json::json!({ "pricing_plan_id": target_plan, "prorate": true }),
            Some(&alice.access_token),
            &idempotency_key,
        )
        .await;
    resp.assert_status(StatusCode::OK);

    // Assert local DB state was updated to point at the new plan.
    let new_plan_id: Option<Uuid> =
        sqlx::query_scalar("SELECT pricing_plan_id FROM subscriptions WHERE id = $1")
            .bind(sub_id)
            .fetch_one(app.db())
            .await
            .expect("read pricing_plan_id");
    assert_eq!(new_plan_id, Some(target_plan));

    // Assert the audit row landed.
    let audit_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM admin_actions WHERE action = 'member.subscription.switch_plan'",
    )
    .fetch_one(app.db())
    .await
    .expect("count audit rows");
    assert_eq!(audit_count, 1, "expected exactly one switch_plan audit row");

    // Assert the wire-format Stripe POST: `items[0][id]`, `items[0][price]`,
    // `proration_behavior=create_prorations`, plus the `Idempotency-Key`
    // header forwarded verbatim.
    let received = server.received_requests().await.expect("received");
    let post_swap = received
        .iter()
        .find(|r| {
            r.method == wiremock::http::Method::POST
                && r.url.path() == "/v1/subscriptions/sub_test_swap"
        })
        .expect("POST /v1/subscriptions/{id} was issued");

    let body = String::from_utf8(post_swap.body.clone()).expect("form body utf-8");
    // Bracket encoding: `[` → %5B, `]` → %5D.
    assert!(
        body.contains("items%5B0%5D%5Bid%5D=si_test_item_swap"),
        "items[0][id] not in body: {body}"
    );
    assert!(
        body.contains("items%5B0%5D%5Bprice%5D=price_test_target_b"),
        "items[0][price] not in body: {body}"
    );
    assert!(
        body.contains("proration_behavior=create_prorations"),
        "proration_behavior=create_prorations not in body: {body}"
    );

    let forwarded_key = post_swap
        .headers
        .get("Idempotency-Key")
        .map(|v| v.to_str().unwrap_or_default())
        .unwrap_or_default();
    assert_eq!(
        forwarded_key, idempotency_key,
        "Idempotency-Key must be forwarded to Stripe verbatim"
    );
}

// ── Test C — switch-plan with prorate=false ────────────────────────────

#[tokio::test]
async fn switch_plan_with_prorate_false_passes_proration_behavior_none() {
    let server = MockServer::start().await;
    let Some(app) = TestApp::try_new_with_stripe(&server.uri()).await else {
        return;
    };

    Mock::given(method("GET"))
        .and(path("/v1/subscriptions/sub_test_no_prorate"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(subscription_envelope(
                "sub_test_no_prorate",
                "si_test_item_noprorate",
            )),
        )
        .expect(1)
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/v1/subscriptions/sub_test_no_prorate"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": "sub_test_no_prorate",
            "object": "subscription",
            "status": "active"
        })))
        .expect(1)
        .mount(&server)
        .await;

    let alice = app.seed_user().await.expect("seed alice");
    let from_plan = seed_pricing_plan(app.db(), "monthly-from-c", "month", "price_old_c").await;
    let target_plan =
        seed_pricing_plan(app.db(), "monthly-to-c", "month", "price_test_target_c").await;
    let sub_id =
        seed_stripe_subscription(app.db(), alice.id, "sub_test_no_prorate", Some(from_plan)).await;

    let idempotency_key = format!("idem-switch-{}", Uuid::new_v4());
    let resp = app
        .post_json_with_idempotency_key(
            &format!("/api/member/subscriptions/{sub_id}/switch-plan"),
            &serde_json::json!({ "pricing_plan_id": target_plan, "prorate": false }),
            Some(&alice.access_token),
            &idempotency_key,
        )
        .await;
    resp.assert_status(StatusCode::OK);

    let received = server.received_requests().await.expect("received");
    let post_swap = received
        .iter()
        .find(|r| {
            r.method == wiremock::http::Method::POST
                && r.url.path() == "/v1/subscriptions/sub_test_no_prorate"
        })
        .expect("POST /v1/subscriptions/{id} was issued");
    let body = String::from_utf8(post_swap.body.clone()).expect("form body utf-8");
    assert!(
        body.contains("proration_behavior=none"),
        "expected proration_behavior=none in body: {body}"
    );
    assert!(
        !body.contains("proration_behavior=create_prorations"),
        "did not expect create_prorations when prorate=false: {body}"
    );
}

// ── Test D — switch-plan Stripe error path ─────────────────────────────

#[tokio::test]
async fn switch_plan_stripe_card_declined_bubbles_up_and_does_not_mutate_db() {
    let server = MockServer::start().await;
    let Some(app) = TestApp::try_new_with_stripe(&server.uri()).await else {
        return;
    };

    Mock::given(method("GET"))
        .and(path("/v1/subscriptions/sub_test_decline"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(subscription_envelope(
                "sub_test_decline",
                "si_test_item_decline",
            )),
        )
        .expect(1)
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/v1/subscriptions/sub_test_decline"))
        .respond_with(ResponseTemplate::new(402).set_body_json(serde_json::json!({
            "error": {
                "type": "card_error",
                "code": "card_declined",
                "message": "Your card was declined."
            }
        })))
        .expect(1)
        .mount(&server)
        .await;

    let alice = app.seed_user().await.expect("seed alice");
    let from_plan = seed_pricing_plan(app.db(), "monthly-from-d", "month", "price_old_d").await;
    let target_plan =
        seed_pricing_plan(app.db(), "annual-to-d", "year", "price_test_target_d").await;
    let sub_id =
        seed_stripe_subscription(app.db(), alice.id, "sub_test_decline", Some(from_plan)).await;

    let idempotency_key = format!("idem-switch-{}", Uuid::new_v4());
    let resp = app
        .post_json_with_idempotency_key(
            &format!("/api/member/subscriptions/{sub_id}/switch-plan"),
            &serde_json::json!({ "pricing_plan_id": target_plan, "prorate": true }),
            Some(&alice.access_token),
            &idempotency_key,
        )
        .await;
    assert!(
        !resp.status().is_success(),
        "expected non-2xx when Stripe declines, got {}",
        resp.status()
    );
    let body_text = resp.text();
    assert!(
        body_text.contains("card_declined") || body_text.contains("declined"),
        "expected Stripe error to bubble up; body: {body_text}"
    );

    // Crucially — the local row must NOT have been swapped to the target.
    let still_old: Option<Uuid> =
        sqlx::query_scalar("SELECT pricing_plan_id FROM subscriptions WHERE id = $1")
            .bind(sub_id)
            .fetch_one(app.db())
            .await
            .expect("read pricing_plan_id");
    assert_eq!(
        still_old,
        Some(from_plan),
        "pricing_plan_id must not change when Stripe rejects the swap"
    );

    // No audit row should have been written either — the handler aborts
    // before the audit call when the Stripe step errors.
    let audit_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM admin_actions WHERE action = 'member.subscription.switch_plan'",
    )
    .fetch_one(app.db())
    .await
    .expect("count audit rows");
    assert_eq!(
        audit_count, 0,
        "no switch_plan audit row should land when Stripe errors"
    );
}

// ── Test E — preview Stripe error path ─────────────────────────────────

#[tokio::test]
async fn preview_returns_non_2xx_when_stripe_errors() {
    let server = MockServer::start().await;
    let Some(app) = TestApp::try_new_with_stripe(&server.uri()).await else {
        return;
    };

    // Simulate Stripe returning 404 on the subscription lookup — e.g.
    // local row drifted from the Stripe twin. The handler should surface a
    // non-2xx rather than a misleading 200 with empty proration.
    Mock::given(method("GET"))
        .and(path("/v1/subscriptions/sub_test_missing"))
        .respond_with(ResponseTemplate::new(404).set_body_json(serde_json::json!({
            "error": {
                "type": "invalid_request_error",
                "code": "resource_missing",
                "message": "No such subscription: sub_test_missing"
            }
        })))
        .expect(1)
        .mount(&server)
        .await;

    let alice = app.seed_user().await.expect("seed alice");
    let from_plan = seed_pricing_plan(app.db(), "monthly-from-e", "month", "price_old_e").await;
    let target_plan =
        seed_pricing_plan(app.db(), "annual-to-e", "year", "price_test_target_e").await;
    let sub_id =
        seed_stripe_subscription(app.db(), alice.id, "sub_test_missing", Some(from_plan)).await;

    let resp = app
        .get(
            &format!(
                "/api/member/subscriptions/{sub_id}/switch-plan/preview?pricing_plan_id={target_plan}"
            ),
            Some(&alice.access_token),
        )
        .await;
    assert!(
        !resp.status().is_success(),
        "expected non-2xx when Stripe is missing the subscription, got {}",
        resp.status()
    );
    let text = resp.text();
    assert!(
        text.contains("resource_missing") || text.contains("No such subscription"),
        "expected Stripe error to bubble up; body: {text}"
    );
}
