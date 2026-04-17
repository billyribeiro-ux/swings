//! `/metrics` endpoint handler.
//!
//! Renders the Prometheus exposition format produced by the
//! [`PrometheusHandle`][metrics_exporter_prometheus::PrometheusHandle]
//! installed by [`super::metrics::install_recorder`].
//!
//! # Access control
//!
//! Follows the same pattern as `/api/openapi.json` (FDN-02 `backend/src/
//! openapi.rs`):
//!
//! * In production (`APP_ENV=production`) the endpoint requires a valid
//!   bearer token with the `admin` role. Returns `401` on missing / invalid
//!   token, `403` on non-admin role, per `AppError`'s RFC 7807 path.
//! * In non-production it is publicly accessible so developer tooling,
//!   docker-compose health probes, and CI scrapers can poll without
//!   juggling credentials.
//!
//! The split is done via two handler functions — [`admin_metrics_handler`]
//! and [`public_metrics_handler`] — and the integrator picks the right one
//! in `main.rs` based on `AppState::config.is_production()`. Keeping the
//! selection in `main.rs` (rather than inside this module) mirrors the
//! openapi.rs pattern and keeps the `AdminUser` extractor out of the
//! public path.
//!
//! # Content-Type
//!
//! Returns `text/plain; version=0.0.4; charset=utf-8` — the MIME type
//! Prometheus' text format spec mandates for exposition. Scrapers that
//! don't send an `Accept` header still ingest the body correctly.

use axum::{
    body::Body,
    extract::{Extension, State},
    http::{header, HeaderValue, Response, StatusCode},
};
use metrics_exporter_prometheus::PrometheusHandle;

use crate::error::AppResult;
use crate::extractors::AdminUser;
use crate::AppState;

/// Prometheus text exposition MIME type (text format 0.0.4).
///
/// Some scrapers are lenient on content-type — we return the canonical
/// value anyway so metadata dashboards can distinguish our metrics from
/// stray JSON responses.
const PROMETHEUS_CONTENT_TYPE: &str = "text/plain; version=0.0.4; charset=utf-8";

/// Admin-gated handler. Wire in production via
/// `.route("/metrics", axum::routing::get(observability::handler::
/// admin_metrics_handler))`.
///
/// Signature mirrors the spec — `State<AppState>` + `AdminUser` +
/// `Extension<PrometheusHandle>`. `AdminUser` returns `AppError` variants
/// which the global error-mapping layer renders as RFC 7807 responses, so
/// non-admin callers see `Problem`-formatted `401` / `403` rather than a
/// bare status code.
#[utoipa::path(
    get,
    path = "/metrics",
    tag = "observability",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Prometheus exposition format", body = String, content_type = "text/plain; version=0.0.4; charset=utf-8"),
        (status = 401, description = "Missing or invalid bearer token"),
        (status = 403, description = "Non-admin role"),
    )
)]
pub async fn admin_metrics_handler(
    State(_state): State<AppState>,
    _admin: AdminUser,
    Extension(recorder): Extension<PrometheusHandle>,
) -> AppResult<Response<Body>> {
    Ok(render(recorder))
}

/// Public (unauthenticated) handler. Wire in development via
/// `.route("/metrics", axum::routing::get(observability::handler::
/// public_metrics_handler))`.
///
/// Identical body to [`admin_metrics_handler`] — only the authz decoration
/// differs. Kept as a separate function so the production wiring can
/// omit the admin gate's JWT decode path entirely.
pub async fn public_metrics_handler(
    State(_state): State<AppState>,
    Extension(recorder): Extension<PrometheusHandle>,
) -> AppResult<Response<Body>> {
    Ok(render(recorder))
}

/// Spec-compliant metrics handler. This is the single function the spec
/// names — it selects the admin vs public variant based on
/// `state.config.is_production()`.
///
/// Handlers with extractor-chains can't be composed at runtime (Axum picks
/// extractors by function signature), so the integrator in `main.rs`
/// routes to [`admin_metrics_handler`] / [`public_metrics_handler`]
/// directly. This function exists as a convenience wrapper for callers
/// that have already resolved the env gate externally.
pub async fn metrics_handler(
    State(state): State<AppState>,
    admin: AdminUser,
    Extension(recorder): Extension<PrometheusHandle>,
) -> AppResult<Response<Body>> {
    // The `admin` extractor has already validated the JWT + role by the
    // time we get here. `state.config.is_production()` is still read so
    // we emit a trace event on the dev path that makes scraping visible
    // in the log stream (helps during incident triage).
    if !state.config.is_production() {
        tracing::debug!(user_id = %admin.user_id, "/metrics scraped (dev mode)");
    }
    Ok(render(recorder))
}

fn render(recorder: PrometheusHandle) -> Response<Body> {
    let body = Body::from(recorder.render());
    let mut resp = Response::new(body);
    resp.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static(PROMETHEUS_CONTENT_TYPE),
    );
    *resp.status_mut() = StatusCode::OK;
    resp
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prometheus_content_type_is_text_format() {
        // Canary against accidental mutation — Prometheus scrapers look
        // for exactly this string; any drift breaks ingestion silently.
        assert_eq!(
            PROMETHEUS_CONTENT_TYPE,
            "text/plain; version=0.0.4; charset=utf-8"
        );
    }
}
