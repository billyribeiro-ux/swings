//! Outbound webhook channel stub.
//!
//! Distinct from [`crate::events::handlers::webhook_out::WebhookOutHandler`]
//! (the outbox subscriber). This channel is for human-facing notifications
//! delivered as an HTTP POST; the event-fanout handler is for machine-to-
//! machine integrations.

use super::{BoxFuture, Channel, ChannelError, DeliveryRequest, ProviderId};

pub struct WebhookChannel {
    _private: (),
}

impl WebhookChannel {
    #[must_use]
    pub fn stub() -> Self {
        Self { _private: () }
    }
}

impl Channel for WebhookChannel {
    fn name(&self) -> &'static str {
        "webhook"
    }

    fn send<'a>(
        &'a self,
        _req: &'a DeliveryRequest<'a>,
    ) -> BoxFuture<'a, Result<ProviderId, ChannelError>> {
        Box::pin(async move {
            Err(ChannelError::Permanent(
                "webhook channel is not implemented".into(),
            ))
        })
    }
}

// TODO: wire in outbound HTTP POST under a future subsystem (e.g. FORM-07 inbox
// integrations).
