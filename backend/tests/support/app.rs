//! In-process test harness for the `swings-api` Axum application.
//!
//! [`TestApp`] assembles a full `Router<()>` with production state plumbing
//! but an ephemeral database schema and a non-functional email service so
//! tests run with zero external dependencies beyond a Postgres instance the
//! harness owner specified via `DATABASE_URL_TEST`.
//!
//! ## Relationship to `main.rs`
//!
//! The router hierarchy mirrors what `src/main.rs` composes at startup. Any
//! time a new `.nest(…)` / `.merge(…)` is added there, it MUST be added here
//! too. See `docs/wiring/FDN-TESTHARNESS-WIRING.md` for the recommended long-term
//! refactor (extracting a `build_router(state) -> Router`) that would let
//! the harness reuse the canonical wiring instead of mirroring it.
//!
//! ## Rate-limit shim
//!
//! The production routes layer `tower_governor`'s IP-keyed rate limiter. In
//! tests we cannot avoid the layer because we refuse to patch `handlers::*`,
//! so each [`TestApp`] instead stamps a unique `X-Forwarded-For` on every
//! request. `SmartIpKeyExtractor` accepts that header first, which keeps the
//! governor buckets isolated per test and avoids cross-test bleeding.

use std::sync::Arc;

use axum::{
    body::Body,
    http::{header, HeaderValue, Method, Request, StatusCode},
    response::Response,
    Router,
};
use serde::Serialize;
use tempfile::TempDir;
use tower::ServiceExt;
use uuid::Uuid;

use swings_api::{
    authz::{Policy, PolicyHandle},
    config::Config,
    events::WorkerShutdown,
    handlers::{
        admin, admin_audit, admin_consent, admin_dsar, admin_impersonation, admin_ip_allowlist,
        admin_members, admin_orders, admin_roles, admin_security, admin_settings,
        admin_subscriptions, analytics, auth, blog, coupons, courses, csp_report, forms, member,
        notifications, outbox, popups, pricing, products, webhooks,
    },
    middleware::{
        admin_ip_allowlist as admin_ip_allowlist_mw,
        impersonation_banner as impersonation_banner_mw, maintenance_mode as maintenance_mode_mw,
        rate_limit::Backend as RateLimitBackend,
    },
    notifications::Service as NotificationsService,
    services::MediaBackend,
    AppState,
};

use super::db::TestDb;
use super::error::{TestAppError, TestResult};
use super::response::TestResponse;
use super::user::{self, TestRole, TestUser};

/// In-process test application.
///
/// Construct with [`TestApp::new`] (fails on a missing test database) or
/// [`TestApp::try_new`] (returns `None` so tests can skip cleanly).
///
/// Cloning is NOT `impl Clone` — each test owns its own `TestApp`, which in
/// turn owns its schema + `TempDir`. Drop order tears everything down.
pub struct TestApp {
    /// Router-with-state ready to receive `tower::Service::oneshot` calls.
    router: Router<()>,
    /// Uploads directory scoped to this app; dropped with the struct.
    _upload_dir: TempDir,
    /// Schema-scoped DB handle. Dropped last so the `Router` can still
    /// resolve queries in flight.
    db: TestDb,
    /// Stable per-instance IP (via `X-Forwarded-For`) to isolate the
    /// governor rate-limit bucket between tests.
    client_ip: String,
    /// Live handle on the per-test `AppState.settings` cache. Tests
    /// that exercise settings-driven workers (idempotency-gc,
    /// audit-retention, …) flip values here via
    /// `Cache::insert_for_tests` to hit specific code paths
    /// deterministically.
    settings: swings_api::settings::Cache,
    /// Live handle on the per-test `AppState.policy` cache. Tests that
    /// want to verify per-action `policy.require(...)` gates fire
    /// independently of the role string can rebuild the snapshot here
    /// to revoke specific perms from the `admin` role.
    policy: Arc<PolicyHandle>,
    /// Bearer token default; overridden per request via the `auth` arg.
    _marker: (),
}

impl TestApp {
    /// Returns a ready-to-use [`TestApp`], or `None` when no test database
    /// URL is configured.
    ///
    /// Integration tests should use this via `let Some(app) = TestApp::try_new().await else { return; }`
    /// so a missing `DATABASE_URL_TEST` skips rather than aborts the suite.
    pub async fn try_new() -> Option<Self> {
        if !super::has_test_database() {
            eprintln!(
                "[TestApp] skipping: neither DATABASE_URL_TEST nor DATABASE_URL is set. \
                 Start a Postgres and set one to enable integration tests."
            );
            return None;
        }
        match Self::new().await {
            Ok(app) => Some(app),
            Err(e) => {
                eprintln!("[TestApp] skipping: harness init failed: {e}");
                None
            }
        }
    }

