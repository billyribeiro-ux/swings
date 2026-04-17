//! FDN-08: CSP violation report sink.
//!
//! Browsers POST to `/api/csp-report` directly when a page violates the CSP
//! served by `src/hooks.server.ts`. The endpoint is unauthenticated (browsers
//! don't carry app credentials on these requests), heavily rate-limited, and
//! log-only for this subsystem — a dedicated `csp_reports` table / dashboard
//! can land in a future hardening pass.
//!
//! Two content-types are supported to cover both legacy (Level 2) and
//! modern (Reporting API) browsers:
//!
//! * `application/csp-report` — Level 2 `{"csp-report": {...}}` envelope.
//! * `application/reports+json` — Reporting API `[{"type":"csp-violation","body":{...}}, ...]`.
//!
//! Payloads are clamped at [`MAX_REPORT_BYTES`]; oversized bodies return 413
//! via [`AppError::PayloadTooLarge`] (RFC 7807 Problem response).

use axum::{
    body::{to_bytes, Body, Bytes},
    extract::Request,
    http::{header, HeaderMap, StatusCode},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::post,
    Router,
};
use serde::Deserialize;

use crate::{
    error::{AppError, AppResult},
    AppState,
};

/// Maximum size of a single CSP-report body. Browsers normally send ~1KB, so
/// 8KB is generous enough for large multi-directive violations while still
/// protecting the endpoint from payload-stuffing.
pub const MAX_REPORT_BYTES: usize = 8 * 1024;

// Level 2 envelope (`application/csp-report`).
#[derive(Debug, Deserialize)]
struct CspReportEnvelope {
    #[serde(rename = "csp-report")]
    csp_report: CspReportBody,
}

// Reporting API envelope entry (`application/reports+json`).
#[derive(Debug, Deserialize)]
struct ReportsApiEntry {
    #[serde(default)]
    #[serde(rename = "type")]
    kind: Option<String>,
    #[serde(default)]
    body: Option<CspReportBody>,
}

/// The subset of CSP report fields we log. Every field is optional because the
/// spec has evolved and older browsers omit some, newer omit others.
#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct CspReportBody {
    #[serde(rename = "document-uri", alias = "documentURL")]
    document_uri: Option<String>,
    #[serde(rename = "violated-directive", alias = "effectiveDirective")]
    violated_directive: Option<String>,
    #[serde(rename = "blocked-uri", alias = "blockedURL")]
    blocked_uri: Option<String>,
    #[serde(rename = "source-file", alias = "sourceFile")]
    source_file: Option<String>,
    #[serde(rename = "line-number", alias = "lineNumber")]
    line_number: Option<u32>,
    #[serde(rename = "column-number", alias = "columnNumber")]
    column_number: Option<u32>,
    #[serde(rename = "script-sample")]
    script_sample: Option<String>,
    disposition: Option<String>,
    referrer: Option<String>,
    #[serde(rename = "status-code")]
    status_code: Option<u32>,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/csp-report", post(csp_report))
        .layer(middleware::from_fn(clamp_body_size))
        .layer(crate::middleware::rate_limit::csp_report_layer())
}

/// Middleware — short-circuit requests whose body exceeds [`MAX_REPORT_BYTES`]
/// before handing them to the handler. Browsers never exceed this budget in
/// practice; anything larger is treated as an attack.
async fn clamp_body_size(req: Request, next: Next) -> Response {
    let (parts, body) = req.into_parts();
    match read_limited_body(body).await {
        Ok(bytes) => {
            next.run(Request::from_parts(parts, Body::from(bytes)))
                .await
        }
        Err(err) => IntoResponse::into_response(err),
    }
}

/// Core body-reader used by [`clamp_body_size`]. Extracted so it can be
/// exercised by unit tests without constructing an opaque [`Next`].
async fn read_limited_body(body: Body) -> Result<Bytes, AppError> {
    // `to_bytes` with a `limit + 1` cap distinguishes "equal to max" (fine)
    // from "exceeds max" (413). The extra byte is intentional.
    let bytes = to_bytes(body, MAX_REPORT_BYTES + 1).await.map_err(|_| {
        AppError::PayloadTooLarge(format!("CSP report exceeds {MAX_REPORT_BYTES} bytes"))
    })?;

    if bytes.len() > MAX_REPORT_BYTES {
        return Err(AppError::PayloadTooLarge(format!(
            "CSP report exceeds {MAX_REPORT_BYTES} bytes"
        )));
    }

    Ok(bytes)
}

