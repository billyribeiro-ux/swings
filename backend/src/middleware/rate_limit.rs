//! FDN-08: rate-limit abstraction and per-route layers.
//!
//! Two backends are supported; the active choice is driven by `RATE_LIMIT_BACKEND`
//! (`inprocess` default, `postgres` for multi-instance deployments):
//!
//! * [`Backend::InProcess`] — wraps the `tower_governor` token-bucket implementation.
//!   IP-keyed (via [`tower_governor::key_extractor::SmartIpKeyExtractor`]); a single
//!   instance of the binary stores the quota in-memory. This is the current
//!   behavior and keeps zero external dependencies for single-instance deploys.
//!
//! * [`Backend::Postgres`] — uses the `rate_limit_buckets` table
//!   (migration `022_rate_limits.sql`) as a 1-second sliding-window counter.
//!   On each request, the handler `INSERT ... ON CONFLICT DO UPDATE` bumps the
//!   current second's bucket and sums the last `window_secs` of buckets for the
//!   key; when the sum exceeds the configured per-window quota the request is
//!   rejected with [`AppError::TooManyRequests`].
//!
//! **Trust note (IP extractor):** `SmartIpKeyExtractor` trusts
//! `X-Forwarded-For` / `Forwarded` values. Only wire this up behind a reverse
//! proxy (Railway, Vercel) that strips or overwrites client-supplied values —
//! the headers are spoofable otherwise.

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;

use axum::{
    body::Body,
    extract::{ConnectInfo, Request, State},
    http::HeaderMap,
    middleware::Next,
    response::{IntoResponse, Response},
};
use chrono::{DateTime, DurationRound, TimeDelta, Utc};
use governor::middleware::NoOpMiddleware;
use serde::Deserialize;
use sqlx::PgPool;
use tower_governor::{
    governor::GovernorConfigBuilder, key_extractor::SmartIpKeyExtractor, GovernorLayer,
};

use crate::{error::AppError, AppState};

pub type AuthGovernorLayer = GovernorLayer<SmartIpKeyExtractor, NoOpMiddleware, Body>;

/// Local boxed future type for the [`RateLimitBackend`] trait. Avoids taking a
/// direct dependency on `futures_core` (the type alias is identical in shape).
pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

// ── Policy: quota + keying strategy ──────────────────────────────────────

/// How a request is mapped to a quota bucket key.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyStrategy {
    /// Key off the best-effort client IP (proxy-forwarded or socket peer).
    Ip,
    /// Prefer the authenticated user id (Bearer JWT `sub`); fall back to IP.
    UserThenIp,
    /// Prefer a provider / source header (e.g. webhook `X-Webhook-Source`);
    /// fall back to IP.
    SourceThenIp { header: &'static str },
}

/// A per-route quota policy.
///
/// Quotas are expressed as "at most `max_requests` within a `window_secs`
/// rolling window". Both backends interpret the numbers identically at the
/// policy level so operators can flip the selector with no code change.
#[derive(Debug, Clone, Copy)]
pub struct Policy {
    /// Stable, human-readable name used as the bucket-key prefix in the
    /// Postgres backend (e.g. `"login"`, `"popup-submit"`).
    pub name: &'static str,
    /// Total requests permitted in `window_secs`.
    pub max_requests: u32,
    /// Length of the rolling window, in seconds.
    pub window_secs: u32,
    /// Keying strategy.
    pub key: KeyStrategy,
}

impl Policy {
    /// Period per permitted request (for configuring the governor token bucket).
    fn governor_period(self) -> Duration {
        // `window_secs / max_requests` seconds per token. Using `max(1, ...)`
        // so the `tower_governor` constraint `period > 0` holds even in the
        // degenerate `max_requests == window_secs` case.
        let secs = self.window_secs / self.max_requests.max(1);
        Duration::from_secs(secs.max(1) as u64)
    }
}

// ── Policy catalog (per §12 of docs/archive/AUDIT_PHASE3_PLAN.md) ────────