    /// Build a fresh [`TestApp`] against the configured test database.
    ///
    /// Errors propagate via [`TestAppError`] so the caller can decide
    /// whether to skip (via [`TestApp::try_new`]) or fail outright.
    pub async fn new() -> TestResult<Self> {
        let db = TestDb::new().await?;
        let upload_dir = TempDir::new().map_err(TestAppError::from)?;
        let upload_path = upload_dir
            .path()
            .to_str()
            .ok_or_else(|| {
                TestAppError::Io(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    "uploads tempdir path is not valid UTF-8",
                ))
            })?
            .to_string();

        let config = test_config(upload_path.clone())?;
        // Hydrate the authz policy from the freshly migrated schema. The
        // migration `021_rbac.sql` seeds the role_permissions catalogue, so
        // the snapshot here matches what `main.rs` would load in production.
        let policy = Policy::load(db.pool())
            .await
            .map_err(|e| TestAppError::Config(format!("authz policy load: {e}")))?;
        let state = AppState {
            db: db.pool().clone(),
            config: Arc::new(config),
            email_service: None,
            media_backend: MediaBackend::Local {
                upload_dir: upload_path,
            },
            policy: Arc::new(PolicyHandle::new(policy)),
            // Tests never start outbox workers, so the broadcast handle is an
            // inert stand-in. Handlers that subscribe (none as of FDN-04) see
            // a sender that never signals — which is the right behavior in a
            // short-lived test process.
            outbox_shutdown: WorkerShutdown::default(),
            // Pin the in-process (governor) backend regardless of the
            // ambient `RATE_LIMIT_BACKEND` — we do not want tests
            // accidentally hitting the Postgres bucket table from whatever
            // env vars the developer happens to have set.
            rate_limit: RateLimitBackend::InProcess(Arc::new(Default::default())),
            // FDN-05: notifications service wired with a Noop email provider
            // so admin preview / test-send routes remain reachable without
            // hitting the network. Assertion-only tests inspect the
            // synthesised `"noop-{uuid}"` provider id.
            notifications: Arc::new(NotificationsService::new(
                Some(Arc::new(
                    swings_api::notifications::channels::email::NoopProvider::new(),
                )),
                "Swings <noreply@example.test>".into(),
            )),
            // ADM-08: settings cache is built per-test and warmed from
            // the freshly-migrated schema (which seeds the three
            // `system.maintenance_*` keys via 062_app_settings.sql).
            // Tests that flip a setting must reload via
            // `state.settings.reload(...)` themselves; the harness does
            // not expose a re-warm helper because the production
            // upsert handler always reloads.
            settings: {
                let cache = swings_api::settings::Cache::new();
                cache
                    .reload(db.pool())
                    .await
                    .map_err(|e| TestAppError::Config(format!("settings cache warm: {e}")))?;
                cache
            },
        };

        let router = build_router(&state);
        let client_ip = allocate_client_ip();
        let settings = state.settings.clone();
        let policy = state.policy.clone();

