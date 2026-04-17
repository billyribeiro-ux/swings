//! Prometheus metrics scaffolding.
//!
//! Uses the `metrics` crate as the call-site facade and
//! `metrics-exporter-prometheus` as the recorder. The recorder is installed
//! once at startup via [`install_recorder`] and its [`PrometheusHandle`] is
//! shared with the `/metrics` handler as an axum `Extension`.
//!
//! Every metric is pre-registered (with [`describe_counter!`] /
//! [`describe_histogram!`] / [`describe_gauge!`]) so operators who scrape
//! `/metrics` immediately see the full catalogue — even on a freshly
//! started instance before any request fires.
//!
//! # HTTP middleware
//!
//! [`http_middleware`] records two metrics per request:
//!
//! * `http_requests_total{route,method,status}` — counter, incremented once
//!   per response.
//! * `http_request_duration_seconds{route,method}` — histogram, observed
//!   with the wall-clock elapsed time.
//!
//! The `route` label is sourced from [`axum::extract::MatchedPath`] (the
//! router pattern, not the concrete URL). This is load-bearing: routing
//! patterns are a bounded set (one per `.route(…)` call), whereas raw URIs
//! are unbounded (the cardinality would explode with user ids, slugs,
//! pagination, etc.). Requests that do not match a route (404s) fall back
//! to the literal `"unmatched"` label so the bucket size stays finite.
//!
//! # Bucket rationale
//!
//! `http_request_duration_seconds` uses logarithmic buckets from 5 ms to
//! 10 s: `[0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1, 2.5, 5, 10]`.
//!
//! * 5 ms lower bound — anything faster is a cache-hit / trivial handler;
//!   measuring at higher resolution wastes storage on a flat histogram tail.
//! * 10 s upper bound — handlers slower than 10 s have bigger problems than
//!   a metric bucket; they show up as a dedicated `+Inf` overflow.
//! * The `2.5x` step matches the Prometheus `DefBuckets` convention so our
//!   dashboards align with community panels.

use std::sync::OnceLock;

use axum::{
    body::Body,
    extract::{MatchedPath, Request, State},
    http::Response,
    middleware::Next,
};
use metrics::{counter, describe_counter, describe_gauge, describe_histogram, gauge, histogram};
use metrics_exporter_prometheus::{PrometheusBuilder, PrometheusHandle};

use crate::error::AppError;
use crate::AppState;

// ── Metric names (single source of truth — tests assert on these) ────────

pub const HTTP_REQUESTS_TOTAL: &str = "http_requests_total";
pub const HTTP_REQUEST_DURATION_SECONDS: &str = "http_request_duration_seconds";
pub const OUTBOX_PENDING: &str = "outbox_pending";
pub const OUTBOX_ATTEMPTS_TOTAL: &str = "outbox_attempts_total";
pub const OUTBOX_DEAD_LETTER_TOTAL: &str = "outbox_dead_letter_total";
pub const NOTIFICATIONS_SENT_TOTAL: &str = "notifications_sent_total";
pub const STRIPE_WEBHOOK_TOTAL: &str = "stripe_webhook_total";
pub const CONSENT_RECORDS_TOTAL: &str = "consent_records_total";
pub const DSAR_REQUESTS_TOTAL: &str = "dsar_requests_total";
pub const ORDERS_TOTAL: &str = "orders_total";

/// Buckets for `http_request_duration_seconds` in seconds. Logarithmic from
/// 5 ms to 10 s; see module docs for rationale.
pub const HTTP_DURATION_BUCKETS: &[f64] = &[
    0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0,
];

/// Label value used when an incoming request does not match any route
/// (typically a 404). Keeping the label bounded matters more than carrying
/// the raw URI — see module docs.
const UNMATCHED_ROUTE: &str = "unmatched";

/// Hold the installed recorder handle. Installation is idempotent (the
/// second call returns the same handle the first call produced). A
/// `OnceLock` keeps the cell lock-free after first write.
///
/// This is deliberately an `OnceLock<PrometheusHandle>` (not
/// `OnceLock<Result<PrometheusHandle, …>>`) because a failed install is a
/// configuration error at startup, and returning the unwrapped handle
/// lets callers avoid an allocator per request.
static RECORDER: OnceLock<PrometheusHandle> = OnceLock::new();