pub const LOGIN: Policy = Policy {
    name: "login",
    max_requests: 5,
    window_secs: 60,
    key: KeyStrategy::Ip,
};
pub const REGISTER: Policy = Policy {
    name: "register",
    max_requests: 10,
    window_secs: 3_600,
    key: KeyStrategy::Ip,
};
pub const FORGOT_PASSWORD: Policy = Policy {
    name: "forgot-password",
    max_requests: 3,
    window_secs: 3_600,
    key: KeyStrategy::Ip,
};
pub const ANALYTICS: Policy = Policy {
    name: "analytics",
    max_requests: 120,
    window_secs: 60,
    key: KeyStrategy::Ip,
};
pub const WEBHOOKS: Policy = Policy {
    name: "webhooks",
    max_requests: 500,
    window_secs: 60,
    key: KeyStrategy::SourceThenIp {
        header: "x-webhook-source",
    },
};
pub const POPUP_SUBMIT: Policy = Policy {
    name: "popup-submit",
    max_requests: 20,
    window_secs: 60,
    key: KeyStrategy::Ip,
};
pub const POPUP_EVENT: Policy = Policy {
    name: "popup-event",
    max_requests: 120,
    window_secs: 60,
    key: KeyStrategy::Ip,
};
pub const COUPON_APPLY: Policy = Policy {
    name: "coupon-apply",
    max_requests: 5,
    window_secs: 60,
    key: KeyStrategy::UserThenIp,
};
pub const CSP_REPORT: Policy = Policy {
    name: "csp-report",
    max_requests: 1_000,
    window_secs: 60,
    key: KeyStrategy::Ip,
};
pub const MEMBER: Policy = Policy {
    name: "member",
    max_requests: 120,
    window_secs: 60,
    key: KeyStrategy::UserThenIp,
};
// FORM-03: public form endpoints.
pub const FORM_SUBMIT: Policy = Policy {
    name: "form-submit",
    max_requests: 20,
    window_secs: 60,
    key: KeyStrategy::Ip,
};
pub const FORM_PARTIAL: Policy = Policy {
    name: "form-partial",
    max_requests: 60,
    window_secs: 60,
    key: KeyStrategy::Ip,
};
// CONSENT-03: public consent record + DSAR endpoints.
pub const CONSENT_RECORD: Policy = Policy {
    name: "consent-record",
    max_requests: 30,
    window_secs: 60,
    key: KeyStrategy::Ip,
};
pub const DSAR_SUBMIT: Policy = Policy {
    name: "dsar-submit",
    max_requests: 5,
    window_secs: 3600,
    key: KeyStrategy::Ip,
};

/// ADM-18: per-actor token-bucket rate-limit on admin write endpoints
/// (`POST` / `PUT` / `PATCH` / `DELETE`). Sized so a sustained `4 rps`
/// burst from one operator (~240/min) is allowed — comfortably above
/// any plausible UI-driven workflow but well below the rate at which
/// a stolen credential could exfiltrate or destroy data.
///
/// Keyed on the JWT `sub` so it survives the operator changing IPs
/// (mobile, VPN); the same actor cannot circumvent it by hopping IPs.
/// Falls back to the calling IP when no Bearer token is present (the
/// admin auth extractor already rejects unauthenticated mutations, so
/// this branch is a defence-in-depth hedge for misrouted requests).
pub const ADMIN_MUTATION: Policy = Policy {
    name: "admin-mutation",
    max_requests: 240,
    window_secs: 60,
    key: KeyStrategy::UserThenIp,
};

// ── Backend abstraction ──────────────────────────────────────────────────

/// Distributed-quota backend trait. Current implementers:
/// [`InProcessBackend`] (default) and [`PostgresBackend`] (opt-in).
pub trait RateLimitBackend: Send + Sync + 'static {
    /// Decide whether a request (already keyed/policed by the caller) is
    /// allowed; returns `Err(AppError::TooManyRequests)` on exhaustion.
    ///
    /// Only used by the *non-governor* path (the Postgres backend). The
    /// in-process backend goes through a dedicated tower layer built from
    /// [`governor_layer_for`] and does not call this method.
    fn check(&self, policy: Policy, key: &str) -> BoxFuture<'_, Result<(), AppError>>;
}

/// Enum wrapper so the [`AppState`] can carry whichever backend is configured
/// without boxing trait objects across the axum state type.
#[derive(Clone)]
pub enum Backend {
    InProcess(Arc<InProcessBackend>),
    Postgres(Arc<PostgresBackend>),
}

