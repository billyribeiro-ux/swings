//! No-op [`EmailProvider`] — logs, skips the network, returns a synthetic id.
//!
//! Active when `EMAIL_PROVIDER=noop` (explicit) or — in non-production — when
//! neither `RESEND_API_KEY` nor `SMTP_USER` is present. Integration tests also
//! wire this directly to avoid provider round-trips.

use tracing::info;

use super::{EmailProvider, EmailProviderError, EmailSendRequest};

/// E-mail provider stub.
///
/// Always returns `Ok("noop-{uuid}")`. The provider id is synthetic but carries
/// the `noop-` prefix so delivery rows captured in tests or dry-run flights are
/// trivially grep-able.
pub struct NoopProvider;

impl NoopProvider {
    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}

impl Default for NoopProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl EmailProvider for NoopProvider {
    async fn send(&self, req: &EmailSendRequest) -> Result<String, EmailProviderError> {
        info!(
            to = %req.to,
            subject = %req.subject,
            template = ?req.tags.iter().find(|(k, _)| k == "template").map(|(_, v)| v.as_str()),
            "noop email provider: accepted send (not delivered)"
        );
        Ok(format!("noop-{}", uuid::Uuid::new_v4()))
    }

    fn name(&self) -> &'static str {
        "noop"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn noop_send_returns_prefixed_id() {
        let p = NoopProvider::new();
        let req = EmailSendRequest {
            to: "a@b.co".into(),
            from: "Swings <noreply@example.test>".into(),
            subject: "Hi".into(),
            html_body: "<p>x</p>".into(),
            plain_body: None,
            reply_to: None,
            tags: vec![("template".into(), "welcome".into())],
            idempotency_key: None,
        };
        let id = p.send(&req).await.expect("noop always ok");
        assert!(id.starts_with("noop-"), "got: {id}");
        assert_eq!(p.name(), "noop");
    }
}
