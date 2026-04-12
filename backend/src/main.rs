use std::sync::Arc;

use axum::Router;
use sqlx::postgres::PgPoolOptions;
use tower_http::cors::{Any, CorsLayer};
use axum::http::header::{AUTHORIZATION, CONTENT_TYPE, ACCEPT};
use axum::http::Method;
use tower_http::services::ServeDir;
use tower_http::trace::TraceLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod config;
mod db;
mod email;
mod error;
mod extractors;
mod handlers;
mod models;
mod middleware;
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
        .with(tracing_subscriber::EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| "swings_api=debug,tower_http=debug".into()))
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

    // Seed admin (override via ADMIN_EMAIL / ADMIN_PASSWORD / ADMIN_NAME in backend/.env)
    let admin_email = std::env::var("ADMIN_EMAIL").unwrap_or_else(|_| {
        "welberribeirodrums@gmail.com".to_string()
    });
    let admin_password =
        std::env::var("ADMIN_PASSWORD").unwrap_or_else(|_| "Davedicenso01!!!".to_string());
    let admin_name =
        std::env::var("ADMIN_NAME").unwrap_or_else(|_| "Billy Ribeiro".to_string());
    db::seed_admin(&pool, &admin_email, &admin_password, &admin_name)
        .await
        .expect("Failed to seed admin user");

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

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::DELETE,
            Method::OPTIONS,
        ])
        .allow_headers([AUTHORIZATION, CONTENT_TYPE, ACCEPT]);
    tokio::fs::create_dir_all(&upload_dir)
        .await
        .expect("Failed to create uploads directory");

    let app = Router::new()
        .nest("/api/auth", handlers::auth::router())
        .nest("/api/analytics", handlers::analytics::router())
        .nest("/api/admin", handlers::admin::router())
        .nest("/api/admin/blog", handlers::blog::admin_router())
        .nest("/api/blog", handlers::blog::public_router())
        .nest("/api/member", handlers::member::router())
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
