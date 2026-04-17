//! Ephemeral per-test Postgres isolation via dedicated schemas.
//!
//! ## Why schema-level isolation?
//!
//! The `swings-api` crate's `Cargo.toml` cannot be touched from this harness, so
//! we cannot add `sqlx-testcontainers`, spawn Docker, or enable the sqlx
//! `macros` feature needed by `#[sqlx::test]`. A database-per-test would require
//! CREATE DATABASE privileges and external orchestration; a SCHEMA-per-test
//! only needs `CREATE SCHEMA` (and a reused pool), which both local dev boxes
//! and shared CI Postgres instances comfortably grant.
//!
//! ## Isolation semantics
//!
//! Each [`TestDb::new`] call:
//! 1. Connects to `DATABASE_URL_TEST` (or falls back to `DATABASE_URL`).
//! 2. Generates a fresh `test_<hex32>` schema name.
//! 3. Creates that schema and pins it on every pooled connection via a
//!    `SET search_path = '<schema>'` hook in [`PgPoolOptions::after_connect`].
//! 4. Runs the committed `sqlx::migrate!("./migrations")` set into the schema.
//!
//! Every handler/query in the `swings-api` crate uses unqualified relation names
//! (e.g. `FROM users`), so with `search_path` pinned they resolve against the
//! test schema instead of `public` — no application code needs to know it is
//! running under a sandbox.
//!
//! On `Drop`, the schema is removed with `DROP SCHEMA … CASCADE` on a best
//! effort basis: if the user's test panicked while holding a connection, the
//! drop blocks only briefly before the pool is torn down by Tokio. Orphans are
//! harmless beyond disk usage and can be cleaned up with the SQL one-liner
//! documented in `FDN-TESTHARNESS-WIRING.md`.

use std::time::Duration;

use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
use sqlx::{Executor, PgPool};
use uuid::Uuid;

use super::error::{TestAppError, TestResult};

/// Integration-test database handle bound to a single throwaway schema.
///
/// Cloning shares the underlying `PgPool` (cheap — `Arc`-backed). The schema is
/// dropped on the last owner's `Drop`.
pub struct TestDb {
    pool: PgPool,
    schema: String,
    /// Toggle to skip `DROP SCHEMA` when the harness user wants to inspect the
    /// sandbox after a failure. Populated from `KEEP_TEST_SCHEMA=1`.
    keep_on_drop: bool,
}

impl TestDb {
    /// Build a fresh schema and run the migration set against it.
    ///
    /// Resolves the database URL in this order:
    /// 1. `DATABASE_URL_TEST`
    /// 2. `DATABASE_URL`
    ///
    /// Returns [`TestAppError::MissingDatabase`] if neither is set.
    pub async fn new() -> TestResult<Self> {
        let url = std::env::var("DATABASE_URL_TEST")
            .or_else(|_| std::env::var("DATABASE_URL"))
            .map_err(|_| {
                TestAppError::MissingDatabase(
                    "neither DATABASE_URL_TEST nor DATABASE_URL is set".into(),
                )
            })?;

        let schema = format!("test_{}", Uuid::new_v4().simple());
        let keep_on_drop = std::env::var("KEEP_TEST_SCHEMA").as_deref() == Ok("1");

        // Admin connection to create the schema, outside the search_path-scoped pool.
        let admin_opts: PgConnectOptions = url.parse().map_err(|e: sqlx::Error| {
            TestAppError::MissingDatabase(format!("invalid DATABASE_URL_TEST/DATABASE_URL: {e}"))
        })?;
        let admin_pool = PgPoolOptions::new()
            .max_connections(1)
            .acquire_timeout(Duration::from_secs(10))
            .connect_with(admin_opts.clone())
            .await?;

        sqlx::query(&format!("CREATE SCHEMA \"{schema}\""))
            .execute(&admin_pool)
            .await?;
        admin_pool.close().await;

        // Test pool: `search_path` pinned on every connection so unqualified
        // relation names resolve against the sandbox schema.
        let schema_for_hook = schema.clone();
        let pool = PgPoolOptions::new()
            .max_connections(5)
            .min_connections(0)
            .acquire_timeout(Duration::from_secs(10))
            .idle_timeout(Duration::from_secs(60))
            .after_connect(move |conn, _meta| {
                let schema = schema_for_hook.clone();
                Box::pin(async move {
                    conn.execute(format!("SET search_path TO \"{schema}\"").as_str())
                        .await?;
                    Ok(())
                })
            })
            .connect_with(admin_opts)
            .await?;

        sqlx::migrate!("./migrations")
            .run(&pool)
            .await
            .map_err(TestAppError::from)?;

        Ok(Self {
            pool,
            schema,
            keep_on_drop,
        })
    }

    #[must_use]
    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    #[must_use]
    pub fn schema_name(&self) -> &str {
        &self.schema
    }
}

impl Drop for TestDb {
    fn drop(&mut self) {
        if self.keep_on_drop {
            tracing::warn!(
                schema = %self.schema,
                "KEEP_TEST_SCHEMA=1 — leaving schema in place for inspection"
            );
            return;
        }

        // Best-effort teardown. The pool may have open connections still in
        // flight; `close` waits up to a short timeout before we drop the
        // schema. A surviving runtime is available on nearly every use case
        // because tests run inside `#[tokio::test]`, but we use a
        // fallback-to-thread dance for cases where they are not.
        let pool = self.pool.clone();
        let schema = self.schema.clone();
        let work = async move {
            pool.close().await;
            // Reconnect on a single admin connection to drop the schema.
            let url = std::env::var("DATABASE_URL_TEST")
                .or_else(|_| std::env::var("DATABASE_URL"))
                .ok();
            if let Some(url) = url {
                if let Ok(admin) = PgPoolOptions::new()
                    .max_connections(1)
                    .acquire_timeout(Duration::from_secs(5))
                    .connect(&url)
                    .await
                {
                    let stmt = format!("DROP SCHEMA IF EXISTS \"{schema}\" CASCADE");
                    if let Err(e) = sqlx::query(&stmt).execute(&admin).await {
                        tracing::warn!(%schema, "failed to drop test schema: {e}");
                    }
                    admin.close().await;
                }
            }
        };

        match tokio::runtime::Handle::try_current() {
            Ok(handle) => {
                // Spawn on the current runtime without blocking it; if the
                // runtime is shutting down the task is simply dropped, leaving
                // at most an orphaned schema (covered by the cleanup SQL in
                // the wiring doc).
                handle.spawn(work);
            }
            Err(_) => {
                // No active runtime (rare for tests). Start a minimal one.
                if let Ok(rt) = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                {
                    rt.block_on(work);
                }
            }
        }
    }
}
