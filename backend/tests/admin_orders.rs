#![deny(warnings)]
#![forbid(unsafe_code)]

//! ADM-12 integration coverage for the orders admin surface.
//!
//! Asserts:
//!   * RBAC gates on every verb (member / support / admin).
//!   * Manual create flows: happy path (pending), `mark_completed` walks
//!     the FSM, validation errors, unknown product → `400`.
//!   * Void: happy path, terminal state rejection, unknown id → `404`.
//!   * Refund: partial leaves the order in `Completed`, full flips it to
//!     `Refunded`, over-balance → `400`, refund of cancelled order →
//!     `400`.
//!   * Listing: pagination + `q` substring + `status` filter.
//!   * CSV export emits the right `Content-Type` + header row.

mod support;

use axum::http::StatusCode;
use serde_json::{json, Value};
use sqlx::Row;
use support::TestApp;
use uuid::Uuid;

// ── Fixtures ───────────────────────────────────────────────────────────

/// Insert a published product so manual-create has a valid `product_id`
/// to satisfy `order_items.product_id` FK.
async fn seed_product(app: &TestApp) -> Uuid {
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
    .execute(app.db())
    .await
    .expect("seed product");
    id
}

/// Mint a pending order via the admin manual-create endpoint and return
/// the new order id. Used by void / refund tests as their setup.
async fn create_pending_order(app: &TestApp, admin_token: &str, total: i64) -> Uuid {
    let product_id = seed_product(app).await;
    let resp = app
        .post_json::<Value>(
            "/api/admin/orders",
            &json!({
                "email":    "buyer@example.com",
                "currency": "usd",
                "items": [{
                    "product_id":       product_id,
                    "quantity":         1,
                    "unit_price_cents": total,
                    "name":             "Test Item",
                }],
            }),
            Some(admin_token),
        )
        .await;
    resp.assert_status(StatusCode::CREATED);
    let body: Value = resp.json().expect("body");
    Uuid::parse_str(body["order"]["id"].as_str().expect("id")).expect("uuid")
}

async fn create_completed_order(app: &TestApp, admin_token: &str, total: i64) -> Uuid {
    let product_id = seed_product(app).await;
    let resp = app
        .post_json::<Value>(
            "/api/admin/orders",
            &json!({
                "email":          "buyer@example.com",
                "currency":       "usd",
                "mark_completed": true,
                "items": [{
                    "product_id":       product_id,
                    "quantity":         1,
                    "unit_price_cents": total,
                    "name":             "Test Item",
                }],
            }),
            Some(admin_token),
        )
        .await;
    resp.assert_status(StatusCode::CREATED);
    let body: Value = resp.json().expect("body");
    Uuid::parse_str(body["order"]["id"].as_str().expect("id")).expect("uuid")
}

// ── RBAC gates ─────────────────────────────────────────────────────────

#[tokio::test]
async fn member_cannot_touch_admin_orders_surface() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let member = app.seed_user().await.expect("seed member");

    let list = app
        .get("/api/admin/orders", Some(&member.access_token))
        .await;
    list.assert_status(StatusCode::FORBIDDEN);

    let create = app
        .post_json::<Value>(
            "/api/admin/orders",
            &json!({"email": "x@y.z", "items": []}),
            Some(&member.access_token),
        )
        .await;
    create.assert_status(StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn support_can_read_and_refund_but_not_create_or_void() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let admin = app.seed_admin().await.expect("seed admin");
    let support = app.seed_support().await.expect("seed support");
    let order_id = create_completed_order(&app, &admin.access_token, 5000).await;

    // Read works for support — `admin.order.read` is granted.
    let read = app
        .get(
            &format!("/api/admin/orders/{order_id}"),
            Some(&support.access_token),
        )
        .await;
    read.assert_status(StatusCode::OK);

    // Refund works for support — `admin.order.refund` is granted.
    let refund = app
        .post_json::<Value>(
            &format!("/api/admin/orders/{order_id}/refund"),
            &json!({"amount_cents": 100, "reason": "credit"}),
            Some(&support.access_token),
        )
        .await;
    refund.assert_status(StatusCode::OK);

    // Create is admin-only.
    let create = app
        .post_json::<Value>(
            "/api/admin/orders",
            &json!({"email": "a@b.c", "items": [{
                "product_id": Uuid::new_v4(),
                "quantity": 1, "unit_price_cents": 100, "name": "X"
            }]}),
            Some(&support.access_token),
        )
        .await;
    create.assert_status(StatusCode::FORBIDDEN);

    // Void is admin-only.
    let void = app
        .post_json::<Value>(
            &format!("/api/admin/orders/{order_id}/void"),
            &json!({"reason": "no"}),
            Some(&support.access_token),
        )
        .await;
    void.assert_status(StatusCode::FORBIDDEN);
}

