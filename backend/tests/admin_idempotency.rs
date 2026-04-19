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

/// ADM-15 race-condition coverage. The middleware claims a row by
/// inserting `(user_id, key)` with `in_flight = TRUE` under the PK
/// uniqueness constraint, so concurrent requests with the same key
/// race for that INSERT. Whoever loses the race re-reads the row,
/// sees `in_flight = TRUE`, and returns `409 idempotency-in-flight`.
///
/// We fan out N parallel POSTs against the same `(actor, key, body)`
/// and assert the only invariant that actually matters in production:
/// **exactly one order is created**. The HTTP response distribution
/// is timing-dependent (a fast first request finishes before the
/// next claim starts → the rest get cached 201s; a slow first request
/// is still in flight when others claim → the rest get 409s), so we
/// assert the union: every response is either a successful 201 (one
/// real, the rest replays) or a 409 in-flight error.
///
/// The unique-constraint approach in `try_claim` is the load-bearing
/// guarantee here; if anyone ever weakens it (e.g. swaps to an
/// upsert that overwrites in-flight rows), this test catches it.
#[tokio::test]
async fn concurrent_same_key_creates_exactly_one_resource() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let admin = app.seed_admin().await.expect("seed admin");
    let product_id = seed_product(&app).await;
    let key = "race-key-001";
    let email = "race-buyer@example.com";

    let body = json!({
        "email": email,
        "currency": "usd",
        "items": [{
            "product_id": product_id,
            "quantity": 1,
            "unit_price_cents": 1999,
            "name": "Race Item",
        }],
    });

    // Fan out 16 parallel requests. The number is deliberately well
    // above the connection-pool size so we exercise both the
    // "second request lands while first is in flight" path (→ 409)
    // and the "second request lands after first completes" path
    // (→ cached 201) in the same run.
    // Share the app across tasks via Arc — TestApp owns a TempDir
    // that must not be dropped twice, so we cannot derive Clone on
    // the struct itself. tokio::spawn requires 'static, hence the Arc.
    const PARALLELISM: usize = 16;
    let app = std::sync::Arc::new(app);
    // Synchronise all spawned tasks at a barrier just before the
    // request goes out so they actually race. Without the barrier,
    // the early-spawned tasks would often complete before the
    // late-spawned ones even start, collapsing this back to a serial
    // replay test.
    let barrier = std::sync::Arc::new(tokio::sync::Barrier::new(PARALLELISM));
    let mut handles = Vec::with_capacity(PARALLELISM);
    for _ in 0..PARALLELISM {
        let app_for_task = app.clone();
        let barrier_for_task = barrier.clone();
        let token = admin.access_token.clone();
        let body_clone = body.clone();
        handles.push(tokio::spawn(async move {
            barrier_for_task.wait().await;
            app_for_task
                .post_json_with_idempotency_key(
                    "/api/admin/orders",
                    &body_clone,
                    Some(&token),
                    key,
                )
                .await
        }));
    }

    let mut created = 0;
    let mut in_flight = 0;
    for h in handles {
        let resp = h.await.expect("join task");
        let status = resp.status();
        if status == StatusCode::CREATED {
            created += 1;
        } else if status == StatusCode::CONFLICT {
            in_flight += 1;
            // Confirm the problem document is the one we expect, not
            // some other 409 (e.g. an order-state transition error).
            let body: Value = resp.json().expect("problem body");
            assert_eq!(
                body["type"], "/problems/idempotency-in-flight",
                "409s during the race must be the in-flight problem, not some other conflict"
            );
        } else {
            panic!(
                "unexpected status {status} during idempotency race: {:?}",
                resp.json::<Value>().ok()
            );
        }
    }

    assert!(
        created >= 1,
        "at least one parallel request must have claimed the key and run the side effect"
    );
    assert_eq!(
        created + in_flight,
        PARALLELISM,
        "every response must be 201 (real or replayed) or 409 in-flight"
    );

    // The one that matters: the side effect ran exactly once, no
    // matter how the responses got distributed.
    let count: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM orders WHERE email = $1")
            .bind(email)
            .fetch_one(app.db())
            .await
            .expect("count orders");
    assert_eq!(
        count.0, 1,
        "{PARALLELISM} parallel requests must produce exactly one order row, got {}",
        count.0
    );

    // And the cache row must reflect the completed state: in_flight
    // cleared, status_code=201, response_body populated. If any
    // 409-losers wrote a stale row this assertion will catch it.
    let row = sqlx::query(
        "SELECT in_flight, status_code FROM idempotency_keys WHERE key = $1",
    )
    .bind(key)
    .fetch_one(app.db())
    .await
    .expect("fetch cache row");
    let in_flight_final: bool = row.get("in_flight");
    let status_code_final: i32 = row.get("status_code");
    assert!(
        !in_flight_final,
        "after the race the cache row must be marked completed"
    );
    assert_eq!(status_code_final, 201);
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
