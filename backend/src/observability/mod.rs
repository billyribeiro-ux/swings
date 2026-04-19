//! FDN-08-adjacent observability scaffolding.
//!
//! This module groups together the four building blocks the ops team needs to
//! turn a running `swings-api` binary into a log-and-metric citizen of our
//! production environment:
//!
//! * [`tracing_json`] — JSON (or pretty, in dev) `tracing_subscriber` layer
//!   feeding structured logs to stdout.
//! * [`correlation`] — `X-Request-Id` passthrough middleware that binds a
//!   per-request id to the tracing span and surfaces it back to the client.
//! * [`metrics`] — Prometheus recorder installation + a Tower middleware that
//!   records `http_requests_total` and `http_request_duration_seconds` for
//!   every handler call, labelled by [`axum::extract::MatchedPath`] (bounded
//!   cardinality by construction).
//! * [`handler`] — the `/metrics` endpoint itself, admin-gated in production
//!   and publicly accessible in dev (same pattern as `/api/openapi.json`).
//!
//! See `docs/wiring/OBSERVABILITY-WIRING.md` for the exact `Cargo.toml`,
//! `main.rs`, and `lib.rs` additions the integrator needs to apply to wire
//! this module into the live binary.
//!
//! OpenTelemetry integration is deliberately left out — it's the Phase 5
//! follow-up per `docs/archive/AUDIT_PHASE3_PLAN.md` §11.

pub mod correlation;
pub mod handler;
pub mod metrics;
pub mod tracing_json;

pub use correlation::{current_request_id, RequestId};
pub use handler::metrics_handler;
pub use metrics::{http_middleware, install_recorder};
pub use tracing_json::layer as tracing_layer;