        Ok(Self {
            router,
            _upload_dir: upload_dir,
            db,
            client_ip,
            settings,
            policy,
            _marker: (),
        })
    }

    /// Atomically swap the policy snapshot the in-process handlers see.
    /// Used by RBAC tests to verify that per-action `policy.require(...)`
    /// gates fire independently of the extractor's role-string check —
    /// e.g. mint an "admin"-role JWT, revoke `blog.post.create` from the
    /// admin role here, hit the handler, assert 403.
    pub fn replace_policy_for_tests(&self, policy: swings_api::authz::Policy) {
        self.policy.replace(policy);
    }

    /// Borrow the live settings cache attached to this app's
    /// `AppState`. Mutating it via `insert_for_tests(...)` takes
    /// effect immediately for any handler / worker that reads the
    /// same handle.
    #[must_use]
    pub fn settings(&self) -> &swings_api::settings::Cache {
        &self.settings
    }

    /// Borrow the schema-scoped pool for tests that need to insert fixtures
    /// directly.
    #[must_use]
    pub fn db(&self) -> &sqlx::PgPool {
        self.db.pool()
    }

    /// Path to the per-test upload directory. Tests that exercise the
    /// local-storage path of an artefact persistence flow (DSAR async
    /// worker, etc.) can read/write here directly.
    #[must_use]
    pub fn upload_dir(&self) -> &std::path::Path {
        self._upload_dir.path()
    }

    /// Build a `MediaBackend::Local` pointing at this app's upload
    /// directory. Mirrors what `main.rs` would resolve in dev mode and
    /// is intended for tests that drive worker fixtures end-to-end.
    #[must_use]
    pub fn media_backend(&self) -> MediaBackend {
        MediaBackend::Local {
            upload_dir: self._upload_dir.path().to_string_lossy().into_owned(),
        }
    }

    /// Build a `MediaBackend::R2` pointing at an externally-managed
    /// S3-compatible emulator (MinIO / LocalStack) when the suite is
    /// run with the `R2_TEST_*` env vars set. Returns `None`
    /// otherwise so the caller can `let Some(backend) = … else { return; };`
    /// and skip cleanly on machines without Docker.
    ///
    /// Required vars (a fresh, unique bucket per test is created):
    ///
    ///   - `R2_TEST_ENDPOINT`     — `http://localhost:9000` (MinIO default)
    ///   - `R2_TEST_ACCESS_KEY`   — `minioadmin` (default)
    ///   - `R2_TEST_SECRET_KEY`   — `minioadmin` (default)
    ///   - `R2_TEST_REGION`       — optional, defaults to `us-east-1`
    ///   - `R2_TEST_PUBLIC_BASE`  — optional, defaults to the endpoint
    ///
    /// The bucket name is generated per call (`swings-test-{uuid8}`)
    /// and ensured to exist via `R2Storage::ensure_bucket()`.
    pub async fn try_media_backend_r2(&self) -> Option<MediaBackend> {
        let endpoint = std::env::var("R2_TEST_ENDPOINT")
            .ok()
            .filter(|s| !s.is_empty())?;
        let access = std::env::var("R2_TEST_ACCESS_KEY").unwrap_or_else(|_| "minioadmin".into());
        let secret = std::env::var("R2_TEST_SECRET_KEY").unwrap_or_else(|_| "minioadmin".into());
        let region = std::env::var("R2_TEST_REGION").unwrap_or_else(|_| "us-east-1".into());
        let public_base = std::env::var("R2_TEST_PUBLIC_BASE").unwrap_or_else(|_| endpoint.clone());

        // Per-test bucket so concurrent tests cannot stomp each
        // other's keys. Lower-cased per S3 bucket-name rules.
        let bucket = format!(
            "swings-test-{}",
            uuid::Uuid::new_v4().to_string().split('-').next().unwrap()
        );
        let r2 = swings_api::services::storage::R2Storage::for_endpoint(
            endpoint,
            region,
            bucket,
            public_base,
            access,
            secret,
        );
        r2.ensure_bucket()
            .await
            .map_err(|e| {
                eprintln!("R2 emulator unreachable, skipping R2 test: {e}");
                e
            })
            .ok()?;
        Some(MediaBackend::R2(r2))
    }

    /// The ephemeral schema's name — handy for `SET LOCAL search_path` in
    /// ad-hoc queries that bypass the pool helper.
    #[must_use]
    pub fn schema_name(&self) -> &str {
        self.db.schema_name()
    }

    /// Seed a member. Shorthand for `seed_user(TestRole::Member)`.
    pub async fn seed_user(&self) -> TestResult<TestUser> {
        self.seed_user_with_role(TestRole::Member).await
    }

    /// Seed an admin. Shorthand for `seed_user(TestRole::Admin)`.
    pub async fn seed_admin(&self) -> TestResult<TestUser> {
        self.seed_user_with_role(TestRole::Admin).await
    }

    /// Seed a support agent. Shorthand for `seed_user(TestRole::Support)`.
    pub async fn seed_support(&self) -> TestResult<TestUser> {
        self.seed_user_with_role(TestRole::Support).await
    }

    /// Seed a user with the specified role.
    pub async fn seed_user_with_role(&self, role: TestRole) -> TestResult<TestUser> {
        // The `Config` is `Arc<Config>`; reach through the pool to borrow the
        // fields we need. The harness uses `hours=24`, `days=30` — same as
        // the default env values so seeded JWTs match production semantics.
        user::seed(
            self.db.pool(),
            test_jwt_secret_current().as_str(),
            24,
            30,
            role,
        )
        .await
    }

    /// Dispatch a `GET` request.
    pub async fn get(&self, path: &str, auth: Option<&str>) -> TestResponse {
        self.request(Method::GET, path, None::<&()>, auth).await
    }

    /// Dispatch a `POST` with a JSON body.
    pub async fn post_json<B: Serialize + ?Sized>(
        &self,
        path: &str,
        body: &B,
        auth: Option<&str>,
    ) -> TestResponse {
        self.request(Method::POST, path, Some(body), auth).await
    }

    /// Dispatch a `POST` with a JSON body and an extra `Idempotency-Key`
    /// header. Used by the ADM-15 idempotency integration tests; kept on
    /// the harness so callers don't have to drop down to raw `Request`
    /// builders.
    pub async fn post_json_with_idempotency_key<B: Serialize + ?Sized>(
        &self,
        path: &str,
        body: &B,
        auth: Option<&str>,
        key: &str,
    ) -> TestResponse {
        let result = self
            .request_inner_with_extra_header(
                Method::POST,
                path,
                Some(body),
                auth,
                Some(("idempotency-key", key)),
            )
            .await;
        match result {
            Ok(resp) => resp,
            Err(e) => panic!("TestApp dispatch failed: {e}"),
        }
    }

    /// Dispatch a `PUT` with a JSON body.
    pub async fn put_json<B: Serialize + ?Sized>(
        &self,
        path: &str,
        body: &B,
        auth: Option<&str>,
    ) -> TestResponse {
        self.request(Method::PUT, path, Some(body), auth).await
    }

    /// Dispatch a `PATCH` with a JSON body.
    pub async fn patch_json<B: Serialize + ?Sized>(
        &self,
        path: &str,
        body: &B,
        auth: Option<&str>,
    ) -> TestResponse {
        self.request(Method::PATCH, path, Some(body), auth).await
    }

    /// Dispatch a `DELETE` request without a body.
    pub async fn delete(&self, path: &str, auth: Option<&str>) -> TestResponse {
        self.request(Method::DELETE, path, None::<&()>, auth).await
    }

    /// BFF (Phase 1.3): drive a request whose ONLY auth carrier is a
    /// `Cookie: <name>=<value>` header. No `Authorization: Bearer …`.
    ///
    /// Used by the auth-cookie integration tests to prove the cookie path
    /// resolves end-to-end without falling back to the bearer header.
    pub async fn request_with_cookie(
        &self,
        method: &str,
        path: &str,
        cookie_name: &str,
        cookie_value: &str,
    ) -> TestResponse {
        let m = parse_method(method);
        let cookie_header = format!("{cookie_name}={cookie_value}");
        let result = self
            .request_inner_with_cookie(m, path, None::<&()>, None, Some(&cookie_header))
            .await;
        match result {
            Ok(resp) => resp,
            Err(e) => panic!("TestApp dispatch failed: {e}"),
        }
    }

    /// BFF (Phase 1.3): drive a request that carries BOTH a `Cookie:` header
    /// and an `Authorization: Bearer …` header. Asserts the
    /// cookie-takes-precedence behaviour during the rollout window when
    /// stale clients might present both carriers.
    pub async fn request_with_cookie_and_bearer(
        &self,
        method: &str,
        path: &str,
        cookie_name: &str,
        cookie_value: &str,
        bearer: &str,
    ) -> TestResponse {
        let m = parse_method(method);
        let cookie_header = format!("{cookie_name}={cookie_value}");
        let result = self
            .request_inner_with_cookie(m, path, None::<&()>, Some(bearer), Some(&cookie_header))
            .await;
        match result {
            Ok(resp) => resp,
            Err(e) => panic!("TestApp dispatch failed: {e}"),
        }
    }

    async fn request_inner_with_cookie<B: Serialize + ?Sized>(
        &self,
        method: Method,
        path: &str,
        body: Option<&B>,
        auth: Option<&str>,
        cookie: Option<&str>,
    ) -> TestResult<TestResponse> {
        let mut builder = Request::builder()
            .method(method)
            .uri(path)
            .header("X-Forwarded-For", self.client_ip.as_str());

        if let Some(token) = auth {
            builder = builder.header(
                header::AUTHORIZATION,
                HeaderValue::from_str(&format!("Bearer {token}"))
                    .map_err(|e| TestAppError::Http(format!("build auth header: {e}")))?,
            );
        }

        if let Some(cookie_value) = cookie {
            builder = builder.header(
                header::COOKIE,
                HeaderValue::from_str(cookie_value)
                    .map_err(|e| TestAppError::Http(format!("build cookie header: {e}")))?,
            );
        }

        let req = if let Some(body) = body {
            let bytes = serde_json::to_vec(body)
                .map_err(|e| TestAppError::Http(format!("serialize body: {e}")))?;
            builder
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(bytes))
                .map_err(|e| TestAppError::Http(format!("build request: {e}")))?
        } else {
            builder
                .body(Body::empty())
                .map_err(|e| TestAppError::Http(format!("build request: {e}")))?
        };

        let resp: Response = self
            .router
            .clone()
            .oneshot(req)
            .await
            .map_err(|e| TestAppError::Http(format!("dispatch: {e}")))?;

        TestResponse::from_response(resp).await
    }

    /// Core request dispatch. Serializes `body` as JSON when present and
    /// always injects the per-`TestApp` rate-limit IP.
    ///
    /// Panics are funneled through [`TestResponse`] so the caller sees the
    /// actual server output on failure — the only way this returns
    /// unsuccessfully is if the harness itself is misconfigured.
    async fn request<B: Serialize + ?Sized>(
        &self,
        method: Method,
        path: &str,
        body: Option<&B>,
        auth: Option<&str>,
    ) -> TestResponse {
        let result = self.request_inner(method, path, body, auth).await;
        match result {
            Ok(resp) => resp,
            Err(e) => panic!("TestApp dispatch failed: {e}"),
        }
    }

    async fn request_inner<B: Serialize + ?Sized>(
        &self,
        method: Method,
        path: &str,
        body: Option<&B>,
        auth: Option<&str>,
    ) -> TestResult<TestResponse> {
        self.request_inner_with_extra_header(method, path, body, auth, None)
            .await
    }

    async fn request_inner_with_extra_header<B: Serialize + ?Sized>(
        &self,
        method: Method,
        path: &str,
        body: Option<&B>,
        auth: Option<&str>,
        extra: Option<(&str, &str)>,
    ) -> TestResult<TestResponse> {
        let mut builder = Request::builder()
            .method(method)
            .uri(path)
            .header("X-Forwarded-For", self.client_ip.as_str());

        if let Some(token) = auth {
            builder = builder.header(
                header::AUTHORIZATION,
                HeaderValue::from_str(&format!("Bearer {token}"))
                    .map_err(|e| TestAppError::Http(format!("build auth header: {e}")))?,
            );
        }

        if let Some((name, value)) = extra {
            builder = builder.header(
                name,
                HeaderValue::from_str(value)
                    .map_err(|e| TestAppError::Http(format!("build extra header: {e}")))?,
            );
        }

        let req = if let Some(body) = body {
            let bytes = serde_json::to_vec(body)
                .map_err(|e| TestAppError::Http(format!("serialize body: {e}")))?;
            builder
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(bytes))
                .map_err(|e| TestAppError::Http(format!("build request: {e}")))?
        } else {
            builder
                .body(Body::empty())
                .map_err(|e| TestAppError::Http(format!("build request: {e}")))?
        };

        let resp: Response = self
            .router
            .clone()
            .oneshot(req)
            .await
            .map_err(|e| TestAppError::Http(format!("dispatch: {e}")))?;

        TestResponse::from_response(resp).await
    }
}

