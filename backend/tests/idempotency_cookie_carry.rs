#![deny(warnings)]
#![forbid(unsafe_code)]

//! BFF Phase 1.4 — CONTRACT-D2 regression coverage.
//!
//! Before the fix the ADM-15 idempotency middleware only inspected
//! `Authorization: Bearer …` to derive its actor scope. Once the SPA
//! migrated to httpOnly cookies (`swings_access`), every retry from a
//! cookie-only client silently bypassed the cache because
//! `decode_subject` returned `None`. The middleware then treated the
//! second POST as a brand-new request and re-ran the side effect.
//!
//! This file proves the fix is live: a cookie-only client that POSTs
//! the same `(Idempotency-Key, body)` twice must observe a cached
//! replay (flagged with `Idempotency-Replayed: true`) and the
//! underlying `orders` table must contain exactly one row.

mod support;

use axum::http::StatusCode;
use serde_json::{json, Value};
use support::TestApp;
use uuid::Uuid;

/// Pull `swings_access=<value>` out of the `Set-Cookie` headers minted by
/// `/api/auth/login`. The harness already exercises the cookie-shape
/// assertions in `auth_cookies.rs`; here we only need the raw value.
fn access_cookie_value(headers: &axum::http::HeaderMap) -> String {
    for raw in headers.get_all(axum::http::header::SET_COOKIE).iter() {
        let Ok(s) = raw.to_str() else { continue };
        if let Some(rest) = s.strip_prefix("swings_access=") {
            // `name=value; Attr1; …` → keep up to the first `;`.
            let value = rest.split(';').next().unwrap_or(rest);
            return value.to_string();
        }
    }
    panic!(
        "expected a swings_access Set-Cookie on the /api/auth/login response, got {:?}",
        headers
            .get_all(axum::http::header::SET_COOKIE)
            .iter()
            .filter_map(|v| v.to_str().ok())
            .collect::<Vec<_>>()
    );
}

async fn seed_product(app: &TestApp) -> Uuid {
    let id = Uuid::new_v4();
    sqlx::query(
        r#"
        INSERT INTO products
            (id, slug, name, product_type, status, price_cents, currency)
        VALUES ($1, $2, 'Cookie Carry Product', 'simple', 'published', 2500, 'USD')
        "#,
    )
    .bind(id)
    .bind(format!("cookie-prod-{}", &id.to_string()[..8]))
    .execute(app.db())
    .await
    .expect("seed product");
    id
}

#[tokio::test]
async fn cookie_only_replay_returns_cached_response_and_does_not_re_create() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };

    // Mint a real BFF session: hit /api/auth/login so the server stamps a
    // genuine `swings_access` cookie. Round-tripping login proves the
    // cache key works against the same JWT shape production mints.
    let admin = app.seed_admin().await.expect("seed admin");
    let login = app
        .post_json(
            "/api/auth/login",
            &json!({ "email": admin.email, "password": admin.password }),
            None,
        )
        .await;
    login.assert_status(StatusCode::OK);
    let cookie_value = access_cookie_value(login.headers());

    let product_id = seed_product(&app).await;
    let body = json!({
        "email": "cookie-buyer@example.com",
        "currency": "usd",
        "items": [{
            "product_id": product_id,
            "quantity": 1,
            "unit_price_cents": 2500,
            "name": "Cookie Item",
        }],
    });
    let key = "cookie-carry-replay-1";

    let first = app
        .post_json_with_cookie_and_idempotency_key(
            "/api/admin/orders",
            &body,
            "swings_access",
            &cookie_value,
            key,
        )
        .await;
    first.assert_status(StatusCode::CREATED);
    let first_body: Value = first.json().expect("first body");
    let first_order_id = first_body["order"]["id"]
        .as_str()
        .expect("order id present")
        .to_string();
    assert_eq!(
        first.header("idempotency-replayed"),
        None,
        "the first call must NOT be flagged as a replay"
    );

    let second = app
        .post_json_with_cookie_and_idempotency_key(
            "/api/admin/orders",
            &body,
            "swings_access",
            &cookie_value,
            key,
        )
        .await;
    second.assert_status(StatusCode::CREATED);
    let second_body: Value = second.json().expect("second body");
    let second_order_id = second_body["order"]["id"]
        .as_str()
        .expect("replayed order id present")
        .to_string();

    assert_eq!(
        first_order_id, second_order_id,
        "cookie-only replay must echo the original order id, not create a new one"
    );
    assert_eq!(
        second.header("idempotency-replayed"),
        Some("true"),
        "cookie-only replay must carry the Idempotency-Replayed marker — \
         absence proves the middleware bypassed caching (the original CONTRACT-D2 bug)"
    );

    let count: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM orders WHERE email = 'cookie-buyer@example.com'")
            .fetch_one(app.db())
            .await
            .expect("count orders");
    assert_eq!(
        count.0, 1,
        "exactly one order row must exist after the cookie-only replay (got {})",
        count.0
    );
}