// ── Manual create ──────────────────────────────────────────────────────

#[tokio::test]
async fn create_manual_pending_order_persists_and_audits() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let admin = app.seed_admin().await.expect("seed admin");
    let product_id = seed_product(&app).await;

    let resp = app
        .post_json::<Value>(
            "/api/admin/orders",
            &json!({
                "email":          "wholesale@example.com",
                "currency":       "usd",
                "discount_cents": 200,
                "tax_cents":      150,
                "notes":          "Net 30",
                "items": [
                    { "product_id": product_id, "quantity": 2,
                      "unit_price_cents": 1500, "name": "Widget" },
                    { "product_id": product_id, "quantity": 1,
                      "unit_price_cents": 1000, "name": "Bracket" },
                ],
            }),
            Some(&admin.access_token),
        )
        .await;
    resp.assert_status(StatusCode::CREATED);
    let body: Value = resp.json().expect("body");

    // 2*1500 + 1*1000 - 200 + 150 = 3950
    assert_eq!(body["order"]["total_cents"], json!(3950));
    assert_eq!(body["order"]["status"], json!("pending"));
    assert_eq!(body["items"].as_array().expect("items").len(), 2);
    assert_eq!(body["notes"].as_array().expect("notes").len(), 1);

    let audit_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM admin_actions WHERE action = 'admin.order.create'",
    )
    .fetch_one(app.db())
    .await
    .expect("audit count");
    assert_eq!(audit_count, 1);
}

#[tokio::test]
async fn create_manual_with_mark_completed_walks_fsm() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let admin = app.seed_admin().await.expect("seed admin");
    let order_id = create_completed_order(&app, &admin.access_token, 4200).await;

    let status: String = sqlx::query_scalar("SELECT status::text FROM orders WHERE id = $1")
        .bind(order_id)
        .fetch_one(app.db())
        .await
        .expect("status");
    assert_eq!(status, "completed");

    // Two state-log rows: pending→processing, processing→completed.
    let trans_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM order_state_transitions WHERE order_id = $1")
            .bind(order_id)
            .fetch_one(app.db())
            .await
            .expect("transitions");
    assert_eq!(trans_count, 2);
}

#[tokio::test]
async fn create_manual_rejects_unknown_product() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let admin = app.seed_admin().await.expect("seed admin");

    let resp = app
        .post_json::<Value>(
            "/api/admin/orders",
            &json!({
                "email": "x@y.z",
                "items": [{
                    "product_id":       Uuid::new_v4(),
                    "quantity":         1,
                    "unit_price_cents": 100,
                    "name":             "Ghost",
                }],
            }),
            Some(&admin.access_token),
        )
        .await;
    resp.assert_status(StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn create_manual_rejects_empty_items() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let admin = app.seed_admin().await.expect("seed admin");

    let resp = app
        .post_json::<Value>(
            "/api/admin/orders",
            &json!({"email": "x@y.z", "items": []}),
            Some(&admin.access_token),
        )
        .await;
    resp.assert_status(StatusCode::BAD_REQUEST);
}

// ── Void ───────────────────────────────────────────────────────────────

#[tokio::test]
async fn void_pending_order_transitions_to_cancelled() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let admin = app.seed_admin().await.expect("seed admin");
    let order_id = create_pending_order(&app, &admin.access_token, 2500).await;

    let resp = app
        .post_json::<Value>(
            &format!("/api/admin/orders/{order_id}/void"),
            &json!({"reason": "duplicate"}),
            Some(&admin.access_token),
        )
        .await;
    resp.assert_status(StatusCode::OK);
    let body: Value = resp.json().expect("body");
    assert_eq!(body["order"]["status"], json!("cancelled"));

    let audit_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM admin_actions WHERE action = 'admin.order.void'")
            .fetch_one(app.db())
            .await
            .expect("audit count");
    assert_eq!(audit_count, 1);
}

