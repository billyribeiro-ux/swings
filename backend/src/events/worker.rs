//! Outbox worker: claim → dispatch → settle, repeated until shutdown.
//!
//! Workers are spawned as long-running tokio tasks; each worker leases its own
//! batch via [`super::outbox::claim_batch`] (which uses `SELECT … FOR UPDATE
//! SKIP LOCKED`), so multiple workers running in the same process — or across
//! replicas — cannot double-deliver a given row.
//!
//! # Shutdown
//!
//! Workers subscribe to a [`tokio::sync::broadcast`] channel. On shutdown
//! signal the loop exits on the next iteration; the process supervisor should
//! `join` the returned [`WorkerHandle`]s with a bounded grace window before
//! exiting so any in-flight claim has a chance to land a `mark_failed` /
//! `mark_delivered` write.

use std::sync::Arc;
use std::time::Duration;

use sqlx::PgPool;
use tokio::select;
use tokio::sync::broadcast;
use tokio::task::JoinHandle;
use tokio::time::sleep;
use tracing::{debug, error, info, warn};

use super::dispatcher::{DispatchError, Dispatcher};
use super::outbox::{self, OutboxError, OutboxRecord};

/// Polling cadence when the last tick returned an empty batch. Short enough to
/// feel real-time in dev, long enough to avoid hammering Postgres at idle.
const IDLE_POLL_INTERVAL: Duration = Duration::from_millis(500);

/// Errors raised by the worker lifecycle itself. Per-event errors are handled
/// inside the loop and never bubble up to the caller.
#[derive(Debug, thiserror::Error)]
pub enum WorkerError {
    #[error("outbox error: {0}")]
    Outbox(#[from] OutboxError),
}

/// Runtime config for a single worker. Built from env vars in `main.rs`
/// (`OUTBOX_WORKERS`, `OUTBOX_BATCH_SIZE`) and passed into [`Worker::spawn`].
#[derive(Debug, Clone, Copy)]
pub struct WorkerConfig {
    pub batch_size: i64,
}

impl Default for WorkerConfig {
    fn default() -> Self {
        Self { batch_size: 16 }
    }
}

/// Single worker handle. Drop it to abort; prefer the cooperative shutdown
/// via the broadcast sender stored in `AppState`.
pub struct WorkerHandle {
    worker_id: usize,
    join: JoinHandle<Result<(), WorkerError>>,
}

impl WorkerHandle {
    pub fn id(&self) -> usize {
        self.worker_id
    }

