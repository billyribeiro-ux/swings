//! Blog scheduler worker.
//!
//! Forensic Wave-2 PR-7 (C-6): the blog `status='scheduled'` + `scheduled_at`
//! columns existed since migration 058 but no background job ever flipped a
//! scheduled post to `published`. Posts authored with "publish at 2am
//! tomorrow" stayed invisible forever — public listing filters by
//! `status='published'` and never consulted `scheduled_at`.
//!
//! ## Strategy
//!
//! Each tick:
//!   1. `UPDATE blog_posts SET status='published', published_at=COALESCE(published_at, NOW())
//!      WHERE status='scheduled' AND scheduled_at <= NOW() RETURNING id`.
//!   2. The publish-time `published_at` is COALESCEd so a re-publish of a
//!      post that had a prior `published_at` (e.g. unpublish → re-schedule)
//!      keeps the original first-publish timestamp; only fresh schedules
//!      stamp NOW().
//!   3. Per-row tracing for the audit / debug trail. The transition is
//!      attributed to the worker — there is no human actor and no
//!      `admin_actions` row for system-driven publishes (the `scheduled`
//!      → `published` flip is bookkeeping, not a discretionary
//!      privileged action).
//!
//! ## Concurrency
//!
//! The UPDATE acquires per-row exclusive locks. Two replicas of the worker
//! racing on the same tick will pick disjoint subsets via Postgres' default
//! row-locking semantics; the second tx becomes a no-op. We do not need
//! `FOR UPDATE SKIP LOCKED` because the predicate is monotonic (once a row
//! is `published` it never matches the WHERE again) and the publish
//! operation is idempotent.
//!
//! ## Observability
//!
//! * `blog_scheduler_published_total` — monotonic counter of rows transitioned.
//! * `blog_scheduler_tick_duration_seconds` — per-tick wall time histogram.
//! * `blog_scheduler_last_success_unixtime` — the SLO gauge that
//!   `ops/prometheus/admin-alerts.rules.yml` should alert on (no advance
//!   for `> 600s` ⇒ wedged worker).

use std::time::{Duration, Instant};

use sqlx::PgPool;
use tokio::sync::broadcast;
use uuid::Uuid;

/// Promote every scheduled post whose `scheduled_at` is in the past. Returns
/// the row ids that flipped to `published`.
///
/// Exposed (not `async fn run_loop` only) so integration tests can drive
/// the publication step deterministically without a sleeping loop.
pub async fn publish_due_posts(pool: &PgPool) -> Result<Vec<Uuid>, sqlx::Error> {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        UPDATE blog_posts
           SET status       = 'published',
               published_at = COALESCE(published_at, NOW()),
               updated_at   = NOW()
         WHERE status = 'scheduled'
           AND scheduled_at IS NOT NULL
           AND scheduled_at <= NOW()
        RETURNING id
        "#,
    )
    .fetch_all(pool)
    .await
}

/// Single tick: publish + emit metrics. Returns the count of posts published.
pub async fn tick_once(pool: &PgPool) -> u64 {
    let started = Instant::now();
    let result = match publish_due_posts(pool).await {
        Ok(ids) => ids,
        Err(err) => {
            metrics::counter!("blog_scheduler_tick_failed_total").increment(1);
            tracing::error!(error = %err, "blog-scheduler tick failed");
            return 0;
        }
    };

    let count = result.len() as u64;
    metrics::counter!("blog_scheduler_published_total").increment(count);
    metrics::histogram!("blog_scheduler_tick_duration_seconds")
        .record(started.elapsed().as_secs_f64());
    metrics::gauge!("blog_scheduler_last_success_unixtime")
        .set(chrono::Utc::now().timestamp() as f64);

    if count > 0 {
        for id in &result {
            tracing::info!(post_id = %id, "blog-scheduler published post");
        }
    }

    count
}

/// Long-lived worker. Spawn once per process from `main.rs`. Sleeps
/// `interval` between iterations and exits when `shutdown` fires.
pub async fn run_loop(pool: PgPool, mut shutdown: broadcast::Receiver<()>, interval: Duration) {
    tracing::info!(
        interval_secs = interval.as_secs(),
        "blog-scheduler worker started"
    );
    let mut ticker = tokio::time::interval(interval);
    ticker.tick().await; // discard the immediate first tick

    loop {
        tokio::select! {
            _ = ticker.tick() => {
                let _ = tick_once(&pool).await;
            }
            _ = shutdown.recv() => {
                tracing::info!("blog-scheduler worker received shutdown");
                return;
            }
        }
    }
}
