//! FDN-05: Notifications subsystem.
//!
//! Provides a multi-channel delivery pipeline for transactional and marketing
//! notifications:
//!
//! * [`templates`] — versioned template registry (one row per key × channel ×
//!   locale × version) rendered through Tera with JSON context.
//! * [`channels`] — the [`channels::Channel`] trait + concrete implementations.
//!   Only [`channels::email::EmailChannel`] is wired for v1; SMS / push / Slack
//!   / Discord / webhook stubs return [`crate::error::AppError::NotImplemented`]
//!   so the surface is discoverable and the trait is future-compatible.
//! * [`preferences`] — per-user opt-in plus quiet-hours evaluation.
//! * [`suppression`] — provider-bounce / user-unsubscribe-all deny list.
//! * [`unsubscribe`] — one-shot HMAC-SHA256 tokens issued by `send_notification`
//!   and redeemed by the public `/u/unsubscribe` route.
//! * [`send`]  — the entry point. Runs preference + suppression checks, inserts
//!   a `notification_deliveries` row in `queued`, and publishes a
//!   `notification.queued` outbox event so the worker picks up delivery.
//! * [`webhooks`] — inbound provider callback handlers (Resend lands in FDN-09).
//!
//! # Dispatch lifecycle
//!
//! 1. `send_notification(...)` → row inserted, event published through the
//!    transactional outbox (FDN-04).
//! 2. Worker claims the event via [`crate::events::Worker`] and routes it to
//!    [`crate::events::handlers::notify::NotifyHandler`].
//! 3. `NotifyHandler` resolves the template, calls the matching channel, and
//!    moves the delivery row to `sent` / `failed` (terminal or retryable).
//!
//! # Why a standalone service and not just the outbox?
//!
//! The outbox gives at-least-once dispatch. Notifications need an extra layer
//! of domain logic on top — preference checks, suppression gates, template
//! resolution with locale fallback, quiet-hours windows. Keeping that logic in
//! a dedicated `Service` type keeps the outbox handler thin (just "look up the
//! delivery row and send it through the right channel") and lets unit tests
//! stub out channels without pulling in the worker pool.

pub mod channels;
pub mod preferences;
pub mod send;
pub mod suppression;
pub mod templates;
pub mod unsubscribe;
pub mod webhooks;

pub use channels::{
    Channel, ChannelError, ChannelRegistry, DeliveryRequest, EmailProvider, EmailProviderError,
    EmailSendRequest, ProviderId,
};
pub use preferences::{is_allowed, NotificationPreference, PreferenceUpdate};
pub use send::{send_notification, NotifyError, Recipient, SendOptions};
pub use suppression::{is_suppressed, suppress, unsuppress, Suppression};
pub use templates::{RenderedTemplate, Template, TemplateError};
pub use unsubscribe::{consume_token, mint_token, UnsubscribeAction, UnsubscribeError};

use std::sync::Arc;

/// Application-scoped handle held on [`crate::AppState`].
///
/// The [`Service`] owns the [`ChannelRegistry`] so handlers (and the outbox
/// worker's notify adapter) can route through channel traits without re-wiring
/// per-provider configuration. Construction lives in `main.rs`.
#[derive(Clone)]
pub struct Service {
    channels: Arc<ChannelRegistry>,
}

impl Service {
    /// Build a service wired for the current runtime.
    ///
    /// `email_provider` is the concrete transport (Resend, Lettre, Noop) that
    /// `main.rs` selected via the `EMAIL_PROVIDER` env var. When `None` the
    /// email channel is replaced by a `DisabledChannel` that fails every send
    /// permanently — so the outbox DLQs stuck deliveries instead of retrying
    /// forever.
    #[must_use]
    pub fn new(email_provider: Option<Arc<dyn EmailProvider>>, default_from: String) -> Self {
        let registry = ChannelRegistry::build(email_provider, default_from);
        Self {
            channels: Arc::new(registry),
        }
    }

    #[must_use]
    pub fn channels(&self) -> &Arc<ChannelRegistry> {
        &self.channels
    }
}
