//! Push channel stub (WebPush / APNs / FCM).
//!
//! FDN-05 ships the stub. Real wiring comes with the WebPush VAPID
//! implementation under a later subsystem.

use super::{BoxFuture, Channel, ChannelError, DeliveryRequest, ProviderId};

pub struct PushChannel {
    _private: (),
}

impl PushChannel {
    #[must_use]
    pub fn stub() -> Self {
        Self { _private: () }
    }
}

impl Channel for PushChannel {
    fn name(&self) -> &'static str {
        "push"
    }

    fn send<'a>(
        &'a self,
        _req: &'a DeliveryRequest<'a>,
    ) -> BoxFuture<'a, Result<ProviderId, ChannelError>> {
        Box::pin(async move {
            Err(ChannelError::Permanent(
                "push channel is not implemented".into(),
            ))
        })
    }
}

// TODO: wire in WebPush VAPID under a later subsystem.
