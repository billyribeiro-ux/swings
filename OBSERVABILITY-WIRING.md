# Observability Scaffolding — Integrator Wiring

This document is the handoff for applying the `backend/src/observability/`
module to the live `swings-api` binary. The module lands in isolation
(new files only, byte-identical `Cargo.toml` / `main.rs` / `lib.rs`); the
three edits below turn it on.

Spec: see `AUDIT_PHASE3_PLAN.md` §11 ("Observability"). Scope:
structured JSON logs, `X-Request-Id` correlation middleware, Prometheus
metrics with bounded cardinality, admin-gated `/metrics` endpoint. No
OpenTelemetry (Phase 5).

---

## 1. `backend/Cargo.toml`

Append the two new `metrics` crates under `[dependencies]` and enable
the `json` feature on the already-present `tracing-subscriber`:

```toml
# Observability (FDN-08-adjacent)
metrics = "0.24"
metrics-exporter-prometheus = "0.17"
```

And replace the existing `tracing-subscriber` line:

```toml
# Before:
# tracing-subscriber = { version = "0.3", features = ["env-filter"] }

# After:
tracing-subscriber = { version = "0.3", features = ["env-filter", "json"] }
```

No other crate additions needed — `uuid` (already present with `v4`
feature) handles correlation-id generation; `axum::extract::MatchedPath`
(no extra feature required) handles cardinality-bounded route labels;
the `Extension` + `State` extractors used by the handler module ship
with axum's default feature set.

`cargo update -p metrics -p metrics-exporter-prometheus` after applying
the edits will resolve the dep graph. Neither crate has known conflicts
with the existing transitive tree (both crates' `crossbeam-*` and
`parking_lot` transitive deps are already in the lockfile via `tokio`
and `moka`).

---

## 2. `backend/src/lib.rs`

Declare the module. Maintain alphabetical order — place between
`middleware` and `openapi`:

```rust
pub mod authz;
pub mod common;
pub mod config;
pub mod db;
pub mod email;
pub mod error;
pub mod events;
pub mod extractors;
pub mod handlers;
pub mod middleware;
pub mod models;
pub mod observability;   // ← add this line
pub mod openapi;
pub mod services;
pub mod stripe_api;
```

No other change in `lib.rs`. `AppState` does not need new fields —
the `PrometheusHandle` is passed as an axum `Extension` (request-level
extension) rather than a state field so the `/metrics` route can opt
in without every handler paying the clone cost.

---

## 3. `backend/src/main.rs`

Three edits to `main()`. Pre-conditions: `use` imports for
`axum::Extension` and `axum::routing::get` (the latter is usually already
present; the former is new).

### 3.a — replace the tracing subscriber init (lines 35–41)

```rust
// Before:
tracing_subscriber::registry()
    .with(
        tracing_subscriber::EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| "swings_api=debug,tower_http=debug".into()),
    )
    .with(tracing_subscriber::fmt::layer())
    .init();

// After:
tracing_subscriber::registry()
    .with(
        tracing_subscriber::EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| "swings_api=debug,tower_http=debug".into()),
    )
    .with(swings_api::observability::tracing_layer())
    .init();
```

### 3.b — install the metrics recorder right after the subscriber is up

```rust
// Insert immediately after the tracing setup and before `load_dotenv()`
// (earliest point where metric writes can fire):
let metrics_handle = swings_api::observability::install_recorder();
```

`install_recorder()` is idempotent — safe to call more than once.

### 3.c — wire the middleware layers + the `/metrics` route

Anywhere after `let mut app = Router::new()…` and before
`let app = app.layer(cors).layer(TraceLayer::new_for_http()).with_state(state);`,
add:

```rust
// Mount `/metrics` using the admin-gated handler in production, the
// public one in dev. Mirror the `openapi::mount` pattern.
let metrics_route = if state.config.is_production() {
    axum::routing::get(swings_api::observability::handler::admin_metrics_handler)
} else {
    axum::routing::get(swings_api::observability::handler::public_metrics_handler)
};
app = app.route("/metrics", metrics_route);
```

Then adjust the `.layer(…)` chain at the bottom of the router
composition to include, *before* `TraceLayer`:

```rust
let app = app
    .layer(axum::Extension(metrics_handle))
    .layer(axum::middleware::from_fn_with_state(
        state.clone(),
        swings_api::observability::metrics::http_middleware,
    ))
    .layer(swings_api::observability::correlation::layer())
    .layer(cors)
    .layer(TraceLayer::new_for_http())
    .with_state(state);
```

**Layer order matters.** Axum applies layers outside-in: the last `.layer(…)`
wraps the outermost. With the composition above:

1. `TraceLayer` is outermost → captures every request.
2. `cors` → CORS preflight still sees a correlation id because
   `correlation::layer` is innermost of the observability stack.
3. `correlation::layer` runs next → stamps `RequestId` into request
   extensions and the span.
4. `http_middleware` runs next → reads `MatchedPath` (populated by the
   router) and records the metric with the correlation-scoped span.
5. `Extension(metrics_handle)` is innermost → the `/metrics` handler
   pulls the handle via `Extension<PrometheusHandle>`.

If the integrator places `correlation::layer` *after* `http_middleware`,
metric observations still work but the log events the HTTP middleware
emits (via `tracing::debug!` or similar) lose their `request_id` field.
Keep the order above.

---

## 4. Env vars

| Var          | Values            | Default                          | Effect                                          |
| ------------ | ----------------- | -------------------------------- | ----------------------------------------------- |
| `LOG_FORMAT` | `json` \| `pretty` | auto (json if APP_ENV=production) | Force log format regardless of APP_ENV.         |
| `APP_ENV`    | `production`, else | `development`                    | Secondary signal; `json` when `=production`.     |
| `RUST_LOG`   | tracing filter    | `swings_api=debug,tower_http=debug` | Filter passed to `EnvFilter::try_from_default_env`. |

No new secrets — correlation ids are generated locally; metrics are
served over the same port as the API behind the admin gate.

---

## 5. Integrator verification

Run from repo root:

```bash
cd backend
cargo fmt --check
cargo clippy -- -D warnings
cargo test
```

Then start the binary and scrape:

```bash
RUST_LOG=info cargo run &
curl -i http://localhost:3001/metrics | head -30        # dev mode, public
curl -i -H 'X-Request-Id: test-id-0123456789abcdef' \
     http://localhost:3001/api/blog                     # see id echoed back
```

Expected:

* `/metrics` returns `200 OK` with `Content-Type: text/plain; version=0.0.4; charset=utf-8`.
* The body starts with `# HELP http_requests_total …` (and the rest of
  the catalogue).
* Subsequent requests increase the `http_requests_total{route="/api/blog",…}` sample.
* Every response carries an `X-Request-Id` header.

After verification, unignore the E2E observability test:

```bash
cargo test --test observability -- --ignored
```

Remove the `#[ignore]` attribute in `backend/tests/observability.rs`
once the wiring lands.

---

## 6. Non-scope (explicitly deferred)

* **OpenTelemetry**: traces + OTLP exporter are Phase 5. The JSON log
  format carries `request_id` so pivoting from logs → traces is possible
  by trace-id propagation as a follow-up.
* **Alerts**: PagerDuty / Grafana wiring lives at the
  infrastructure repo level; metric names here are chosen to match the
  catalogue in `AUDIT_PHASE3_PLAN.md` §11.
* **Per-handler business spans**: every handler already uses
  `#[tracing::instrument(…)]`; correlation-id binding is enough to tie
  logs back to a request. Business-level span conventions
  (`checkout.create_session` et al. from §11) are a FDN-05/EC-05 scope
  item.
