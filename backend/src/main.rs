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
mod error;
mod extractors;
mod handlers;
mod models;
mod middleware;
mod stripe_api;

use config::Config;

#[derive(Clone)]
pub struct AppState {
    pub db: sqlx::PgPool,
    pub config: Arc<Config>,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| "swings_api=debug,tower_http=debug".into()))
        .with(tracing_subscriber::fmt::layer())
        .init();

    dotenvy::dotenv().ok();

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

    // Seed admin user
    db::seed_admin(
        &pool,
        "welberribeirodrums@gmail.com",
        "Davedicenso01!!!",
        "Billy Ribeiro",
    )
    .await
    .expect("Failed to seed admin user");

    // Ensure uploads directory exists
    let upload_dir = config.upload_dir.clone();

    let state = AppState {
        db: pool,
        config: Arc::new(config),
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
