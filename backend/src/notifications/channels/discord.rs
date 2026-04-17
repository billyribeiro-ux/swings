//! Discord channel stub.

use super::{BoxFuture, Channel, ChannelError, DeliveryRequest, ProviderId};

pub struct DiscordChannel {
    _private: (),
}

impl DiscordChannel {
    #[must_use]
    pub fn stub() -> Self {
        Self { _private: () }
    }
}

impl Channel for DiscordChannel {
    fn name(&self) -> &'static str {
        "discord"
    }

    fn send<'a>(
        &'a self,
        _req: &'a DeliveryRequest<'a>,
    ) -> BoxFuture<'a, Result<ProviderId, ChannelError>> {
        Box::pin(async move {
            Err(ChannelError::Permanent(
                "discord channel is not implemented".into(),
            ))
        })
    }
}

// TODO: wire in Discord webhook under a future subsystem.
