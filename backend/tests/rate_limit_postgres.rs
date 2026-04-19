#![deny(warnings)]
#![forbid(unsafe_code)]

//! FDN-08 integration coverage for the Postgres-backed rate-limit
//! backend. Exercises the real `rate_limit_buckets` table (migration
//! `022_rate_limits.sql`) under concurrent load so we can prove the
//! `INSERT … ON CONFLICT DO UPDATE` + `SUM(count) FILTER (WHERE …)` path
//! enforces the sliding-window quota atomically across tasks.
//!
//! Previously the same scenario lived inside `src/middleware/rate_limit.rs`
//! behind `#[ignore = "requires Postgres TEST DB"]`, which meant it never
//! ran — neither locally nor in CI — and left the parallel-atomicity
//! guarantee untested. It now runs in the regular integration-test suite
//! against a pristine schema provisioned by the `support::TestDb` harness,
//! so every `cargo test` invocation that has a test Postgres available
//! (`DATABASE_URL_TEST` or `DATABASE_URL`) proves the invariant.
//!
//! Skipped cleanly — not ignored — when no test database is configured,
//! mirroring the pattern used by the other harness-backed integration
//! tests in this directory.

mod support;

use std::sync::Arc;

use sqlx::Row;
use support::TestDb;
use swings_api::middleware::rate_limit::{KeyStrategy, Policy, PostgresBackend, RateLimitBackend};

/// Burst 10 tasks at a 5-rps policy; 5 must succeed, 5 must be rejected.
/// If the Postgres backend ever loses atomicity (for example by dropping
/// the `ON CONFLICT` clause or introducing a read-modify-write gap) this
/// test will flake because more than five tasks will observe the bucket
/// below the cap simultaneously.
#[tokio::test]
async fn postgres_backend_increments_atomically_on_parallel_invocation() {
    let Ok(db) = TestDb::new().await else {
        eprintln!(
            "skipping rate_limit_postgres: no test Postgres (DATABASE_URL_TEST / DATABASE_URL)"
        );
        return;
    };

    let backend = Arc::new(PostgresBackend::new(db.pool().clone()));
    let policy = Policy {
        name: "unit-test",
        max_requests: 5,
        window_secs: 1,
        key: KeyStrategy::Ip,
    };
    // Every test gets a fresh schema, so the key collision surface is
    // already zero; the UUID suffix is belt-and-braces for the case where
    // a later test in the same binary reuses the key constant.
    let key = format!("unit-test:pg:{}", uuid::Uuid::new_v4().simple());

    let mut handles: Vec<tokio::task::JoinHandle<bool>> = Vec::with_capacity(10);
    for _ in 0..10 {
        let b = backend.clone();
        let k = key.clone();
        handles.push(tokio::spawn(async move {
            RateLimitBackend::check(&*b, policy, &k).await.is_ok()
        }));
    }

    let mut allowed = 0_u32;
    for h in handles {
        if h.await.expect("rate-limit task joined cleanly") {
            allowed += 1;
        }
    }
    assert_eq!(
        allowed, 5,
        "exactly 5/10 parallel attempts should be admitted under a 5-rps policy"
    );

    // And the bucket row actually landed — guards against a silent no-op
    // where every check would return `Ok` because no rows were written.
    let rows: i64 = sqlx::query(
        "SELECT COALESCE(SUM(count), 0)::BIGINT AS total
         FROM rate_limit_buckets
         WHERE key = $1",
    )
    .bind(&key)
    .fetch_one(db.pool())
    .await
    .expect("bucket lookup")
    .get("total");
    assert!(
        rows >= 5,
        "expected at least 5 bucket increments, saw {rows}"
    );
}