impl Backend {
    /// Resolve the active backend from env (`RATE_LIMIT_BACKEND=inprocess|postgres`).
    pub fn from_env(pool: PgPool) -> Self {
        let raw = std::env::var("RATE_LIMIT_BACKEND")
            .unwrap_or_else(|_| "inprocess".to_string())
            .to_ascii_lowercase();
        match raw.as_str() {
            "postgres" | "pg" => {
                tracing::info!("rate_limit backend: postgres");
                Backend::Postgres(Arc::new(PostgresBackend::new(pool)))
            }
            _ => {
                tracing::info!("rate_limit backend: inprocess");
                Backend::InProcess(Arc::new(InProcessBackend))
            }
        }
    }

    /// `true` if the backend relies on the Postgres table. Used by route
    /// nesting code to decide whether to wrap a route with the middleware
    /// that performs the SQL check.
    pub fn is_postgres(&self) -> bool {
        matches!(self, Backend::Postgres(_))
    }
}

/// In-process backend. No-op on the trait; all enforcement happens in the
/// governor tower layer (see [`governor_layer_for`]).
#[derive(Default)]
pub struct InProcessBackend;

impl RateLimitBackend for InProcessBackend {
    fn check(&self, _policy: Policy, _key: &str) -> BoxFuture<'_, Result<(), AppError>> {
        Box::pin(async { Ok(()) })
    }
}

/// Postgres-backed sliding-window backend. Writes one row per `(key, second)`
/// and sums the last `window_secs` seconds to determine the effective rate.
pub struct PostgresBackend {
    pool: PgPool,
}

impl PostgresBackend {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Core check: increment the bucket for the current second, then sum the
    /// last `window_secs` buckets. Returns `true` when the request is allowed.
    async fn increment_and_count(
        &self,
        policy: Policy,
        key: &str,
        now: DateTime<Utc>,
    ) -> Result<bool, AppError> {
        let window_start = now
            .duration_trunc(TimeDelta::seconds(1))
            .map_err(|e| AppError::Internal(anyhow::anyhow!("duration_trunc: {e}")))?;
        let window_begin = window_start - TimeDelta::seconds(policy.window_secs as i64);

        // Atomic increment-or-insert. Uses the runtime (non-macro) sqlx API to
        // avoid the compile-time `DATABASE_URL` requirement; the rest of the
        // codebase follows the same convention.
        sqlx::query(
            r#"
            INSERT INTO rate_limit_buckets (key, window_start, count)
            VALUES ($1, $2, 1)
            ON CONFLICT (key, window_start)
            DO UPDATE SET count = rate_limit_buckets.count + 1
            "#,
        )
        .bind(key)
        .bind(window_start)
        .execute(&self.pool)
        .await?;

        let total: Option<i64> = sqlx::query_scalar(
            r#"
            SELECT COALESCE(SUM(count), 0)::bigint
              FROM rate_limit_buckets
             WHERE key = $1
               AND window_start > $2
            "#,
        )
        .bind(key)
        .bind(window_begin)
        .fetch_one(&self.pool)
        .await?;

        let sum = total.unwrap_or(0) as u64;
        Ok(sum <= policy.max_requests as u64)
    }
}

impl RateLimitBackend for PostgresBackend {
    fn check(&self, policy: Policy, key: &str) -> BoxFuture<'_, Result<(), AppError>> {
        let key = key.to_owned();
        Box::pin(async move {
            match self.increment_and_count(policy, &key, Utc::now()).await {
                Ok(true) => Ok(()),
                Ok(false) => Err(AppError::TooManyRequests),
                // Fail-open on DB errors so a Postgres hiccup doesn't take
                // down the whole surface; the error has already been logged
                // by the `Database` variant's tracing hook on conversion.
                Err(err) => {
                    tracing::warn!(error = %err, policy = policy.name, "rate_limit backend error; allowing request");
                    Ok(())
                }
            }
        })
    }
}

// ── Governor (in-process) layer factory ─────────────────────────────────

fn governor_layer_for(policy: Policy) -> AuthGovernorLayer {
    let mut b = GovernorConfigBuilder::default();
    b.period(policy.governor_period());
    b.burst_size(policy.max_requests);
    // SAFETY: `GovernorConfigBuilder::finish()` only fails when `period` or
    // `burst_size` is zero. Every `Policy` in this module asserts non-zero
    // values at construction; `governor_period()` floors at 1s; so this is
    // structurally infallible.
    GovernorLayer::new(
        b.key_extractor(SmartIpKeyExtractor)
            .finish()
            .expect("rate-limit quota is statically non-zero"),
    )
}

