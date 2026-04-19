//! ADM-06: gate `/api/admin/*` traffic by source IP.
//!
//! Behaviour matches the migration contract:
//!
//!   * Empty allowlist → pass through unconditionally. Avoids locking
//!     fresh installs out before the operator can add their first entry.
//!   * Non-empty allowlist → caller's IP MUST fall inside one of the
//!     active CIDR ranges. Mismatches return `403 Forbidden` and are
//!     logged at WARN.
//!   * Database / lookup errors → fail open (logged at WARN). The
//!     allowlist is a defence-in-depth layer, not the primary auth gate;
//!     an outage of the allowlist table must not take the admin UI
//!     offline.
//!
//! The middleware uses the same IP-extraction precedence as
//! [`crate::extractors::ClientInfo`] (X-Forwarded-For → X-Real-IP →
//! `ConnectInfo` peer) so audit-log entries and allowlist decisions key
//! on identical values.

use std::net::{IpAddr, SocketAddr};

use axum::{
    extract::{ConnectInfo, Request, State},
    http::HeaderMap,
    middleware::Next,
    response::Response,
};

use crate::{error::AppError, security::ip_allowlist, AppState};

/// Axum-compatible middleware fn. Mount via
/// `axum::middleware::from_fn_with_state(state.clone(), enforce)`.
pub async fn enforce(
    State(state): State<AppState>,
    request: Request,
    next: Next,
) -> Result<Response, AppError> {
    let headers = request.headers();
    let connect_info = request.extensions().get::<ConnectInfo<SocketAddr>>();
    let ip = match resolve_client_ip(headers, connect_info) {
        Some(ip) => ip,
        None => {
            // No client IP available at all — allow the request so local
            // tooling and unit-test harnesses without ConnectInfo aren't
            // accidentally blocked. Production traffic always has one.
            tracing::debug!("admin_ip_allowlist: no client IP available; allowing");
            return Ok(next.run(request).await);
        }
    };

    let allowed = ip_allowlist::is_ip_allowed(&state.db, ip).await;
    if !allowed {
        let path = request.uri().path().to_string();
        let user_agent = headers
            .get(axum::http::header::USER_AGENT)
            .and_then(|v| v.to_str().ok())
            .map(str::to_owned);

        // Structured log + metric. We deliberately do NOT write to
        // `admin_actions` because that table FKs `actor_id ->
        // users(id) ON DELETE RESTRICT` — a rejected request never
        // authenticated, so there is no actor to attribute. SIEMs
        // should ingest the structured log via the JSON tracing
        // exporter; alerting fires off the metric counter.
        tracing::warn!(
            event = "admin_ip_allowlist.rejected",
            client_ip = %ip,
            path = %path,
            user_agent = %user_agent.as_deref().unwrap_or("-"),
            "admin_ip_allowlist: rejected request from non-allowlisted IP"
        );
        metrics::counter!(
            "admin_ip_allowlist_rejections_total",
            "path" => path,
        )
        .increment(1);

        return Err(AppError::Forbidden);
    }

    Ok(next.run(request).await)
}

/// Mirror of `crate::extractors::ClientInfo`'s IP precedence so admin
/// audit entries and allowlist decisions compare like-for-like values.
fn resolve_client_ip(
    headers: &HeaderMap,
    connect_info: Option<&ConnectInfo<SocketAddr>>,
) -> Option<IpAddr> {
    if let Some(forwarded) = headers.get("X-Forwarded-For").and_then(|v| v.to_str().ok()) {
        if let Some(first) = forwarded.split(',').next() {
            if let Ok(ip) = first.trim().parse::<IpAddr>() {
                return Some(ip);
            }
        }
    }
    if let Some(real_ip) = headers.get("X-Real-IP").and_then(|v| v.to_str().ok()) {
        if let Ok(ip) = real_ip.trim().parse::<IpAddr>() {
            return Some(ip);
        }
    }
    connect_info.map(|ci| ci.0.ip())
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::HeaderValue;

    fn headers(pairs: &[(&'static str, &str)]) -> HeaderMap {
        let mut h = HeaderMap::new();
        for (k, v) in pairs {
            h.insert(*k, HeaderValue::from_str(v).unwrap());
        }
        h
    }

    #[test]
    fn resolves_xff_first() {
        let h = headers(&[("X-Forwarded-For", "203.0.113.7, 198.51.100.1")]);
        let peer = ConnectInfo("10.0.0.1:1234".parse::<SocketAddr>().unwrap());
        let ip = resolve_client_ip(&h, Some(&peer)).unwrap();
        assert_eq!(ip.to_string(), "203.0.113.7");
    }

    #[test]
    fn falls_back_to_real_ip() {
        let h = headers(&[("X-Real-IP", "203.0.113.42")]);
        let peer = ConnectInfo("10.0.0.1:1234".parse::<SocketAddr>().unwrap());
        let ip = resolve_client_ip(&h, Some(&peer)).unwrap();
        assert_eq!(ip.to_string(), "203.0.113.42");
    }

    #[test]
    fn falls_back_to_connect_info() {
        let h = HeaderMap::new();
        let peer = ConnectInfo("10.0.0.5:443".parse::<SocketAddr>().unwrap());
        let ip = resolve_client_ip(&h, Some(&peer)).unwrap();
        assert_eq!(ip.to_string(), "10.0.0.5");
    }

    #[test]
    fn returns_none_without_any_source() {
        let h = HeaderMap::new();
        assert!(resolve_client_ip(&h, None).is_none());
    }
}
