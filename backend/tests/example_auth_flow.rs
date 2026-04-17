#![deny(warnings)]
#![forbid(unsafe_code)]

//! Exemplar integration test for the `backend/tests/support/` harness.
//!
//! Double-booked as:
//!
//! 1. a living example any future test author can copy/paste, and
//! 2. a sanity check that the harness's `build_router`, schema isolation,
//!    and token-minting stay in sync with `src/main.rs` + `handlers::auth`.
//!
//! This test is **skipped** when neither `DATABASE_URL_TEST` nor
//! `DATABASE_URL` is set, so the suite keeps running green on machines
//! without a local Postgres (e.g. pre-push hook boxes).
//!
//! Run it locally with:
//! ```bash
//! docker compose up -d db
//! DATABASE_URL_TEST=postgres://swings:swings_secret@localhost:5432/swings \
//!   cargo test --test example_auth_flow
//! ```

mod support;

use axum::http::StatusCode;
use serde_json::json;
use support::{AssertProblem, TestApp};

#[tokio::test]
async fn auth_flow_happy_and_unhappy_paths() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };

    let user = app.seed_user().await.expect("seed member");

    // --- Login with a bad password → 401 RFC 7807 Problem ---
    let resp = app
        .post_json(
            "/api/auth/login",
            &json!({ "email": user.email, "password": "definitely-not-the-password" }),
            None,
        )
        .await;
    resp.assert_problem(AssertProblem {
        status: StatusCode::UNAUTHORIZED,
        type_suffix: "unauthorized",
        title: "Unauthorized",
    });

    // --- Login with the correct password → 200 + AuthResponse body ---
    let resp = app
        .post_json(
            "/api/auth/login",
            &json!({ "email": user.email, "password": user.password }),
            None,
        )
        .await;
    resp.assert_status(StatusCode::OK);
    let body: serde_json::Value = resp.json().expect("auth body JSON");
    let access_token = body["access_token"]
        .as_str()
        .expect("access_token in auth body");
    assert!(
        !access_token.is_empty(),
        "access_token should be non-empty; body={body}"
    );
    assert_eq!(
        body["user"]["email"].as_str(),
        Some(user.email.as_str()),
        "login echo user.email mismatch; body={body}"
    );

    // --- /me with the fresh access token → 200 ---
    let resp = app.get("/api/auth/me", Some(access_token)).await;
    resp.assert_status(StatusCode::OK);
    let me: serde_json::Value = resp.json().expect("me body JSON");
    assert_eq!(me["id"].as_str(), Some(user.id.to_string().as_str()));
    assert_eq!(me["email"].as_str(), Some(user.email.as_str()));

    // --- /me without a token → 401 Problem ---
    let resp = app.get("/api/auth/me", None).await;
    resp.assert_problem(AssertProblem {
        status: StatusCode::UNAUTHORIZED,
        type_suffix: "unauthorized",
        title: "Unauthorized",
    });

    // --- Seeded refresh token is also good. The harness's `TestUser` carries
    //     the same token the DB row was inserted with, so the rotation path
    //     succeeds and hands back a new pair. This doubles as a check that
    //     schema-pinned `search_path` is carrying through every pool
    //     connection (the `refresh_tokens` lookup would 401 otherwise).
    let resp = app
        .post_json(
            "/api/auth/refresh",
            &json!({ "refresh_token": user.refresh_token }),
            None,
        )
        .await;
    resp.assert_status(StatusCode::OK);
    let rotated: serde_json::Value = resp.json().expect("refresh body");
    assert!(
        rotated["access_token"].is_string() && rotated["refresh_token"].is_string(),
        "expected rotated token pair; body={rotated}"
    );
}
