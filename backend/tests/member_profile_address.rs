#![deny(warnings)]
#![forbid(unsafe_code)]

//! Phase 5 integration coverage for the extended `PUT /api/member/profile`
//! body — phone + billing_address are now writable from the SPA. Asserts:
//!
//!   * A full billing address persists across columns and survives the
//!     COALESCE overlay (no clobbering of unset fields).
//!   * Country codes are normalised to upper-case (Stripe convention).
//!   * Partial updates (just `phone`, or just `billing_address.line1`)
//!     leave the other columns untouched.

mod support;

use axum::http::StatusCode;
use serde_json::Value;
use sqlx::PgPool;
use support::TestApp;
use uuid::Uuid;

#[derive(Debug, sqlx::FromRow)]
struct BillingRow {
    phone: Option<String>,
    billing_line1: Option<String>,
    billing_line2: Option<String>,
    billing_city: Option<String>,
    billing_state: Option<String>,
    billing_postal_code: Option<String>,
    billing_country: Option<String>,
}

async fn read_billing(pool: &PgPool, user_id: Uuid) -> BillingRow {
    sqlx::query_as::<_, BillingRow>(
        r#"
        SELECT phone, billing_line1, billing_line2, billing_city, billing_state,
               billing_postal_code, billing_country
          FROM users WHERE id = $1
        "#,
    )
    .bind(user_id)
    .fetch_one(pool)
    .await
    .expect("read users billing")
}

#[tokio::test]
async fn update_profile_persists_phone_and_full_billing_address() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let user = app.seed_user().await.expect("seed");

    let resp = app
        .put_json::<Value>(
            "/api/member/profile",
            &serde_json::json!({
                "phone": "+1-415-555-0123",
                "billing_address": {
                    "line1": "1 Market St",
                    "line2": "Suite 200",
                    "city": "San Francisco",
                    "state": "CA",
                    "postal_code": "94105",
                    "country": "us"
                }
            }),
            Some(&user.access_token),
        )
        .await;
    resp.assert_status(StatusCode::OK);

    let row = read_billing(app.db(), user.id).await;
    assert_eq!(row.phone.as_deref(), Some("+1-415-555-0123"));
    assert_eq!(row.billing_line1.as_deref(), Some("1 Market St"));
    assert_eq!(row.billing_line2.as_deref(), Some("Suite 200"));
    assert_eq!(row.billing_city.as_deref(), Some("San Francisco"));
    assert_eq!(row.billing_state.as_deref(), Some("CA"));
    assert_eq!(row.billing_postal_code.as_deref(), Some("94105"));
    // Country normalised to upper-case (Stripe convention).
    assert_eq!(row.billing_country.as_deref(), Some("US"));
}

#[tokio::test]
async fn partial_update_does_not_clobber_unset_fields() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let user = app.seed_user().await.expect("seed");

    // Initial PUT: full address.
    app.put_json::<Value>(
        "/api/member/profile",
        &serde_json::json!({
            "phone": "+1-415-555-0123",
            "billing_address": {
                "line1": "1 Market St",
                "city": "San Francisco",
                "country": "US"
            }
        }),
        Some(&user.access_token),
    )
    .await
    .assert_status(StatusCode::OK);

    // Second PUT: change ONLY phone.
    app.put_json::<Value>(
        "/api/member/profile",
        &serde_json::json!({
            "phone": "+44-20-7946-0958"
        }),
        Some(&user.access_token),
    )
    .await
    .assert_status(StatusCode::OK);

    let row = read_billing(app.db(), user.id).await;
    assert_eq!(row.phone.as_deref(), Some("+44-20-7946-0958"));
    // Address fields must survive the partial update.
    assert_eq!(row.billing_line1.as_deref(), Some("1 Market St"));
    assert_eq!(row.billing_city.as_deref(), Some("San Francisco"));
    assert_eq!(row.billing_country.as_deref(), Some("US"));
}

#[tokio::test]
async fn billing_address_partial_overlay_within_object() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let user = app.seed_user().await.expect("seed");

    // Seed address.
    app.put_json::<Value>(
        "/api/member/profile",
        &serde_json::json!({
            "billing_address": {
                "line1": "Old Line 1",
                "line2": "Old Line 2",
                "city": "OldCity",
                "country": "US"
            }
        }),
        Some(&user.access_token),
    )
    .await
    .assert_status(StatusCode::OK);

    // Send only `line1` inside the address — `line2` and `city` must
    // survive because the COALESCE-overlay treats absent inner fields
    // as "no change".
    app.put_json::<Value>(
        "/api/member/profile",
        &serde_json::json!({
            "billing_address": { "line1": "New Line 1" }
        }),
        Some(&user.access_token),
    )
    .await
    .assert_status(StatusCode::OK);

    let row = read_billing(app.db(), user.id).await;
    assert_eq!(row.billing_line1.as_deref(), Some("New Line 1"));
    assert_eq!(row.billing_line2.as_deref(), Some("Old Line 2"));
    assert_eq!(row.billing_city.as_deref(), Some("OldCity"));
    assert_eq!(row.billing_country.as_deref(), Some("US"));
}

#[tokio::test]
async fn update_profile_response_includes_billing_in_user_payload() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let user = app.seed_user().await.expect("seed");

    let resp = app
        .put_json::<Value>(
            "/api/member/profile",
            &serde_json::json!({
                "phone": "+1-415-555-0001",
                "billing_address": {
                    "line1": "100 Embarcadero",
                    "country": "us"
                }
            }),
            Some(&user.access_token),
        )
        .await;
    resp.assert_status(StatusCode::OK);
    let body: Value = resp.json().expect("body");
    assert_eq!(body["phone"], serde_json::json!("+1-415-555-0001"));
    assert_eq!(body["billing_line1"], serde_json::json!("100 Embarcadero"));
    assert_eq!(body["billing_country"], serde_json::json!("US"));
}
