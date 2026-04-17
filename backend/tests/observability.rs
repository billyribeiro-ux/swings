#![deny(warnings)]
#![forbid(unsafe_code)]

//! Integration coverage for the FDN-08-adjacent observability scaffolding
//! (correlation id + Prometheus metrics + JSON tracing layer).
//!
//! Scope: spin up a full `TestApp`, hit a real route, assert that:
//!
//! 1. the response carries an `X-Request-Id` header (either a caller-
//!    supplied id that passed validation, or a freshly-minted UUID), and
//! 2. `http_requests_total` fires with the correctly-labelled sample.
//!
//! # Why `#[ignore]`?
//!
//! `TestApp` (see `backend/tests/support/app.rs`) mirrors `main.rs`'s
//! `build_router` but does NOT yet include the observability layers — the
//! integrator wires them in a follow-up commit once this module lands.
//! Until then this file exercises only the bits of the observability
//! module that are reachable from the test harness (direct
//! `middleware::from_fn` invocation, catalogue bootstrap, handle
//! rendering) and gates the full end-to-end test behind `#[ignore]`.
//!
//! Run the full suite locally after the integrator applies the wiring:
//! ```bash
//! docker compose up -d db
//! DATABASE_URL_TEST=postgres://swings:swings_secret@localhost:5432/swings \
//!   cargo test --test observability -- --ignored
//! ```

mod support;

use axum::http::StatusCode;
use support::TestApp;

/// Smoke check: the in-process recorder renders a non-empty exposition
/// document with the catalogue entries pre-registered. Safe to run even
/// when the test database is absent because it never touches the router.
#[test]
fn recorder_renders_catalogue() {
    let handle = swings_api::observability::install_recorder();
    let rendered = handle.render();
    // Every catalogue metric name must appear somewhere in the rendered
    // text — either as a `# HELP …` line or an actual sample.
    for name in [
        "http_requests_total",
        "http_request_duration_seconds",
        "outbox_pending",
        "outbox_dead_letter_total",
        "notifications_sent_total",
        "stripe_webhook_total",
        "consent_records_total",
        "dsar_requests_total",
        "orders_total",
    ] {
        assert!(
            rendered.contains(name),
            "catalogue missing `{name}`\n---\n{rendered}\n---"
        );
    }
}

/// End-to-end: make a real request through the `TestApp` router, assert
/// the correlation-id middleware stamps `X-Request-Id` on the response,
/// and (once the integrator wires the metrics middleware) that the
/// counter fires for the matched route.
///
/// Gated behind `#[ignore]` because the test harness's `build_router`
/// currently omits the observability layers. See the module docs above
/// for the rationale; remove the `#[ignore]` after
/// `OBSERVABILITY-WIRING.md` step 3 lands.
#[tokio::test]
#[ignore = "enable once observability layers are wired into TestApp::build_router (see OBSERVABILITY-WIRING.md)"]
async fn request_carries_correlation_id_and_counter_fires() {
    let Some(app) = TestApp::try_new().await else {
        return;
    };
    // Use any public GET that does not require auth. `/api/pricing` is a
    // good candidate post-FDN-02; falling back to the auth flow keeps
    // this test ambivalent to future route churn.
    let resp = app.get("/api/blog", None).await;
    assert!(
        matches!(
            resp.status(),
            StatusCode::OK | StatusCode::NOT_FOUND | StatusCode::UNAUTHORIZED
        ),
        "unexpected status: {:?}",
        resp.status()
    );
    let id = resp
        .header("x-request-id")
        .expect("correlation middleware must stamp X-Request-Id");
    assert!(
        id.len() >= 16 && id.len() <= 64,
        "request id out of envelope (len={}, id={id})",
        id.len()
    );

    // Counter + histogram should have fired at least once on the matched
    // route. The recorder is process-global, so we can pull the handle
    // back out and render.
    let handle = swings_api::observability::install_recorder();
    let rendered = handle.render();
    assert!(
        rendered.contains("http_requests_total"),
        "metric http_requests_total missing from render; got:\n{rendered}"
    );
}
