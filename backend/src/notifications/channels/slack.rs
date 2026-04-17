//! Slack channel stub.

use super::{BoxFuture, Channel, ChannelError, DeliveryRequest, ProviderId};

pub struct SlackChannel {
    _private: (),
}

impl SlackChannel {
    #[must_use]
    pub fn stub() -> Self {
        Self { _private: () }
    }
}

impl Channel for SlackChannel {
    fn name(&self) -> &'static str {
        "slack"
    }

    fn send<'a>(
        &'a self,
        _req: &'a DeliveryRequest<'a>,
    ) -> BoxFuture<'a, Result<ProviderId, ChannelError>> {
        Box::pin(async move {
            Err(ChannelError::Permanent(
                "slack channel is not implemented".into(),
            ))
        })
    }
}

// TODO: wire in Slack incoming-webhook under a future subsystem.