/// Install the global Prometheus recorder and register the metric
/// catalogue. Idempotent on double-call — the second invocation returns
/// the clone of the handle produced by the first.
///
/// # Errors
///
/// Panics in neither path. When [`PrometheusBuilder::install_recorder`]
/// fails (e.g. because another recorder — `tracing-metrics`,
/// `metrics-util`'s debugging sink — is already installed), the function
/// falls back to a render-only handle attached to the existing recorder,
/// which is still useful for the `/metrics` endpoint but will not receive
/// new writes. This path is vanishingly unusual in production; tests that
/// exercise it should call [`install_recorder`] from the same binary.
///
/// See the `integration_tests_share_recorder` note below: `install_recorder`
/// is safe to call from every test in the same binary.
#[must_use]
pub fn install_recorder() -> PrometheusHandle {
    // Use `get_or_init` so the install + catalogue bootstrap happens
    // exactly once across the process, even under parallel callers
    // (tests, rayon workers, etc.). Subsequent callers observe the
    // already-stored handle without touching the Prometheus builder.
    RECORDER
        .get_or_init(|| {
            // Build the recorder with our histogram buckets pre-configured so
            // the Prometheus exporter reports them even before any
            // observation fires. `set_buckets_for_metric` only errors on
            // an empty bucket slice; we hard-code a non-empty literal so
            // the `unwrap_or_else` branch is unreachable — handled
            // defensively to honour "no .unwrap() in prod".
            let builder = PrometheusBuilder::new()
                .set_buckets_for_metric(
                    metrics_exporter_prometheus::Matcher::Full(
                        HTTP_REQUEST_DURATION_SECONDS.to_string(),
                    ),
                    HTTP_DURATION_BUCKETS,
                )
                .unwrap_or_else(|_err| PrometheusBuilder::new());

            // `install_recorder` returns the render handle on success. On
            // failure (another recorder already installed — e.g. a test
            // that installed `metrics-util::DebuggingRecorder`) it
            // returns `BuildError::FailedToSetGlobalRecorder`; fall back
            // to a standalone handle so the function does not panic.
            // Callers will see an empty exposition format until the
            // upstream recorder issue is resolved.
            let handle = match builder.install_recorder() {
                Ok(handle) => handle,
                Err(err) => {
                    tracing::warn!(
                        error = %err,
                        "metrics recorder install failed; falling back to render-only handle"
                    );
                    PrometheusBuilder::new().build_recorder().handle()
                }
            };

            describe_catalogue();
            register_zeroes();

            handle
        })
        .clone()
}

/// Register the catalogue with human-readable descriptions. `describe_*`
/// macros are idempotent; calling them on every `install_recorder` invocation
/// is cheap and keeps the exposition format self-documenting.
fn describe_catalogue() {
    describe_counter!(
        HTTP_REQUESTS_TOTAL,
        "Total HTTP requests served, labelled by matched route, method, and status."
    );
    describe_histogram!(
        HTTP_REQUEST_DURATION_SECONDS,
        "HTTP request handler latency in seconds, labelled by matched route and method."
    );
    describe_gauge!(
        OUTBOX_PENDING,
        "Current count of outbox rows awaiting dispatch (FDN-04)."
    );
    describe_counter!(
        OUTBOX_ATTEMPTS_TOTAL,
        "Total outbox delivery attempts by subscriber and terminal result."
    );
    describe_counter!(
        OUTBOX_DEAD_LETTER_TOTAL,
        "Total outbox rows transitioned to dead_letter after exhausting retries."
    );
    describe_counter!(
        NOTIFICATIONS_SENT_TOTAL,
        "Total notifications dispatched by channel, provider, and terminal status (FDN-05)."
    );
    describe_counter!(
        STRIPE_WEBHOOK_TOTAL,
        "Total Stripe webhooks processed by event type and terminal result."
    );
    describe_counter!(
        CONSENT_RECORDS_TOTAL,
        "Total consent records created/updated by category and action."
    );
    describe_counter!(
        DSAR_REQUESTS_TOTAL,
        "Total DSAR requests by kind (access/deletion/portability) and current status."
    );
    describe_counter!(
        ORDERS_TOTAL,
        "Total orders transitioned to a terminal status (EC-05 will emit)."
    );
}

