//! ADM-07: stamp `X-Impersonation-*` response headers when the
//! incoming request carries a valid impersonation token.
//!
//! How it integrates with the rest of the stack
//! --------------------------------------------
//! * The [`crate::extractors::AuthUser`] extractor validates the
//!   `imp_session` claim against the `impersonation_sessions` row on
//!   every authenticated request, and on success inserts an
//!   [`crate::extractors::ImpersonationContext`] into the request
//!   extensions.
//! * This middleware re-reads that extension after the inner service
//!   runs and stamps the corresponding headers on the response.
//! * Requests that never reach an authenticated extractor (public
//!   pages, 401 rejections before the extractor inserted anything)
//!   simply do not carry the extension — the middleware is a no-op for
//!   them.
//!
//! Why a separate middleware instead of adding headers from each
//! handler: handlers cannot mutate response headers without taking
//! `Response` ownership, and we want the banner contract to be uniform
//! across **every** SPA-facing route, not just admin ones. A wrapping
//! middleware is the cheapest way to keep that invariant centralised.
//!
//! Header contract (consumed by the SvelteKit `+layout.svelte` banner):
//!
//! * `X-Impersonation-Active: true`
//! * `X-Impersonator-Id: <uuid>` — the real admin
//! * `X-Impersonator-Role: <role>` — `admin` (other roles refused at
//!   mint time)
//! * `X-Impersonation-Session: <uuid>` — row id, same value carried in
//!   the JWT `imp_session` claim
//! * `X-Impersonation-Target: <uuid>` — the impersonated user

use axum::{extract::Request, http::HeaderValue, middleware::Next, response::Response};

use crate::extractors::ImpersonationContext;

const HEADER_ACTIVE: &str = "X-Impersonation-Active";
const HEADER_ACTOR_ID: &str = "X-Impersonator-Id";
const HEADER_ACTOR_ROLE: &str = "X-Impersonator-Role";
const HEADER_SESSION: &str = "X-Impersonation-Session";
const HEADER_TARGET: &str = "X-Impersonation-Target";

/// Axum-compatible middleware fn. Mount globally via
/// `axum::middleware::from_fn(stamp)` so every response is normalised.
pub async fn stamp(request: Request, next: Next) -> Response {
    // Take a clone of the (Option<>) extension before consuming the
    // request — the extension was inserted by the AuthUser extractor
    // when (and only when) the JWT carried a valid `imp_session`.
    let ctx = request.extensions().get::<ImpersonationContext>().cloned();

    let mut response = next.run(request).await;

    let Some(ctx) = ctx else {
        return response;
    };

    let headers = response.headers_mut();
    insert_static(headers, HEADER_ACTIVE, "true");
    insert_uuid(headers, HEADER_ACTOR_ID, ctx.actor_user_id);
    insert_static(headers, HEADER_ACTOR_ROLE, ctx.actor_role.as_str());
    insert_uuid(headers, HEADER_SESSION, ctx.session_id);
    insert_uuid(headers, HEADER_TARGET, ctx.target_user_id);

    response
}

fn insert_static(headers: &mut axum::http::HeaderMap, name: &'static str, value: &str) {
    if let Ok(v) = HeaderValue::from_str(value) {
        headers.insert(name, v);
    }
}

fn insert_uuid(headers: &mut axum::http::HeaderMap, name: &'static str, value: uuid::Uuid) {
    headers.insert(
        name,
        HeaderValue::from_str(&value.to_string()).expect("uuid is ASCII"),
    );
}
