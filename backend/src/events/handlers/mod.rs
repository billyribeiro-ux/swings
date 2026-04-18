//! Event subscribers: every outgoing integration implements [`EventHandler`].
//!
//! The trait uses a boxed-future signature rather than pulling in the
//! `async-trait` crate — [`EventHandler`] objects are stored in
//! `Arc<dyn EventHandler>` behind the [`super::Dispatcher`], and the
//! boxed-future form is what the compiler accepts there without macros.

use std::future::Future;
use std::pin::Pin;

use super::dispatcher::DispatchError;
use super::outbox::OutboxRecord;

pub mod digital_delivery;
pub mod notify;
pub mod popup_attribution;
pub mod webhook_out;

pub use digital_delivery::DigitalDeliveryHandler;
pub use notify::NotifyHandler;
pub use popup_attribution::PopupAttributionHandler;
pub use webhook_out::WebhookOutHandler;

/// Boxed, pinned, `Send` future alias used in the trait signature. Lifetime
/// `'a` borrows from `self` and `event` so handlers can keep references alive
/// for the duration of the call without cloning the payload.
pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// Every outbox subscriber implements this trait. The dispatcher fan-outs to
/// all handlers matching an event's `event_type`.
///
/// Errors must be typed ([`DispatchError::Transient`] vs. [`DispatchError::Permanent`])
/// so the worker can decide between retry with backoff and immediate
/// dead-letter routing.
pub trait EventHandler: Send + Sync {
    fn handle<'a>(&'a self, event: &'a OutboxRecord) -> BoxFuture<'a, Result<(), DispatchError>>;
}
