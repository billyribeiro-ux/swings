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

/// Load `backend/.env` regardless of process CWD (`cargo run` from `backend/`, from the
/// monorepo root with `--manifest-path`, or from an IDE that sets CWD to the workspace root).
/// Then load `./.env` from CWD so local overrides still work.
fn load_dotenv() {
    let backend_env = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(".env");
    let _ = dotenvy::from_path(&backend_env);
    let _ = dotenvy::dotenv();
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

    // Postgres pool sizing is env-driven so operators can tune per-env
    // without a redeploy. Defaults are the "safe for a single 2-vCPU
    // container" numbers that match a small managed Postgres (e.g. Neon
    // free, Railway hobby). Production should set `PGPOOL_MAX` based on
    // `(server max_connections - other clients) / container replicas`.
    //
    // * `PGPOOL_MAX`             — default 10, ceiling of in-flight queries
    // * `PGPOOL_MIN`             — default 0,  warm-pool floor
    // * `PGPOOL_ACQUIRE_TIMEOUT` — default 10 s, slow-dependency surface
    // * `PGPOOL_IDLE_TIMEOUT`    — default 300 s, drop long-idle conns so
    //                              Postgres-side `idle_in_transaction`
    //                              watchdogs don't kill us first
    // * `PGPOOL_MAX_LIFETIME`    — default 1800 s, hedge against proxy-
    //                              level TCP timeouts / PgBouncer recycling
    let pgpool_max: u32 = std::env::var("PGPOOL_MAX")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(10);
    let pgpool_min: u32 = std::env::var("PGPOOL_MIN")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(0);
    let pgpool_acquire_timeout_secs: u64 = std::env::var("PGPOOL_ACQUIRE_TIMEOUT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(10);
    let pgpool_idle_timeout_secs: u64 = std::env::var("PGPOOL_IDLE_TIMEOUT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(300);
    let pgpool_max_lifetime_secs: u64 = std::env::var("PGPOOL_MAX_LIFETIME")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(1800);

    tracing::info!(
        max = pgpool_max,
        min = pgpool_min,
        acquire_timeout_s = pgpool_acquire_timeout_secs,
        idle_timeout_s = pgpool_idle_timeout_secs,
        max_lifetime_s = pgpool_max_lifetime_secs,
        "postgres pool configured"
    );

    let pool = PgPoolOptions::new()
        .max_connections(pgpool_max)
        .min_connections(pgpool_min)
        .acquire_timeout(Duration::from_secs(pgpool_acquire_timeout_secs))
        .idle_timeout(Duration::from_secs(pgpool_idle_timeout_secs))
        .max_lifetime(Duration::from_secs(pgpool_max_lifetime_secs))
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

    // FDN-09: select the email provider from the `EMAIL_PROVIDER` env var.
    //
    // Matrix:
    //   * EMAIL_PROVIDER=resend  -> Resend HTTP API (always, regardless of env)
    //   * EMAIL_PROVIDER=smtp    -> Lettre SMTP transport (reuses EmailService)
    //   * EMAIL_PROVIDER=noop    -> log-only stub (never hits the wire)
    //   * unset + production     -> resend (fails fast if RESEND_API_KEY missing
    //                                via Config::assert_production_ready)
    //   * unset + dev + SMTP_USER-> smtp  (legacy dev workflow preserved)
    //   * unset + dev + no SMTP  -> noop  (`pnpm dev` runs without an inbox)
    //
    // We still construct a full [`email::EmailService`] for Lettre mode to keep
    // the SMTP codepath unchanged. Resend + Noop never touch it, so missing
    // SMTP config is fine when they are active.
    let email_service: Option<Arc<email::EmailService>> = if config.smtp_user.is_empty() {
        None
    } else {
        match email::EmailService::new(&config) {
            Ok(svc) => Some(Arc::new(svc)),
            Err(e) => {
                tracing::error!("SMTP EmailService init failed: {e}");
                None
            }
        }
    };
    let email_provider: Option<Arc<dyn notifications::EmailProvider>> = {
        use notifications::channels::email::{LettreProvider, NoopProvider, ResendProvider};
        let selection = std::env::var("EMAIL_PROVIDER").ok();
        let selection = match selection.as_deref() {
            Some(s) => s.trim().to_ascii_lowercase(),
            None => {
                if config.is_production() {
                    "resend".to_string()
                } else if email_service.is_some() {
                    "smtp".to_string()
                } else {
                    "noop".to_string()
                }
            }
        };
        match selection.as_str() {
            "resend" => match ResendProvider::from_env() {
                Ok(p) => {
                    tracing::info!("email provider: resend");
                    Some(Arc::new(p) as Arc<dyn notifications::EmailProvider>)
                }
                Err(e) => {
                    // In production, treat as a hard failure — `assert_production_ready`
                    // enforces the key already, so we should never land here. In
                    // dev, fall back to the noop provider so `cargo run` does not
                    // bail when `RESEND_API_KEY` is intentionally unset.
                    if config.is_production() {
                        bail!("EMAIL_PROVIDER=resend but config invalid: {e}");
                    }
                    tracing::warn!("resend provider init failed ({e}); falling back to noop");
                    Some(Arc::new(NoopProvider::new()) as Arc<dyn notifications::EmailProvider>)
                }
            },
            "smtp" => match email_service.clone() {
                Some(svc) => {
                    tracing::info!("email provider: lettre (SMTP {})", config.smtp_host);
                    Some(Arc::new(LettreProvider::new(svc)) as Arc<dyn notifications::EmailProvider>)
                }
                None => {
                    tracing::warn!("EMAIL_PROVIDER=smtp but SMTP_USER missing; disabling email");
                    None
                }
            },
            "noop" => {
                tracing::info!("email provider: noop");
                Some(Arc::new(NoopProvider::new()) as Arc<dyn notifications::EmailProvider>)
            }
            other => {
                bail!("EMAIL_PROVIDER={other} is not recognised (expected `resend`|`smtp`|`noop`)");
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
    // provider wrappers. FDN-09: the concrete provider (Resend / Lettre /
    // Noop) is resolved above; only its trait object is threaded here.
    let default_from = std::env::var("RESEND_FROM")
        .ok()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| config.smtp_from.clone());
    let notifications_service = Arc::new(notifications::Service::new(
        email_provider.clone(),
        default_from,
    ));

    // ADM-08: warm the typed-settings cache before the first request so
    // the maintenance middleware never serves traffic from an empty
    // snapshot (which would default to "open" and miss a maintenance
    // flip set just before the deploy).
    let settings_cache = swings_api::settings::Cache::new();
    settings_cache
        .reload(&pool)
        .await
        .context("failed to warm settings cache")?;

    let state = AppState {
        db: pool,
        config: Arc::new(config),
        email_service,
        media_backend,
        policy: Arc::new(authz::PolicyHandle::new(policy)),
        outbox_shutdown: outbox_shutdown.clone(),
        rate_limit: rate_limit_backend,
        notifications: notifications_service.clone(),
        settings: settings_cache,
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
            // POP-06: revenue attribution subscriber. Narrowly scoped to
            // the two events that carry a session_id — a broader glob
            // would fire the no-op branch on every event in the system.
            .register(
                "order.completed",
                Arc::new(events::handlers::PopupAttributionHandler::new(
                    state.db.clone(),
                )),
            )
            .register(
                "subscription.started",
                Arc::new(events::handlers::PopupAttributionHandler::new(
                    state.db.clone(),
                )),
            )
            // EC-07: digital-delivery subscriber. Mints `user_downloads`
            // rows for downloadable products in the completed order and
            // publishes `user.download.granted` for the mailer. Without
            // this registration the grants never fire.
            .register(
                "order.completed",
                Arc::new(events::handlers::DigitalDeliveryHandler::new(
                    state.db.clone(),
                )),
            )
            // FORM-07: outbound webhook stub. Scoped to `form.*` so it
            // never swallows real work that belongs to another handler
            // (previously `"*"`, which hid missing subscribers by
            // masquerading as a successful no-op).
            .register(
                "form.*",
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

    // ADM-16: audit-log retention sweeper. Reads the sweep cadence from
    // `AUDIT_RETENTION_INTERVAL_SECS` (default 1h) and the row-level
    // retention from `app_settings` so it can be tuned without redeploy.
    // Subscribes to the same shutdown broadcast as the outbox so a
    // single shutdown signal drains every background task.
    let audit_retention_interval_secs = std::env::var("AUDIT_RETENTION_INTERVAL_SECS")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .filter(|n| *n > 0)
        .unwrap_or(3_600);
    let audit_retention_handle = tokio::spawn(swings_api::services::audit_retention::run_loop(
        state.db.clone(),
        state.settings.clone(),
        outbox_shutdown.subscribe(),
        std::time::Duration::from_secs(audit_retention_interval_secs),
    ));
    tracing::info!(
        interval_secs = audit_retention_interval_secs,
        "audit-retention worker spawned"
    );

    // ADM-17: async DSAR export worker. Polls `dsar_jobs` for pending
    // exports, composes the JSON envelope, uploads to the configured
    // media backend (R2 or local upload dir), and stamps the row with
    // the storage key + TTL. Cadence is short (default 30s) because
    // export latency materially affects operator experience; tunable
    // via `DSAR_WORKER_INTERVAL_SECS`.
    let dsar_worker_interval_secs = std::env::var("DSAR_WORKER_INTERVAL_SECS")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .filter(|n| *n > 0)
        .unwrap_or(30);
    let dsar_worker_handle = tokio::spawn(swings_api::services::dsar_worker::run_loop(
        state.db.clone(),
        state.media_backend.clone(),
        outbox_shutdown.subscribe(),
        std::time::Duration::from_secs(dsar_worker_interval_secs),
    ));
    tracing::info!(
        interval_secs = dsar_worker_interval_secs,
        "dsar-export worker spawned"
    );

    // ADM-19: TTL sweep for completed DSAR artefacts. Deletes the
    // underlying R2 object / local file once `artifact_expires_at`
    // has passed, then NULLs the artefact pointer columns. Runs less
    // frequently than the compose worker because cleanup tolerates
    // hour-scale lag; tunable via `DSAR_SWEEP_INTERVAL_SECS`.
    let dsar_sweep_interval_secs = std::env::var("DSAR_SWEEP_INTERVAL_SECS")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .filter(|n| *n > 0)
        .unwrap_or(3600);
    let dsar_sweep_handle = tokio::spawn(swings_api::services::dsar_artifact_sweep::run_loop(
        state.db.clone(),
        state.media_backend.clone(),
        outbox_shutdown.subscribe(),
        std::time::Duration::from_secs(dsar_sweep_interval_secs),
    ));
    tracing::info!(
        interval_secs = dsar_sweep_interval_secs,
        "dsar-artifact-sweep worker spawned"
    );

    // ADM-20: garbage collector for the `idempotency_keys` cache.
    // The middleware writes one row per admin POST with a 24h TTL;
    // without this loop the table grows unbounded. Tunable via
    // `IDEMPOTENCY_GC_INTERVAL_SECS` (default 5 min) plus the
    // `idempotency.gc_batch_size` setting for batch sizing.
    let idempotency_gc_interval_secs = std::env::var("IDEMPOTENCY_GC_INTERVAL_SECS")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .filter(|n| *n > 0)
        .unwrap_or(300);
    let idempotency_gc_handle = tokio::spawn(swings_api::services::idempotency_gc::run_loop(
        state.db.clone(),
        state.settings.clone(),
        outbox_shutdown.subscribe(),
        std::time::Duration::from_secs(idempotency_gc_interval_secs),
    ));
    tracing::info!(
        interval_secs = idempotency_gc_interval_secs,
        "idempotency-gc worker spawned"
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

    // ADM-06: gather every `/api/admin/*` nest into one Router so the IP
    // allowlist middleware can be applied a single time. The middleware is a
    // no-op when the allowlist table is empty (open-mode default), so this is
    // always safe to wire up.
    let admin_routes = Router::<AppState>::new()
        // ADM-05: the admin security console is merged INTO the same
        // `/api/admin` nest as the legacy member/dashboard routes so the two
        // routers share one path prefix (Axum panics on duplicate `.nest`
        // calls with the same prefix).
        .nest(
            "/api/admin",
            handlers::admin::router()
                .merge(handlers::admin_security::router())
                // ADM-10: indexed members search + manual create.
                // Merged (not nested) so it shares the
                // `/api/admin/members` prefix with the legacy
                // member routes in `admin::router()` without an
                // Axum prefix collision.
                .merge(Router::new().nest("/members", handlers::admin_members::router()))
                // ADM-06: IP-allowlist CRUD lives under /api/admin/security/ip-allowlist.
                .nest(
                    "/security/ip-allowlist",
                    handlers::admin_ip_allowlist::router(),
                )
                // ADM-07: impersonation CRUD lives under
                // /api/admin/security/impersonation.
                .nest(
                    "/security/impersonation",
                    handlers::admin_impersonation::router(),
                )
                // ADM-08: typed settings catalogue (incl. maintenance
                // mode kill-switch). The maintenance middleware uses
                // `state.settings` directly, so this nest only owns
                // the CRUD surface.
                .nest("/settings", handlers::admin_settings::router())
                // ADM-09: role / permission matrix. Mutations
                // hot-reload `state.policy`.
                .nest("/security/roles", handlers::admin_roles::router())
                // ADM-11: manual subscription operations
                // (comp / extend / billing-cycle override). Wrapped in
                // the ADM-15 Idempotency-Key middleware so retried
                // POSTs do not double-grant comps or duplicate
                // billing-cycle overrides.
                .nest(
                    "/subscriptions",
                    handlers::admin_subscriptions::router().layer(
                        axum::middleware::from_fn_with_state(
                            state.clone(),
                            swings_api::middleware::idempotency::enforce,
                        ),
                    ),
                )
                // ADM-12: orders admin (list / read / manual create
                // / void / partial refund / CSV export). Manual
                // create + refund are the high-impact mutations the
                // idempotency layer protects.
                .nest(
                    "/orders",
                    handlers::admin_orders::router().layer(axum::middleware::from_fn_with_state(
                        state.clone(),
                        swings_api::middleware::idempotency::enforce,
                    )),
                )
                // ADM-13: admin-initiated DSAR jobs
                // (export + dual-control right-to-erasure tombstone).
                // Tombstone approval is irreversible — idempotency is
                // a hard requirement, not a nicety.
                .nest(
                    "/dsar",
                    handlers::admin_dsar::router().layer(axum::middleware::from_fn_with_state(
                        state.clone(),
                        swings_api::middleware::idempotency::enforce,
                    )),
                )
                // ADM-14: audit log viewer (FTS over admin_actions).
                .nest("/audit", handlers::admin_audit::router()),
        )
        .nest("/api/admin/blog", handlers::blog::admin_router())
        .nest("/api/admin/courses", handlers::courses::admin_router())
        .nest("/api/admin/pricing", handlers::pricing::admin_router())
        .nest("/api/admin/coupons", handlers::coupons::admin_router())
        .nest("/api/admin/popups", handlers::popups::admin_router())
        .nest("/api/admin/products", handlers::products::admin_router())
        .nest("/api/admin/outbox", handlers::outbox::router())
        .nest(
            "/api/admin/notifications",
            handlers::notifications::admin_router(),
        )
        // CONSENT-07 admin CRUD (banners / categories / services / policies
        // + log view + integrity anchor list).
        .nest("/api/admin/consent", handlers::admin_consent::router())
        // CONSENT-03: admin DSAR fulfilment (separate sub-router from
        // CONSENT-07 so both can mount under /api/admin/consent).
        .nest("/api/admin/consent", handlers::consent::admin_router())
        // ADM-18: per-actor token-bucket rate-limit on every admin
        // mutation (`POST` / `PUT` / `PATCH` / `DELETE`). GETs pass
        // through. Sits above the IP allowlist so a credential whose
        // IP is allowlisted still cannot burst-write past the quota,
        // and below the policy/auth checks so an unauthenticated
        // probe doesn't pollute the bucket map.
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            swings_api::middleware::rate_limit::admin_mutation_rate_limit,
        ))
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            swings_api::middleware::admin_ip_allowlist::enforce,
        ));

    let mut app = Router::new()
        // Liveness + readiness probes. Unauthenticated by design so
        // orchestrators (Railway / Render / K8s / ECS) can poll without
        // secrets; both endpoints return no PII and no configuration.
        .merge(handlers::health::router())
        // Auth & analytics
        .nest("/api/auth", handlers::auth::router())
        // ADM-07: self-exit endpoint for the impersonated session. Lives
        // under /api/auth/* so the impersonated user can call it without
        // touching the IP-allowlist-gated /api/admin/* tree.
        .nest(
            "/api/auth/impersonation",
            handlers::admin_impersonation::auth_router(),
        )
        .nest("/api/analytics", handlers::analytics::router())
        .merge(admin_routes)
        // Public routes
        .nest("/api/blog", handlers::blog::public_router())
        .nest("/api/courses", handlers::courses::public_router())
        .nest("/api/pricing", handlers::pricing::public_router())
        .nest("/api/coupons", handlers::coupons::public_router())
        .nest("/api/popups", handlers::popups::public_router())
        .nest("/api/products", handlers::products::public_router())
        // EC-02: public catalog search + facets + nested categories.
        .nest("/api/catalog", handlers::catalog::public_router())
        // EC-03: persistent cart — guest + authed; OptionalAuthUser extractor.
        .nest("/api/cart", handlers::cart::router())
        // Consent (CONSENT-01: public banner + category lookup; admin lives under /api/admin/consent in CONSENT-07)
        .nest("/api/consent", handlers::consent::public_router())
        .nest("/api/dsar", handlers::consent::public_dsar_router())
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
        // ADM-07: stamp X-Impersonation-* response headers when the
        // request was made under an impersonation token. The middleware
        // is a no-op for unauthenticated and ordinary access tokens.
        .layer(axum::middleware::from_fn(
            swings_api::middleware::impersonation_banner::stamp,
        ))
        // ADM-08: maintenance-mode kill-switch. Reads three keys from
        // the in-memory settings cache; defaults to a no-op when the
        // cache is empty. Must run AFTER `with_state` is unreachable —
        // we apply it before so the State extractor still resolves.
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            swings_api::middleware::maintenance_mode::enforce,
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

    // ADM-16: audit-retention sweeper share the outbox shutdown broadcast
    // (single fan-out signal). Awaiting it here keeps the runtime alive
    // until the worker observes the broadcast and exits its select loop.
    if tokio::time::timeout(Duration::from_secs(5), audit_retention_handle)
        .await
        .is_err()
    {
        tracing::warn!("audit-retention worker did not stop within grace window");
    } else {
        tracing::info!("audit-retention worker stopped");
    }

    if tokio::time::timeout(Duration::from_secs(5), dsar_worker_handle)
        .await
        .is_err()
    {
        tracing::warn!("dsar-export worker did not stop within grace window");
    } else {
        tracing::info!("dsar-export worker stopped");
    }

    if tokio::time::timeout(Duration::from_secs(5), dsar_sweep_handle)
        .await
        .is_err()
    {
        tracing::warn!("dsar-artifact-sweep worker did not stop within grace window");
    } else {
        tracing::info!("dsar-artifact-sweep worker stopped");
    }

    if tokio::time::timeout(Duration::from_secs(5), idempotency_gc_handle)
        .await
        .is_err()
    {
        tracing::warn!("idempotency-gc worker did not stop within grace window");
    } else {
        tracing::info!("idempotency-gc worker stopped");
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