/// Build the same router tree `main.rs` wires up at startup.
///
/// Kept in lockstep with `backend/src/main.rs`. When a new `.nest(…)` is
/// introduced there, mirror it here or integration tests will 404 the new
/// endpoints.
fn build_router(state: &AppState) -> Router<()> {
    // ADM-06: same wrapping pattern used in `main.rs` so the IP allowlist
    // middleware is exercised end-to-end by integration tests.
    let admin_routes: Router<AppState> = Router::new()
        .nest(
            "/api/admin",
            admin::router()
                .merge(admin_security::router())
                .nest("/security/ip-allowlist", admin_ip_allowlist::router())
                .nest("/security/impersonation", admin_impersonation::router())
                .nest("/settings", admin_settings::router())
                .nest("/security/roles", admin_roles::router())
                .nest(
                    "/subscriptions",
                    admin_subscriptions::router().layer(axum::middleware::from_fn_with_state(
                        state.clone(),
                        swings_api::middleware::idempotency::enforce,
                    )),
                )
                .nest(
                    "/orders",
                    admin_orders::router().layer(axum::middleware::from_fn_with_state(
                        state.clone(),
                        swings_api::middleware::idempotency::enforce,
                    )),
                )
                .nest(
                    "/dsar",
                    admin_dsar::router().layer(axum::middleware::from_fn_with_state(
                        state.clone(),
                        swings_api::middleware::idempotency::enforce,
                    )),
                )
                .nest("/audit", admin_audit::router())
                .merge(axum::Router::new().nest("/members", admin_members::router())),
        )
        // Phase 5.3: mirror the production wiring in `main.rs` —
        // every legacy admin tree below gets the ADM-15 idempotency
        // middleware so integration tests exercise the same retry
        // semantics as production. The middleware is a no-op for
        // non-POST verbs and for requests without `Idempotency-Key`.
        .nest(
            "/api/admin/blog",
            blog::admin_router().layer(axum::middleware::from_fn_with_state(
                state.clone(),
                swings_api::middleware::idempotency::enforce,
            )),
        )
        .nest(
            "/api/admin/courses",
            courses::admin_router().layer(axum::middleware::from_fn_with_state(
                state.clone(),
                swings_api::middleware::idempotency::enforce,
            )),
        )
        .nest(
            "/api/admin/pricing",
            pricing::admin_router().layer(axum::middleware::from_fn_with_state(
                state.clone(),
                swings_api::middleware::idempotency::enforce,
            )),
        )
        .nest(
            "/api/admin/coupons",
            coupons::admin_router().layer(axum::middleware::from_fn_with_state(
                state.clone(),
                swings_api::middleware::idempotency::enforce,
            )),
        )
        .nest(
            "/api/admin/popups",
            popups::admin_router().layer(axum::middleware::from_fn_with_state(
                state.clone(),
                swings_api::middleware::idempotency::enforce,
            )),
        )
        .nest(
            "/api/admin/products",
            products::admin_router().layer(axum::middleware::from_fn_with_state(
                state.clone(),
                swings_api::middleware::idempotency::enforce,
            )),
        )
        .nest(
            "/api/admin/outbox",
            outbox::router().layer(axum::middleware::from_fn_with_state(
                state.clone(),
                swings_api::middleware::idempotency::enforce,
            )),
        )
        .nest(
            "/api/admin/forms",
            forms::admin_router().layer(axum::middleware::from_fn_with_state(
                state.clone(),
                swings_api::middleware::idempotency::enforce,
            )),
        )
        .nest(
            "/api/admin/notifications",
            notifications::admin_router().layer(axum::middleware::from_fn_with_state(
                state.clone(),
                swings_api::middleware::idempotency::enforce,
            )),
        )
        .nest(
            "/api/admin/consent",
            admin_consent::router().layer(axum::middleware::from_fn_with_state(
                state.clone(),
                swings_api::middleware::idempotency::enforce,
            )),
        )
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            swings_api::middleware::rate_limit::admin_mutation_rate_limit,
        ))
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            admin_ip_allowlist_mw::enforce,
        ));

    let router: Router<AppState> = Router::new()
        // Auth & analytics
        .nest("/api/auth", auth::router())
        .nest(
            "/api/auth/impersonation",
            admin_impersonation::auth_router(),
        )
        .nest("/api/analytics", analytics::router())
        .merge(admin_routes)
        // Public routes
        .nest("/api/blog", blog::public_router())
        .nest("/api/courses", courses::public_router())
        .nest("/api/pricing", pricing::public_router())
        .nest("/api/coupons", coupons::public_router())
        .nest("/api/popups", popups::public_router())
        // Member routes
        .nest("/api/member", member::router())
        .nest("/api/member", courses::member_router())
        .nest("/api/member", notifications::member_router())
        // Webhooks
        .nest("/api/webhooks", webhooks::router())
        // Security reports (FDN-08)
        .nest("/api", csp_report::router())
        .nest("/u", notifications::public_router())
        // ADM-07: stamp X-Impersonation-* response headers — applied
        // here (after all routes are mounted) so every test request
        // also exercises the production banner contract.
        .layer(axum::middleware::from_fn(impersonation_banner_mw::stamp))
        // ADM-08: maintenance-mode kill-switch. The cache is warmed
        // at TestApp startup with the seeded defaults
        // (`maintenance_mode=false`), so this layer is a no-op for
        // every test that does not explicitly flip the flag.
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            maintenance_mode_mw::enforce,
        ));

    router.with_state(state.clone())
}

