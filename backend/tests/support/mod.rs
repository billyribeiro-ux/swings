//! Backend integration-test harness.
//!
//! The support module is intended to be imported from any `tests/<name>.rs`
//! integration test via:
//!
//! ```rust,ignore
//! mod support;
//! use support::{TestApp, TestUser};
//!
//! #[tokio::test]
//! async fn my_integration() {
//!     let Some(app) = support::TestApp::try_new().await else { return };
//!     let user = app.seed_user().await.expect("seed");
//!     let resp = app.post_json("/api/auth/login", &serde_json::json!({
//!         "email": user.email,
//!         "password": user.password,
//!     }), None).await;
//!     resp.assert_status(axum::http::StatusCode::OK);
//! }
//! ```
//!
//! Each [`TestApp`] owns:
//!
//! * a dedicated schema in Postgres (via [`TestDb`]) so every test gets a
//!   pristine database sandbox,
//! * an `AppState` built from the environment with a noop email service and a
//!   temporary `MediaBackend::Local` upload directory,
//!   and
//! * a fresh `axum::Router` that mirrors the route nesting in `main.rs`.
//!
//! The tests in `backend/tests/` run in parallel by default; because each
//! [`TestApp`] is scoped to its own schema, they do not interfere.
//!
//! See `FDN-TESTHARNESS-WIRING.md` at the repo root for environment
//! prerequisites and a cleanup script for orphaned schemas.

// `cargo test` compiles each `tests/<name>.rs` as its own crate; any helper
// a given test does not exercise shows up as a `dead_code` /
// `unused_imports` warning that `#![deny(warnings)]` turns into an error.
// The harness is deliberately buffet-style, so suppress the noise here.
#![allow(dead_code, unused_imports)]

pub mod app;
pub mod db;
pub mod error;
pub mod response;
pub mod user;

pub use app::TestApp;
pub use db::TestDb;
pub use error::{TestAppError, TestResult};
pub use response::{AssertProblem, ProblemBody, TestResponse};
pub use user::{TestRole, TestUser};

/// Returns `true` if a test database URL is configured, `false` otherwise.
///
/// Integration tests call this as a fast-path skip so the whole suite still
/// passes in environments without a Postgres (e.g. a CI step that only
/// compiles) — callers should `return` early and print a descriptive message
/// when it returns `false`.
pub fn has_test_database() -> bool {
    std::env::var("DATABASE_URL_TEST")
        .or_else(|_| std::env::var("DATABASE_URL"))
        .is_ok()
}
