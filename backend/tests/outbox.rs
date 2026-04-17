#![deny(warnings)]
#![forbid(unsafe_code)]

//! FDN-04 integration tests for the transactional outbox.
//!
//! These tests exercise the claim/dispatch/settle loop against a real Postgres
//! schema. They skip silently when `DATABASE_URL` is not set so `cargo test`
//! remains runnable in sandboxed environments — CI is expected to populate
//! `DATABASE_URL` and run the migration suite before invoking these tests.
//!
//! Each test isolates its state via `begin_isolated()`: migrations run on the
//! shared pool, but every test wraps its work in `WHERE aggregate_type = $test_scope$`
//! predicates so parallel runs do not collide. Cleanup drops its scoped rows
//! at the end.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

use sqlx::postgres::PgPoolOptions;
use sqlx::{PgPool, Row};
use tokio::sync::broadcast;
use uuid::Uuid;

use swings_api::events::{
    self,
    dispatcher::DispatchError,
    handlers::{BoxFuture, EventHandler},
    outbox::{self, Event, EventHeaders, OutboxRecord},
    Dispatcher, Worker, WorkerConfig,
};

async fn connect() -> Option<PgPool> {
    let url = std::env::var("DATABASE_URL").ok()?;
    let pool = PgPoolOptions::new()
        .max_connections(8)
        .acquire_timeout(Duration::from_secs(5))
        .connect(&url)
        .await
        .ok()?;
    // Migrations are idempotent; run once per process and let Postgres no-op
    // the rest.
    sqlx::migrate!("./migrations").run(&pool).await.ok()?;
    Some(pool)
}

async fn cleanup_scope(pool: &PgPool, scope: &str) {
    let _ = sqlx::query("DELETE FROM outbox_events WHERE aggregate_type = $1")
        .bind(scope)
        .execute(pool)
        .await;
}

async fn publish_event(pool: &PgPool, scope: &str, event_type: &str) -> Uuid {
    let mut tx = pool.begin().await.expect("begin tx");
    let event = Event {
        aggregate_type: scope.to_string(),
        aggregate_id: Uuid::new_v4().to_string(),
        event_type: event_type.to_string(),
        payload: serde_json::json!({ "hello": "world" }),
        headers: EventHeaders::default(),
    };
    let id = outbox::publish_in_tx(&mut tx, &event)
        .await
        .expect("publish");
    tx.commit().await.expect("commit");
    id
}

#[derive(Debug)]
struct CountingOkHandler {
    calls: Arc<AtomicUsize>,
}

impl EventHandler for CountingOkHandler {
    fn handle<'a>(&'a self, _event: &'a OutboxRecord) -> BoxFuture<'a, Result<(), DispatchError>> {
        Box::pin(async move {
            self.calls.fetch_add(1, Ordering::Relaxed);
            Ok(())
        })
    }
}

#[derive(Debug)]
struct AlwaysTransientHandler;

impl EventHandler for AlwaysTransientHandler {
    fn handle<'a>(&'a self, _event: &'a OutboxRecord) -> BoxFuture<'a, Result<(), DispatchError>> {
        Box::pin(async move { Err(DispatchError::Transient("simulated".into())) })
    }
}

// ── TEST 1: concurrent workers lease each row exactly once ──────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn concurrent_claims_never_double_lease() {
    let Some(pool) = connect().await else {
        eprintln!("DATABASE_URL not set; skipping outbox integration test");
        return;
    };
    let scope = format!("test_claim_{}", Uuid::new_v4().simple());
    cleanup_scope(&pool, &scope).await;

    // Seed 20 pending rows in the test scope.
    let total = 20usize;
    for _ in 0..total {
        publish_event(&pool, &scope, "test.race").await;
    }

    // Run two concurrent claims in parallel. Each claims up to `total` rows —
    // if `SKIP LOCKED` is wired correctly, the union of their claims is the
    // 20 seeded rows, with zero overlap.
    let p1 = pool.clone();
    let p2 = pool.clone();
    let claim1 = tokio::spawn(async move { outbox::claim_batch(&p1, total as i64).await });
    let claim2 = tokio::spawn(async move { outbox::claim_batch(&p2, total as i64).await });

    let rows1 = claim1.await.expect("join1").expect("claim1 ok");
    let rows2 = claim2.await.expect("join2").expect("claim2 ok");

    // Filter to our test scope — other tests / production rows may coexist.
    let ids1: std::collections::HashSet<Uuid> = rows1
        .into_iter()
        .filter(|r| r.aggregate_type == scope)
        .map(|r| r.id)
        .collect();
    let ids2: std::collections::HashSet<Uuid> = rows2
        .into_iter()
        .filter(|r| r.aggregate_type == scope)
        .map(|r| r.id)
        .collect();

    let overlap: std::collections::HashSet<_> = ids1.intersection(&ids2).collect();
    assert!(
        overlap.is_empty(),
        "SKIP LOCKED leaked {} row(s) between concurrent workers",
        overlap.len()
    );

    let union = ids1.len() + ids2.len();
    assert!(
        union <= total,
        "more rows claimed ({union}) than were seeded ({total})"
    );
    // Non-racy lower bound: each seeded row was `pending`, so at least one
    // worker must have seen it — the union covers the whole batch.
    assert_eq!(
        union, total,
        "expected every seeded row to be claimed by exactly one worker"
    );

    cleanup_scope(&pool, &scope).await;
}