#[tokio::test]
async fn void_unknown_order_is_404() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let admin = app.seed_admin().await.expect("seed admin");

    let resp = app
        .post_json::<Value>(
            &format!("/api/admin/orders/{}/void", Uuid::new_v4()),
            &json!({"reason": "missing"}),
            Some(&admin.access_token),
        )
        .await;
    resp.assert_status(StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn void_already_cancelled_is_400() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let admin = app.seed_admin().await.expect("seed admin");
    let order_id = create_pending_order(&app, &admin.access_token, 2500).await;

    app.post_json::<Value>(
        &format!("/api/admin/orders/{order_id}/void"),
        &json!({}),
        Some(&admin.access_token),
    )
    .await
    .assert_status(StatusCode::OK);

    let again = app
        .post_json::<Value>(
            &format!("/api/admin/orders/{order_id}/void"),
            &json!({}),
            Some(&admin.access_token),
        )
        .await;
    again.assert_status(StatusCode::BAD_REQUEST);
}

// ── Refund ─────────────────────────────────────────────────────────────

#[tokio::test]
async fn partial_refund_leaves_order_in_completed() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let admin = app.seed_admin().await.expect("seed admin");
    let order_id = create_completed_order(&app, &admin.access_token, 5000).await;

    let resp = app
        .post_json::<Value>(
            &format!("/api/admin/orders/{order_id}/refund"),
            &json!({"amount_cents": 1500, "reason": "goodwill"}),
            Some(&admin.access_token),
        )
        .await;
    resp.assert_status(StatusCode::OK);
    let body: Value = resp.json().expect("body");
    assert_eq!(body["order_marked_refunded"], json!(false));
    assert_eq!(body["remaining_refundable_cents"], json!(3500));

    let status: String = sqlx::query_scalar("SELECT status::text FROM orders WHERE id = $1")
        .bind(order_id)
        .fetch_one(app.db())
        .await
        .expect("status");
    assert_eq!(status, "completed");
}

#[tokio::test]
async fn full_refund_flips_order_to_refunded() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let admin = app.seed_admin().await.expect("seed admin");
    let order_id = create_completed_order(&app, &admin.access_token, 5000).await;

    let resp = app
        .post_json::<Value>(
            &format!("/api/admin/orders/{order_id}/refund"),
            &json!({
                "amount_cents":     5000,
                "reason":           "return",
                "stripe_refund_id": "re_test_123",
            }),
            Some(&admin.access_token),
        )
        .await;
    resp.assert_status(StatusCode::OK);
    let body: Value = resp.json().expect("body");
    assert_eq!(body["order_marked_refunded"], json!(true));
    assert_eq!(body["remaining_refundable_cents"], json!(0));

    let status: String = sqlx::query_scalar("SELECT status::text FROM orders WHERE id = $1")
        .bind(order_id)
        .fetch_one(app.db())
        .await
        .expect("status");
    assert_eq!(status, "refunded");

    let stripe_id: Option<String> =
        sqlx::query_scalar("SELECT stripe_refund_id FROM order_refunds WHERE order_id = $1")
            .bind(order_id)
            .fetch_one(app.db())
            .await
            .expect("stripe_refund_id");
    assert_eq!(stripe_id.as_deref(), Some("re_test_123"));
}

#[tokio::test]
async fn refund_overbalance_is_rejected() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let admin = app.seed_admin().await.expect("seed admin");
    let order_id = create_completed_order(&app, &admin.access_token, 1000).await;

    let resp = app
        .post_json::<Value>(
            &format!("/api/admin/orders/{order_id}/refund"),
            &json!({"amount_cents": 9999}),
            Some(&admin.access_token),
        )
        .await;
    resp.assert_status(StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn refund_of_cancelled_order_is_rejected() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let admin = app.seed_admin().await.expect("seed admin");
    let order_id = create_pending_order(&app, &admin.access_token, 2000).await;

    app.post_json::<Value>(
        &format!("/api/admin/orders/{order_id}/void"),
        &json!({}),
        Some(&admin.access_token),
    )
    .await
    .assert_status(StatusCode::OK);

    let refund = app
        .post_json::<Value>(
            &format!("/api/admin/orders/{order_id}/refund"),
            &json!({"amount_cents": 100}),
            Some(&admin.access_token),
        )
        .await;
    refund.assert_status(StatusCode::BAD_REQUEST);
}

// ── List + CSV ─────────────────────────────────────────────────────────

