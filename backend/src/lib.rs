#![deny(warnings)]
#![forbid(unsafe_code)]

//! Shared library crate for the Swings API binary. The library form is used by the
//! binary (`main.rs`) and by integration tests under `backend/tests/` — most notably
//! `tests/openapi_snapshot.rs`, which needs `openapi::ApiDoc` to compare the generated
//! OpenAPI document against the committed snapshot.

use std::sync::Arc;

pub mod authz;
pub mod commerce;
pub mod common;
pub mod config;
pub mod consent;
pub mod db;
pub mod email;
pub mod error;
pub mod events;
pub mod extractors;
pub mod forms;
pub mod handlers;
pub mod middleware;
pub mod models;
pub mod notifications;
pub mod observability;
pub mod openapi;
pub mod pdf;
pub mod popups;
pub mod security;
pub mod services;
pub mod settings;
pub mod stripe_api;

/// Shared application state passed to all Axum handlers.
#[derive(Clone)]
pub struct AppState {
    pub db: sqlx::PgPool,
    pub config: Arc<config::Config>,
    pub email_service: Option<Arc<email::EmailService>>,
    pub media_backend: services::MediaBackend,
    /// FDN-07 authz policy cache. Loaded once at startup from the
    /// `role_permissions` catalogue and atomically replaced via
    /// [`authz::PolicyHandle::reload_from_db`] after admin mutations
    /// to the role/permission matrix.
    pub policy: Arc<authz::PolicyHandle>,
    /// FDN-04 broadcast handle used to tell outbox workers (spawned in
    /// `main.rs`) to drain and exit at shutdown time.
    pub outbox_shutdown: events::WorkerShutdown,
    /// FDN-08: distributed-quota rate-limit backend. Selected via
    /// `RATE_LIMIT_BACKEND=inprocess|postgres` at startup; used by the
    /// Postgres middleware path, inert for the in-process (governor) path.
    pub rate_limit: middleware::rate_limit::Backend,
    /// FDN-05: notifications service — template registry, preference engine,
    /// and channel dispatch. Constructed in `main.rs`; handlers reach for
    /// `state.notifications.channels()` to send outside the outbox path
    /// (e.g. admin `test-send`).
    pub notifications: Arc<notifications::Service>,
    /// ADM-08: hot-cached typed settings catalogue. Reloaded on every
    /// admin mutation; consumed by the maintenance-mode middleware
    /// without a DB round-trip.
    pub settings: settings::Cache,
}
