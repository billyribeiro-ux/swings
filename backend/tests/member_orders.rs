#![deny(warnings)]
#![forbid(unsafe_code)]

//! Phase 5 integration coverage for the member-facing orders surface:
//! `GET /api/member/orders` (list) and `GET /api/member/orders/{id}`
//! (detail). Asserts:
//!
//!   * The list returns ONLY the authenticated member's own orders (no
//!     bleeding from neighbouring members).
//!   * Pagination obeys `page` / `per_page`; oversized `per_page` is
//!     clamped to 50.
//!   * Detail returns the full embed (items, refunds, state log) for
//!     owned orders.
//!   * Detail returns `404` (not 403) for a foreign order — the
//!     surface must not leak existence.

mod support;

use axum::http::StatusCode;
use serde_json::Value;
use sqlx::PgPool;
use support::TestApp;
use uuid::Uuid;

// ── Fixtures ───────────────────────────────────────────────────────────

async fn seed_product(pool: &PgPool) -> Uuid {
    let id = Uuid::new_v4();
    sqlx::query(
        r#"
        INSERT INTO products
            (id, slug, name, product_type, status, price_cents, currency)
        VALUES ($1, $2, 'Test Product', 'simple', 'published', 1999, 'USD')
        "#,
    )
    .bind(id)
    .bind(format!("prod-{}", &id.to_string()[..8]))
    .execute(pool)
    .await
    .expect("seed product");
    id
}

/// Insert an order for the given user, plus a single line item. Returns
/// the new order id. Bypasses the admin handler so we stay independent
/// of the RBAC stack.
async fn seed_order_for_user(pool: &PgPool, user_id: Uuid, total_cents: i64) -> Uuid {
    let product_id = seed_product(pool).await;
    let order_id = Uuid::new_v4();
    let number = format!("ORD-TEST-{}", &order_id.to_string()[..8]);
    sqlx::query(
        r#"
        INSERT INTO orders
            (id, number, user_id, status, currency,
             subtotal_cents, discount_cents, tax_cents, total_cents, email,
             metadata, placed_at)
        VALUES ($1, $2, $3, 'pending'::order_status, 'usd',
                $4, 0, 0, $4, 'buyer@example.test', '{}'::jsonb, NOW())
        "#,
    )
    .bind(order_id)
    .bind(&number)
    .bind(user_id)
    .bind(total_cents)
    .execute(pool)
    .await
    .expect("seed order");
    sqlx::query(
        r#"
        INSERT INTO order_items
            (order_id, product_id, sku, name, quantity, unit_price_cents, line_total_cents)
        VALUES ($1, $2, NULL, 'Widget', 1, $3, $3)
        "#,
    )
    .bind(order_id)
    .bind(product_id)
    .bind(total_cents)
    .execute(pool)
    .await
    .expect("seed order_items");
    order_id
}

// ── Tests ──────────────────────────────────────────────────────────────

#[tokio::test]
async fn list_returns_only_the_callers_orders() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let alice = app.seed_user().await.expect("seed alice");
    let bob = app.seed_user().await.expect("seed bob");

    // Two orders for alice, one for bob.
    seed_order_for_user(app.db(), alice.id, 1000).await;
    seed_order_for_user(app.db(), alice.id, 2000).await;
    seed_order_for_user(app.db(), bob.id, 3000).await;

    let resp = app
        .get("/api/member/orders", Some(&alice.access_token))
        .await;
    resp.assert_status(StatusCode::OK);
    let body: Value = resp.json().expect("body");
    assert_eq!(body["total"], serde_json::json!(2));
    let data = body["data"].as_array().expect("data array");
    assert_eq!(data.len(), 2);
    for row in data {
        // Each row carries an `item_count` which must reflect the seed.
        assert_eq!(row["item_count"], serde_json::json!(1));
        assert!(row["number"].is_string());
        assert!(row["currency"].is_string());
    }
}

#[tokio::test]
async fn list_pagination_clamps_per_page_to_fifty() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let alice = app.seed_user().await.expect("seed alice");
    let resp = app
        .get("/api/member/orders?per_page=999", Some(&alice.access_token))
        .await;
    resp.assert_status(StatusCode::OK);
    let body: Value = resp.json().expect("body");
    assert_eq!(body["per_page"], serde_json::json!(50));
}

#[tokio::test]
async fn detail_returns_full_embed_for_owned_order() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let alice = app.seed_user().await.expect("seed alice");
    let order_id = seed_order_for_user(app.db(), alice.id, 1500).await;

    let resp = app
        .get(
            &format!("/api/member/orders/{order_id}"),
            Some(&alice.access_token),
        )
        .await;
    resp.assert_status(StatusCode::OK);
    let body: Value = resp.json().expect("body");
    assert_eq!(body["order"]["id"], serde_json::json!(order_id.to_string()));
    assert_eq!(body["items"].as_array().expect("items").len(), 1);
    assert!(body["refunds"].is_array());
    assert!(body["state_log"].is_array());
}

#[tokio::test]
async fn detail_returns_404_for_foreign_order() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let alice = app.seed_user().await.expect("seed alice");
    let bob = app.seed_user().await.expect("seed bob");
    let bobs_order = seed_order_for_user(app.db(), bob.id, 4200).await;

    let resp = app
        .get(
            &format!("/api/member/orders/{bobs_order}"),
            Some(&alice.access_token),
        )
        .await;
    // 404 (not 403) so the surface doesn't leak existence.
    resp.assert_status(StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn detail_returns_404_for_unknown_id() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let alice = app.seed_user().await.expect("seed alice");
    let bogus = Uuid::new_v4();
    let resp = app
        .get(
            &format!("/api/member/orders/{bogus}"),
            Some(&alice.access_token),
        )
        .await;
    resp.assert_status(StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn list_requires_authentication() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let resp = app.get("/api/member/orders", None).await;
    resp.assert_status(StatusCode::UNAUTHORIZED);
}