// Existing public surface — preserved for the handler modules that already
// import these names. New additions follow the same convention.

/// ~5 requests per minute per IP.
pub fn login_layer() -> AuthGovernorLayer {
    governor_layer_for(LOGIN)
}
/// 10 requests per hour per IP.
pub fn register_layer() -> AuthGovernorLayer {
    governor_layer_for(REGISTER)
}
/// 3 requests per hour per IP.
pub fn forgot_password_layer() -> AuthGovernorLayer {
    governor_layer_for(FORGOT_PASSWORD)
}
/// 120 requests per minute per IP (public analytics ingest).
pub fn analytics_ingest_layer() -> AuthGovernorLayer {
    governor_layer_for(ANALYTICS)
}
/// 500/min — keyed by IP for the in-process backend (source-header keying
/// only applies to the Postgres backend).
pub fn webhooks_layer() -> AuthGovernorLayer {
    governor_layer_for(WEBHOOKS)
}
/// 20/min/IP.
pub fn popup_submit_layer() -> AuthGovernorLayer {
    governor_layer_for(POPUP_SUBMIT)
}
/// 120/min/IP.
pub fn popup_event_layer() -> AuthGovernorLayer {
    governor_layer_for(POPUP_EVENT)
}
/// 5/min — keyed by IP for the in-process backend (user keying only applies
/// to the Postgres backend).
pub fn coupon_apply_layer() -> AuthGovernorLayer {
    governor_layer_for(COUPON_APPLY)
}
/// 1000/min/IP.
pub fn csp_report_layer() -> AuthGovernorLayer {
    governor_layer_for(CSP_REPORT)
}
/// 120/min — keyed by IP for the in-process backend (user keying only applies
/// to the Postgres backend).
pub fn member_layer() -> AuthGovernorLayer {
    governor_layer_for(MEMBER)
}
/// FORM-03: public `/api/forms/{slug}/submit` — 20/min/IP.
pub fn form_submit_layer() -> AuthGovernorLayer {
    governor_layer_for(FORM_SUBMIT)
}
/// FORM-03: public `/api/forms/{slug}/partial` — 60/min/IP.
pub fn form_partial_layer() -> AuthGovernorLayer {
    governor_layer_for(FORM_PARTIAL)
}
/// CONSENT-03: public `/api/consent/record` — 30/min/IP.
pub fn consent_record_layer() -> AuthGovernorLayer {
    governor_layer_for(CONSENT_RECORD)
}
/// CONSENT-03: public `/api/dsar` — 5/hour/IP.
pub fn dsar_submit_layer() -> AuthGovernorLayer {
    governor_layer_for(DSAR_SUBMIT)
}

// ── Postgres-backed middleware ──────────────────────────────────────────

/// Extract the best-effort client IP from standard proxy headers, falling back
/// to the TCP peer when headers are absent. Matches `SmartIpKeyExtractor`'s
/// precedence so in-process and Postgres backends key on the same value.
fn client_ip(headers: &HeaderMap, peer: Option<&std::net::SocketAddr>) -> String {
    if let Some(v) = headers.get("x-forwarded-for").and_then(|v| v.to_str().ok()) {
        if let Some(first) = v.split(',').next() {
            let t = first.trim();
            if !t.is_empty() {
                return t.to_owned();
            }
        }
    }
    if let Some(v) = headers.get("forwarded").and_then(|v| v.to_str().ok()) {
        for part in v.split(';') {
            if let Some(rest) = part.trim().strip_prefix("for=") {
                let t = rest.trim_matches(|c: char| c == '"' || c == '[' || c == ']');
                if !t.is_empty() {
                    return t.to_owned();
                }
            }
        }
    }
    if let Some(v) = headers.get("x-real-ip").and_then(|v| v.to_str().ok()) {
        let t = v.trim();
        if !t.is_empty() {
            return t.to_owned();
        }
    }
    peer.map(|s| s.ip().to_string())
        .unwrap_or_else(|| "unknown".to_string())
}