    /// Await worker completion, surfacing any `WorkerError` left behind.
    pub async fn join(self) -> Result<(), WorkerError> {
        match self.join.await {
            Ok(result) => result,
            Err(join_err) if join_err.is_cancelled() => {
                warn!(worker = self.worker_id, "worker task cancelled");
                Ok(())
            }
            Err(join_err) => {
                error!(worker = self.worker_id, error = ?join_err, "worker task panicked");
                Ok(())
            }
        }
    }
}

/// The claim/dispatch/settle loop.
pub struct Worker;

impl Worker {
    /// Spawn a long-running worker task. Returns immediately with a handle
    /// the caller can join at shutdown time.
    ///
    /// `shutdown_rx` is how the worker learns to stop — see
    /// [`WorkerShutdown`]. Use [`WorkerShutdown::sender`] to get the matching
    /// `broadcast::Sender` stored in `AppState`.
    pub fn spawn(
        worker_id: usize,
        pool: PgPool,
        dispatcher: Arc<Dispatcher>,
        config: WorkerConfig,
        mut shutdown_rx: broadcast::Receiver<()>,
    ) -> WorkerHandle {
        let join = tokio::spawn(async move {
            info!(
                worker = worker_id,
                batch_size = config.batch_size,
                "outbox worker starting"
            );
            loop {
                select! {
                    biased;
                    _ = shutdown_rx.recv() => {
                        info!(worker = worker_id, "outbox worker shutdown signal received");
                        return Ok(());
                    }
                    tick_result = tick(worker_id, &pool, &dispatcher, &config) => {
                        match tick_result {
                            Ok(0) => {
                                // Empty batch — sleep briefly, but wake on shutdown.
                                select! {
                                    _ = shutdown_rx.recv() => {
                                        info!(worker = worker_id, "outbox worker shutdown during idle");
                                        return Ok(());
                                    }
                                    _ = sleep(IDLE_POLL_INTERVAL) => {}
                                }
                            }
                            Ok(n) => {
                                debug!(worker = worker_id, processed = n, "outbox tick drained batch");
                            }
                            Err(err) => {
                                warn!(worker = worker_id, error = %err, "outbox tick failed; backing off");
                                select! {
                                    _ = shutdown_rx.recv() => {
                                        return Ok(());
                                    }
                                    _ = sleep(IDLE_POLL_INTERVAL) => {}
                                }
                            }
                        }
                    }
                }
            }
        });
        WorkerHandle { worker_id, join }
    }
}

/// One iteration of claim → dispatch → settle. Returns the number of events
/// processed. Transient per-event errors are logged and the row is re-queued
/// with backoff; only outbox-layer failures bubble up as [`WorkerError`].
async fn tick(
    worker_id: usize,
    pool: &PgPool,
    dispatcher: &Arc<Dispatcher>,
    config: &WorkerConfig,
) -> Result<usize, WorkerError> {
    let batch = outbox::claim_batch(pool, config.batch_size).await?;
    if batch.is_empty() {
        return Ok(0);
    }
    let count = batch.len();
    for event in batch {
        dispatch_one(worker_id, pool, dispatcher, event).await;
    }
    Ok(count)
}

/// Dispatch a single leased event and settle its row. Never panics; each
/// error branch lands the row in a definite terminal state or schedules a
/// retry, so the worker can keep ticking.
async fn dispatch_one(
    worker_id: usize,
    pool: &PgPool,
    dispatcher: &Arc<Dispatcher>,
    event: OutboxRecord,
) {
    let event_id = event.id;
    let event_type = event.event_type.clone();
    let attempts_so_far = event.attempts;
    let max_attempts = event.max_attempts;

    let dispatch_result = dispatcher.dispatch(&event).await;

    match dispatch_result {
        Ok(()) => settle_delivered(worker_id, pool, event_id).await,
        Err(DispatchError::NoHandler(pattern)) => {
            // No subscribers for this type — treat as delivered so the row
            // stops cycling. Log at info so operators still see it.
            info!(
                worker = worker_id,
                event_id = %event_id,
                event_type = %event_type,
                pattern = %pattern,
                "no handler registered for event; marking delivered"
            );
            settle_delivered(worker_id, pool, event_id).await;
        }
        Err(DispatchError::Permanent(msg)) => {
            warn!(
                worker = worker_id,
                event_id = %event_id,
                event_type = %event_type,
                error = %msg,
                "permanent failure — routing to DLQ"
            );
            if let Err(e) = outbox::mark_dead_letter(pool, event_id, &msg).await {
                error!(
                    worker = worker_id,
                    event_id = %event_id,
                    error = %e,
                    "failed to move event to DLQ"
                );
            }
        }
        Err(DispatchError::Transient(msg)) => {
            let next_attempts = attempts_so_far + 1;
            if next_attempts >= max_attempts {
                warn!(
                    worker = worker_id,
                    event_id = %event_id,
                    event_type = %event_type,
                    attempts = attempts_so_far,
                    max_attempts,
                    error = %msg,
                    "exhausted max_attempts — routing to DLQ"
                );
                if let Err(e) = outbox::mark_dead_letter(pool, event_id, &msg).await {
                    error!(
                        worker = worker_id,
                        event_id = %event_id,
                        error = %e,
                        "failed to move event to DLQ"
                    );
                }
            } else if let Err(e) =
                outbox::mark_failed_with_backoff(pool, event_id, attempts_so_far, &msg).await
            {
                error!(
                    worker = worker_id,
                    event_id = %event_id,
                    error = %e,
                    "failed to record retry"
                );
            }
        }
    }
}

async fn settle_delivered(worker_id: usize, pool: &PgPool, id: uuid::Uuid) {
    if let Err(e) = outbox::mark_delivered(pool, id).await {
        error!(
            worker = worker_id,
            event_id = %id,
            error = %e,
            "failed to mark event delivered"
        );
    }
}

/// Convenience wrapper that owns the broadcast sender kept in `AppState`
/// and hands out fresh receivers per worker.
#[derive(Debug, Clone)]
pub struct WorkerShutdown {
    sender: broadcast::Sender<()>,
}

impl WorkerShutdown {
    pub fn new() -> Self {
        let (sender, _) = broadcast::channel(1);
        Self { sender }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<()> {
        self.sender.subscribe()
    }

    pub fn sender(&self) -> broadcast::Sender<()> {
        self.sender.clone()
    }

    /// Tell every subscribed worker to stop. Safe to call multiple times;
    /// returns the number of receivers notified (which can be 0 if the
    /// workers already exited).
    pub fn shutdown(&self) -> usize {
        self.sender.send(()).unwrap_or(0)
    }
}

impl Default for WorkerShutdown {
    fn default() -> Self {
        Self::new()
    }
}
