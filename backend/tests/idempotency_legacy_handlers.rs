#![deny(warnings)]
#![forbid(unsafe_code)]

//! Phase 5.3 smoke coverage for the ten "legacy" admin handlers (blog,
//! courses, coupons, popups, products, pricing, forms, admin_consent,
//! notifications, outbox) that are now wrapped in the ADM-15
//! `Idempotency-Key` middleware.
//!
//! For each handler we pick **one** representative POST and assert the
//! contract that matters in production:
//!
//!   1. First call with a fresh `Idempotency-Key` → success (2xx).
//!   2. Second call with the same key + same body → byte-identical
//!      cached response, flagged with `Idempotency-Replayed: true`.
//!   3. Where the underlying mutation creates a row, exactly **one**
//!      row exists after the replay (no double-create).
//!
//! The exhaustive race / mismatch / per-actor-scoping assertions live
//! in `admin_idempotency.rs` against `/api/admin/orders`; this file's
//! job is to prove the layer is wired everywhere it needs to be, not
//! to re-prove the middleware.
//!
//! Skipped automatically when neither `DATABASE_URL_TEST` nor
//! `DATABASE_URL` is set, matching the rest of the integration suite.

mod support;

use axum::http::StatusCode;
use serde_json::{json, Value};
use support::TestApp;
use uuid::Uuid;

/// Hit `path` with `body + key` twice and assert the second response is
/// the cached replay of the first. The `expected_status` lets a handler
/// returning `200 OK` (most legacy handlers) coexist with one returning
/// `201 CREATED` (orders / subscriptions style).
///
/// Returns the first response so the caller can extract any handler-
/// specific identifiers (created row id, etc.) for the row-count
/// invariant.
async fn post_twice_assert_cached(
    app: &TestApp,
    path: &str,
    body: &Value,
    token: &str,
    key: &str,
    expected_status: StatusCode,
) -> support::TestResponse {
    let first = app
        .post_json_with_idempotency_key(path, body, Some(token), key)
        .await;
    first.assert_status(expected_status);
    assert_eq!(
        first.header("idempotency-replayed"),
        None,
        "first call to {path} must not be flagged as a replay"
    );

    let second = app
        .post_json_with_idempotency_key(path, body, Some(token), key)
        .await;
    second.assert_status(expected_status);
    assert_eq!(
        second.header("idempotency-replayed"),
        Some("true"),
        "second call to {path} with the same Idempotency-Key must be a cached replay"
    );
    assert_eq!(
        first.body_bytes(),
        second.body_bytes(),
        "replayed body for {path} must be byte-identical to the cached original"
    );

    first
}

/// Count rows in `table` matching `where_sql` bound to `value`. Used to
/// prove the side effect ran exactly once across the two POSTs.
async fn count_rows(pool: &sqlx::PgPool, table: &str, where_sql: &str, value: &str) -> i64 {
    let q = format!("SELECT COUNT(*) FROM {table} WHERE {where_sql}");
    sqlx::query_scalar::<_, i64>(&q)
        .bind(value)
        .fetch_one(pool)
        .await
        .unwrap_or_else(|e| panic!("count {table}: {e}"))
}

// ── blog ────────────────────────────────────────────────────────────────

#[tokio::test]
async fn idempotency_blog_create_post_replay_returns_cached_row() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let admin = app.seed_admin().await.expect("seed admin");
    let slug = "idem-blog-1";
    let body = json!({
        "title": "Idempotent post",
        "content": "<p>hi</p>",
        "slug": slug,
    });
    post_twice_assert_cached(
        &app,
        "/api/admin/blog/posts",
        &body,
        &admin.access_token,
        "idem-blog-key-1",
        StatusCode::OK,
    )
    .await;
    assert_eq!(
        count_rows(app.db(), "blog_posts", "slug = $1", slug).await,
        1,
        "replay must not have created a second blog_posts row"
    );
}

// ── courses ─────────────────────────────────────────────────────────────

