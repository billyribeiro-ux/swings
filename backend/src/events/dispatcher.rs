//! Event → handler registry with glob-ish pattern matching.
//!
//! Patterns are simple `token` / `token.*` strings — the single wildcard token
//! appears at the end and matches one-or-more segments. That is deliberately
//! more restrictive than generic globs: the whole point is that `order.*`
//! should match `order.created` and `order.refunded` but *not* the unrelated
//! `subscription.created`.
//!
//! Handlers are registered at [`Dispatcher`] construction and immutable
//! thereafter — this keeps the dispatch hot path lock-free.

use std::sync::Arc;

use tracing::{debug, warn};

use super::handlers::EventHandler;
use super::outbox::OutboxRecord;

// `async-trait` is not a dependency — [`EventHandler::handle`] returns a boxed
// future directly (see `handlers::mod`) to avoid pulling in the macro crate.

/// Errors raised while dispatching a single event. Kept distinct from
/// [`super::outbox::OutboxError`] so the worker can decide whether to retry
/// (transient) or dead-letter (permanent) on the wire.
#[derive(Debug, thiserror::Error)]
pub enum DispatchError {
    /// No handler is registered for `event_type`. Not a failure — the worker
    /// will mark the event delivered and move on (events can legitimately
    /// have zero subscribers at runtime).
    #[error("no handler matched event type `{0}`")]
    NoHandler(String),

    /// A handler returned a transient failure — the worker should schedule a
    /// retry with exponential backoff.
    #[error("transient handler failure: {0}")]
    Transient(String),

    /// A handler returned a permanent failure — the worker should dead-letter
    /// the event immediately regardless of `attempts`.
    #[error("permanent handler failure: {0}")]
    Permanent(String),
}

/// A simple pattern: either an exact `event_type` match or a `prefix.*` glob.
#[derive(Debug, Clone)]
struct EventPattern {
    raw: String,
    /// For `order.*`, this is `order.` (trailing dot kept so a naive `starts_with`
    /// does not accidentally match `orderless.created`).
    prefix: Option<String>,
}

impl EventPattern {
    fn parse(raw: impl Into<String>) -> Self {
        let raw = raw.into();
        let prefix = raw
            .strip_suffix(".*")
            .map(|p| format!("{p}."))
            .or_else(|| raw.strip_suffix('*').map(|p| p.to_owned()));
        EventPattern { raw, prefix }
    }

    /// Whether this pattern matches a given `event_type`.
    fn matches(&self, event_type: &str) -> bool {
        match &self.prefix {
            Some(p) => event_type.starts_with(p),
            None => self.raw == event_type,
        }
    }
}

/// Registry mapping event-type patterns to handlers. Construction is the only
/// mutation point — once the dispatcher is built it is read-only, so callers
/// can wrap it in an [`Arc`] and share it across worker tasks without locks.
#[derive(Default)]
pub struct Dispatcher {
    routes: Vec<(EventPattern, Arc<dyn EventHandler>)>,
}

impl Dispatcher {
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a handler for every event type matching `pattern`. Multiple
    /// patterns may be routed to the same handler; multiple handlers may match
    /// the same event (fan-out).
    pub fn register(mut self, pattern: &str, handler: Arc<dyn EventHandler>) -> Self {
        self.routes.push((EventPattern::parse(pattern), handler));
        self
    }

    /// Dispatch one event to every matching handler. If any handler returns
    /// [`DispatchError::Permanent`], the overall result is `Permanent`. If at
    /// least one handler returns [`DispatchError::Transient`] (and none are
    /// `Permanent`), the overall result is `Transient`. Zero matches is
    /// reported as [`DispatchError::NoHandler`] — the worker can decide to
    /// treat that as success (see [`Worker::run`](super::Worker)).
    pub async fn dispatch(&self, event: &OutboxRecord) -> Result<(), DispatchError> {
        let matched: Vec<&Arc<dyn EventHandler>> = self
            .routes
            .iter()
            .filter(|(pat, _)| pat.matches(&event.event_type))
            .map(|(_, h)| h)
            .collect();

        if matched.is_empty() {
            return Err(DispatchError::NoHandler(event.event_type.clone()));
        }

        let mut transient_errs: Vec<String> = Vec::new();
        for handler in matched {
            match handler.handle(event).await {
                Ok(()) => {
                    debug!(
                        event_id = %event.id,
                        event_type = %event.event_type,
                        "handler delivered"
                    );
                }
                Err(DispatchError::Permanent(msg)) => {
                    warn!(
                        event_id = %event.id,
                        event_type = %event.event_type,
                        error = %msg,
                        "permanent handler failure — routing to DLQ"
                    );
                    return Err(DispatchError::Permanent(msg));
                }
                Err(DispatchError::Transient(msg)) => {
                    warn!(
                        event_id = %event.id,
                        event_type = %event.event_type,
                        error = %msg,
                        "transient handler failure — will retry"
                    );
                    transient_errs.push(msg);
                }
                Err(DispatchError::NoHandler(pattern)) => {
                    // A handler should never return NoHandler itself; treat as
                    // a permanent bug in the handler.
                    return Err(DispatchError::Permanent(format!(
                        "handler reported NoHandler for {pattern}"
                    )));
                }
            }
        }

        if transient_errs.is_empty() {
            Ok(())
        } else {
            Err(DispatchError::Transient(transient_errs.join("; ")))
        }
    }
}