/// Emit a zero-valued sample for every catalogue entry so the `/metrics`
/// endpoint shows the full list even before any real event fires.
///
/// The Prometheus exporter only emits `# HELP` / `# TYPE` lines for metrics
/// that have *at least one* observation recorded, so `describe_*` alone
/// is not enough to surface a metric's name at bootstrap. Emitting a
/// zero sample is the canonical workaround; it keeps dashboards / alerts
/// deterministic (operators don't have to trigger a code path just to
/// confirm a metric exists).
///
/// A `"bootstrap"` sentinel label is used on every series that would
/// otherwise take dynamic labels (route, method, etc.). This keeps the
/// cardinality bounded at startup: one extra series per catalogue entry
/// rather than one per label combination.
fn register_zeroes() {
    // Counters — `absolute(0)` records a zero sample, which registers the
    // name in the exporter's internal catalogue. Labels are `"bootstrap"`
    // so real traffic shows up as separate series.
    counter!(HTTP_REQUESTS_TOTAL, "route" => "bootstrap", "method" => "bootstrap", "status" => "0")
        .absolute(0);
    counter!(OUTBOX_ATTEMPTS_TOTAL, "subscriber" => "bootstrap", "result" => "bootstrap")
        .absolute(0);
    counter!(OUTBOX_DEAD_LETTER_TOTAL).absolute(0);
    counter!(
        NOTIFICATIONS_SENT_TOTAL,
        "channel" => "bootstrap",
        "provider" => "bootstrap",
        "status" => "bootstrap"
    )
    .absolute(0);
    counter!(STRIPE_WEBHOOK_TOTAL, "event_type" => "bootstrap", "result" => "bootstrap")
        .absolute(0);
    counter!(CONSENT_RECORDS_TOTAL, "category" => "bootstrap", "action" => "bootstrap").absolute(0);
    counter!(DSAR_REQUESTS_TOTAL, "kind" => "bootstrap", "status" => "bootstrap").absolute(0);
    counter!(ORDERS_TOTAL, "status" => "bootstrap").absolute(0);

    // Gauges — `set(0.0)` registers the series.
    gauge!(OUTBOX_PENDING).set(0.0);

    // Histograms — `record(0.0)` registers the series with a single
    // zero observation (lands in the smallest bucket).
    histogram!(HTTP_REQUEST_DURATION_SECONDS, "route" => "bootstrap", "method" => "bootstrap")
        .record(0.0);
}

/// Bounded cardinality normaliser for the `route` label.
///
/// Pulls [`MatchedPath`] from the request extensions when present (set by
/// Axum's router once it has matched a route), otherwise returns a fixed
/// `"unmatched"` sentinel. Never returns the raw URI — that would let a
/// caller inflate the metric cardinality to unbounded by permuting path
/// segments.
fn route_label(req: &Request) -> String {
    req.extensions()
        .get::<MatchedPath>()
        .map(|mp| mp.as_str().to_owned())
        .unwrap_or_else(|| UNMATCHED_ROUTE.to_string())
}