/// POST /api/csp-report — accept a browser violation report, log it, and
/// return 204 No Content. Never exposes internal state or reflects the body.
#[utoipa::path(
    post,
    path = "/api/csp-report",
    tag = "security",
    request_body(
        content = String,
        description = "CSP violation report (application/csp-report or application/reports+json)"
    ),
    responses(
        (status = 204, description = "Report accepted"),
        (status = 400, description = "Malformed report body"),
        (status = 413, description = "Report body exceeds 8KB"),
        (status = 429, description = "Rate-limited")
    )
)]
pub(crate) async fn csp_report(headers: HeaderMap, body: Bytes) -> AppResult<StatusCode> {
    // Determine content-type; default to CSP Level 2 if the header is missing,
    // since some browsers/extensions omit it. The payload shape itself is the
    // source of truth (we try both and log on fallthrough).
    let content_type = headers
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("application/csp-report")
        .to_ascii_lowercase();

    if body.is_empty() {
        return Err(AppError::BadRequest("empty CSP report body".into()));
    }

    // Per the module doc, pick the parser by content-type prefix so
    // `application/csp-report; charset=utf-8` still matches.
    if content_type.starts_with("application/reports+json") {
        let entries: Vec<ReportsApiEntry> = serde_json::from_slice(&body)
            .map_err(|e| AppError::BadRequest(format!("invalid reports+json body: {e}")))?;
        for entry in entries {
            if let Some(report) = entry.body {
                log_violation(&report, entry.kind.as_deref().unwrap_or("csp-violation"));
            }
        }
        return Ok(StatusCode::NO_CONTENT);
    }

    // Fall through to Level 2 (`application/csp-report`).
    let envelope: CspReportEnvelope = serde_json::from_slice(&body)
        .map_err(|e| AppError::BadRequest(format!("invalid csp-report body: {e}")))?;
    log_violation(&envelope.csp_report, "csp-report");

    Ok(StatusCode::NO_CONTENT)
}

fn log_violation(r: &CspReportBody, kind: &str) {
    tracing::warn!(
        kind = kind,
        document_uri = r.document_uri.as_deref().unwrap_or(""),
        violated_directive = r.violated_directive.as_deref().unwrap_or(""),
        blocked_uri = r.blocked_uri.as_deref().unwrap_or(""),
        source_file = r.source_file.as_deref().unwrap_or(""),
        line_number = r.line_number.unwrap_or(0),
        column_number = r.column_number.unwrap_or(0),
        disposition = r.disposition.as_deref().unwrap_or(""),
        referrer = r.referrer.as_deref().unwrap_or(""),
        status_code = r.status_code.unwrap_or(0),
        script_sample = r.script_sample.as_deref().unwrap_or(""),
        "CSP violation reported",
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::HeaderValue;

    fn headers_with_ct(ct: &str) -> HeaderMap {
        let mut h = HeaderMap::new();
        h.insert(header::CONTENT_TYPE, HeaderValue::from_str(ct).unwrap());
        h
    }

    #[tokio::test]
    async fn accepts_level_2_envelope() {
        let body = Bytes::from_static(
            br#"{"csp-report":{"document-uri":"https://app.example/page","violated-directive":"script-src 'self'","blocked-uri":"https://evil.example/x.js"}}"#
        );
        let out = csp_report(headers_with_ct("application/csp-report"), body)
            .await
            .expect("valid report");
        assert_eq!(out, StatusCode::NO_CONTENT);
    }

    #[tokio::test]
    async fn accepts_reports_api_entries() {
        let body = Bytes::from_static(
            br#"[{"type":"csp-violation","body":{"documentURL":"https://app.example/page","effectiveDirective":"script-src-elem","blockedURL":"https://evil.example/y.js"}}]"#
        );
        let out = csp_report(headers_with_ct("application/reports+json"), body)
            .await
            .expect("valid reports+json");
        assert_eq!(out, StatusCode::NO_CONTENT);
    }

    #[tokio::test]
    async fn reports_api_with_charset_suffix_accepted() {
        // Real browsers send `application/reports+json; charset=UTF-8`.
        let body = Bytes::from_static(
            br#"[{"type":"csp-violation","body":{"effectiveDirective":"img-src"}}]"#,
        );
        let out = csp_report(
            headers_with_ct("application/reports+json; charset=utf-8"),
            body,
        )
        .await
        .expect("valid reports+json with charset");
        assert_eq!(out, StatusCode::NO_CONTENT);
    }

    #[tokio::test]
    async fn rejects_empty_body_with_400() {
        let err = csp_report(headers_with_ct("application/csp-report"), Bytes::new())
            .await
            .expect_err("empty body");
        match err {
            AppError::BadRequest(_) => {}
            other => panic!("expected BadRequest, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn rejects_malformed_body_with_400() {
        let body = Bytes::from_static(br#"{not-json"#);
        let err = csp_report(headers_with_ct("application/csp-report"), body)
            .await
            .expect_err("invalid json");
        match err {
            AppError::BadRequest(_) => {}
            other => panic!("expected BadRequest, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn clamp_rejects_oversized_body_with_413() {
        let oversized = vec![b'a'; MAX_REPORT_BYTES + 1];
        let err = read_limited_body(Body::from(oversized))
            .await
            .expect_err("should reject oversized body");
        match err {
            AppError::PayloadTooLarge(_) => {}
            other => panic!("expected PayloadTooLarge, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn clamp_accepts_body_at_limit() {
        let at_limit = vec![b'a'; MAX_REPORT_BYTES];
        let bytes = read_limited_body(Body::from(at_limit))
            .await
            .expect("should accept body at the limit");
        assert_eq!(bytes.len(), MAX_REPORT_BYTES);
    }
}
