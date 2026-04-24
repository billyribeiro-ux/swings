//! Liveness (`/health`) and readiness (`/ready`) probes.
//!
//! Two endpoints on purpose:
//!
//! * `GET /health` — returns `200 OK` as long as the Axum process is alive
//!   and can execute a handler. Never touches Postgres / downstreams, so it
//!   stays green during DB outages — the purpose of liveness is "is the
//!   process wedged?", not "is the downstream healthy?".
//!
//! * `GET /ready` — runs a short `SELECT 1` against Postgres with a bounded
//!   timeout. Returns `200 OK` on success and `503` + RFC 7807 `Problem`
//!   body when the pool can't satisfy the probe. Orchestrators (Kubernetes,
//!   Railway, Render, ECS) should route traffic gated on this endpoint so
//!   a cold-starting instance doesn't serve 500s while migrations / pool
//!   warm-up is still in flight.
//!
//! Both handlers are intentionally unauthenticated so platform probes don't
//! need to juggle credentials; they return no secrets.

use std::time::Duration;

use axum::{extract::State, routing::get, Json, Router};
use serde::Serialize;

use crate::{
    error::{AppError, AppResult},
    AppState,
};

const READY_PROBE_TIMEOUT: Duration = Duration::from_secs(2);

#[derive(Debug, Serialize)]
pub struct HealthResponse {
    pub status: &'static str,
    /// `CARGO_PKG_VERSION` embedded at compile time. Lets operators confirm
    /// which build is serving traffic without shelling into the container.
    pub version: &'static str,
}

#[derive(Debug, Serialize)]
pub struct ReadyResponse {
    pub status: &'static str,
    pub database: &'static str,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/health", get(health))
        .route("/ready", get(ready))
        .route("/version", get(version))
}

/// Build metadata surfaced for ops / support.
///
/// Embedded at compile time by `backend/build.rs` via `rustc-env` — the
/// binary carries its own identity, so `curl /version` on a running
/// instance is enough to answer "which commit is serving traffic?"
/// without shelling in. Safe to expose publicly: no secrets, no PII.
#[derive(Debug, Serialize)]
pub struct VersionResponse {
    pub version: &'static str,
    pub git_sha: &'static str,
    pub git_sha_long: &'static str,
    pub build_time: &'static str,
    pub rust_edition: &'static str,
}

async fn version() -> Json<VersionResponse> {
    Json(VersionResponse {
        version: env!("CARGO_PKG_VERSION"),
        git_sha: env!("GIT_SHA"),
        git_sha_long: env!("GIT_SHA_LONG"),
        build_time: env!("BUILD_TIME"),
        rust_edition: "2021",
    })
}

/// Liveness probe — never fails unless the process is wedged.
async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok",
        version: env!("CARGO_PKG_VERSION"),
    })
}

/// Readiness probe — 200 iff Postgres answers a `SELECT 1` within
/// [`READY_PROBE_TIMEOUT`]. Anything else (timeout, connection refused,
/// etc.) collapses to `503 Service Unavailable` via `AppError`.
async fn ready(State(state): State<AppState>) -> AppResult<Json<ReadyResponse>> {
    let probe = sqlx::query_scalar::<_, i32>("SELECT 1").fetch_one(&state.db);
    match tokio::time::timeout(READY_PROBE_TIMEOUT, probe).await {
        Ok(Ok(_)) => Ok(Json(ReadyResponse {
            status: "ok",
            database: "ok",
        })),
        Ok(Err(err)) => {
            tracing::warn!(error = %err, "readiness probe: database query failed");
            Err(AppError::ServiceUnavailable("database unavailable".into()))
        }
        Err(_) => {
            tracing::warn!(
                timeout_ms = READY_PROBE_TIMEOUT.as_millis() as u64,
                "readiness probe: database query timed out"
            );
            Err(AppError::ServiceUnavailable(
                "database probe timed out".into(),
            ))
        }
    }
}