/// HTTP metrics middleware. Wire into the router via
/// `axum::middleware::from_fn_with_state(state, http_middleware)`.
///
/// The `State<AppState>` parameter is unused today but kept in the
/// signature so future label additions (user role via JWT, tenant id,
/// etc.) can reach into `AppState` without refactoring the wiring site.
pub async fn http_middleware(
    State(_state): State<AppState>,
    req: Request,
    next: Next,
) -> Result<Response<Body>, AppError> {
    let method = req.method().as_str().to_owned();
    let route = route_label(&req);
    let start = std::time::Instant::now();

    let response = next.run(req).await;
    let elapsed = start.elapsed().as_secs_f64();
    let status = response.status().as_u16().to_string();

    // Record the histogram first so a panic in `counter!` still leaves a
    // latency sample; the ordering is belt-and-braces — neither macro
    // panics in the current `metrics` release.
    histogram!(
        HTTP_REQUEST_DURATION_SECONDS,
        "route" => route.clone(),
        "method" => method.clone()
    )
    .record(elapsed);

    counter!(
        HTTP_REQUESTS_TOTAL,
        "route" => route,
        "method" => method,
        "status" => status
    )
    .increment(1);

    Ok(response)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn install_recorder_is_idempotent() {
        // Two sequential calls should return handles that share a backing
        // recorder; both renders should name every catalogue entry. We
        // can't compare the rendered strings byte-for-byte because
        // counter writes from other parallel tests can land between the
        // two calls — instead, confirm both renders expose the same
        // catalogue of metric names.
        let a = install_recorder();
        let b = install_recorder();
        let ra = a.render();
        let rb = b.render();
        for name in [
            HTTP_REQUESTS_TOTAL,
            OUTBOX_PENDING,
            OUTBOX_DEAD_LETTER_TOTAL,
        ] {
            assert!(
                ra.contains(name) && rb.contains(name),
                "both render outputs must name `{name}` — idempotent install is broken"
            );
        }
    }

    #[test]
    fn install_recorder_registers_catalogue() {
        let handle = install_recorder();
        let rendered = handle.render();
        // `describe_counter!` emits a `# HELP` line whether or not the
        // metric has observations. Assert on a representative subset.
        for name in [
            HTTP_REQUESTS_TOTAL,
            HTTP_REQUEST_DURATION_SECONDS,
            OUTBOX_PENDING,
            OUTBOX_DEAD_LETTER_TOTAL,
            NOTIFICATIONS_SENT_TOTAL,
            STRIPE_WEBHOOK_TOTAL,
            CONSENT_RECORDS_TOTAL,
            DSAR_REQUESTS_TOTAL,
            ORDERS_TOTAL,
        ] {
            assert!(
                rendered.contains(name),
                "rendered output is missing metric `{name}`\n---\n{rendered}\n---",
            );
        }
    }

    #[test]
    fn http_duration_buckets_are_monotonic() {
        // Prometheus requires strictly-increasing bucket boundaries. A
        // typo here is a configuration error that only surfaces at scrape
        // time; catching it in unit tests is cheap insurance.
        let mut previous = f64::NEG_INFINITY;
        for b in HTTP_DURATION_BUCKETS {
            assert!(
                *b > previous,
                "buckets must be strictly increasing ({previous} >= {b})"
            );
            previous = *b;
        }
        assert_eq!(
            HTTP_DURATION_BUCKETS.len(),
            11,
            "bucket count changed; update dashboards + this assertion together"
        );
    }

    #[test]
    fn counter_increment_shows_up_in_render() {
        let handle = install_recorder();
        counter!(
            HTTP_REQUESTS_TOTAL,
            "route" => "/api/ping",
            "method" => "GET",
            "status" => "200"
        )
        .increment(1);

        let rendered = handle.render();
        // Label order in Prometheus exposition is implementation-defined;
        // assert on the counter name + a distinctive label fragment.
        assert!(rendered.contains(HTTP_REQUESTS_TOTAL));
        assert!(rendered.contains("/api/ping"));
        assert!(rendered.contains("method=\"GET\""));
    }

    #[test]
    fn route_label_falls_back_to_unmatched_without_matched_path() {
        // `MatchedPath` has no public constructor (its tuple field is
        // `pub(crate)`) so we can't build one directly from a unit test.
        // The fallback branch is covered here; the happy path is exercised
        // by the integration test in `backend/tests/observability.rs`
        // which runs a real Router that populates `MatchedPath` on match.
        let req: Request = Request::builder()
            .uri("/nope")
            .body(Body::empty())
            .expect("request builds");
        assert_eq!(route_label(&req), UNMATCHED_ROUTE);
    }
}
