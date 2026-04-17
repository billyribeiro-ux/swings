//! POP-06: outbox subscriber that writes popup revenue attribution.
//!
//! Listens on `order.completed` and `subscription.started`. The payload is
//! expected to carry `session_id`, `amount_cents`, and `currency` (plus the
//! aggregate id itself). Missing fields are treated as a `Permanent`
//! failure so malformed events go to DLQ for investigation rather than
//! retrying forever.
//!
//! The attribution window defaults to 24h; override with the
//! `ATTRIBUTION_WINDOW_HOURS` env var.

use std::str::FromStr;

use chrono::{Duration, Utc};
use sqlx::PgPool;
use tracing::warn;
use uuid::Uuid;

use super::super::dispatcher::DispatchError;
use super::super::outbox::OutboxRecord;
use super::{BoxFuture, EventHandler};
use crate::popups::attribution::{self, RevenueEvent, DEFAULT_WINDOW_HOURS};

/// Event subscriber that turns revenue events into `popup_attributions`
/// rows.
#[derive(Debug)]
pub struct PopupAttributionHandler {
    pool: PgPool,
    window_hours: i64,
}

impl PopupAttributionHandler {
    pub fn new(pool: PgPool) -> Self {
        let window_hours = std::env::var("ATTRIBUTION_WINDOW_HOURS")
            .ok()
            .and_then(|v| i64::from_str(&v).ok())
            .filter(|v| *v > 0)
            .unwrap_or(DEFAULT_WINDOW_HOURS);
        Self { pool, window_hours }
    }
}

impl EventHandler for PopupAttributionHandler {
    fn handle<'a>(&'a self, event: &'a OutboxRecord) -> BoxFuture<'a, Result<(), DispatchError>> {
        Box::pin(async move {
            // Only react to the two event types we attribute. Other
            // events pattern-match into this handler if the registration
            // glob is broader — just return Ok so the worker advances.
            match event.event_type.as_str() {
                "order.completed" | "subscription.started" => {}
                _ => return Ok(()),
            }

            let session_id = match extract_uuid(&event.payload, "session_id") {
                Some(s) => s,
                None => {
                    // Missing session id = cannot attribute. Not an error,
                    // just a no-op: most orders will not carry one and
                    // that is fine.
                    return Ok(());
                }
            };

            let amount_cents = event
                .payload
                .get("amount_cents")
                .and_then(|v| v.as_i64())
                .ok_or_else(|| {
                    DispatchError::Permanent(format!(
                        "popup_attribution: {} missing amount_cents",
                        event.event_type
                    ))
                })?;

            let currency = event
                .payload
                .get("currency")
                .and_then(|v| v.as_str())
                .unwrap_or("USD")
                .to_string();

            let (order_id, subscription_id) = match event.event_type.as_str() {
                "order.completed" => (extract_uuid(&event.payload, "order_id"), None),
                "subscription.started" => (None, extract_uuid(&event.payload, "subscription_id")),
                _ => (None, None),
            };

            let rev = RevenueEvent {
                session_id,
                order_id,
                subscription_id,
                amount_cents,
            };
            let window = Duration::hours(self.window_hours);
            attribution::attribute_order(&self.pool, rev, &currency, window, Utc::now())
                .await
                .map_err(|e| {
                    // DB errors are transient — the worker will retry.
                    warn!(error = %e, "popup_attribution write failed; retrying");
                    DispatchError::Transient(format!("popup_attribution: {e}"))
                })?;

            Ok(())
        })
    }
}

fn extract_uuid(payload: &serde_json::Value, key: &str) -> Option<Uuid> {
    payload
        .get(key)
        .and_then(|v| v.as_str())
        .and_then(|s| Uuid::parse_str(s).ok())
}
