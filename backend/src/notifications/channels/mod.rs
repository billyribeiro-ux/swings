//! Delivery channels — the low-level "send a rendered template to a recipient"
//! surface.
//!
//! Every concrete channel implements the [`Channel`] trait; the
//! [`ChannelRegistry`] owns one of each wired at startup, so the outbox
//! notify-handler can dispatch by string key without learning about provider
//! construction details.
//!
//! # Provider-swap readiness
//!
//! The trait is the substitution surface. When FDN-09 replaces the current
//! `lettre`-backed [`email::EmailChannel`] with a `Resend` implementation,
//! the registry gets the new `Channel` instead and nothing else changes.

pub mod discord;
pub mod email;
pub mod in_app;
pub mod push;
pub mod slack;
pub mod sms;
pub mod webhook;

use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use crate::email::EmailService;

/// Provider-side identifier returned on a successful send (Resend message id,
/// Twilio sid, …). Kept opaque so channels are free to pick their own shape.
pub type ProviderId = String;

/// Typed errors emitted by a [`Channel`] implementation. Maps directly to the
/// outbox dispatcher's transient/permanent split via [`Into`] conversions.
#[derive(Debug, thiserror::Error)]
pub enum ChannelError {
    /// Transient provider failure — the outbox should retry with backoff.
    #[error("transient channel failure: {0}")]
    Transient(String),

    /// Permanent provider failure — the outbox should dead-letter the event.
    /// Typical causes: invalid recipient address, account suspended, 4xx from
    /// the provider that will not self-heal on retry.
    #[error("permanent channel failure: {0}")]
    Permanent(String),
}

/// A single delivery attempt shaped for a [`Channel`]. Fields are borrowed
/// rather than owned so channels can avoid cloning rendered bodies on the
/// happy path.
#[derive(Debug, Clone)]
pub struct DeliveryRequest<'a> {
    /// Recipient address (e-mail, phone number, webhook URL, …).
    pub to: &'a str,
    /// Optional display name — used as "Name <email>" by the e-mail channel.
    pub to_name: Option<&'a str>,
    /// Template key, echoed so channels can write it into provider metadata.
    pub template_key: &'a str,
    /// Fully-rendered subject (channels that ignore subjects, e.g. SMS, drop
    /// this).
    pub subject: Option<&'a str>,
    /// Fully-rendered body.
    pub body: &'a str,
    /// Locale the template was rendered in — logged / forwarded for
    /// observability.
    pub locale: &'a str,
    /// Caller-supplied idempotency key for providers that support it
    /// (Resend's `Idempotency-Key`, Twilio's `MessagingServiceSid`).
    pub idempotency_key: Option<&'a str>,
}

/// A pinned boxed future. Matches the trait shape used by
/// [`crate::events::handlers::EventHandler`] — consistency across the crate's
/// async trait surface.
pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// Core channel trait. Channels are stored as `Arc<dyn Channel>` in the
/// registry; the boxed-future signature avoids pulling in `async-trait`.
pub trait Channel: Send + Sync {
    /// Provider/channel label (e.g. `"email"`) — used for logs + DB rows.
    fn name(&self) -> &'static str;

    /// Execute a single delivery. Implementations must be idempotent when the
    /// caller supplies an `idempotency_key`.
    fn send<'a>(
        &'a self,
        req: &'a DeliveryRequest<'a>,
    ) -> BoxFuture<'a, Result<ProviderId, ChannelError>>;
}

/// Registry of channels keyed by [`Channel::name`]. Built once at startup;
/// look-ups are O(1) on the hot path.
pub struct ChannelRegistry {
    by_name: HashMap<&'static str, Arc<dyn Channel>>,
}

impl ChannelRegistry {
    /// Construct the registry with the default channel set.
    ///
    /// * `email` — real [`email::EmailChannel`] when an [`EmailService`] is
    ///   provided, otherwise a [`DisabledChannel`] that fails permanently.
    /// * `sms`, `push`, `in_app`, `slack`, `discord`, `webhook` — stubs
    ///   returning [`ChannelError::Permanent`] with a "not implemented"
    ///   message. Swapping a stub for a real impl is a one-line change in
    ///   this function.
    #[must_use]
    pub fn build(email_service: Option<Arc<EmailService>>) -> Self {
        let mut by_name: HashMap<&'static str, Arc<dyn Channel>> = HashMap::new();

        let email_channel: Arc<dyn Channel> = match email_service {
            Some(svc) => Arc::new(email::EmailChannel::new(svc)),
            None => Arc::new(DisabledChannel {
                name: "email",
                reason: "email service not configured (missing SMTP_USER)",
            }),
        };
        by_name.insert(email_channel.name(), email_channel);

        // SMS / push / Slack / Discord / webhook — FDN-05 ships the stubs.
        // Swap in real impls under the respective Phase 4 subsystems.
        by_name.insert("sms", Arc::new(sms::SmsChannel::stub()));
        by_name.insert("push", Arc::new(push::PushChannel::stub()));
        by_name.insert("in_app", Arc::new(in_app::InAppChannel::stub()));
        by_name.insert("slack", Arc::new(slack::SlackChannel::stub()));
        by_name.insert("discord", Arc::new(discord::DiscordChannel::stub()));
        by_name.insert("webhook", Arc::new(webhook::WebhookChannel::stub()));

        Self { by_name }
    }

    /// Look up a channel by the DB-stored string label
    /// (`'email' | 'sms' | ...`). Returns `None` when the registry has no
    /// matching entry — the caller (typically the outbox notify handler)
    /// should treat this as a permanent failure.
    #[must_use]
    pub fn get(&self, name: &str) -> Option<&Arc<dyn Channel>> {
        self.by_name.get(name)
    }

    /// Iterate over every registered channel name.
    pub fn names(&self) -> impl Iterator<Item = &'static str> + '_ {
        self.by_name.keys().copied()
    }
}

/// Channel placeholder returned when the real provider is unavailable (e.g.
/// SMTP not configured). Fails every send permanently so the outbox routes
/// the event to the DLQ instead of retrying forever.
pub struct DisabledChannel {
    pub name: &'static str,
    pub reason: &'static str,
}

impl Channel for DisabledChannel {
    fn name(&self) -> &'static str {
        self.name
    }

    fn send<'a>(
        &'a self,
        _req: &'a DeliveryRequest<'a>,
    ) -> BoxFuture<'a, Result<ProviderId, ChannelError>> {
        Box::pin(async move { Err(ChannelError::Permanent(self.reason.into())) })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn disabled_channel_fails_permanently() {
        let c = DisabledChannel {
            name: "email",
            reason: "not wired",
        };
        let req = DeliveryRequest {
            to: "x@example.com",
            to_name: None,
            template_key: "t",
            subject: None,
            body: "b",
            locale: "en",
            idempotency_key: None,
        };
        let err = c.send(&req).await.expect_err("should fail");
        assert!(matches!(err, ChannelError::Permanent(_)));
    }

    #[test]
    fn registry_without_email_service_still_has_all_channels() {
        let r = ChannelRegistry::build(None);
        assert!(r.get("email").is_some());
        assert!(r.get("sms").is_some());
        assert!(r.get("push").is_some());
        assert!(r.get("in_app").is_some());
        assert!(r.get("slack").is_some());
        assert!(r.get("discord").is_some());
        assert!(r.get("webhook").is_some());
        assert!(r.get("carrier_pigeon").is_none());
    }
}
