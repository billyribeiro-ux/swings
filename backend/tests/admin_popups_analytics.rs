#![deny(warnings)]
#![forbid(unsafe_code)]

//! Integration coverage for the collection-level
//! `GET /api/admin/popups/analytics` endpoint (Phase 4.4).
//!
//! The route exists alongside the per-popup `/{id}/analytics` handler.
//! Critically the literal `/analytics` path is registered *before* the
//! `/{id}/analytics` parameterised route so axum's matcher does not try to
//! parse the string `"analytics"` as a UUID. These tests confirm both the
//! RBAC gate (support → 403) and the happy-path admin response shape.

mod support;

use axum::http::StatusCode;
use serde_json::Value;
use support::TestApp;

#[tokio::test]
async fn popup_analytics_summary_requires_admin_role() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    // Support staff lack the `popup.read_analytics` capability and the
    // `AdminUser` extractor itself rejects non-admin role tokens, so the
    // gate fires before any DB work is attempted.
    let support = app.seed_support().await.expect("seed support");
    let resp = app
        .get("/api/admin/popups/analytics", Some(&support.access_token))
        .await;
    resp.assert_status(StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn popup_analytics_summary_returns_array_for_admin() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    let admin = app.seed_admin().await.expect("seed admin");

    let resp = app
        .get("/api/admin/popups/analytics", Some(&admin.access_token))
        .await;
    resp.assert_status(StatusCode::OK);

    let body: Value = resp.json().expect("envelope");
    // Empty database → empty list. The contract says we always return an
    // array, never an object or 204.
    assert!(body.is_array(), "expected an array, got: {body:?}");
}
