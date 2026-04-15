//! Per-route rate limits (IP-based via [`tower_governor::key_extractor::SmartIpKeyExtractor`]).
//!
//! **Trust note:** `SmartIpKeyExtractor` reads `X-Forwarded-For` / `Forwarded`. Only enable that
//! behavior when your reverse proxy (e.g. Railway) strips or overwrites client-supplied values.

use std::time::Duration;

use axum::body::Body;
use governor::middleware::NoOpMiddleware;
use tower_governor::{
    governor::GovernorConfigBuilder,
    key_extractor::SmartIpKeyExtractor,
    GovernorLayer,
};

type AuthGovernorLayer = GovernorLayer<SmartIpKeyExtractor, NoOpMiddleware, Body>;

fn ip_layer(period: Duration, burst: u32) -> AuthGovernorLayer {
    let mut b = GovernorConfigBuilder::default();
    b.period(period);
    b.burst_size(burst);
    GovernorLayer::new(b.key_extractor(SmartIpKeyExtractor).finish().expect("non-zero quota"))
}

/// ~5 requests per minute per IP (burst 5, one token every 12s).
pub fn login_layer() -> AuthGovernorLayer {
    ip_layer(Duration::from_secs(12), 5)
}

/// 10 requests per hour per IP (burst 10, one token every 360s).
pub fn register_layer() -> AuthGovernorLayer {
    ip_layer(Duration::from_secs(360), 10)
}

/// 3 requests per hour per IP (burst 3, one token every 1200s).
pub fn forgot_password_layer() -> AuthGovernorLayer {
    ip_layer(Duration::from_secs(1200), 3)
}

/// Public analytics ingest: generous limit to avoid dropping legitimate SPA traffic (~120/min per IP).
pub fn analytics_ingest_layer() -> AuthGovernorLayer {
    ip_layer(Duration::from_secs(1), 120)
}
