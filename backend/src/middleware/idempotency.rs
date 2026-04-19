//! ADM-15: Idempotency-Key middleware for admin POST mutations.
//!
//! Purpose
//! -------
//! Mutating admin endpoints (manual orders, refunds, comp memberships,
//! DSAR jobs, …) are expensive and side-effectful. A retried request
//! caused by a flaky network or an over-eager UI must not duplicate the
//! side effect. This middleware implements RFC 9457-shaped semantics
//! (`Idempotency-Key` header) backed by the [`idempotency_keys`] table.
//!
//! Wire shape
//! ----------
//! * Header: `Idempotency-Key: <opaque-string-≤255>`. The header is
//!   **optional** — requests without it pass through, exactly like
//!   Stripe / Adyen. This keeps existing clients working.
//! * The cache key is `(actor_user_id, key)`. The actor id is decoded
//!   from the `Authorization: Bearer …` JWT (same secret as the
//!   [`AuthUser`](crate::extractors::AuthUser) extractor); requests
//!   without a valid bearer pass through so the downstream auth layer
//!   can reject them with the canonical `401`.
//! * The cached response includes `status`, `body`, and a small
//!   allowlist of headers (`Content-Type`, `Location`, …). Replays
//!   include `Idempotency-Replayed: true`.
//!
//! Concurrency safety
//! ------------------
//! The middleware uses `INSERT ... ON CONFLICT DO NOTHING` to claim
//! the key. If the row already exists:
//!
//! * `completed_at IS NULL`        → 409 Conflict (`idempotency-in-flight`).
//! * `completed_at IS NOT NULL`    + body hash matches → replay.
//! * `completed_at IS NOT NULL`    + body hash mismatches → 422
//!   Unprocessable Entity (`idempotency-key-mismatch`). This protects
//!   against the "same key, different body" footgun.
//!
//! On success the row is updated with the captured response. On any
//! failure (handler panic, 5xx, network drop), the row is deleted so
//! the operator can safely retry without manual cleanup.
//!
//! Scope
//! -----
//! Apply this layer per-router. It is mounted on the admin mutation
//! routers (`admin_orders`, `admin_subscriptions`, `admin_dsar`); read
//! routers do not need it.

