#![deny(warnings)]
#![forbid(unsafe_code)]

//! ADM-15 integration coverage for the Idempotency-Key middleware.
//!
//! Asserts:
//!   * Replay of an identical (key, body) returns the cached response
//!     and never re-runs the side effect (no duplicate orders).
//!   * Different body under the same key surfaces 422
//!     `idempotency-key-mismatch`.
//!   * Requests without the header behave exactly like before
//!     (no caching, every call hits the handler).
//!   * Cache is keyed per actor: two admins reusing the same key value
//!     do NOT collide.
//!   * Failed (non-2xx) responses are not cached, so retrying after a
//!     transient validation failure can succeed.

mod support;

use axum::http::StatusCode;
use serde_json::{json, Value};
use sqlx::Row;
use support::TestApp;
use uuid::Uuid;

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

#[tokio::test]
async fn replay_returns_cached_response_and_does_not_re_create() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let admin = app.seed_admin().await.expect("seed admin");
    let product_id = seed_product(&app).await;

    let body = json!({
        "email": "buyer@example.com",
        "currency": "usd",
        "items": [{
            "product_id": product_id,
            "quantity": 1,
            "unit_price_cents": 1999,
            "name": "Test Item",
        }],
    });
    let key = "test-key-replay-1";

    let first = app
        .post_json_with_idempotency_key(
            "/api/admin/orders",
            &body,
            Some(&admin.access_token),
            key,
        )
        .await;
    first.assert_status(StatusCode::CREATED);
    let first_body: Value = first.json().expect("first body");
    let first_order_id = first_body["order"]["id"].as_str().unwrap().to_string();

    let second = app
        .post_json_with_idempotency_key(
            "/api/admin/orders",
            &body,
            Some(&admin.access_token),
            key,
        )
        .await;
    second.assert_status(StatusCode::CREATED);
    let second_body: Value = second.json().expect("second body");
    let second_order_id = second_body["order"]["id"].as_str().unwrap().to_string();

    assert_eq!(
        first_order_id, second_order_id,
        "replay must return the original order id, not create a new one"
    );
    assert_eq!(
        second.header("idempotency-replayed"),
        Some("true"),
        "replays must be flagged so the client can distinguish them"
    );

    // Database invariant: only one row was actually inserted.
    let count: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM orders WHERE email = 'buyer@example.com'")
            .fetch_one(app.db())
            .await
            .expect("count orders");
    assert_eq!(count.0, 1, "second call must not have created a second order");
}

#[tokio::test]
async fn same_key_different_body_returns_422_mismatch() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let admin = app.seed_admin().await.expect("seed admin");
    let product_id = seed_product(&app).await;
    let key = "test-key-mismatch-1";

    let body_a = json!({
        "email": "buyer-a@example.com",
        "currency": "usd",
        "items": [{
            "product_id": product_id,
            "quantity": 1,
            "unit_price_cents": 1999,
            "name": "Test Item",
        }],
    });
    let body_b = json!({
        "email": "buyer-b@example.com",
        "currency": "usd",
        "items": [{
            "product_id": product_id,
            "quantity": 1,
            "unit_price_cents": 1999,
            "name": "Test Item",
        }],
    });

    app.post_json_with_idempotency_key(
        "/api/admin/orders",
        &body_a,
        Some(&admin.access_token),
        key,
    )
    .await
    .assert_status(StatusCode::CREATED);

    let conflict = app
        .post_json_with_idempotency_key(
            "/api/admin/orders",
            &body_b,
            Some(&admin.access_token),
            key,
        )
        .await;
    conflict.assert_status(StatusCode::UNPROCESSABLE_ENTITY);
    let problem: Value = conflict.json().expect("problem body");
    assert_eq!(problem["type"], "/problems/idempotency-key-mismatch");
    assert_eq!(problem["status"], 422);
}

#[tokio::test]
async fn no_header_means_no_caching() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let admin = app.seed_admin().await.expect("seed admin");
    let product_id = seed_product(&app).await;

    let body = json!({
        "email": "no-key@example.com",
        "currency": "usd",
        "items": [{
            "product_id": product_id,
            "quantity": 1,
            "unit_price_cents": 500,
            "name": "Test Item",
        }],
    });

    let r1 = app
        .post_json::<Value>("/api/admin/orders", &body, Some(&admin.access_token))
        .await;
    r1.assert_status(StatusCode::CREATED);
    let r2 = app
        .post_json::<Value>("/api/admin/orders", &body, Some(&admin.access_token))
        .await;
    r2.assert_status(StatusCode::CREATED);

    assert_eq!(r2.header("idempotency-replayed"), None);

    let id1 = r1.json::<Value>().unwrap()["order"]["id"].as_str().unwrap().to_string();
    let id2 = r2.json::<Value>().unwrap()["order"]["id"].as_str().unwrap().to_string();
    assert_ne!(
        id1, id2,
        "without the header each call must create a new order"
    );
}