#[tokio::test]
async fn idempotency_courses_create_replay_returns_cached_row() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let admin = app.seed_admin().await.expect("seed admin");
    let slug = "idem-course-1";
    let body = json!({ "title": "Idempotent course", "slug": slug });
    post_twice_assert_cached(
        &app,
        "/api/admin/courses",
        &body,
        &admin.access_token,
        "idem-courses-key-1",
        StatusCode::OK,
    )
    .await;
    assert_eq!(
        count_rows(app.db(), "courses", "slug = $1", slug).await,
        1,
        "replay must not have created a second courses row"
    );
}

// ── coupons ─────────────────────────────────────────────────────────────

#[tokio::test]
async fn idempotency_coupons_create_replay_returns_cached_row() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let admin = app.seed_admin().await.expect("seed admin");
    let code = "IDEMCOUPON1";
    let body = json!({
        "code": code,
        "discount_type": "percentage",
        "discount_value": 10.0,
    });
    post_twice_assert_cached(
        &app,
        "/api/admin/coupons",
        &body,
        &admin.access_token,
        "idem-coupons-key-1",
        StatusCode::OK,
    )
    .await;
    assert_eq!(
        count_rows(app.db(), "coupons", "code = $1", code).await,
        1,
        "replay must not have created a second coupons row"
    );
}

// ── popups ──────────────────────────────────────────────────────────────

#[tokio::test]
async fn idempotency_popups_create_replay_returns_cached_row() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let admin = app.seed_admin().await.expect("seed admin");
    let name = "Idem Popup 1";
    let body = json!({ "name": name });
    post_twice_assert_cached(
        &app,
        "/api/admin/popups/",
        &body,
        &admin.access_token,
        "idem-popups-key-1",
        StatusCode::OK,
    )
    .await;
    assert_eq!(
        count_rows(app.db(), "popups", "name = $1", name).await,
        1,
        "replay must not have created a second popups row"
    );
}

// ── products ────────────────────────────────────────────────────────────

#[tokio::test]
async fn idempotency_products_create_replay_returns_cached_row() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let admin = app.seed_admin().await.expect("seed admin");
    let slug = "idem-product-1";
    let body = json!({
        "slug": slug,
        "name": "Idem Product",
        "product_type": "digital",
        "price_cents": 1000,
    });
    post_twice_assert_cached(
        &app,
        "/api/admin/products",
        &body,
        &admin.access_token,
        "idem-products-key-1",
        StatusCode::OK,
    )
    .await;
    assert_eq!(
        count_rows(app.db(), "products", "slug = $1", slug).await,
        1,
        "replay must not have created a second products row"
    );
}

// ── pricing ─────────────────────────────────────────────────────────────

#[tokio::test]
async fn idempotency_pricing_create_plan_replay_returns_cached_row() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let admin = app.seed_admin().await.expect("seed admin");
    let slug = "idem-plan-1";
    let body = json!({
        "name": "Idem Plan",
        "slug": slug,
        "amount_cents": 999,
        "currency": "usd",
        "interval": "month",
    });
    post_twice_assert_cached(
        &app,
        "/api/admin/pricing/plans",
        &body,
        &admin.access_token,
        "idem-pricing-key-1",
        StatusCode::OK,
    )
    .await;
    assert_eq!(
        count_rows(app.db(), "pricing_plans", "slug = $1", slug).await,
        1,
        "replay must not have created a second pricing_plans row"
    );
}

// ── forms ───────────────────────────────────────────────────────────────
//
// `admin_bulk_update_submissions` is the only POST exposed by the forms
// admin router. Bulk updates are inherently retryable (status transitions
// are idempotent on their own), but we still assert the layer caches the
// response so a flaky retry sees the original `updated` count instead of
// re-querying with empty results.

#[tokio::test]
async fn idempotency_forms_bulk_update_replay_returns_cached_response() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let admin = app.seed_admin().await.expect("seed admin");
    let form_id = Uuid::new_v4();
    let body = json!({
        "ids": [Uuid::new_v4()],
        "action": "restore",
    });
    let path = format!("/api/admin/forms/{form_id}/submissions/bulk");
    let first = post_twice_assert_cached(
        &app,
        &path,
        &body,
        &admin.access_token,
        "idem-forms-key-1",
        StatusCode::OK,
    )
    .await;
    let parsed: Value = first.json().expect("forms bulk body");
    assert!(
        parsed.get("updated").is_some(),
        "forms bulk response must carry an `updated` count"
    );
}

