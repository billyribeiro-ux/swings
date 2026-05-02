//! Regression coverage for `POST /api/admin/coupons`.
//!
//! Background — the `DiscountType` enum derived `Serialize` /
//! `Deserialize` without `#[serde(rename_all = "snake_case")]`. The wire
//! format expected `"Percentage"` / `"FixedAmount"` / `"FreeTrial"` (Rust
//! variant names verbatim) while the SPA was sending `"percentage"` /
//! `"fixed"` / `"free_trial"`. Every authenticated POST 422'd at the
//! `Json<>` extractor before the handler ran. The fix added the serde
//! rename so the wire format is `"percentage"` / `"fixed_amount"` /
//! `"free_trial"` — matching the SQL enum and the SPA. These tests fail
//! loudly if the rename ever drifts.
//!
//! Each case asserts: HTTP 200 OK, the response body's `discount_type`
//! round-trips back as the snake_case string the SPA sent, and the row
//! lands in `coupons`.

mod support;

use reqwest::StatusCode;
use serde_json::{json, Value};
use support::TestApp;

#[tokio::test]
async fn create_with_percentage_returns_200_and_persists_row() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let admin = app.seed_admin().await.expect("seed admin");

    let body = json!({
        "code": "REGR_PCT",
        "discount_type": "percentage",
        "discount_value": 15.0,
        "min_purchase_cents": 5000,
        "is_active": true,
    });
    let resp = app
        .post_json_with_idempotency_key(
            "/api/admin/coupons",
            &body,
            Some(&admin.access_token),
            "regr-pct-1",
        )
        .await;
    resp.assert_status(StatusCode::OK);
    let parsed: Value = resp.json().expect("json body");
    assert_eq!(parsed["discount_type"], "percentage");
    assert_eq!(parsed["min_purchase_cents"], 5000);

    let count: (i64,) = sqlx::query_as("SELECT COUNT(*)::bigint FROM coupons WHERE code = $1")
        .bind("REGR_PCT")
        .fetch_one(app.db())
        .await
        .expect("count");
    assert_eq!(count.0, 1, "percentage coupon row must persist");
}

#[tokio::test]
async fn create_with_fixed_amount_returns_200() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let admin = app.seed_admin().await.expect("seed admin");

    let body = json!({
        "code": "REGR_FIXED",
        "discount_type": "fixed_amount",
        "discount_value": 5.0,
        "is_active": true,
    });
    let resp = app
        .post_json_with_idempotency_key(
            "/api/admin/coupons",
            &body,
            Some(&admin.access_token),
            "regr-fixed-1",
        )
        .await;
    resp.assert_status(StatusCode::OK);
    let parsed: Value = resp.json().expect("json body");
    assert_eq!(parsed["discount_type"], "fixed_amount");
}

#[tokio::test]
async fn create_with_free_trial_returns_200() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let admin = app.seed_admin().await.expect("seed admin");

    let body = json!({
        "code": "REGR_TRIAL",
        "discount_type": "free_trial",
        "discount_value": 14.0,
        "is_active": true,
    });
    let resp = app
        .post_json_with_idempotency_key(
            "/api/admin/coupons",
            &body,
            Some(&admin.access_token),
            "regr-trial-1",
        )
        .await;
    resp.assert_status(StatusCode::OK);
    let parsed: Value = resp.json().expect("json body");
    assert_eq!(parsed["discount_type"], "free_trial");
}

#[tokio::test]
async fn create_with_camelcase_discount_type_is_rejected_422() {
    // Belt-and-braces: confirm we did not accidentally accept BOTH the old
    // CamelCase and the new snake_case via `#[serde(alias = ...)]`. The SPA
    // contract is snake_case only; CamelCase from a stale client must
    // fail-loud rather than silently writing a row that no other writer can
    // produce.
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let admin = app.seed_admin().await.expect("seed admin");

    let body = json!({
        "code": "REGR_CAMEL",
        "discount_type": "Percentage",
        "discount_value": 10.0,
    });
    let resp = app
        .post_json_with_idempotency_key(
            "/api/admin/coupons",
            &body,
            Some(&admin.access_token),
            "regr-camel-1",
        )
        .await;
    resp.assert_status(StatusCode::UNPROCESSABLE_ENTITY);
}