impl std::fmt::Debug for Dispatcher {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let patterns: Vec<&str> = self.routes.iter().map(|(p, _)| p.raw.as_str()).collect();
        f.debug_struct("Dispatcher")
            .field("patterns", &patterns)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use uuid::Uuid;

    fn test_record(event_type: &str) -> OutboxRecord {
        OutboxRecord {
            id: Uuid::nil(),
            aggregate_type: "test".into(),
            aggregate_id: "0".into(),
            event_type: event_type.into(),
            payload: serde_json::json!({}),
            headers: serde_json::json!({}),
            status: crate::events::outbox::OutboxStatus::InFlight,
            attempts: 0,
            max_attempts: 8,
            next_attempt_at: Utc::now(),
            last_error: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    struct CountingHandler {
        calls: Arc<AtomicUsize>,
        result: Result<(), DispatchError>,
    }

    impl CountingHandler {
        fn ok() -> (Arc<Self>, Arc<AtomicUsize>) {
            let calls = Arc::new(AtomicUsize::new(0));
            (
                Arc::new(Self {
                    calls: calls.clone(),
                    result: Ok(()),
                }),
                calls,
            )
        }
        fn transient(msg: &str) -> Arc<Self> {
            Arc::new(Self {
                calls: Arc::new(AtomicUsize::new(0)),
                result: Err(DispatchError::Transient(msg.into())),
            })
        }
        fn permanent(msg: &str) -> Arc<Self> {
            Arc::new(Self {
                calls: Arc::new(AtomicUsize::new(0)),
                result: Err(DispatchError::Permanent(msg.into())),
            })
        }
    }

    impl EventHandler for CountingHandler {
        fn handle<'a>(
            &'a self,
            _event: &'a OutboxRecord,
        ) -> std::pin::Pin<
            Box<dyn std::future::Future<Output = Result<(), DispatchError>> + Send + 'a>,
        > {
            Box::pin(async move {
                self.calls.fetch_add(1, Ordering::Relaxed);
                match &self.result {
                    Ok(()) => Ok(()),
                    Err(DispatchError::Transient(m)) => Err(DispatchError::Transient(m.clone())),
                    Err(DispatchError::Permanent(m)) => Err(DispatchError::Permanent(m.clone())),
                    Err(DispatchError::NoHandler(m)) => Err(DispatchError::NoHandler(m.clone())),
                }
            })
        }
    }

    #[test]
    fn exact_pattern_matches_only_exact() {
        let p = EventPattern::parse("order.created");
        assert!(p.matches("order.created"));
        assert!(!p.matches("order.refunded"));
        assert!(!p.matches("order.created.retried"));
    }

    #[test]
    fn wildcard_pattern_matches_prefix_and_rejects_siblings() {
        let p = EventPattern::parse("order.*");
        assert!(p.matches("order.created"));
        assert!(p.matches("order.refunded"));
        assert!(p.matches("order.foo.bar")); // allowed — one or more segments
        assert!(!p.matches("subscription.created"));
        assert!(!p.matches("orderless.created"));
    }

    #[tokio::test]
    async fn no_handler_returns_no_handler_error() {
        let d = Dispatcher::new();
        let err = d.dispatch(&test_record("x.y")).await.unwrap_err();
        assert!(matches!(err, DispatchError::NoHandler(_)));
    }

    #[tokio::test]
    async fn wildcard_dispatch_invokes_all_matching() {
        let (h1, c1) = CountingHandler::ok();
        let (h2, c2) = CountingHandler::ok();
        let d = Dispatcher::new()
            .register("order.*", h1)
            .register("order.created", h2);
        d.dispatch(&test_record("order.created"))
            .await
            .expect("both match");
        assert_eq!(c1.load(Ordering::Relaxed), 1);
        assert_eq!(c2.load(Ordering::Relaxed), 1);
    }

    #[tokio::test]
    async fn permanent_error_short_circuits_fanout() {
        let perm = CountingHandler::permanent("nope");
        let d = Dispatcher::new().register("order.*", perm);
        let err = d.dispatch(&test_record("order.created")).await.unwrap_err();
        assert!(matches!(err, DispatchError::Permanent(_)));
    }

    #[tokio::test]
    async fn transient_error_aggregates_after_fanout() {
        let (ok_h, ok_calls) = CountingHandler::ok();
        let trans = CountingHandler::transient("slow");
        let d = Dispatcher::new()
            .register("order.*", ok_h)
            .register("order.*", trans);
        let err = d.dispatch(&test_record("order.created")).await.unwrap_err();
        assert!(matches!(err, DispatchError::Transient(_)));
        // The OK handler still ran, so the failure is aggregated, not short-circuited.
        assert_eq!(ok_calls.load(Ordering::Relaxed), 1);
    }
}
