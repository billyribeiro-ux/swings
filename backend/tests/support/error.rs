//! Typed error surface for the integration-test harness.
//!
//! Every fallible helper in `support::*` returns a `Result<_, TestAppError>`.
//! Tests that use the harness can convert errors into panics with contextual
//! detail via [`TestAppError`]'s `Display` impl, which is what `expect()` on
//! a `Result<T, TestAppError>` prints.

use thiserror::Error;

/// Error surface for the harness utilities. Each variant carries enough context
/// to diagnose failures without the user needing a debugger.
#[derive(Debug, Error)]
pub enum TestAppError {
    /// Neither `DATABASE_URL_TEST` nor `DATABASE_URL` is set or usable.
    #[error("test database not configured: {0}")]
    MissingDatabase(String),

    /// A PostgreSQL / sqlx runtime error.
    #[error("database error: {0}")]
    Database(#[from] sqlx::Error),

    /// Running the `sqlx::Migrator` against the test schema failed.
    #[error("migration error: {0}")]
    Migrate(#[from] sqlx::migrate::MigrateError),

    /// Error constructing the in-process `Config`, router, or seeding state.
    #[error("config error: {0}")]
    Config(String),

    /// Hashing, signing, or token generation in `support::user`.
    #[error("auth/crypto error: {0}")]
    Auth(String),

    /// Filesystem error constructing a temporary uploads directory.
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    /// Axum / tower request-dispatch failure.
    #[error("http dispatch error: {0}")]
    Http(String),

    /// Decoding or parsing a response body.
    #[error("response decode error: {0}")]
    Decode(String),
}

pub type TestResult<T> = Result<T, TestAppError>;