/// JWT Bearer `sub` — extracted without enforcing the token is valid-on-its-own
/// so we can still rate-limit flows with expired tokens. The caller's handler
/// will return 401/403 separately when it demands a fresh token.
#[derive(Deserialize)]
struct SubOnly {
    sub: String,
}

fn bearer_subject(headers: &HeaderMap) -> Option<String> {
    let h = headers
        .get(axum::http::header::AUTHORIZATION)?
        .to_str()
        .ok()?;
    let tok = h.strip_prefix("Bearer ")?;
    // JWT = base64url(header).base64url(payload).base64url(sig)
    let payload = tok.split('.').nth(1)?;
    let bytes = base64_decode_url_nopad(payload).ok()?;
    let parsed: SubOnly = serde_json::from_slice(&bytes).ok()?;
    Some(parsed.sub)
}

/// RFC 4648 base64url decoder without `+` / `/` / `=` handling; matches the
/// `URL_SAFE_NO_PAD` engine. Kept local so we don't pull a `base64` dep solely
/// for this inspection.
fn base64_decode_url_nopad(s: &str) -> Result<Vec<u8>, &'static str> {
    const TABLE: &[u8; 128] = {
        const fn build() -> [u8; 128] {
            let mut t = [0xFF_u8; 128];
            let mut i: u8 = 0;
            while i < 26 {
                t[(b'A' + i) as usize] = i;
                t[(b'a' + i) as usize] = 26 + i;
                i += 1;
            }
            let mut j: u8 = 0;
            while j < 10 {
                t[(b'0' + j) as usize] = 52 + j;
                j += 1;
            }
            t[b'-' as usize] = 62;
            t[b'_' as usize] = 63;
            t
        }
        &build()
    };

    let mut buf = 0_u32;
    let mut bits = 0_u32;
    let mut out = Vec::with_capacity(s.len() * 3 / 4);
    for b in s.bytes() {
        let c = *TABLE.get(b as usize).ok_or("non-ascii")?;
        if c == 0xFF {
            return Err("bad char");
        }
        buf = (buf << 6) | c as u32;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((buf >> bits) as u8);
            buf &= (1 << bits) - 1;
        }
    }
    Ok(out)
}

/// Build the canonical bucket key for a request against a policy.
fn bucket_key(policy: Policy, headers: &HeaderMap, peer: Option<&std::net::SocketAddr>) -> String {
    let suffix = match policy.key {
        KeyStrategy::Ip => client_ip(headers, peer),
        KeyStrategy::UserThenIp => bearer_subject(headers).unwrap_or_else(|| {
            let ip = client_ip(headers, peer);
            format!("ip:{ip}")
        }),
        KeyStrategy::SourceThenIp { header } => headers
            .get(header)
            .and_then(|v| v.to_str().ok())
            .map(|v| format!("src:{}", v.trim()))
            .filter(|v| v.len() > 4) // "src:" + non-empty
            .unwrap_or_else(|| {
                let ip = client_ip(headers, peer);
                format!("ip:{ip}")
            }),
    };
    format!("{}:{}", policy.name, suffix)
}

/// Axum middleware — Postgres-backed rate-limit check. Only wire this up when
/// `Backend::Postgres` is active; the in-process path uses the governor layer
/// directly on each route (see [`governor_layer_for`]).
pub async fn postgres_rate_limit(
    State(state): State<AppState>,
    req: Request,
    next: Next,
) -> Response {
    // The policy is carried in request extensions by [`with_policy`] so the
    // same middleware function can be reused across routes.
    let Some(policy) = req.extensions().get::<Policy>().copied() else {
        return next.run(req).await;
    };

    let backend = match &state.rate_limit {
        Backend::Postgres(pg) => pg.clone(),
        Backend::InProcess(_) => return next.run(req).await,
    };

    let peer = req.extensions().get::<ConnectInfo<std::net::SocketAddr>>();
    let peer_ref = peer.as_ref().map(|c| &c.0);
    let key = bucket_key(policy, req.headers(), peer_ref);

    match RateLimitBackend::check(&*backend, policy, &key).await {
        Ok(()) => next.run(req).await,
        Err(err) => IntoResponse::into_response(err),
    }
}

