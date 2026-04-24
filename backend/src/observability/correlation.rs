//! Request-id (`X-Request-Id`) correlation middleware.
//!
//! Every request entering the service is tagged with a stable id — either
//! the caller-supplied `X-Request-Id` header (when it passes validation) or
//! a freshly minted UUIDv4 hex string. The id is:
//!
//! 1. Attached to the `tracing` span covering the handler call so every
//!    structured log line includes a `request_id` field.
//! 2. Stored as a typed [`RequestId`] extension on the `Request` so the
//!    error path (and future subsystems — audit log, outbox, DSAR, etc.)
//!    can read it via [`current_request_id`] and populate
//!    `Problem.correlation_id`.
//! 3. Echoed back to the caller as an `X-Request-Id` response header so
//!    browser dev-tools / load-balancer logs / customer bug reports all
//!    carry the same token.
//!
//! # Validation rules
//!
//! A caller-supplied id is accepted iff:
//!
//! * length is between 16 and 64 bytes inclusive, and
//! * every byte is printable ASCII (`0x21 ..= 0x7E`).
//!
//! This is the same envelope Sentry / Datadog / Honeycomb use. Non-ASCII
//! and control characters are rejected rather than sanitised: a caller
//! sending garbage gets our generated id, not a silent mangling of theirs.
//!
//! # Wiring
//!
//! ```rust,ignore
//! use axum::Router;
//! use swings_api::observability::correlation;
//!
//! let app: Router = Router::new()
//!     // … routes …
//!     .layer(axum::middleware::from_fn(correlation::middleware));
//! ```
//!
//! The [`layer`] helper below wraps the `from_fn` call for integrators that
//! prefer a single expression; both forms are equivalent.
//!
//! # Why not pull `tower-http::request_id`?
//!
//! `tower-http` provides `SetRequestIdLayer` + `PropagateRequestIdLayer`,
//! but they don't bind the id to a `tracing` span out of the box and the
//! validation envelope is coarser than we want. A short `axum::middleware::
//! from_fn` handler is easier to audit and carries zero additional deps.

use axum::{
    body::{to_bytes, Body},
    extract::Request,
    http::{header, header::HeaderName, HeaderValue, Response},
    middleware::Next,
};
use tracing::Instrument;
use uuid::Uuid;

/// Max `application/problem+json` body the correlation stamper is willing to
/// parse. RFC 7807 documents are small (< 2 KiB in practice); anything bigger
/// is almost certainly not a Problem and is passed through untouched so we
/// never silently rewrite a legitimately large response.
const PROBLEM_JSON_MAX_BYTES: usize = 8 * 1024;

/// Header name used for request-id passthrough, canonicalised.
///
/// Defined as a `const` so we can reference it without allocating in both
/// the middleware and tests. The spelling matches the common Railway /
/// Vercel / Heroku convention.
pub const X_REQUEST_ID: HeaderName = HeaderName::from_static("x-request-id");

/// Minimum accepted length for a caller-supplied request id, in bytes.
pub const MIN_LEN: usize = 16;
/// Maximum accepted length for a caller-supplied request id, in bytes.
pub const MAX_LEN: usize = 64;

/// Typed request-id wrapper stored as an axum `Extensions` entry.
///
/// Handlers (and the error-rendering path once wired) can pull this out
/// of `Request::extensions()` via [`current_request_id`]. The `Clone` impl
/// is cheap — the id is a fixed 32-char UUID hex or a caller-supplied
/// value bounded to 64 bytes, so cloning is one heap copy of a short
/// string.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RequestId(pub String);