#[tokio::test]
async fn cache_is_scoped_per_actor() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let admin_a = app.seed_admin().await.expect("seed admin a");
    let admin_b = app.seed_admin().await.expect("seed admin b");
    let product_id = seed_product(&app).await;

    let body = json!({
        "email": "shared-key@example.com",
        "currency": "usd",
        "items": [{
            "product_id": product_id,
            "quantity": 1,
            "unit_price_cents": 1500,
            "name": "Test Item",
        }],
    });
    let key = "shared-key-across-actors";

    let r_a = app
        .post_json_with_idempotency_key(
            "/api/admin/orders",
            &body,
            Some(&admin_a.access_token),
            key,
        )
        .await;
    r_a.assert_status(StatusCode::CREATED);

    let r_b = app
        .post_json_with_idempotency_key(
            "/api/admin/orders",
            &body,
            Some(&admin_b.access_token),
            key,
        )
        .await;
    r_b.assert_status(StatusCode::CREATED);

    let id_a = r_a.json::<Value>().unwrap()["order"]["id"].as_str().unwrap().to_string();
    let id_b = r_b.json::<Value>().unwrap()["order"]["id"].as_str().unwrap().to_string();
    assert_ne!(id_a, id_b, "two actors with the same key must not collide");

    let count: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM orders WHERE email = 'shared-key@example.com'")
            .fetch_one(app.db())
            .await
            .expect("count orders");
    assert_eq!(count.0, 2);
}

#[tokio::test]
async fn failed_responses_are_not_cached() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let admin = app.seed_admin().await.expect("seed admin");
    let product_id = seed_product(&app).await;
    let key = "test-key-failure-then-retry";

    // First call: invalid product reference triggers a validation 400.
    let bad_body = json!({
        "email": "retry@example.com",
        "currency": "usd",
        "items": [{
            "product_id": Uuid::new_v4(),  // does not exist
            "quantity": 1,
            "unit_price_cents": 1000,
            "name": "Bogus",
        }],
    });
    let bad = app
        .post_json_with_idempotency_key(
            "/api/admin/orders",
            &bad_body,
            Some(&admin.access_token),
            key,
        )
        .await;
    assert!(
        bad.status().is_client_error(),
        "expected 4xx, got {}",
        bad.status()
    );

    // Second call: valid body under the same key should succeed because
    // the failed row was not cached and the cleanup deleted it.
    let good_body = json!({
        "email": "retry@example.com",
        "currency": "usd",
        "items": [{
            "product_id": product_id,
            "quantity": 1,
            "unit_price_cents": 1000,
            "name": "Real Item",
        }],
    });
    let good = app
        .post_json_with_idempotency_key(
            "/api/admin/orders",
            &good_body,
            Some(&admin.access_token),
            key,
        )
        .await;
    good.assert_status(StatusCode::CREATED);
    assert_eq!(
        good.header("idempotency-replayed"),
        None,
        "first successful call must not be flagged as a replay"
    );
}

#[tokio::test]
async fn cache_row_is_persisted_with_response_metadata() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let admin = app.seed_admin().await.expect("seed admin");
    let product_id = seed_product(&app).await;
    let key = "test-key-row-persisted";

    let body = json!({
        "email": "rowtest@example.com",
        "currency": "usd",
        "items": [{
            "product_id": product_id,
            "quantity": 1,
            "unit_price_cents": 1234,
            "name": "Row Test",
        }],
    });
    app.post_json_with_idempotency_key(
        "/api/admin/orders",
        &body,
        Some(&admin.access_token),
        key,
    )
    .await
    .assert_status(StatusCode::CREATED);

    let row = sqlx::query(
        "SELECT status_code, in_flight, completed_at, response_body, method, path
         FROM idempotency_keys WHERE key = $1",
    )
    .bind(key)
    .fetch_one(app.db())
    .await
    .expect("fetch cache row");

    let status: i32 = row.get("status_code");
    let in_flight: bool = row.get("in_flight");
    let completed: Option<chrono::DateTime<chrono::Utc>> = row.get("completed_at");
    let body_bytes: Vec<u8> = row.get("response_body");
    let method: String = row.get("method");
    let path: String = row.get("path");

    assert_eq!(status, 201);
    assert!(!in_flight, "completed rows must clear the in_flight flag");
    assert!(completed.is_some());
    assert!(!body_bytes.is_empty(), "response body must be cached");
    assert_eq!(method, "POST");
    assert_eq!(path, "/api/admin/orders");
}