/// Produce a deterministic-per-process `Config` for tests.
///
/// Secrets / provider keys are stubbed to obviously-bogus values. Tests that
/// need a real Stripe or R2 client must override the relevant env vars
/// before constructing the [`TestApp`].
fn test_config(upload_dir: String) -> TestResult<Config> {
    let database_url = std::env::var("DATABASE_URL_TEST")
        .or_else(|_| std::env::var("DATABASE_URL"))
        .map_err(|_| {
            TestAppError::Config(
                "DATABASE_URL_TEST/DATABASE_URL must be set for Config::test".into(),
            )
        })?;

    let jwt_secret = test_jwt_secret_current();
    let frontend_url =
        std::env::var("FRONTEND_URL").unwrap_or_else(|_| "http://localhost:5173".to_string());

    Ok(Config {
        database_url,
        jwt_secret,
        jwt_expiration_hours: 24,
        refresh_token_expiration_days: 30,
        port: 0,
        frontend_url: frontend_url.clone(),
        stripe_secret_key: String::new(),
        stripe_webhook_secret: String::new(),
        upload_dir,
        api_url: "http://localhost:3001".to_string(),
        smtp_host: "smtp.example.test".to_string(),
        smtp_port: 587,
        smtp_user: String::new(),
        smtp_password: String::new(),
        smtp_from: "noreply@example.test".to_string(),
        app_url: frontend_url.clone(),
        app_env: "test".to_string(),
        cors_allowed_origins: vec![frontend_url],
    })
}

