//! Unified error type + RFC 7807 `application/problem+json` representation.
//!
//! Every handler returns [`AppResult<T>`]; errors surface through [`AppError`] and are
//! rendered as [`Problem`] documents. Internal / database errors are logged at the
//! error layer but redacted on the wire to avoid leaking stack traces or SQL state.
//!
//! # Adding a new variant
//! 1. Extend [`AppError`].
//! 2. Map it in [`AppError::to_problem`] — choose a stable, dash-cased `type` slug; it
//!    becomes part of the URI reference in the response body.
//! 3. Add a round-trip test in the [`tests`] module below.

use axum::{
    http::{header, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;

/// Problem type URI prefix. Relative refs per RFC 7807 §3.1 resolve against the
/// request URL when the client dereferences them; we keep them relative so the
/// error payload stays deploy-independent.
const PROBLEM_TYPE_PREFIX: &str = "/problems/";

/// RFC 7807 Problem Details for HTTP APIs.
///
/// Fields follow the spec; `correlation_id` is an allowed extension carrying
/// the request-scoped trace id that downstream logs also record, so operators
/// can pivot from a user-reported error back to the span.
#[derive(Debug, Clone, Serialize)]
pub struct Problem {
    /// URI reference that identifies the problem type. Relative.
    #[serde(rename = "type")]
    pub type_uri: String,
    /// Short, human-readable summary of the problem type.
    pub title: String,
    /// HTTP status code.
    pub status: u16,
    /// Human-readable explanation specific to this occurrence.
    pub detail: String,
    /// URI reference that identifies this specific occurrence, if the caller
    /// supplied or the server generated one (e.g. a request path).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instance: Option<String>,
    /// Request/trace correlation id (extension field).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub correlation_id: Option<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("Authentication required")]
    Unauthorized,

    #[error("Insufficient permissions")]
    Forbidden,

    #[error("{0}")]
    NotFound(String),

    #[error("{0}")]
    BadRequest(String),

    #[error("{0}")]
    Conflict(String),

    /// Business-rule violation (422). Distinct from [`AppError::Validation`],
    /// which is reserved for `validator`-derived schema errors. Wired in Phase 4
    /// subsystems (forms, checkout, subscription-change).
    #[error("{0}")]
    Unprocessable(String),

    /// Wired in Phase 4 FDN-08 (app-layer rate limits beyond `tower_governor`).
    #[error("Too many requests")]
    TooManyRequests,

    /// Wired in Phase 4 FDN-04/FDN-05 (outbox + notifications provider failures).
    #[error("{0}")]
    ServiceUnavailable(String),

    #[error("{0}")]
    PayloadTooLarge(String),

    /// Wired in Phase 4 when any endpoint intentionally stubs a feature.
    #[error("Not implemented")]
    NotImplemented,

    #[error("Internal server error")]
    Internal(#[from] anyhow::Error),

    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("Validation error: {0}")]
    Validation(String),

    /// FORM-03: structured 422 body carrying per-field validation errors.
    /// Rendered as `application/problem+json` with an `errors: [...]`
    /// extension field (RFC 7807 §3.2 allows custom extensions).
    #[error("Validation failed")]
    ValidationBody(serde_json::Value),

    #[error("{0}")]
    TokenReuseDetected(String),

    #[error(transparent)]
    Storage(#[from] crate::services::StorageError),
}

impl AppError {
    /// Map the error into (status, Problem). Server-internal details
    /// (`Internal`, `Database`, some `Storage` subvariants) log the full
    /// cause at `error` level but return a redacted Problem body.
    pub fn to_problem(&self) -> (StatusCode, Problem) {
        match self {
            AppError::Unauthorized => (
                StatusCode::UNAUTHORIZED,
                problem("unauthorized", "Unauthorized", 401, self.to_string()),
            ),
            AppError::Forbidden => (
                StatusCode::FORBIDDEN,
                problem("forbidden", "Forbidden", 403, self.to_string()),
            ),
            AppError::NotFound(msg) => (
                StatusCode::NOT_FOUND,
                problem("not-found", "Not Found", 404, msg.clone()),
            ),
            AppError::BadRequest(msg) => (
                StatusCode::BAD_REQUEST,
                problem("bad-request", "Bad Request", 400, msg.clone()),
            ),
            AppError::Conflict(msg) => (
                StatusCode::CONFLICT,
                problem("conflict", "Conflict", 409, msg.clone()),
            ),
            AppError::Unprocessable(msg) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                problem("unprocessable", "Unprocessable Entity", 422, msg.clone()),
            ),
            AppError::TooManyRequests => (
                StatusCode::TOO_MANY_REQUESTS,
                problem(
                    "too-many-requests",
                    "Too Many Requests",
                    429,
                    self.to_string(),
                ),
            ),
            AppError::ServiceUnavailable(msg) => (
                StatusCode::SERVICE_UNAVAILABLE,
                problem(
                    "service-unavailable",
                    "Service Unavailable",
                    503,
                    msg.clone(),
                ),
            ),
            AppError::PayloadTooLarge(msg) => (
                StatusCode::PAYLOAD_TOO_LARGE,
                problem("payload-too-large", "Payload Too Large", 413, msg.clone()),
            ),
            AppError::NotImplemented => (
                StatusCode::NOT_IMPLEMENTED,
                problem("not-implemented", "Not Implemented", 501, self.to_string()),
            ),
            AppError::Validation(msg) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                problem("validation", "Validation Error", 422, msg.clone()),
            ),
            AppError::ValidationBody(_) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                problem(
                    "validation",
                    "Validation Error",
                    422,
                    "One or more fields failed validation.".into(),
                ),
            ),
            AppError::TokenReuseDetected(msg) => (
                StatusCode::UNAUTHORIZED,
                problem("token-reuse", "Token Reuse Detected", 401, msg.clone()),
            ),
            AppError::Internal(err) => {
                tracing::error!(error = ?err, "internal error");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    problem(
                        "internal",
                        "Internal Server Error",
                        500,
                        "An unexpected error occurred.".into(),
                    ),
                )
            }
            AppError::Database(err) => {
                tracing::error!(error = ?err, "database error");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    problem(
                        "internal",
                        "Internal Server Error",
                        500,
                        "An unexpected error occurred.".into(),
                    ),
                )
            }
            AppError::Storage(err) => match err {
                crate::services::StorageError::Config(_) => (
                    StatusCode::BAD_REQUEST,
                    problem("storage-config", "Bad Request", 400, err.to_string()),
                ),
                crate::services::StorageError::Upload(_)
                | crate::services::StorageError::Delete(_) => {
                    tracing::error!(error = %err, "storage error");
                    (
                        StatusCode::BAD_GATEWAY,
                        problem(
                            "storage",
                            "Bad Gateway",
                            502,
                            "Storage operation failed.".into(),
                        ),
                    )
                }
            },
        }
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        // FORM-03: `ValidationBody` inlines a structured `errors` field
        // alongside the Problem document so the client can pin messages
        // to fields without string-matching on `detail`.
        if let AppError::ValidationBody(errors) = &self {
            let (status, p) = self.to_problem();
            let body = serde_json::json!({
                "type": p.type_uri,
                "title": p.title,
                "status": p.status,
                "detail": p.detail,
                "errors": errors,
            });
            let mut resp = Json(body).into_response();
            resp.headers_mut().insert(
                header::CONTENT_TYPE,
                HeaderValue::from_static("application/problem+json"),
            );
            *resp.status_mut() = status;
            return resp;
        }
        let (status, body) = self.to_problem();
        let mut resp = Json(body).into_response();
        resp.headers_mut().insert(
            header::CONTENT_TYPE,
            HeaderValue::from_static("application/problem+json"),
        );
        *resp.status_mut() = status;
        resp
    }
}

