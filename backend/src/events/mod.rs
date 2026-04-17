// FDN-04: Transactional outbox. Consumers of the `publish_in_tx` helper and
// the `Event` struct will arrive in Round 2b (domain handler wiring) — the
// types exist now so migrations, the worker, and admin ops endpoints can be
// stood up ahead of their first caller. Silence the module-level dead-code
// warning until the handlers wire in.
#![allow(dead_code)]

//! Transactional outbox and asynchronous event dispatch.
//!
//! The outbox is the durable bridge between domain mutations and asynchronous
//! side-effects (email delivery, outbound webhooks, analytics fan-out, …).
//! Producers call [`outbox::publish_in_tx`] from inside a [`sqlx::Transaction`]
//! so the event row is committed atomically with the domain mutation. A pool
//! of [`Worker`]s leases pending rows with `SELECT … FOR UPDATE SKIP LOCKED`,
//! dispatches each event through a [`Dispatcher`] registry, and advances the
//! row's status (`delivered`, retried, or `dead_letter`).
//!
//! # Module layout
//!
//! * [`outbox`] — row-level CRUD: publish, claim, mark-delivered,
//!   mark-failed-with-backoff, mark-dead-letter. Exponential backoff with
//!   jitter lives here.
//! * [`dispatcher`] — pattern → handler registry and the event-type glob
//!   matcher (`order.*` vs. `order.created`).
//! * [`worker`] — long-running tokio task that owns the claim/dispatch loop
//!   and honors a broadcast shutdown signal.
//! * [`handlers`] — trait + stub implementations ([`handlers::notify`] and
//!   [`handlers::webhook_out`]). Real wiring arrives in FDN-05 / FORM-07.
//!
//! # Why write-through the outbox instead of a direct channel?
//!
//! Downstream subscribers are third-party, flaky, and often slow. Writing to
//! Postgres first preserves at-least-once delivery across process restarts,
//! eliminates the "we committed but the email didn't send" gap, and lets ops
//! replay / reset individual events via `POST /api/admin/outbox/{id}/retry`.

pub mod dispatcher;
pub mod handlers;
pub mod outbox;
pub mod worker;

pub use dispatcher::{DispatchError, Dispatcher};
pub use outbox::{publish_in_tx, Event, EventHeaders, OutboxError, OutboxRecord, OutboxStatus};
pub use worker::{Worker, WorkerConfig, WorkerError, WorkerHandle, WorkerShutdown};