/// Small helper: attach a policy to the request via extensions so the
/// [`postgres_rate_limit`] middleware can find it.
pub async fn attach_policy(mut req: Request, next: Next, policy: Policy) -> Response {
    req.extensions_mut().insert(policy);
    next.run(req).await
}

// ── ADM-18: per-actor admin-mutation rate-limit ─────────────────────────
//
// Unlike the per-IP `tower_governor` layers above, admin mutations need
// to be metered per *operator* (Bearer JWT `sub`). We can't lean on
// `SmartIpKeyExtractor` for that, so the implementation forks per
// backend:
//
// * Postgres backend  → reuses [`bucket_key`] + [`PostgresBackend::check`]
//   so the quota is shared across replicas.
// * In-process backend → uses a process-local keyed `governor`
//   limiter. Single-instance deployments get correct enforcement; multi-
//   instance deployments should flip `RATE_LIMIT_BACKEND=postgres` for
//   coherent quotas (the in-process limit becomes per-replica).
use std::num::NonZeroU32;
use std::sync::OnceLock;

use governor::{clock::DefaultClock, state::keyed::DefaultKeyedStateStore, Quota, RateLimiter};

type ActorRateLimiter = RateLimiter<String, DefaultKeyedStateStore<String>, DefaultClock>;

fn admin_mutation_limiter() -> &'static Arc<ActorRateLimiter> {
    static LIMITER: OnceLock<Arc<ActorRateLimiter>> = OnceLock::new();
    LIMITER.get_or_init(|| {
        let max = NonZeroU32::new(ADMIN_MUTATION.max_requests)
            .expect("ADMIN_MUTATION.max_requests is non-zero");
        // Refill cadence: one token every `PERIOD_SECS` seconds.
        // Computed as `window / max` (truncated) and clamped to 1
        // so we never produce a zero-second period. With the
        // current constants (240 tokens / 60s) the integer
        // division rounds to 0 and the clamp picks 1 — yielding a
        // 1-token/sec steady-state refill on top of the 240-token
        // burst capacity. Done in a `const` block so clippy can
        // prove the value at compile time without complaining
        // about `.max(1)` on a constant-foldable expression.
        const PERIOD_SECS: u64 = {
            let raw = (ADMIN_MUTATION.window_secs / ADMIN_MUTATION.max_requests) as u64;
            if raw == 0 {
                1
            } else {
                raw
            }
        };
        let quota = Quota::with_period(Duration::from_secs(PERIOD_SECS))
            .expect("ADMIN_MUTATION period is non-zero")
            .allow_burst(max);
        Arc::new(RateLimiter::keyed(quota))
    })
}

/// Axum middleware: per-actor token-bucket rate-limit on admin
/// mutation endpoints. Wire it as
/// `axum::middleware::from_fn_with_state(state.clone(), admin_mutation_rate_limit)`
/// on each admin write router. Read-only `GET` / `HEAD` requests pass
/// through untouched so dashboards remain responsive even when the
/// operator's mutation quota is exhausted.
pub async fn admin_mutation_rate_limit(
    State(state): State<AppState>,
    req: Request,
    next: Next,
) -> Response {
    use axum::http::Method;
    if matches!(
        req.method(),
        &Method::GET | &Method::HEAD | &Method::OPTIONS
    ) {
        return next.run(req).await;
    }

    let peer = req.extensions().get::<ConnectInfo<std::net::SocketAddr>>();
    let peer_ref = peer.as_ref().map(|c| &c.0);
    let key = bucket_key(ADMIN_MUTATION, req.headers(), peer_ref);

    match &state.rate_limit {
        Backend::Postgres(pg) => {
            let backend = pg.clone();
            match RateLimitBackend::check(&*backend, ADMIN_MUTATION, &key).await {
                Ok(()) => next.run(req).await,
                Err(err) => {
                    metrics::counter!(
                        "admin_mutation_rate_limited_total",
                        "backend" => "postgres"
                    )
                    .increment(1);
                    tracing::warn!(actor_key = %key, "admin mutation blocked: rate limit");
                    IntoResponse::into_response(err)
                }
            }
        }
        Backend::InProcess(_) => {
            let limiter = admin_mutation_limiter();
            match limiter.check_key(&key) {
                Ok(_) => next.run(req).await,
                Err(_) => {
                    metrics::counter!(
                        "admin_mutation_rate_limited_total",
                        "backend" => "inprocess"
                    )
                    .increment(1);
                    tracing::warn!(actor_key = %key, "admin mutation blocked: rate limit");
                    IntoResponse::into_response(AppError::TooManyRequests)
                }
            }
        }
    }
}