// ── admin_consent ───────────────────────────────────────────────────────
//
// The banner table has a UNIQUE (region, locale) constraint, so a naive
// retry without idempotency would 5xx on the duplicate insert. With the
// middleware wired, the second call returns the cached row instead.

#[tokio::test]
async fn idempotency_consent_create_banner_replay_returns_cached_row() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let admin = app.seed_admin().await.expect("seed admin");
    // Use a unique-per-test region so concurrent integration tests do
    // not collide on the (region, locale) UNIQUE constraint.
    let region = format!("R-{}", &Uuid::new_v4().to_string()[..8]);
    let body = json!({
        "region": region,
        "locale": "en",
        "layout": "modal",
        "position": "center",
        "copy_json": {},
    });
    post_twice_assert_cached(
        &app,
        "/api/admin/consent/banners",
        &body,
        &admin.access_token,
        "idem-consent-key-1",
        StatusCode::OK,
    )
    .await;
    assert_eq!(
        count_rows(app.db(), "consent_banner_configs", "region = $1", &region).await,
        1,
        "replay must not have created a second consent_banner_configs row"
    );
}

// ── notifications ───────────────────────────────────────────────────────

#[tokio::test]
async fn idempotency_notifications_create_template_replay_returns_cached_row() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let admin = app.seed_admin().await.expect("seed admin");
    // Per-test key keeps parallel test binaries from colliding on the
    // (key, channel, locale) version sequence.
    let key = format!("idem.test.{}", &Uuid::new_v4().to_string()[..8]);
    let body = json!({
        "key": key,
        "channel": "email",
        "body_source": "<p>hi</p>",
    });
    post_twice_assert_cached(
        &app,
        "/api/admin/notifications/templates",
        &body,
        &admin.access_token,
        "idem-notifications-key-1",
        StatusCode::OK,
    )
    .await;
    assert_eq!(
        count_rows(app.db(), "notification_templates", "key = $1", &key).await,
        1,
        "replay must not have inserted a second notification_templates version"
    );
}

// ── outbox ──────────────────────────────────────────────────────────────
//
// The only POST is `/{id}/retry`. We seed a `failed` row, retry it once
// with an Idempotency-Key (transitions to `pending`), then retry it again
// with the same key — the second call must hit the cache and return the
// SAME response payload. Because the cached payload is replayed verbatim,
// the second call also bypasses the underlying UPDATE (no audit row would
// otherwise be re-emitted), which is exactly the production-safety
// guarantee Phase 5.3 wants.

#[tokio::test]
async fn idempotency_outbox_retry_replay_returns_cached_response() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let admin = app.seed_admin().await.expect("seed admin");

    let event_id = Uuid::new_v4();
    sqlx::query(
        r#"
        INSERT INTO outbox_events
            (id, aggregate_type, aggregate_id, event_type, payload,
             status, attempts, max_attempts, next_attempt_at, last_error)
        VALUES ($1, 'test', 'agg-1', 'test.event', '{}'::jsonb,
                'failed', 1, 8, NOW(), 'boom')
        "#,
    )
    .bind(event_id)
    .execute(app.db())
    .await
    .expect("seed outbox row");

    let body = json!({});
    let first = post_twice_assert_cached(
        &app,
        &format!("/api/admin/outbox/{event_id}/retry"),
        &body,
        &admin.access_token,
        "idem-outbox-key-1",
        StatusCode::OK,
    )
    .await;
    let parsed: Value = first.json().expect("outbox retry body");
    assert_eq!(
        parsed["id"].as_str().expect("retry id"),
        event_id.to_string(),
        "retry response must carry the seeded event id"
    );

    // Defence-in-depth: only one `outbox.retry` audit row should exist
    // after the replay. If the middleware were absent the second POST
    // would re-execute the handler and audit_admin would fire twice.
    let audit_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM admin_actions WHERE action = 'outbox.retry' AND target_id = $1",
    )
    .bind(event_id.to_string())
    .fetch_one(app.db())
    .await
    .expect("count audit rows");
    assert_eq!(
        audit_count, 1,
        "replay must not have re-emitted a second outbox.retry audit row"
    );
}