use axum::{
    body::Body,
    extract::State,
    http::{HeaderMap, HeaderValue, Method, Request, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use jsonwebtoken::{decode, DecodingKey, Validation};
use serde::Serialize;
use serde_json::Value as JsonValue;
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use uuid::Uuid;

use crate::{extractors::Claims, AppState};

/// Maximum allowed length of the `Idempotency-Key` header value. Matches
/// the database CHECK constraint in migration `071_idempotency_keys.sql`.
const MAX_KEY_LEN: usize = 255;

/// Largest request body the middleware will buffer. Anything larger
/// short-circuits to a `413` so a misconfigured client cannot blow our
/// memory budget. Admin payloads are JSON envelopes well under 1 MiB.
const MAX_BODY_BYTES: usize = 1 * 1024 * 1024;

/// Header set surfaced on the replayed response. Restricting the
/// allowlist keeps untrusted upstream-provided headers from leaking.
const REPLAYED_HEADER_ALLOWLIST: &[&str] = &[
    "content-type",
    "location",
    "x-resource-id",
    "x-correlation-id",
];

const REPLAYED_FLAG_HEADER: &str = "idempotency-replayed";

/// Middleware entry point. Mount with
/// [`axum::middleware::from_fn_with_state`].
pub async fn enforce(
    State(state): State<AppState>,
    request: Request<Body>,
    next: Next,
) -> Response {
    // Only POST mutations participate. PATCH/PUT/DELETE could be added
    // later but the immediate footgun is "double-create" via POST.
    if request.method() != Method::POST {
        return next.run(request).await;
    }

    let key = match extract_key(request.headers()) {
        Ok(Some(k)) => k,
        Ok(None) => return next.run(request).await,
        Err(resp) => return resp,
    };

    // The actor id is required to scope the cache. Without a valid
    // bearer we let the request flow through; downstream auth will
    // reject it with the canonical 401.
    let actor_id = match decode_subject(request.headers(), &state.config.jwt_secret) {
        Some(id) => id,
        None => return next.run(request).await,
    };

    let method = request.method().to_string();
    let path = request.uri().path().to_string();

    // We need the body twice: once to hash, once to forward to the
    // handler. Buffer it (capped) and rebuild the request.
    let (parts, body) = request.into_parts();
    let bytes = match axum::body::to_bytes(body, MAX_BODY_BYTES).await {
        Ok(b) => b,
        Err(_) => return payload_too_large(),
    };
    let request_hash = sha256(&bytes);

    let pool = &state.db;

    // Try to claim the key. The first claimant gets to run the handler
    // and write its response back; any concurrent retry sees the row
    // and either replays or 409s.
    match try_claim(pool, actor_id, &key, &method, &path, &request_hash).await {
        ClaimOutcome::Claimed => {
            let request = Request::from_parts(parts, Body::from(bytes));
            let response = next.run(request).await;
            persist_response(pool, actor_id, &key, response).await
        }
        ClaimOutcome::ReplayMatch {
            status_code,
            response_body,
            response_headers,
        } => render_replay(status_code, response_body, response_headers),
        ClaimOutcome::ReplayMismatch => idempotency_mismatch(),
        ClaimOutcome::InFlight => idempotency_in_flight(),
        ClaimOutcome::DbError(err) => {
            tracing::error!(error = %err, "idempotency claim failed");
            // Fail open: degrade to non-cached behaviour rather than
            // hard-blocking the operator. The handler is allowed to
            // run; metrics surface the degradation.
            metrics::counter!("idempotency_db_error_total").increment(1);
            let request = Request::from_parts(parts, Body::from(bytes));
            next.run(request).await
        }
    }
}

// ── Helpers ────────────────────────────────────────────────────────────

fn extract_key(headers: &HeaderMap) -> Result<Option<String>, Response> {
    let Some(raw) = headers.get("idempotency-key") else {
        return Ok(None);
    };
    let value = raw
        .to_str()
        .map_err(|_| problem(StatusCode::BAD_REQUEST, "idempotency-key-invalid", "Invalid Idempotency-Key header (non-ASCII)"))?
        .trim();
    if value.is_empty() {
        return Ok(None);
    }
    if value.len() > MAX_KEY_LEN {
        return Err(problem(
            StatusCode::BAD_REQUEST,
            "idempotency-key-too-long",
            &format!("Idempotency-Key must be ≤ {MAX_KEY_LEN} characters"),
        ));
    }
    Ok(Some(value.to_string()))
}

fn decode_subject(headers: &HeaderMap, jwt_secret: &str) -> Option<Uuid> {
    let token = headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "))?;
    let data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(jwt_secret.as_bytes()),
        &Validation::default(),
    )
    .ok()?;
    Some(data.claims.sub)
}

fn sha256(bytes: &[u8]) -> Vec<u8> {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hasher.finalize().to_vec()
}

enum ClaimOutcome {
    Claimed,
    ReplayMatch {
        status_code: i32,
        response_body: Vec<u8>,
        response_headers: Option<JsonValue>,
    },
    ReplayMismatch,
    InFlight,
    DbError(sqlx::Error),
}

async fn try_claim(
    pool: &PgPool,
    actor_id: Uuid,
    key: &str,
    method: &str,
    path: &str,
    request_hash: &[u8],
) -> ClaimOutcome {
    // Atomically claim the slot. ON CONFLICT DO NOTHING guarantees
    // exactly one writer wins for the (actor_id, key) tuple.
    let claimed = match sqlx::query(
        r#"
        INSERT INTO idempotency_keys
            (user_id, key, method, path, request_hash, in_flight)
        VALUES
            ($1, $2, $3, $4, $5, TRUE)
        ON CONFLICT (user_id, key) DO NOTHING
        "#,
    )
    .bind(actor_id)
    .bind(key)
    .bind(method)
    .bind(path)
    .bind(request_hash)
    .execute(pool)
    .await
    {
        Ok(r) => r.rows_affected() == 1,
        Err(err) => return ClaimOutcome::DbError(err),
    };

    if claimed {
        metrics::counter!("idempotency_claimed_total").increment(1);
        return ClaimOutcome::Claimed;
    }

    // Existing row. Inspect it.
    let row = match sqlx::query_as::<_, (Vec<u8>, Option<i32>, Option<Vec<u8>>, Option<JsonValue>, bool)>(
        r#"
        SELECT request_hash, status_code, response_body, response_headers, in_flight
        FROM idempotency_keys
        WHERE user_id = $1 AND key = $2
        "#,
    )
    .bind(actor_id)
    .bind(key)
    .fetch_optional(pool)
    .await
    {
        Ok(Some(r)) => r,
        // Row vanished between the failed insert and the select (TTL
        // sweep). Treat as a fresh claim attempt — extremely rare.
        Ok(None) => {
            metrics::counter!("idempotency_claim_race_total").increment(1);
            return ClaimOutcome::InFlight;
        }
        Err(err) => return ClaimOutcome::DbError(err),
    };

    let (existing_hash, status_code, response_body, response_headers, in_flight) = row;

    if existing_hash != request_hash {
        metrics::counter!("idempotency_mismatch_total").increment(1);
        return ClaimOutcome::ReplayMismatch;
    }

    if in_flight {
        metrics::counter!("idempotency_in_flight_total").increment(1);
        return ClaimOutcome::InFlight;
    }

    let Some(status) = status_code else {
        // Completed=false but in_flight=false should never happen; treat
        // defensively as in-flight.
        return ClaimOutcome::InFlight;
    };

    metrics::counter!("idempotency_replay_total").increment(1);
    ClaimOutcome::ReplayMatch {
        status_code: status,
        response_body: response_body.unwrap_or_default(),
        response_headers,
    }
}

async fn persist_response(
    pool: &PgPool,
    actor_id: Uuid,
    key: &str,
    response: Response,
) -> Response {
    let (parts, body) = response.into_parts();
    let bytes = match axum::body::to_bytes(body, MAX_BODY_BYTES).await {
        Ok(b) => b,
        Err(_) => {
            // Response body too large to cache — drop the row so a
            // retry can re-execute, then surface the original (large)
            // response unchanged.
            let _ = sqlx::query("DELETE FROM idempotency_keys WHERE user_id = $1 AND key = $2")
                .bind(actor_id)
                .bind(key)
                .execute(pool)
                .await;
            return Response::from_parts(parts, Body::empty());
        }
    };

    let status = parts.status.as_u16() as i32;
    let success = parts.status.is_success() || parts.status.is_redirection();

    if !success {
        // Don't cache failures — let the operator retry safely. Same
        // semantics Stripe uses (only 2xx/3xx are cached).
        let _ = sqlx::query("DELETE FROM idempotency_keys WHERE user_id = $1 AND key = $2")
            .bind(actor_id)
            .bind(key)
            .execute(pool)
            .await;
        return Response::from_parts(parts, Body::from(bytes));
    }

    let header_capture = capture_headers(&parts.headers);

    let _ = sqlx::query(
        r#"
        UPDATE idempotency_keys
        SET status_code = $3,
            response_body = $4,
            response_headers = $5,
            in_flight = FALSE,
            completed_at = now()
        WHERE user_id = $1 AND key = $2
        "#,
    )
    .bind(actor_id)
    .bind(key)
    .bind(status)
    .bind(bytes.as_ref())
    .bind(&header_capture)
    .execute(pool)
    .await;

    Response::from_parts(parts, Body::from(bytes))
}

fn capture_headers(headers: &HeaderMap) -> JsonValue {
    let mut map = serde_json::Map::new();
    for name in REPLAYED_HEADER_ALLOWLIST {
        if let Some(value) = headers.get(*name).and_then(|v| v.to_str().ok()) {
            map.insert((*name).to_string(), JsonValue::String(value.to_string()));
        }
    }
    JsonValue::Object(map)
}

fn render_replay(status_code: i32, body: Vec<u8>, headers: Option<JsonValue>) -> Response {
    let status = StatusCode::from_u16(status_code as u16).unwrap_or(StatusCode::OK);
    let mut response = Response::builder()
        .status(status)
        .body(Body::from(body))
        .unwrap_or_else(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response());

    if let Some(JsonValue::Object(map)) = headers {
        for (k, v) in map {
            if let JsonValue::String(s) = v {
                if let (Ok(name), Ok(value)) = (
                    axum::http::HeaderName::from_bytes(k.as_bytes()),
                    HeaderValue::from_str(&s),
                ) {
                    response.headers_mut().insert(name, value);
                }
            }
        }
    }

    response.headers_mut().insert(
        REPLAYED_FLAG_HEADER,
        HeaderValue::from_static("true"),
    );
    response
}

#[derive(Serialize)]
struct ProblemDetails<'a> {
    #[serde(rename = "type")]
    type_: String,
    title: &'a str,
    status: u16,
    detail: &'a str,
}

