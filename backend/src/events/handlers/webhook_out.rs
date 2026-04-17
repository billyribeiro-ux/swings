//! Outbound webhook stub.
//!
//! The finished handler will:
//! 1. Look up active subscribers from `outbox_subscribers` whose `event_pattern`
//!    matches the event type.
//! 2. Sign the payload with HMAC-SHA256 using the per-subscriber secret.
//! 3. POST to the subscriber URL with `Idempotency-Key` and `X-Swings-Signature`
//!    headers, translating HTTP 5xx / timeout into transient failures and 4xx
//!    (except 429) into permanent dead-letter failures.
//!
//! Until the outbound plumbing lands (Phase 4 FORM-07), this stub logs the
//! event and reports success, which is the safer default for the infrastructure
//! scaffolding under FDN-04.

// FORM-07 TODO: replace this stub with the HMAC-signed outbound delivery
// implementation. The real handler will:
//   - Query outbox_subscribers for active matching subscribers (cached).
//   - Build a canonical request body + timestamp + nonce signature.
//   - Respect retry-after headers on 429 / 503.
// Transient (timeout / 5xx / 429) → DispatchError::Transient; permanent (4xx
// other than 429) → DispatchError::Permanent so the row goes straight to DLQ.

use tracing::info;

use super::super::dispatcher::DispatchError;
use super::super::outbox::OutboxRecord;
use super::{BoxFuture, EventHandler};

/// Stub outbound webhook subscriber. See module docs for the FORM-07 plan.
#[derive(Debug, Default)]
pub struct WebhookOutHandler;

impl WebhookOutHandler {
    pub fn new() -> Self {
        Self
    }
}

impl EventHandler for WebhookOutHandler {
    fn handle<'a>(&'a self, event: &'a OutboxRecord) -> BoxFuture<'a, Result<(), DispatchError>> {
        Box::pin(async move {
            info!(
                event_id = %event.id,
                event_type = %event.event_type,
                aggregate_type = %event.aggregate_type,
                aggregate_id = %event.aggregate_id,
                "webhook_out handler stub: event logged (FORM-07 will wire HMAC POST)"
            );
            Ok(())
        })
    }
}