#[tokio::test]
async fn list_filters_by_status_and_paginates() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let admin = app.seed_admin().await.expect("seed admin");
    create_pending_order(&app, &admin.access_token, 1000).await;
    create_pending_order(&app, &admin.access_token, 2000).await;
    create_completed_order(&app, &admin.access_token, 3000).await;

    let pending = app
        .get(
            "/api/admin/orders?status=pending&limit=10",
            Some(&admin.access_token),
        )
        .await;
    pending.assert_status(StatusCode::OK);
    let body: Value = pending.json().expect("body");
    assert_eq!(body["total"], json!(2));
    assert_eq!(body["data"].as_array().expect("data").len(), 2);

    // Unknown status → 400.
    let bad = app
        .get("/api/admin/orders?status=bogus", Some(&admin.access_token))
        .await;
    bad.assert_status(StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn list_substring_search_matches_email() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let admin = app.seed_admin().await.expect("seed admin");
    let product_id = seed_product(&app).await;
    app.post_json::<Value>(
        "/api/admin/orders",
        &json!({
            "email": "needle@example.com",
            "items": [{
                "product_id": product_id, "quantity": 1,
                "unit_price_cents": 100, "name": "X",
            }],
        }),
        Some(&admin.access_token),
    )
    .await
    .assert_status(StatusCode::CREATED);
    create_pending_order(&app, &admin.access_token, 100).await;

    let search = app
        .get("/api/admin/orders?q=needle", Some(&admin.access_token))
        .await;
    search.assert_status(StatusCode::OK);
    let body: Value = search.json().expect("body");
    assert_eq!(body["total"], json!(1));
    assert_eq!(
        body["data"][0]["email"].as_str().expect("email"),
        "needle@example.com"
    );
}

#[tokio::test]
async fn export_csv_returns_csv_with_header_row() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let admin = app.seed_admin().await.expect("seed admin");
    create_pending_order(&app, &admin.access_token, 1000).await;

    let resp = app
        .get("/api/admin/orders/export.csv", Some(&admin.access_token))
        .await;
    resp.assert_status(StatusCode::OK);
    let ct = resp.header("content-type").unwrap_or_default();
    assert!(
        ct.starts_with("text/csv"),
        "expected text/csv content-type, got {ct:?}"
    );
    let cd = resp.header("content-disposition").unwrap_or_default();
    assert!(cd.contains("orders.csv"), "missing filename: {cd:?}");

    let text = resp.text();
    let mut lines = text.lines();
    let header = lines.next().expect("header line");
    assert!(
        header.starts_with("id,number,user_id,status"),
        "header: {header}"
    );
    assert_eq!(lines.count(), 1, "expected exactly one data row");

    let audit_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM admin_actions WHERE action = 'admin.order.export'",
    )
    .fetch_one(app.db())
    .await
    .expect("audit count");
    assert_eq!(audit_count, 1);
}

#[tokio::test]
async fn export_csv_requires_export_permission() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let support = app.seed_support().await.expect("seed support");
    let resp = app
        .get("/api/admin/orders/export.csv", Some(&support.access_token))
        .await;
    resp.assert_status(StatusCode::FORBIDDEN);
}

// ── read_one detail roll-up ────────────────────────────────────────────

#[tokio::test]
async fn read_one_returns_items_refunds_and_remaining() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let admin = app.seed_admin().await.expect("seed admin");
    let order_id = create_completed_order(&app, &admin.access_token, 4000).await;

    app.post_json::<Value>(
        &format!("/api/admin/orders/{order_id}/refund"),
        &json!({"amount_cents": 1000}),
        Some(&admin.access_token),
    )
    .await
    .assert_status(StatusCode::OK);

    let resp = app
        .get(
            &format!("/api/admin/orders/{order_id}"),
            Some(&admin.access_token),
        )
        .await;
    resp.assert_status(StatusCode::OK);
    let body: Value = resp.json().expect("body");
    assert_eq!(body["refunded_cents"], json!(1000));
    assert_eq!(body["remaining_refundable_cents"], json!(3000));
    assert_eq!(body["items"].as_array().expect("items").len(), 1);
    assert_eq!(body["refunds"].as_array().expect("refunds").len(), 1);

    // Sanity: order_state_transitions row count is touchable.
    let trans: i64 =
        sqlx::query("SELECT COUNT(*) AS c FROM order_state_transitions WHERE order_id = $1")
            .bind(order_id)
            .fetch_one(app.db())
            .await
            .expect("trans")
            .try_get("c")
            .expect("c");
    assert!(trans >= 2);
}