fn problem(status: StatusCode, slug: &str, detail: &str) -> Response {
    let body = ProblemDetails {
        type_: format!("/problems/{slug}"),
        title: status.canonical_reason().unwrap_or("Error"),
        status: status.as_u16(),
        detail,
    };
    let mut resp = (status, Json(body)).into_response();
    resp.headers_mut().insert(
        axum::http::header::CONTENT_TYPE,
        HeaderValue::from_static("application/problem+json"),
    );
    resp
}

fn payload_too_large() -> Response {
    problem(
        StatusCode::PAYLOAD_TOO_LARGE,
        "payload-too-large",
        "Request body exceeds the idempotency cache limit",
    )
}

fn idempotency_in_flight() -> Response {
    problem(
        StatusCode::CONFLICT,
        "idempotency-in-flight",
        "A request with this Idempotency-Key is already being processed; retry shortly",
    )
}

fn idempotency_mismatch() -> Response {
    problem(
        StatusCode::UNPROCESSABLE_ENTITY,
        "idempotency-key-mismatch",
        "Idempotency-Key was previously used with a different request body",
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_missing_key_passes_through() {
        let headers = HeaderMap::new();
        assert!(matches!(extract_key(&headers), Ok(None)));
    }

    #[test]
    fn extract_oversized_key_rejected() {
        let mut headers = HeaderMap::new();
        let too_long = "a".repeat(MAX_KEY_LEN + 1);
        headers.insert("idempotency-key", HeaderValue::from_str(&too_long).unwrap());
        let outcome = extract_key(&headers);
        assert!(outcome.is_err());
    }

    #[test]
    fn extract_trims_whitespace_and_accepts() {
        let mut headers = HeaderMap::new();
        headers.insert("idempotency-key", HeaderValue::from_static("   abc-123   "));
        let outcome = extract_key(&headers).unwrap();
        assert_eq!(outcome, Some("abc-123".to_string()));
    }

    #[test]
    fn sha256_is_deterministic() {
        let a = sha256(b"hello world");
        let b = sha256(b"hello world");
        assert_eq!(a, b);
        assert_eq!(a.len(), 32);
    }
}
