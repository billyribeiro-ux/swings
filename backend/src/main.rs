#![deny(warnings)]
#![forbid(unsafe_code)]

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{bail, Context, Result};
use axum::http::HeaderValue;
use axum::http::Method;
use axum::Router;
use sqlx::postgres::PgPoolOptions;
use tower_http::cors::{Any, CorsLayer};
use tower_http::services::ServeDir;
use tower_http::trace::TraceLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use swings_api::{
    authz, config::Config, db, email, events, handlers, middleware::rate_limit, notifications,
    observability, openapi, services, AppState,
};

/// `dotenvy::dotenv()` only reads `./.env` from the process CWD. When invoked as
/// `cargo run --manifest-path backend/Cargo.toml` from the repo root, CWD is the root and env
/// vars in `backend/.env` are missed — try that path as a fallback.
fn load_dotenv() {
    dotenvy::dotenv().ok();
    if std::env::var("DATABASE_URL").is_err() {
        let _ = dotenvy::from_filename("backend/.env");
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "swings_api=debug,tower_http=debug".into()),
        )
        .with(observability::tracing_layer())
        .init();

    // Observability: install the Prometheus recorder exactly once so the
    // `/metrics` endpoint + the http_middleware below can emit against it.
    let metrics_handle = observability::install_recorder();

    load_dotenv();

    let config = Config::from_env()?;
    config.assert_production_ready()?;
    let port = config.port;

    let pool = PgPoolOptions::new()
        .max_connections(10)
        .min_connections(0)
        .acquire_timeout(Duration::from_secs(10))
        .idle_timeout(Duration::from_secs(300))
        .max_lifetime(Duration::from_secs(1800))
        .connect(&config.database_url)
        .await
        .context("failed to connect to database")?;

    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .context("failed to run database migrations")?;

    let admin_email = std::env::var("ADMIN_EMAIL").ok();
    let admin_password = std::env::var("ADMIN_PASSWORD").ok();
    let admin_name = std::env::var("ADMIN_NAME").unwrap_or_else(|_| "Admin".to_string());
    match (admin_email, admin_password) {
        (Some(email), Some(password)) => {
            db::seed_admin(&pool, &email, &password, &admin_name)
                .await
                .context("failed to seed admin user")?;
        }
        _ if config.is_production() => {
            bail!("ADMIN_EMAIL and ADMIN_PASSWORD must be set in production");
        }
        _ => {
            tracing::warn!(
                "ADMIN_EMAIL/ADMIN_PASSWORD not set; skipping admin seed in non-production mode"
            );
        }
    }

    // Ensure uploads directory exists
    let upload_dir = config.upload_dir.clone();

    let email_service = if config.smtp_user.is_empty() {
        tracing::warn!("SMTP_USER not configured — email sending is disabled");
        None
    } else {
        match email::EmailService::new(&config) {
            Ok(svc) => {
                tracing::info!("Email service initialized (SMTP: {})", config.smtp_host);
                Some(Arc::new(svc))
            }
            Err(e) => {
                tracing::error!("Failed to initialize email service: {e}");
                None
            }
        }
    };

    let media_backend = services::MediaBackend::resolve(config.upload_dir.clone());

    // FDN-07: hydrate the role → permission policy from the catalogue the
    // `021_rbac.sql` migration seeded. Must run after `sqlx::migrate!` so the
    // tables exist; wrap in `Arc` so handlers can clone the cache cheaply.
    let policy = authz::Policy::load(&pool)
        .await
        .context("failed to load authz policy from role_permissions")?;
    tracing::info!(
        pairs = policy.len(),
        "authz policy loaded from role_permissions"
    );

    // FDN-04: outbox worker shutdown broadcaster. Stored on `AppState` so
    // handlers (and `main` itself) can trigger a cooperative shutdown.
    let outbox_shutdown = events::WorkerShutdown::new();

    // FDN-08: rate-limit backend selection (governor in-process vs Postgres
    // sliding-window). Honors `RATE_LIMIT_BACKEND=inprocess|postgres`.
    let rate_limit_backend = rate_limit::Backend::from_env(pool.clone());

    // FDN-05: notifications service (template registry + channel dispatch).
    // Constructed before we register the outbox `NotifyHandler` so both can
    // share the `Arc<ChannelRegistry>` without cloning the underlying
    // provider wrappers.
    let notifications_service = Arc::new(notifications::Service::new(email_service.clone()));

    let state = AppState {
        db: pool,
        config: Arc::new(config),
        email_service,
        media_backend,
        policy: Arc::new(policy),
        outbox_shutdown: outbox_shutdown.clone(),
        rate_limit: rate_limit_backend,
        notifications: notifications_service.clone(),
    };

    // FDN-04: build the outbox dispatcher (pattern → handler registry) and
    // spawn the configured number of worker tasks. Workers each open their
    // own cloned PgPool handle and lease rows via `SELECT … FOR UPDATE SKIP
    // LOCKED`, so spawning N never races more than the pool's max_connections.
    let outbox_workers_count = std::env::var("OUTBOX_WORKERS")
        .ok()
        .and_then(|v| v.parse::<usize>().ok())
        .filter(|n| *n > 0)
        .unwrap_or(4);
    let outbox_batch_size = std::env::var("OUTBOX_BATCH_SIZE")
        .ok()
        .and_then(|v| v.parse::<i64>().ok())
        .filter(|n| *n > 0)
        .unwrap_or(16);
    let dispatcher = Arc::new(
        events::Dispatcher::new()
            // FDN-05: real notifications adapter. Narrowly scoped to the
            // `notification.*` pattern so unrelated events (e.g. future
            // `form.*` fan-out) do not run through the channel registry.
            .register(
                "notification.*",
                Arc::new(events::handlers::notify::NotifyHandler::new(
                    state.db.clone(),
                    notifications_service.channels().clone(),
                )),
            )
            // FORM-07 will land a real implementation; the stub keeps the
            // dispatcher from reporting NoHandler for outbound webhook events.
            .register(
                "*",
                Arc::new(events::handlers::webhook_out::WebhookOutHandler::new()),
            ),
    );
    let worker_config = events::WorkerConfig {
        batch_size: outbox_batch_size,
    };
    let mut worker_handles: Vec<events::WorkerHandle> = Vec::with_capacity(outbox_workers_count);
    for idx in 0..outbox_workers_count {
        let handle = events::Worker::spawn(
            idx,
            state.db.clone(),
            dispatcher.clone(),
            worker_config,
            outbox_shutdown.subscribe(),
        );
        worker_handles.push(handle);
    }
    tracing::info!(
        workers = outbox_workers_count,
        batch_size = outbox_batch_size,
        "outbox worker pool started"
    );

    let allowed_origins = state
        .config
        .cors_allowed_origins
        .iter()
        .filter_map(|origin| HeaderValue::from_str(origin).ok())
        .collect::<Vec<_>>();
    if allowed_origins.is_empty() {
        bail!("CORS_ALLOWED_ORIGINS (or FRONTEND_URL) must contain at least one valid origin");
    }

    let cors = CorsLayer::new()
        .allow_origin(allowed_origins)
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::DELETE,
            Method::OPTIONS,
        ])
        // Avoid preflight failures when browsers/extensions send non-standard request headers.
        .allow_headers(Any);
    tokio::fs::create_dir_all(&upload_dir)
        .await
        .with_context(|| format!("failed to create upload directory at {upload_dir}"))?;

    let mount_local_uploads = !(state.config.is_production() && state.media_backend.is_r2());

    let mut app = Router::new()
        // Auth & analytics
        .nest("/api/auth", handlers::auth::router())
        .nest("/api/analytics", handlers::analytics::router())
        // Admin routes
        .nest("/api/admin", handlers::admin::router())
        .nest("/api/admin/blog", handlers::blog::admin_router())
        .nest("/api/admin/courses", handlers::courses::admin_router())
        .nest("/api/admin/pricing", handlers::pricing::admin_router())
        .nest("/api/admin/coupons", handlers::coupons::admin_router())
        .nest("/api/admin/popups", handlers::popups::admin_router())
        .nest("/api/admin/outbox", handlers::outbox::router())
        .nest(
            "/api/admin/notifications",
            handlers::notifications::admin_router(),
        )
        // Public routes
        .nest("/api/blog", handlers::blog::public_router())
        .nest("/api/courses", handlers::courses::public_router())
        .nest("/api/pricing", handlers::pricing::public_router())
        .nest("/api/coupons", handlers::coupons::public_router())
        .nest("/api/popups", handlers::popups::public_router())
        // Member routes
        .nest("/api/member", handlers::member::router())
        .nest("/api/member", handlers::courses::member_router())
        .nest("/api/member", handlers::notifications::member_router())
        // Webhooks
        .nest("/api/webhooks", handlers::webhooks::router())
        // Security reports (FDN-08)
        .nest("/api", handlers::csp_report::router())
        // FDN-05 public unsubscribe — no /api prefix so bounce links work
        // from any mailbox client.
        .nest("/u", handlers::notifications::public_router());

    if mount_local_uploads {
        app = app.nest_service("/uploads", ServeDir::new(&upload_dir));
    }

    // FDN-02: OpenAPI spec + SwaggerUI. Mount before CORS so gated responses still get the layer.
    app = openapi::mount(app, &state);

    // Observability: /metrics is admin-gated in production, public in dev,
    // mirroring the openapi::mount gating pattern.
    let metrics_route = if state.config.is_production() {
        axum::routing::get(observability::handler::admin_metrics_handler)
    } else {
        axum::routing::get(observability::handler::public_metrics_handler)
    };
    app = app.route("/metrics", metrics_route);

    let app = app
        .layer(axum::Extension(metrics_handle))
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            observability::metrics::http_middleware,
        ))
        .layer(axum::middleware::from_fn(
            observability::correlation::middleware,
        ))
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{port}"))
        .await
        .with_context(|| format!("failed to bind TCP listener on port {port}"))?;

    tracing::info!("Swings API listening on port {port}");
    let serve = axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown_signal());
    serve.await.context("axum server terminated unexpectedly")?;

    // FDN-04: tell outbox workers to drain. `shutdown()` is a broadcast — each
    // worker wakes, finishes its in-flight claim, and exits. A bounded grace
    // window keeps a hung handler from blocking process exit forever.
    let fired = outbox_shutdown.shutdown();
    tracing::info!(
        workers_notified = fired,
        "outbox worker shutdown signal broadcast"
    );
    let grace = Duration::from_secs(10);
    let join_all = async {
        for handle in worker_handles {
            let id = handle.id();
            if let Err(e) = handle.join().await {
                tracing::warn!(worker = id, error = %e, "outbox worker exited with error");
            }
        }
    };
    if tokio::time::timeout(grace, join_all).await.is_err() {
        tracing::warn!(
            grace_secs = grace.as_secs(),
            "outbox workers did not drain within grace window; forcing process exit"
        );
    } else {
        tracing::info!("outbox workers drained cleanly");
    }

    Ok(())
}

/// Resolve either `ctrl_c` or (on Unix) `SIGTERM` — whichever arrives first —
/// and return so the caller can kick off the shutdown sequence.
async fn shutdown_signal() {
    let ctrl_c = async {
        if let Err(e) = tokio::signal::ctrl_c().await {
            tracing::error!(error = %e, "failed to install ctrl_c handler");
            // Fall back to never resolving so SIGTERM can still win.
            std::future::pending::<()>().await;
        }
    };

    #[cfg(unix)]
    let terminate = async {
        use tokio::signal::unix::{signal, SignalKind};
        match signal(SignalKind::terminate()) {
            Ok(mut stream) => {
                stream.recv().await;
            }
            Err(e) => {
                tracing::error!(error = %e, "failed to install SIGTERM handler");
                std::future::pending::<()>().await;
            }
        }
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => tracing::info!("ctrl_c received; shutting down"),
        _ = terminate => tracing::info!("SIGTERM received; shutting down"),
    }
}