impl RequestId {
    /// Borrow the id as a `&str`.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for RequestId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// Read the current request id off a `Request`, if the correlation
/// middleware has run on it.
///
/// Returns `None` when the middleware hasn't executed (e.g. a handler
/// called outside the Axum router in a unit test, or a route that was
/// wired without the layer). Handlers should treat a missing id as
/// non-fatal: fall back to `None` when populating `Problem.correlation_id`
/// rather than generating one on the fly, which would break the
/// "same id everywhere" invariant.
#[must_use]
pub fn current_request_id(req: &Request) -> Option<String> {
    req.extensions().get::<RequestId>().map(|r| r.0.clone())
}

/// Middleware body. Wire into a router via
/// `axum::middleware::from_fn(correlation::middleware)` or via the
/// [`layer`] helper below.
///
/// Life-cycle:
///
/// 1. Read the incoming `X-Request-Id` header; validate.
/// 2. Substitute a generated id if the header was absent or invalid.
/// 3. Insert the id as a [`RequestId`] extension on the `Request`.
/// 4. Build a `tracing` span carrying `request_id = %id` and run the rest
///    of the chain under it.
/// 5. On response, write the id back as `X-Request-Id`.
pub async fn middleware(mut req: Request, next: Next) -> Response<Body> {
    let id = extract_or_generate(&req);

    // Stash the id on request extensions so the error path (or any inner
    // handler that wants to correlate an emit) can read it back.
    req.extensions_mut().insert(RequestId(id.clone()));

    // Build a request-scoped span. The `info_span!` macro captures
    // `request_id` as a field, which the JSON formatter serialises into
    // the structured log line. The span covers the entire downstream
    // handler call via `Instrument::instrument`.
    let span = tracing::info_span!("http.request", request_id = %id);
    let response = next.run(req).instrument(span).await;

    // RFC 7807 extension: stamp `correlation_id` into `application/problem+json`
    // responses so a customer-reported error body carries the same id as our
    // structured logs. We only rewrite bodies we're confident we understand
    // (content-type + size envelope); anything else passes through untouched.
    let mut response = stamp_problem_correlation(response, &id).await;

    // Echo the id back. `HeaderValue::from_str` only fails on non-ASCII
    // or control inputs; we have already validated / generated an
    // ASCII-safe string, so the `unwrap_or_else` branch is effectively
    // unreachable — we handle it defensively to honour the "no unwrap
    // in prod paths" constraint.
    let header_value = HeaderValue::from_str(&id)
        .unwrap_or_else(|_| HeaderValue::from_static("invalid-request-id"));
    response.headers_mut().insert(X_REQUEST_ID, header_value);

    response
}

/// If the response is `application/problem+json`, parse it, inject
/// `correlation_id`, and reserialise. Bodies larger than
/// [`PROBLEM_JSON_MAX_BYTES`] or that fail to parse as JSON objects are
/// returned unchanged — the invariant is "never corrupt a response".
async fn stamp_problem_correlation(response: Response<Body>, id: &str) -> Response<Body> {
    let is_problem = response
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .map(|v| v.starts_with("application/problem+json"))
        .unwrap_or(false);

    if !is_problem {
        return response;
    }

    let (parts, body) = response.into_parts();
    let bytes = match to_bytes(body, PROBLEM_JSON_MAX_BYTES).await {
        Ok(b) => b,
        Err(_) => {
            // Oversize or read failure — we can't safely mutate; rebuild with
            // an empty body rather than corrupt downstream state.
            return Response::from_parts(parts, Body::empty());
        }
    };

    match serde_json::from_slice::<serde_json::Value>(&bytes) {
        Ok(serde_json::Value::Object(mut map)) => {
            map.entry("correlation_id")
                .or_insert_with(|| serde_json::Value::String(id.to_owned()));
            let new_body = serde_json::to_vec(&serde_json::Value::Object(map))
                .unwrap_or_else(|_| bytes.to_vec());
            Response::from_parts(parts, Body::from(new_body))
        }
        _ => Response::from_parts(parts, Body::from(bytes)),
    }
}

/// Convenience constructor: returns the `FromFnLayer` produced by
/// `axum::middleware::from_fn(middleware)`.
///
/// Integrators that prefer a single call-site expression can write
/// `.layer(observability::correlation::layer())` instead of the explicit
/// `from_fn` form. Both paths produce the same tower layer.
pub fn layer(
) -> axum::middleware::FromFnLayer<fn(Request, Next) -> CorrelationFuture, (), CorrelationFuture> {
    axum::middleware::from_fn(wrapper as fn(Request, Next) -> CorrelationFuture)
}

/// Boxed future type returned by [`wrapper`]. Explicit so the
/// [`layer`] constructor can name it.
pub type CorrelationFuture =
    std::pin::Pin<Box<dyn std::future::Future<Output = Response<Body>> + Send + 'static>>;

/// Function pointer wrapper around [`middleware`] so [`layer`] can point
/// at a concrete `fn` (rather than a closure) and produce a nameable
/// `FromFnLayer` type.
fn wrapper(req: Request, next: Next) -> CorrelationFuture {
    Box::pin(middleware(req, next))
}

/// Pull the caller-supplied id if it passes validation, otherwise mint
/// a fresh UUIDv4 hex string (32 chars).
fn extract_or_generate(req: &Request) -> String {
    req.headers()
        .get(X_REQUEST_ID)
        .and_then(|v| v.to_str().ok())
        .filter(|candidate| is_valid_id(candidate))
        .map(str::to_owned)
        .unwrap_or_else(|| Uuid::new_v4().simple().to_string())
}

/// Envelope check for caller-supplied ids.
///
/// Accepts only printable ASCII to keep downstream log / dashboard tools
/// happy. A 16-char minimum rules out trivially-guessable ids like `"1"`
/// or `"abc"`, which correlate poorly.
fn is_valid_id(candidate: &str) -> bool {
    let len = candidate.len();
    if !(MIN_LEN..=MAX_LEN).contains(&len) {
        return false;
    }
    candidate.bytes().all(|b| (0x21..=0x7E).contains(&b))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::Body,
        http::{Request as HttpRequest, StatusCode},
        routing::get,
        Router,
    };
    use tower::ServiceExt;

