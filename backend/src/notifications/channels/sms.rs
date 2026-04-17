//! SMS channel stub.
//!
//! FDN-05 ships the stub so the channel trait surface is complete and the
//! registry is discoverable. Real provider wiring (Twilio, Plivo, …) arrives
//! under a future subsystem.

use super::{BoxFuture, Channel, ChannelError, DeliveryRequest, ProviderId};

pub struct SmsChannel {
    _private: (),
}

impl SmsChannel {
    #[must_use]
    pub fn stub() -> Self {
        Self { _private: () }
    }
}

impl Channel for SmsChannel {
    fn name(&self) -> &'static str {
        "sms"
    }

    fn send<'a>(
        &'a self,
        _req: &'a DeliveryRequest<'a>,
    ) -> BoxFuture<'a, Result<ProviderId, ChannelError>> {
        Box::pin(async move {
            Err(ChannelError::Permanent(
                "sms channel is not implemented".into(),
            ))
        })
    }
}

// TODO: wire in Twilio (behind feature flag) under a later subsystem.
