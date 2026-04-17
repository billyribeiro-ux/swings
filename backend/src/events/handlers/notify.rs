//! Notifications fan-out stub.
//!
//! In the finished system this handler bridges the outbox to
//! `crate::notifications` (FDN-05), which owns Resend / Twilio / WebPush
//! channel delivery. Until that module lands we log the event at `info` level
//! so operators can confirm the outbox plumbing is live end-to-end.

// FDN-05 TODO: replace this stub with a real adapter that calls
// `crate::notifications::send_event(event)` once the notifications crate lands.
// The adapter should translate [`OutboxRecord`] into a `NotificationRequest`
// and propagate transient provider failures as [`DispatchError::Transient`].

use tracing::info;

use super::super::dispatcher::DispatchError;
use super::super::outbox::OutboxRecord;
use super::{BoxFuture, EventHandler};

/// Stub subscriber that logs delivery. Ships at startup so the worker has at
/// least one matching handler for common event types, which keeps the
/// dispatcher happy until FDN-05 wires in the real notifier.
#[derive(Debug, Default)]
pub struct NotifyHandler;

impl NotifyHandler {
    pub fn new() -> Self {
        Self
    }
}

impl EventHandler for NotifyHandler {
    fn handle<'a>(&'a self, event: &'a OutboxRecord) -> BoxFuture<'a, Result<(), DispatchError>> {
        Box::pin(async move {
            info!(
                event_id = %event.id,
                event_type = %event.event_type,
                aggregate_type = %event.aggregate_type,
                aggregate_id = %event.aggregate_id,
                "notify handler stub: event received (FDN-05 will wire real delivery)"
            );
            Ok(())
        })
    }
}
