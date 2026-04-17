//! In-app notification channel stub.
//!
//! A persistent `in_app_notifications` inbox table lands with the member-UI
//! subsystem that consumes it. Until then the channel is a stub so the
//! trait surface is uniform.

use super::{BoxFuture, Channel, ChannelError, DeliveryRequest, ProviderId};

pub struct InAppChannel {
    _private: (),
}

impl InAppChannel {
    #[must_use]
    pub fn stub() -> Self {
        Self { _private: () }
    }
}

impl Channel for InAppChannel {
    fn name(&self) -> &'static str {
        "in_app"
    }

    fn send<'a>(
        &'a self,
        _req: &'a DeliveryRequest<'a>,
    ) -> BoxFuture<'a, Result<ProviderId, ChannelError>> {
        Box::pin(async move {
            Err(ChannelError::Permanent(
                "in_app channel is not implemented".into(),
            ))
        })
    }
}

// TODO: wire in persistent in-app inbox under the UI subsystem that renders it.