pub type AppResult<T> = Result<T, AppError>;

fn problem(type_slug: &str, title: &str, status: u16, detail: String) -> Problem {
    Problem {
        type_uri: format!("{PROBLEM_TYPE_PREFIX}{type_slug}"),
        title: title.to_owned(),
        status,
        detail,
        instance: None,
        correlation_id: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::to_bytes;
    use serde_json::Value;

    async fn render(err: AppError) -> (StatusCode, String, Value) {
        let resp = err.into_response();
        let status = resp.status();
        let content_type = resp
            .headers()
            .get(header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .to_owned();
        let bytes = to_bytes(resp.into_body(), 64 * 1024)
            .await
            .expect("body fits in 64kb for test payloads");
        let body: Value = serde_json::from_slice(&bytes).expect("body is JSON");
        (status, content_type, body)
    }

    fn assert_problem(body: &Value, expected_status: u16, expected_type: &str, title: &str) {
        assert_eq!(body["status"].as_u64().unwrap(), expected_status as u64);
        assert_eq!(
            body["type"].as_str().unwrap(),
            format!("/problems/{expected_type}")
        );
        assert_eq!(body["title"].as_str().unwrap(), title);
        assert!(body["detail"].as_str().is_some());
    }

    #[tokio::test]
    async fn unauthorized_round_trip() {
        let (status, ct, body) = render(AppError::Unauthorized).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);
        assert_eq!(ct, "application/problem+json");
        assert_problem(&body, 401, "unauthorized", "Unauthorized");
    }

    #[tokio::test]
    async fn forbidden_round_trip() {
        let (status, _, body) = render(AppError::Forbidden).await;
        assert_eq!(status, StatusCode::FORBIDDEN);
        assert_problem(&body, 403, "forbidden", "Forbidden");
    }

    #[tokio::test]
    async fn not_found_round_trip() {
        let (status, _, body) = render(AppError::NotFound("post".into())).await;
        assert_eq!(status, StatusCode::NOT_FOUND);
        assert_problem(&body, 404, "not-found", "Not Found");
        assert_eq!(body["detail"].as_str().unwrap(), "post");
    }

    #[tokio::test]
    async fn bad_request_round_trip() {
        let (status, _, body) = render(AppError::BadRequest("x".into())).await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_problem(&body, 400, "bad-request", "Bad Request");
    }

    #[tokio::test]
    async fn conflict_round_trip() {
        let (status, _, body) = render(AppError::Conflict("dupe".into())).await;
        assert_eq!(status, StatusCode::CONFLICT);
        assert_problem(&body, 409, "conflict", "Conflict");
    }

    #[tokio::test]
    async fn unprocessable_round_trip() {
        let (status, _, body) = render(AppError::Unprocessable("bad state".into())).await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
        assert_problem(&body, 422, "unprocessable", "Unprocessable Entity");
    }

    #[tokio::test]
    async fn too_many_requests_round_trip() {
        let (status, _, body) = render(AppError::TooManyRequests).await;
        assert_eq!(status, StatusCode::TOO_MANY_REQUESTS);
        assert_problem(&body, 429, "too-many-requests", "Too Many Requests");
    }

    #[tokio::test]
    async fn service_unavailable_round_trip() {
        let (status, _, body) = render(AppError::ServiceUnavailable("down".into())).await;
        assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
        assert_problem(&body, 503, "service-unavailable", "Service Unavailable");
    }

    #[tokio::test]
    async fn payload_too_large_round_trip() {
        let (status, _, body) = render(AppError::PayloadTooLarge("100mb".into())).await;
        assert_eq!(status, StatusCode::PAYLOAD_TOO_LARGE);
        assert_problem(&body, 413, "payload-too-large", "Payload Too Large");
    }

    #[tokio::test]
    async fn not_implemented_round_trip() {
        let (status, _, body) = render(AppError::NotImplemented).await;
        assert_eq!(status, StatusCode::NOT_IMPLEMENTED);
        assert_problem(&body, 501, "not-implemented", "Not Implemented");
    }

    #[tokio::test]
    async fn validation_round_trip() {
        let (status, _, body) = render(AppError::Validation("email: invalid".into())).await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
        assert_problem(&body, 422, "validation", "Validation Error");
    }

    #[tokio::test]
    async fn token_reuse_round_trip() {
        let (status, _, body) = render(AppError::TokenReuseDetected("reuse".into())).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);
        assert_problem(&body, 401, "token-reuse", "Token Reuse Detected");
    }

    #[tokio::test]
    async fn internal_redacts_cause() {
        let err = AppError::Internal(anyhow::anyhow!("secret internal details"));
        let (status, _, body) = render(err).await;
        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_problem(&body, 500, "internal", "Internal Server Error");
        assert_eq!(
            body["detail"].as_str().unwrap(),
            "An unexpected error occurred."
        );
        assert!(!body.to_string().contains("secret internal details"));
    }

    #[tokio::test]
    async fn database_redacts_cause() {
        let err = AppError::Database(sqlx::Error::RowNotFound);
        let (status, _, body) = render(err).await;
        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_problem(&body, 500, "internal", "Internal Server Error");
        assert_eq!(
            body["detail"].as_str().unwrap(),
            "An unexpected error occurred."
        );
    }

    #[tokio::test]
    async fn storage_config_is_bad_request() {
        let err = AppError::Storage(crate::services::StorageError::Config("missing key".into()));
        let (status, _, body) = render(err).await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_problem(&body, 400, "storage-config", "Bad Request");
    }

    #[tokio::test]
    async fn storage_upload_redacts_to_bad_gateway() {
        let err = AppError::Storage(crate::services::StorageError::Upload("s3 boom".into()));
        let (status, _, body) = render(err).await;
        assert_eq!(status, StatusCode::BAD_GATEWAY);
        assert_problem(&body, 502, "storage", "Bad Gateway");
        assert_eq!(
            body["detail"].as_str().unwrap(),
            "Storage operation failed."
        );
        assert!(!body.to_string().contains("s3 boom"));
    }

    #[test]
    fn problem_type_uses_relative_prefix() {
        let p = problem("x", "t", 500, "d".into());
        assert!(p.type_uri.starts_with("/problems/"));
    }

    #[test]
    fn problem_omits_optional_fields_when_absent() {
        let p = problem("x", "t", 500, "d".into());
        let serialized = serde_json::to_string(&p).unwrap();
        assert!(!serialized.contains("instance"));
        assert!(!serialized.contains("correlation_id"));
    }
}