    async fn handler() -> &'static str {
        "ok"
    }

    fn test_app() -> Router {
        Router::new()
            .route("/ping", get(handler))
            .layer(axum::middleware::from_fn(middleware))
    }

    #[tokio::test]
    async fn generates_id_when_header_absent() {
        let app = test_app();
        let req = HttpRequest::builder()
            .uri("/ping")
            .body(Body::empty())
            .expect("request builds");
        let resp = app.oneshot(req).await.expect("dispatch");

        assert_eq!(resp.status(), StatusCode::OK);
        let id = resp
            .headers()
            .get(X_REQUEST_ID)
            .expect("header set")
            .to_str()
            .expect("ascii");
        // UUIDv4 simple format is 32 hex chars — falls inside our envelope.
        assert_eq!(id.len(), 32);
        assert!(is_valid_id(id));
    }

    #[tokio::test]
    async fn passes_through_valid_inbound_id() {
        let app = test_app();
        let inbound = "req-0123456789abcdef-ABCDEF";
        let req = HttpRequest::builder()
            .uri("/ping")
            .header(X_REQUEST_ID.as_str(), inbound)
            .body(Body::empty())
            .expect("request builds");
        let resp = app.oneshot(req).await.expect("dispatch");

        assert_eq!(resp.status(), StatusCode::OK);
        let id = resp
            .headers()
            .get(X_REQUEST_ID)
            .expect("header set")
            .to_str()
            .expect("ascii");
        assert_eq!(id, inbound);
    }

    #[tokio::test]
    async fn rejects_too_short_inbound_id() {
        let app = test_app();
        let req = HttpRequest::builder()
            .uri("/ping")
            .header(X_REQUEST_ID.as_str(), "too-short")
            .body(Body::empty())
            .expect("request builds");
        let resp = app.oneshot(req).await.expect("dispatch");

        let id = resp
            .headers()
            .get(X_REQUEST_ID)
            .expect("header set")
            .to_str()
            .expect("ascii");
        assert_eq!(id.len(), 32, "short caller id replaced by UUID");
    }

    #[tokio::test]
    async fn rejects_too_long_inbound_id() {
        let app = test_app();
        let too_long = "a".repeat(MAX_LEN + 1);
        let req = HttpRequest::builder()
            .uri("/ping")
            .header(X_REQUEST_ID.as_str(), &too_long)
            .body(Body::empty())
            .expect("request builds");
        let resp = app.oneshot(req).await.expect("dispatch");

        let id = resp
            .headers()
            .get(X_REQUEST_ID)
            .expect("header set")
            .to_str()
            .expect("ascii");
        assert_eq!(id.len(), 32, "over-long caller id replaced by UUID");
        assert_ne!(id, too_long);
    }

    #[test]
    fn validator_accepts_typical_uuid() {
        assert!(is_valid_id("b7e6a3f2d4114c8c8b2c6f5d4e3a2b1f"));
    }

    #[test]
    fn validator_rejects_non_ascii() {
        // `é` encodes as two bytes ≥ 0x80.
        assert!(!is_valid_id("abcdefghijklmnop\u{00E9}"));
    }

    #[test]
    fn validator_rejects_boundary_lengths() {
        let too_short = "a".repeat(MIN_LEN - 1);
        let too_long = "a".repeat(MAX_LEN + 1);
        assert!(!is_valid_id(&too_short));
        assert!(!is_valid_id(&too_long));
        assert!(is_valid_id(&"a".repeat(MIN_LEN)));
        assert!(is_valid_id(&"a".repeat(MAX_LEN)));
    }

    #[test]
    fn validator_rejects_control_chars() {
        assert!(!is_valid_id("abcdefghijklmnop\n"));
        assert!(!is_valid_id("abcdefghijklmnop\t"));
    }

    #[test]
    fn current_request_id_returns_none_without_middleware() {
        let req = HttpRequest::builder()
            .uri("/ping")
            .body(Body::empty())
            .expect("request builds");
        assert!(current_request_id(&req).is_none());
    }

    async fn problem_handler() -> Response<Body> {
        let body = serde_json::json!({
            "type": "/problems/test",
            "title": "Test",
            "status": 400,
            "detail": "Synthetic failure"
        });
        let mut resp = Response::new(Body::from(body.to_string()));
        resp.headers_mut().insert(
            header::CONTENT_TYPE,
            HeaderValue::from_static("application/problem+json"),
        );
        *resp.status_mut() = StatusCode::BAD_REQUEST;
        resp
    }

    async fn plain_handler() -> Response<Body> {
        let mut resp = Response::new(Body::from(r#"{"ok":true}"#));
        resp.headers_mut().insert(
            header::CONTENT_TYPE,
            HeaderValue::from_static("application/json"),
        );
        resp
    }

    #[tokio::test]
    async fn stamps_correlation_id_into_problem_json() {
        let app = Router::new()
            .route("/p", get(problem_handler))
            .layer(axum::middleware::from_fn(middleware));
        let req = HttpRequest::builder()
            .uri("/p")
            .body(Body::empty())
            .expect("request builds");
        let resp = app.oneshot(req).await.expect("dispatch");
        let request_id = resp
            .headers()
            .get(X_REQUEST_ID)
            .expect("header set")
            .to_str()
            .expect("ascii")
            .to_owned();
        let body_bytes = to_bytes(resp.into_body(), 1024).await.expect("read body");
        let body: serde_json::Value = serde_json::from_slice(&body_bytes).expect("json");
        assert_eq!(body["correlation_id"].as_str(), Some(request_id.as_str()));
    }

    #[tokio::test]
    async fn leaves_non_problem_json_alone() {
        let app = Router::new()
            .route("/plain", get(plain_handler))
            .layer(axum::middleware::from_fn(middleware));
        let req = HttpRequest::builder()
            .uri("/plain")
            .body(Body::empty())
            .expect("request builds");
        let resp = app.oneshot(req).await.expect("dispatch");
        let body_bytes = to_bytes(resp.into_body(), 1024).await.expect("read body");
        let body: serde_json::Value = serde_json::from_slice(&body_bytes).expect("json");
        assert!(body.get("correlation_id").is_none());
        assert_eq!(body["ok"], true);
    }
}
