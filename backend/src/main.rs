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

mod config;
mod db;
mod email;
mod error;
mod extractors;
mod handlers;
mod middleware;
mod models;
mod services;
mod stripe_api;

use config::Config;

/// `dotenvy::dotenv()` only reads `./.env` from the process CWD. When invoked as
/// `cargo run --manifest-path backend/Cargo.toml` from the repo root, CWD is the root and env
/// vars in `backend/.env` are missed — try that path as a fallback.
fn load_dotenv() {
    dotenvy::dotenv().ok();
    if std::env::var("DATABASE_URL").is_err() {
        let _ = dotenvy::from_filename("backend/.env");
    }
}

#[derive(Clone)]
pub struct AppState {
    pub db: sqlx::PgPool,
    pub config: Arc<Config>,
    pub email_service: Option<Arc<email::EmailService>>,
    pub media_backend: services::MediaBackend,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "swings_api=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

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

    let state = AppState {
        db: pool,
        config: Arc::new(config),
        email_service,
        media_backend,
    };

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
        // Public routes
        .nest("/api/blog", handlers::blog::public_router())
        .nest("/api/courses", handlers::courses::public_router())
        .nest("/api/pricing", handlers::pricing::public_router())
        .nest("/api/coupons", handlers::coupons::public_router())
        .nest("/api/popups", handlers::popups::public_router())
        // Member routes
        .nest("/api/member", handlers::member::router())
        .nest("/api/member", handlers::courses::member_router())
        // Webhooks
        .nest("/api/webhooks", handlers::webhooks::router());

    if mount_local_uploads {
        app = app.nest_service("/uploads", ServeDir::new(&upload_dir));
    }

    let app = app
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{port}"))
        .await
        .with_context(|| format!("failed to bind TCP listener on port {port}"))?;

    tracing::info!("Swings API listening on port {port}");
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await
    .context("axum server terminated unexpectedly")?;

    Ok(())
}
