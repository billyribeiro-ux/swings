//! E-mail channel — delegates to an injected [`EmailProvider`] so FDN-09 can
//! swap the transport without touching `send_notification` / the outbox worker.
//!
//! The channel's only job is to convert a generic [`DeliveryRequest`] into an
//! [`EmailSendRequest`], forward it to the provider, and map the provider's
//! typed error back into [`ChannelError::Transient`] / [`ChannelError::Permanent`]
//! so the outbox's retry + DLQ semantics keep working unchanged.

use std::sync::Arc;

use super::{BoxFuture, Channel, ChannelError, DeliveryRequest, ProviderId};

pub mod lettre_provider;
pub mod noop_provider;
pub mod resend_provider;

pub use lettre_provider::LettreProvider;
pub use noop_provider::NoopProvider;
pub use resend_provider::ResendProvider;

/// Typed error emitted by an [`EmailProvider`].
///
/// The channel layer maps `Transient` → [`ChannelError::Transient`] (outbox
/// retries with backoff) and `Permanent`/`Config` → [`ChannelError::Permanent`]
/// (outbox dead-letters).
#[derive(Debug, thiserror::Error)]
pub enum EmailProviderError {
    /// Temporary fault — 429, 5xx, transport error. Caller should retry.
    #[error("transient email provider error: {0}")]
    Transient(String),

    /// Non-retryable fault — 4xx (except 429), invalid address. Caller should
    /// surface the failure to the user / DLQ the event.
    #[error("permanent email provider error: {0}")]
    Permanent(String),

    /// Configuration problem (missing key, malformed secret). Surfaces at
    /// startup or on first send; usually resolved by an operator, not retry.
    #[error("email provider config error: {0}")]
    Config(String),
}

/// A rendered e-mail on its way to a provider. Fields are owned (rather than
/// borrowed) so the request can cross `async` points + spawn boundaries.
#[derive(Debug, Clone)]
pub struct EmailSendRequest {
    pub to: String,
    pub from: String,
    pub subject: String,
    pub html_body: String,
    pub plain_body: Option<String>,
    pub reply_to: Option<String>,
    /// Provider-specific tag key/value pairs (Resend honors these for
    /// analytics; Lettre ignores). Kept ordered for deterministic ser.
    pub tags: Vec<(String, String)>,
    /// Provider idempotency key. Resend forwards as `Idempotency-Key`; Lettre
    /// ignores (SMTP has no equivalent).
    pub idempotency_key: Option<String>,
}

/// Provider-agnostic send surface.
///
/// Every concrete implementation lives in a sibling module; the [`Channel`]
/// implementation (on [`EmailChannel`]) only talks to the trait.
#[async_trait::async_trait]
pub trait EmailProvider: Send + Sync {
    /// Send one rendered e-mail. Returns the provider-specific id that the
    /// caller (notify-handler / admin `test-send`) persists as
    /// `notification_deliveries.provider_id` — later Resend webhooks resolve
    /// status transitions by matching on this field.
    async fn send(&self, req: &EmailSendRequest) -> Result<String, EmailProviderError>;

    /// Stable short name for logs + the `Channel::name` fallback. Must not
    /// include whitespace.
    fn name(&self) -> &'static str;
}

/// Channel implementation that routes every send through an injected provider.
///
/// Construction is always `Arc<dyn EmailProvider>` so runtime selection
/// (`main.rs` → `EMAIL_PROVIDER` env var) can wire any of the three concrete
/// impls without re-casting.
pub struct EmailChannel {
    provider: Arc<dyn EmailProvider>,
    default_from: String,
}