/// Test-only handle to the in-memory keyed limiter so integration
/// tests can reset state between cases (the `OnceLock` is per-process
/// and bleeds across `#[tokio::test]` invocations otherwise).
#[doc(hidden)]
pub fn _admin_mutation_limiter_for_tests() -> &'static Arc<ActorRateLimiter> {
    admin_mutation_limiter()
}

// ── Unit tests ──────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn policy_governor_period_is_non_zero() {
        // Every static policy floors at a 1s period so governor doesn't panic.
        for p in [
            LOGIN,
            REGISTER,
            FORGOT_PASSWORD,
            ANALYTICS,
            WEBHOOKS,
            POPUP_SUBMIT,
            POPUP_EVENT,
            COUPON_APPLY,
            CSP_REPORT,
            MEMBER,
            FORM_SUBMIT,
            FORM_PARTIAL,
            CONSENT_RECORD,
            DSAR_SUBMIT,
        ] {
            assert!(p.governor_period() >= Duration::from_secs(1), "{}", p.name);
        }
    }

    #[test]
    fn client_ip_prefers_xff_then_forwarded_then_peer() {
        let mut h = HeaderMap::new();
        h.insert("x-forwarded-for", "1.2.3.4, 10.0.0.1".parse().unwrap());
        assert_eq!(client_ip(&h, None), "1.2.3.4");

        let mut h2 = HeaderMap::new();
        h2.insert("forwarded", "for=5.6.7.8;proto=https".parse().unwrap());
        assert_eq!(client_ip(&h2, None), "5.6.7.8");

        let h3 = HeaderMap::new();
        let peer: std::net::SocketAddr = "9.9.9.9:443".parse().unwrap();
        assert_eq!(client_ip(&h3, Some(&peer)), "9.9.9.9");
    }

    #[test]
    fn bearer_subject_reads_jwt_sub() {
        // Hand-crafted JWT: header {}, payload {"sub":"abc-123"}, sig omitted.
        // base64url of `{"sub":"abc-123"}` = `eyJzdWIiOiJhYmMtMTIzIn0`.
        let mut h = HeaderMap::new();
        h.insert(
            axum::http::header::AUTHORIZATION,
            "Bearer e30.eyJzdWIiOiJhYmMtMTIzIn0.xxx".parse().unwrap(),
        );
        assert_eq!(bearer_subject(&h).as_deref(), Some("abc-123"));
    }

    #[test]
    fn bucket_key_ip_strategy_uses_ip() {
        let mut h = HeaderMap::new();
        h.insert("x-forwarded-for", "1.2.3.4".parse().unwrap());
        assert_eq!(bucket_key(LOGIN, &h, None), "login:1.2.3.4");
    }

    #[test]
    fn bucket_key_source_strategy_reads_header() {
        let mut h = HeaderMap::new();
        h.insert("x-webhook-source", "stripe".parse().unwrap());
        assert_eq!(bucket_key(WEBHOOKS, &h, None), "webhooks:src:stripe");

        let h_empty = HeaderMap::new();
        let key = bucket_key(WEBHOOKS, &h_empty, None);
        assert!(key.starts_with("webhooks:ip:"));
    }

    #[test]
    fn bucket_key_user_strategy_prefers_bearer_sub() {
        let mut h = HeaderMap::new();
        h.insert(
            axum::http::header::AUTHORIZATION,
            "Bearer e30.eyJzdWIiOiJ1c2VyLXUuMSJ9.x".parse().unwrap(),
        );
        assert_eq!(bucket_key(MEMBER, &h, None), "member:user-u.1");
    }

    // Parallel-atomicity coverage for the Postgres backend lives in the
    // integration-test suite (`backend/tests/rate_limit_postgres.rs`),
    // where the support harness provisions a throwaway schema and real
    // migrations. It used to sit here behind `#[ignore]` and never ran;
    // moving it out removed the last ignored unit test from the suite
    // and turned a dormant assertion into a runtime one.
}
