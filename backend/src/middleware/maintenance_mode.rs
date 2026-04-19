//! ADM-08: maintenance-mode gate.
//!
//! Reads three settings from the in-memory cache and short-circuits
//! the request when maintenance is active:
//!
//! * `system.maintenance_mode`        (bool, default false)
//! * `system.maintenance_message`     (string, default "We are
//!   performing scheduled maintenance.")
//! * `system.maintenance_admin_only`  (bool, default true) — when
//!   true, requests under `/api/admin/*` continue to flow so
//!   operators can disable maintenance from the same UI that
//!   triggered it. When false, even admin routes are blocked,
//!   except `/api/admin/settings/*` which always remains reachable
//!   so the kill-switch is not self-locking.
//!
//! Response is `503 Service Unavailable` with a `Retry-After: 120`
//! hint, content type `application/problem+json`. The shape mirrors
//! [`crate::error::AppError::ServiceUnavailable`] so SPA error
//! handling does not need a special case.

use axum::{
    extract::{Request, State},
    http::{header, HeaderValue, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};

use crate::{
    settings::{KEY_MAINTENANCE_ADMIN_ONLY, KEY_MAINTENANCE_MESSAGE, KEY_MAINTENANCE_MODE},
    AppState,
};

/// Path prefix that always remains reachable, even when
/// `maintenance_admin_only=false`. Operators must always be able to
/// flip the kill-switch back off.
const ESCAPE_HATCH: &str = "/api/admin/settings";
const ADMIN_PREFIX: &str = "/api/admin";

pub async fn enforce(
    State(state): State<AppState>,
    request: Request,
    next: Next,
) -> Response {
    if !state.settings.get_bool(KEY_MAINTENANCE_MODE, false) {
        return next.run(request).await;
    }

    let path = request.uri().path();

    // Always let the kill-switch route through, full stop.
    if path.starts_with(ESCAPE_HATCH) {
        return next.run(request).await;
    }

    let admin_only = state.settings.get_bool(KEY_MAINTENANCE_ADMIN_ONLY, true);
    if admin_only && path.starts_with(ADMIN_PREFIX) {
        return next.run(request).await;
    }

    let message = state.settings.get_string(
        KEY_MAINTENANCE_MESSAGE,
        "We are performing scheduled maintenance. We will be back shortly.",
    );

    metrics::counter!("maintenance_mode_blocked_total").increment(1);

    let body = Json(serde_json::json!({
        "type":   "/problems/service-unavailable",
        "title":  "Service Unavailable",
        "status": 503,
        "detail": message,
    }));
    let mut resp = body.into_response();
    *resp.status_mut() = StatusCode::SERVICE_UNAVAILABLE;
    resp.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("application/problem+json"),
    );
    // 120s is short enough to discourage long client backoff during a
    // brief deploy, long enough to absorb a typical migration.
    resp.headers_mut()
        .insert(header::RETRY_AFTER, HeaderValue::from_static("120"));
    resp
}