impl EmailChannel {
    /// Build a channel wired against `provider`. `default_from` is used when
    /// a [`DeliveryRequest`] has no `from` override (which — today — is always,
    /// because the channel trait doesn't carry a `from` field).
    #[must_use]
    pub fn new(provider: Arc<dyn EmailProvider>, default_from: impl Into<String>) -> Self {
        Self {
            provider,
            default_from: default_from.into(),
        }
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
            let to = format_to(req.to, req.to_name);
            let subject = req.subject.unwrap_or("(no subject)");

            let send_req = EmailSendRequest {
                to,
                from: self.default_from.clone(),
                subject: subject.to_string(),
                html_body: req.body.to_string(),
                plain_body: None,
                reply_to: None,
                tags: vec![
                    ("template".into(), req.template_key.to_string()),
                    ("locale".into(), req.locale.to_string()),
                ],
                idempotency_key: req.idempotency_key.map(str::to_string),
            };

            self.provider.send(&send_req).await.map_err(|e| match e {
                EmailProviderError::Transient(m) => ChannelError::Transient(m),
                EmailProviderError::Permanent(m) | EmailProviderError::Config(m) => {
                    ChannelError::Permanent(m)
                }
            })
        })
    }
}

/// Format `"Name <email>"` for RFC 5322 `To` headers. Falls back to the bare
/// address when no display name is supplied.
fn format_to(email: &str, name: Option<&str>) -> String {
    match name {
        Some(n) if !n.trim().is_empty() => format!("{n} <{email}>"),
        _ => email.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_to_with_and_without_name() {
        assert_eq!(format_to("a@b.co", None), "a@b.co");
        assert_eq!(format_to("a@b.co", Some("")), "a@b.co");
        assert_eq!(format_to("a@b.co", Some("Ada")), "Ada <a@b.co>");
    }

    #[tokio::test]
    async fn channel_forwards_to_provider_and_maps_errors() {
        use std::sync::atomic::{AtomicUsize, Ordering};

        struct StubProvider {
            calls: AtomicUsize,
            outcome: Option<EmailProviderError>,
        }

        #[async_trait::async_trait]
        impl EmailProvider for StubProvider {
            async fn send(&self, _req: &EmailSendRequest) -> Result<String, EmailProviderError> {
                self.calls.fetch_add(1, Ordering::SeqCst);
                match &self.outcome {
                    None => Ok("stub-1".into()),
                    Some(EmailProviderError::Transient(m)) => {
                        Err(EmailProviderError::Transient(m.clone()))
                    }
                    Some(EmailProviderError::Permanent(m)) => {
                        Err(EmailProviderError::Permanent(m.clone()))
                    }
                    Some(EmailProviderError::Config(m)) => {
                        Err(EmailProviderError::Config(m.clone()))
                    }
                }
            }
            fn name(&self) -> &'static str {
                "stub"
            }
        }

        let provider = Arc::new(StubProvider {
            calls: AtomicUsize::new(0),
            outcome: None,
        });
        let ch = EmailChannel::new(provider.clone(), "Swings <noreply@example.test>");
        let req = DeliveryRequest {
            to: "a@b.co",
            to_name: Some("Ada"),
            template_key: "t",
            subject: Some("Hi"),
            body: "<p>hi</p>",
            locale: "en",
            idempotency_key: Some("key-1"),
        };
        let id = ch.send(&req).await.expect("ok");
        assert_eq!(id, "stub-1");
        assert_eq!(provider.calls.load(Ordering::SeqCst), 1);

        // Transient → Transient
        let tprov = Arc::new(StubProvider {
            calls: AtomicUsize::new(0),
            outcome: Some(EmailProviderError::Transient("429".into())),
        });
        let ch = EmailChannel::new(tprov, "Swings <noreply@example.test>");
        let err = ch.send(&req).await.expect_err("transient");
        assert!(matches!(err, ChannelError::Transient(_)));

        // Permanent → Permanent
        let pprov = Arc::new(StubProvider {
            calls: AtomicUsize::new(0),
            outcome: Some(EmailProviderError::Permanent("invalid_to".into())),
        });
        let ch = EmailChannel::new(pprov, "Swings <noreply@example.test>");
        let err = ch.send(&req).await.expect_err("permanent");
        assert!(matches!(err, ChannelError::Permanent(_)));

        // Config → Permanent (configuration errors never self-heal)
        let cprov = Arc::new(StubProvider {
            calls: AtomicUsize::new(0),
            outcome: Some(EmailProviderError::Config("missing RESEND_API_KEY".into())),
        });
        let ch = EmailChannel::new(cprov, "Swings <noreply@example.test>");
        let err = ch.send(&req).await.expect_err("config");
        assert!(matches!(err, ChannelError::Permanent(_)));
    }
}