/// Stable JWT secret for this process run.
///
/// Re-used by every [`TestApp`] in the same test binary so a token minted
/// from one `TestApp` instance can be presented to another (rare, but
/// useful for cross-test `AuthUser` sanity checks). The value is
/// deliberately long enough to mimic production secrets.
///
/// Made `pub` so integration tests that mint or inspect JWTs directly
/// (e.g. ADM-07 impersonation tests that decode the server-issued
/// token) can sign / verify with the same key. Each test binary is its
/// own crate, so `pub(crate)` would not be re-exportable from
/// `support`.
pub fn test_jwt_secret_current() -> String {
    std::env::var("JWT_SECRET").unwrap_or_else(|_| {
        // 64 bytes, deterministic per binary so tokens within one `cargo test`
        // run are mutually verifiable.
        "test-harness-jwt-secret-0123456789abcdef-0123456789abcdef-fixed".to_string()
    })
}

/// Each [`TestApp`] gets a unique `X-Forwarded-For` IP (10.x.y.z block) so
/// the IP-keyed `tower_governor` limiters do not share buckets across tests.
///
/// UUIDs give us an effective 2^24 random third octets per run — the octet
/// space is small enough that the birthday-bound kicks in after about 4k
/// tests, which is well above any realistic test-binary footprint.
fn allocate_client_ip() -> String {
    let id = Uuid::new_v4().as_u128();
    let a = ((id >> 16) & 0xFF) as u8;
    let b = ((id >> 8) & 0xFF) as u8;
    let c = (id & 0xFF) as u8;
    format!("10.{a}.{b}.{c}")
}

/// Tiny string → `Method` parser for the cookie-test ergonomics — keeps
/// `request_with_cookie("GET", …)` readable while staying defensive: an
/// unknown verb panics so tests fail loudly rather than silently routing
/// through `GET`.
fn parse_method(s: &str) -> Method {
    match s.to_ascii_uppercase().as_str() {
        "GET" => Method::GET,
        "POST" => Method::POST,
        "PUT" => Method::PUT,
        "PATCH" => Method::PATCH,
        "DELETE" => Method::DELETE,
        other => panic!("unsupported test method: {other}"),
    }
}

/// Convenience constants for asserting expected statuses. Tests are free to
/// import these or use the upstream `StatusCode` constants directly.
pub const STATUS_OK: StatusCode = StatusCode::OK;
pub const STATUS_UNAUTHORIZED: StatusCode = StatusCode::UNAUTHORIZED;
pub const STATUS_FORBIDDEN: StatusCode = StatusCode::FORBIDDEN;
pub const STATUS_NOT_FOUND: StatusCode = StatusCode::NOT_FOUND;
