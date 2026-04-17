//! Lettre-backed [`EmailProvider`] — keeps the dev-friendly SMTP path working.
//!
//! SMTP has no portable "provider id" on a successful send, so we synthesise
//! `lettre-{uuid}` to satisfy the caller contract. Resend webhooks will never
//! reference these ids (different source entirely), so collisions are
//! impossible in the `notification_deliveries.provider_id` column.

use std::sync::Arc;

use crate::email::EmailService;

use super::{EmailProvider, EmailProviderError, EmailSendRequest};

/// [`EmailProvider`] that wraps the existing [`EmailService`] SMTP client.
///
/// Constructed at startup by `main.rs` when `EMAIL_PROVIDER=smtp` (or the
/// implicit dev selection when `SMTP_USER` is set + `EMAIL_PROVIDER` unset).
pub struct LettreProvider {
    svc: Arc<EmailService>,
}

impl LettreProvider {
    #[must_use]
    pub fn new(svc: Arc<EmailService>) -> Self {
        Self { svc }
    }
}

#[async_trait::async_trait]
impl EmailProvider for LettreProvider {
    async fn send(&self, req: &EmailSendRequest) -> Result<String, EmailProviderError> {
        // `EmailService::send_rendered` expects bare address + name; the
        // channel-side `format_to` already baked `"Name <addr>"` into
        // `req.to`, so pass it unchanged and use an empty display name.
        self.svc
            .send_rendered(&req.to, "", &req.subject, &req.html_body)
            .await
            // Every SMTP failure is transient by default — network flakes +
            // greylisting dominate. Permanent bounces surface async through
            // the MX layer, not here, so `Transient` is the right default.
            .map_err(|e| EmailProviderError::Transient(e.to_string()))?;
        Ok(format!("lettre-{}", uuid::Uuid::new_v4()))
    }

    fn name(&self) -> &'static str {
        "lettre"
    }
}
