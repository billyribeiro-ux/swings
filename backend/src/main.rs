use std::sync::Arc;

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
}

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "swings_api=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    load_dotenv();

    let config = Config::from_env();
    let port = config.port;

    let pool = PgPoolOptions::new()
        .max_connections(10)
        .connect(&config.database_url)
        .await
        .expect("Failed to connect to database");

    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("Failed to run migrations");

    let admin_email = std::env::var("ADMIN_EMAIL").ok();
    let admin_password = std::env::var("ADMIN_PASSWORD").ok();
    let admin_name = std::env::var("ADMIN_NAME").unwrap_or_else(|_| "Admin".to_string());
    match (admin_email, admin_password) {
        (Some(email), Some(password)) => {
            db::seed_admin(&pool, &email, &password, &admin_name)
                .await
                .expect("Failed to seed admin user");
        }
        _ if config.is_production() => {
            panic!("ADMIN_EMAIL and ADMIN_PASSWORD must be set in production");
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

    let state = AppState {
        db: pool,
        config: Arc::new(config),
        email_service,
    };

    let allowed_origins = state
        .config
        .cors_allowed_origins
        .iter()
        .filter_map(|origin| HeaderValue::from_str(origin).ok())
        .collect::<Vec<_>>();
    if allowed_origins.is_empty() {
        panic!("CORS_ALLOWED_ORIGINS (or FRONTEND_URL) must contain at least one valid origin");
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
        .expect("Failed to create uploads directory");

    let app = Router::new()
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
        // Webhooks & uploads
        .nest("/api/webhooks", handlers::webhooks::router())
        .nest_service("/uploads", ServeDir::new(&upload_dir))
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{port}"))
        .await
        .unwrap();

    tracing::info!("Swings API listening on port {port}");
    axum::serve(listener, app).await.unwrap();
}