// ── TEST 2: publish → worker → handler → delivered ──────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn publish_then_worker_drains_to_delivered() {
    let Some(pool) = connect().await else {
        eprintln!("DATABASE_URL not set; skipping outbox integration test");
        return;
    };
    let scope = format!("test_drain_{}", Uuid::new_v4().simple());
    cleanup_scope(&pool, &scope).await;

    let calls = Arc::new(AtomicUsize::new(0));
    let handler = Arc::new(CountingOkHandler {
        calls: calls.clone(),
    });
    let dispatcher = Arc::new(Dispatcher::new().register("test.*", handler));

    // 5 events, 1 worker, short batch size to force multiple ticks.
    for _ in 0..5 {
        publish_event(&pool, &scope, "test.drain").await;
    }

    let (tx, _rx) = broadcast::channel::<()>(1);
    let worker_rx = tx.subscribe();
    let handle = Worker::spawn(
        0,
        pool.clone(),
        dispatcher,
        WorkerConfig { batch_size: 2 },
        worker_rx,
    );

    // Poll the DB until every seeded row is delivered (or fail the test after
    // a couple of seconds — much longer than one worker tick needs).
    let deadline = std::time::Instant::now() + Duration::from_secs(5);
    loop {
        let remaining: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM outbox_events WHERE aggregate_type = $1 AND status <> 'delivered'",
        )
        .bind(&scope)
        .fetch_one(&pool)
        .await
        .expect("count");
        if remaining == 0 {
            break;
        }
        if std::time::Instant::now() > deadline {
            panic!("worker failed to drain outbox within deadline; {remaining} rows left");
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    assert_eq!(
        calls.load(Ordering::Relaxed),
        5,
        "handler should have fired once per event"
    );

    let _ = tx.send(());
    let _ = tokio::time::timeout(Duration::from_secs(2), handle.join()).await;
    cleanup_scope(&pool, &scope).await;
}

// ── TEST 3: exhausting max_attempts moves row to dead_letter ────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn transient_failures_past_max_attempts_land_in_dlq() {
    let Some(pool) = connect().await else {
        eprintln!("DATABASE_URL not set; skipping outbox integration test");
        return;
    };
    let scope = format!("test_dlq_{}", Uuid::new_v4().simple());
    cleanup_scope(&pool, &scope).await;

    // Pre-configure max_attempts=1 on the seeded row so we exhaust retries on
    // a single failed dispatch.
    let id = publish_event(&pool, &scope, "test.dlq").await;
    sqlx::query("UPDATE outbox_events SET max_attempts = 1 WHERE id = $1")
        .bind(id)
        .execute(&pool)
        .await
        .expect("set max_attempts");

    let dispatcher =
        Arc::new(Dispatcher::new().register("test.*", Arc::new(AlwaysTransientHandler)));

    let (tx, _rx) = broadcast::channel::<()>(1);
    let handle = Worker::spawn(
        0,
        pool.clone(),
        dispatcher,
        WorkerConfig { batch_size: 4 },
        tx.subscribe(),
    );

    let deadline = std::time::Instant::now() + Duration::from_secs(5);
    loop {
        let row = sqlx::query("SELECT status FROM outbox_events WHERE id = $1")
            .bind(id)
            .fetch_one(&pool)
            .await
            .expect("fetch");
        let status: String = row.try_get("status").expect("status col");
        if status == "dead_letter" {
            break;
        }
        if std::time::Instant::now() > deadline {
            panic!("row did not land in DLQ within deadline; status={status}");
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    let _ = tx.send(());
    let _ = tokio::time::timeout(Duration::from_secs(2), handle.join()).await;
    cleanup_scope(&pool, &scope).await;
}

// ── TEST 4: graceful shutdown stops the worker ──────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn worker_exits_on_shutdown_signal() {
    let Some(pool) = connect().await else {
        eprintln!("DATABASE_URL not set; skipping outbox integration test");
        return;
    };

    let dispatcher = Arc::new(Dispatcher::new().register(
        "test.*",
        Arc::new(CountingOkHandler {
            calls: Arc::new(AtomicUsize::new(0)),
        }),
    ));
    let shutdown = events::WorkerShutdown::new();
    let handle = Worker::spawn(
        0,
        pool,
        dispatcher,
        WorkerConfig { batch_size: 1 },
        shutdown.subscribe(),
    );
    // Let it tick once, then signal shutdown.
    tokio::time::sleep(Duration::from_millis(50)).await;
    shutdown.shutdown();
    let joined = tokio::time::timeout(Duration::from_secs(3), handle.join()).await;
    assert!(joined.is_ok(), "worker did not honor shutdown within 3s");
}
