//! E-mail channel — wraps the existing [`EmailService`] (`lettre`/SMTP).
//!
//! This is the only concrete channel shipped in FDN-05. The trait indirection
//! means FDN-09 can drop in a Resend-backed provider behind the same public
//! surface without touching `send_notification` / the outbox worker.
//!
//! # Error mapping
//!
//! [`EmailService::send_*`] returns `Box<dyn Error>` which erases SMTP-level
//! detail. We conservatively map every error to [`ChannelError::Transient`]
//! on the first take (networks flake; we want the outbox to retry) — the
//! `max_attempts` cap enforces an eventual DLQ so we do not loop forever on
//! a genuinely dead provider.

use std::sync::Arc;

use crate::email::EmailService;

use super::{BoxFuture, Channel, ChannelError, DeliveryRequest, ProviderId};

pub struct EmailChannel {
    svc: Arc<EmailService>,
}

impl EmailChannel {
    #[must_use]
    pub fn new(svc: Arc<EmailService>) -> Self {
        Self { svc }
    }
}

impl Channel for EmailChannel {
    fn name(&self) -> &'static str {
        "email"
    }

    fn send<'a>(
        &'a self,
        req: &'a DeliveryRequest<'a>,
    ) -> BoxFuture<'a, Result<ProviderId, ChannelError>> {
        Box::pin(async move {
            let to_name = req.to_name.unwrap_or("");
            let subject = req.subject.unwrap_or("(no subject)");

            self.svc
                .send_rendered(req.to, to_name, subject, req.body)
                .await
                .map_err(|e| ChannelError::Transient(e.to_string()))?;

            // SMTP does not hand back a portable provider id; we synthesize
            // one so the `provider_id` column is always populated. Resend
            // will return a real id under FDN-09.
            Ok(format!("smtp-{}", uuid::Uuid::new_v4()))
        })
    }
}
