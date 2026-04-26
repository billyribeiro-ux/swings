use std::convert::Infallible;
use std::net::IpAddr;

use axum::{
    extract::{ConnectInfo, FromRequestParts},
    http::{request::Parts, HeaderMap},
};
use jsonwebtoken::{decode, Algorithm, DecodingKey, Validation};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{authz::PolicyHandle, error::AppError, models::UserRole, AppState};

/// Issuer baked into every JWT we mint. Verified on decode — a token
/// minted by a different backend (staging, ops tooling) won't pass.
///
/// Intentionally static: JWTs are invalidated by rotating `JWT_SECRET`,
/// not by changing this string. If the public domain ever moves, bump
/// both the mint paths and the verify validation at the same time.
pub const JWT_ISSUER: &str = "precisionoptionsignals.com";

/// Audience binding for user-facing access tokens. Separates
/// SPA-bearer tokens from internal-service tokens (e.g. the Google
/// Sheets OAuth flow mints its own ephemeral JWT with a different
/// `aud`); we do NOT want the SPA to be able to present a service
/// token and vice versa.
pub const JWT_AUDIENCE: &str = "precisionoptionsignals.com/app";

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: Uuid,
    pub role: String,
    pub exp: usize,
    pub iat: usize,
    /// JWT `iss` claim — pinned to [`JWT_ISSUER`] at mint, validated on
    /// decode. Optional on deserialize so legacy tokens minted before
    /// this field was added still decode; the `validate_iss` flag in
    /// [`jwt_validation`] enforces equality when the claim is present.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub iss: Option<String>,
    /// JWT `aud` claim — pinned to [`JWT_AUDIENCE`] at mint, validated
    /// on decode. Same migration story as `iss`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub aud: Option<String>,
    /// ADM-07: real admin user id when this token was minted under
    /// impersonation. Absent for ordinary access tokens; the legacy
    /// claim shape stays valid because all three impersonation fields
    /// default to `None` on deserialise.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub imp_actor: Option<Uuid>,
    /// ADM-07: real admin's role string at mint time. Recorded in JWT
    /// for audit ergonomics; the authoritative copy lives in
    /// `impersonation_sessions.actor_role`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub imp_actor_role: Option<String>,
    /// ADM-07: row id in `impersonation_sessions`. Validated on every
    /// request (see [`AuthUser::from_request_parts`]) so flipping
    /// `revoked_at` immediately invalidates the live token.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub imp_session: Option<Uuid>,
}

/// ADM-07: per-request impersonation context. When present, the JWT
/// was minted by an admin acting on behalf of `target_user_id` (==
/// [`AuthUser::user_id`]). Inserted into request extensions by the
/// AuthUser extractor so the banner middleware (and any audit-aware
/// handler) can recover the real actor without re-decoding the token.
#[derive(Debug, Clone)]
pub struct ImpersonationContext {
    pub session_id: Uuid,
    pub actor_user_id: Uuid,
    pub actor_role: UserRole,
    pub target_user_id: Uuid,
}

pub struct AuthUser {
    pub user_id: Uuid,
    pub role: String,
    /// ADM-07: real admin id when this request is being made under an
    /// impersonation token. None for ordinary access tokens.
    pub impersonator_id: Option<Uuid>,
    pub impersonator_role: Option<String>,
    pub impersonation_session_id: Option<Uuid>,
}

impl AuthUser {
    /// Convenience: is this request running under an impersonation
    /// token? Cheaper to read than threading the three optional fields
    /// individually.
    #[must_use]
    pub fn is_impersonating(&self) -> bool {
        self.impersonation_session_id.is_some()
    }
}

/// SECURITY: explicit `Validation` with a pinned algorithm allow-list.
///
/// `Validation::default()` permits HS256 only today, but depending on the
/// upstream `jsonwebtoken` version the default may also accept tokens with
/// no algorithm (`alg: none`) or with a different HS variant. Pinning the
/// algorithm to the one we mint (HS256) eliminates that class of
/// confusion attacks regardless of crate version.
///
/// `leeway` is a small grace window for clock skew across
/// containers / fleets; the default is already 60s, we set it
/// explicitly so future crate version bumps do not silently widen it.
#[must_use]
pub fn jwt_validation() -> Validation {
    let mut v = Validation::new(Algorithm::HS256);
    v.leeway = 30;
    v.validate_exp = true;
    // jsonwebtoken 10 defaults to `validate_aud = true` with `aud = None`,
    // which rejects ANY token carrying an `aud` claim with `InvalidAudience`.
    // Mint always sets ours (see handlers/auth.rs::generate_tokens), so we
    // must pin the expected audience here for decode to succeed. Issuer is
    // pinned for symmetry; `verify_claim_binding` remains as defense in
    // depth (strict — both claims are required and must match).
    v.set_audience(&[JWT_AUDIENCE]);
    v.set_issuer(&[JWT_ISSUER]);
    v
}

/// BFF (Phase 1.3): pull the JWT out of either the `swings_access` httpOnly
/// cookie (preferred) or the legacy `Authorization: Bearer …` header
/// (fallback during the rollout).
///
/// Cookie wins when both are present so the browser-side migration is
/// monotonic — the moment `/api/auth/login` lands a fresh `Set-Cookie`, every
/// subsequent request resolves through the cookie even if the SPA happens to
/// keep attaching the old header during the same page load.
///
/// Returns `None` when neither carrier yields a non-empty token.
pub(crate) fn extract_access_token(headers: &HeaderMap) -> Option<String> {
    if let Some(cookie_header) = headers
        .get(axum::http::header::COOKIE)
        .and_then(|v| v.to_str().ok())
    {
        // RFC 6265: the request `Cookie` header is one or more
        // `name=value` pairs separated by `; `. We hand-parse rather than
        // pulling in the full `cookie` crate jar machinery here — the
        // header is a hot path on every authenticated request and we want
        // a zero-allocation pass when the target cookie is missing.
        for pair in cookie_header.split(';') {
            let pair = pair.trim();
            if let Some(value) = pair.strip_prefix("swings_access=") {
                if !value.is_empty() {
                    return Some(value.to_string());
                }
            }
        }
    }
    headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|h| h.strip_prefix("Bearer "))
        .map(str::to_owned)
}

/// Strict `iss` / `aud` check.
///
/// Returns `Ok(())` only when both claims are present and match the
/// expected values pinned in [`JWT_ISSUER`] / [`JWT_AUDIENCE`]. A missing
/// or mismatched claim collapses to [`AppError::Unauthorized`].
///
/// Note: [`jwt_validation`] also pins `iss` / `aud` at the
/// `jsonwebtoken` layer (which rejects tokens without the claim before we
/// ever reach this function). This helper is kept as defense in depth
/// against future `jwt_validation` regressions and as the single
/// authoritative check used by the optional / impersonation paths.
///
/// History: this function previously tolerated absent claims as a
/// transitional concession for legacy tokens minted before the iss/aud
/// rollout (commit d0f0eec, 2026-04-24). The rollout window has elapsed
/// — `JWT_EXPIRATION_HOURS` is 24h, all live sessions have rotated through
/// the new mint path, and the absent-claim branch has been promoted to an
/// error.
pub fn verify_claim_binding(claims: &Claims) -> Result<(), AppError> {
    match claims.iss.as_deref() {
        Some(iss) if iss == JWT_ISSUER => {}
        Some(iss) => {
            tracing::warn!(claim = %iss, "jwt issuer mismatch");
            return Err(AppError::Unauthorized);
        }
        None => {
            tracing::warn!("jwt missing iss claim");
            return Err(AppError::Unauthorized);
        }
    }
    match claims.aud.as_deref() {
        Some(aud) if aud == JWT_AUDIENCE => {}
        Some(aud) => {
            tracing::warn!(claim = %aud, "jwt audience mismatch");
            return Err(AppError::Unauthorized);
        }
        None => {
            tracing::warn!("jwt missing aud claim");
            return Err(AppError::Unauthorized);
        }
    }
    Ok(())
}

impl FromRequestParts<AppState> for AuthUser {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        // BFF rollout (Phase 1.3): the access token now lives in the
        // `swings_access` httpOnly cookie. We still accept the legacy
        // `Authorization: Bearer …` header so live sessions minted from the
        // pre-cookie SPA — and the integration-test harness, which mints a
        // bearer token directly without round-tripping `/login` — keep
        // working through the rollout. Phase B drops the bearer fallback.
        let token = extract_access_token(&parts.headers).ok_or(AppError::Unauthorized)?;

        let token_data = decode::<Claims>(
            &token,
            &DecodingKey::from_secret(state.config.jwt_secret.as_bytes()),
            &jwt_validation(),
        )
        .map_err(|_| AppError::Unauthorized)?;

        let claims = token_data.claims;

        // SECURITY: enforce issuer / audience binding. Tolerates legacy
        // tokens (no claim) while rejecting tokens with a wrong claim.
        verify_claim_binding(&claims)?;

        // ADM-07: server-side impersonation check. The JWT alone is not
        // enough to honour the request — we must consult the row so
        // revocations take immediate effect. A missing / revoked /
        // expired row collapses to 401, treating the bearer token as
        // never having existed.
        if let Some(session_id) = claims.imp_session {
            let session = crate::security::impersonation::lookup_active(&state.db, session_id)
                .await
                .map_err(|err| {
                    tracing::error!(error = %err, "impersonation session lookup failed");
                    AppError::Unauthorized
                })?
                .ok_or(AppError::Unauthorized)?;

            // Token / DB consistency: the `sub` claim must equal the
            // session's target. If they disagree, somebody crafted a
            // mismatched token; refuse it.
            if session.target_user_id != claims.sub {
                tracing::warn!(
                    session_id = %session_id,
                    "impersonation token sub mismatch with session target"
                );
                return Err(AppError::Unauthorized);
            }

            parts.extensions.insert(ImpersonationContext {
                session_id,
                actor_user_id: session.actor_user_id,
                actor_role: session.actor_role,
                target_user_id: session.target_user_id,
            });

            return Ok(AuthUser {
                user_id: claims.sub,
                role: claims.role,
                impersonator_id: Some(session.actor_user_id),
                impersonator_role: Some(session.actor_role.as_str().to_string()),
                impersonation_session_id: Some(session_id),
            });
        }

        Ok(AuthUser {
            user_id: claims.sub,
            role: claims.role,
            impersonator_id: None,
            impersonator_role: None,
            impersonation_session_id: None,
        })
    }
}

pub struct AdminUser {
    pub user_id: Uuid,
    /// Stringly-typed role preserved for FDN-07 transition. Once handlers
    /// switch to [`RoleUser`] + `Policy::require(...)` in Round 2b this field
    /// can be removed alongside the extractor.
    pub role: String,
}

impl AdminUser {
    /// Convenience wrapper: does this admin's role carry `perm` under the
    /// provided [`Policy`] snapshot? Kept as a thin helper so handlers can
    /// write `admin.has_permission(&state.policy, "admin.role.manage")`
    /// without re-parsing the role string themselves.
    ///
    /// Per FDN-07 design the extractor does not carry a [`Policy`] field
    /// directly — the cache lives on `AppState` and is resolved at handler
    /// time so admin-initiated reloads are observed without restarting the
    /// request pipeline.
    #[must_use]
    pub fn has_permission(&self, policy: &PolicyHandle, perm: &str) -> bool {
        UserRole::from_str_lower(&self.role).is_some_and(|role| policy.has(role, perm))
    }

    /// Strict permission gate. Returns [`AppError::Forbidden`] if the
    /// admin's role lacks `perm` under the current [`PolicyHandle`]
    /// snapshot, or [`AppError::Unauthorized`] if the role string is
    /// not a known [`UserRole`].
    ///
    /// Use this at the top of every privileged handler to enforce the
    /// FDN-07 matrix beyond the coarse "role == admin" check that the
    /// extractor itself runs.
    pub fn require(&self, policy: &PolicyHandle, perm: &str) -> Result<UserRole, AppError> {
        let role = UserRole::from_str_lower(&self.role).ok_or(AppError::Unauthorized)?;
        if policy.has(role, perm) {
            Ok(role)
        } else {
            Err(AppError::Forbidden)
        }
    }
}

impl FromRequestParts<AppState> for AdminUser {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let auth_user = AuthUser::from_request_parts(parts, state).await?;

        // ADM-07: an admin who is currently impersonating a non-admin
        // user MUST exit impersonation before touching admin endpoints.
        // Allowing this would silently re-elevate the impersonated
        // session and confuse audit attribution. We keep the rejection
        // narrow (Forbidden, not Unauthorized) so the SPA can route the
        // user to the "Exit impersonation" affordance instead of
        // logging out entirely.
        if auth_user.is_impersonating() {
            return Err(AppError::Forbidden);
        }

        // Strict: only the `admin` JWT role passes this gate. Support /
        // helpdesk users reach permission-gated handlers via the
        // `PrivilegedUser` extractor below, which checks the FDN-07
        // policy at handler entry rather than relying on a single
        // role string. Keeping AdminUser strict avoids silently
        // widening access on dozens of existing handlers that have
        // never been audited for support-role exposure.
        if auth_user.role != "admin" {
            return Err(AppError::Forbidden);
        }

        Ok(AdminUser {
            user_id: auth_user.user_id,
            role: auth_user.role,
        })
    }
}

/// Permission-gated extractor for handlers that should be reachable by any
/// role carrying `admin.dashboard.read` — i.e. `admin` always and `support`
/// per the FDN-07 seed in `021_rbac.sql`.
///
/// Handlers must still call [`AdminUser::require`] / [`Self::require`] for
/// per-action permission checks. This extractor only proves the caller
/// has *some* business being on `/api/admin/*`.
pub struct PrivilegedUser {
    pub user_id: Uuid,
    pub role: UserRole,
}

impl PrivilegedUser {
    /// Strict permission gate. Mirrors [`Policy::require`] but takes the
    /// already-typed [`UserRole`] from the extractor so handlers do not
    /// re-parse the JWT claim string.
    pub fn require(&self, policy: &PolicyHandle, perm: &str) -> Result<(), AppError> {
        if policy.has(self.role, perm) {
            Ok(())
        } else {
            Err(AppError::Forbidden)
        }
    }
}

impl FromRequestParts<AppState> for PrivilegedUser {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let auth_user = AuthUser::from_request_parts(parts, state).await?;
        // ADM-07: see AdminUser. Impersonated sessions cannot reach the
        // admin surface — the admin must exit first.
        if auth_user.is_impersonating() {
            return Err(AppError::Forbidden);
        }
        let role = UserRole::from_str_lower(&auth_user.role).ok_or(AppError::Unauthorized)?;
        if !state.policy.has(role, "admin.dashboard.read") {
            return Err(AppError::Forbidden);
        }
        Ok(PrivilegedUser {
            user_id: auth_user.user_id,
            role,
        })
    }
}

/// Like [`AuthUser`] but **infallible**: yields `Some` on a valid bearer/cookie
/// session, `None` on anything else (missing token, expired, malformed,
/// revoked impersonation, etc.). Use on endpoints that must succeed for
/// unauthenticated callers — notably `POST /api/auth/logout`, which has to
/// be safe to call with no session so the browser can always wipe its
/// cookies cleanly.
pub struct MaybeAuthUser(pub Option<AuthUser>);

impl FromRequestParts<AppState> for MaybeAuthUser {
    type Rejection = Infallible;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        // Delegate to the fallible extractor and demote any rejection to
        // `None`. We must not propagate the underlying `AppError` because
        // that would defeat the whole point of an infallible variant.
        let auth = AuthUser::from_request_parts(parts, state).await.ok();
        Ok(MaybeAuthUser(auth))
    }
}

/// Bearer JWT if present and valid; otherwise `user_id: None` (for optional auth on public endpoints).
pub struct OptionalAuthUser {
    pub user_id: Option<Uuid>,
}

impl FromRequestParts<AppState> for OptionalAuthUser {
    type Rejection = Infallible;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        // BFF: same dual-carrier resolution as `AuthUser`. Anonymous on
        // any failure (this extractor is infallible by contract — public
        // endpoints fall back to a guest view rather than 401).
        let user_id = extract_access_token(&parts.headers).and_then(|token| {
            let data = decode::<Claims>(
                &token,
                &DecodingKey::from_secret(state.config.jwt_secret.as_bytes()),
                &jwt_validation(),
            )
            .ok()?;
            // SECURITY: same iss/aud check as AuthUser; failure
            // downgrades the request to anonymous rather than 401
            // because this extractor is infallible by contract.
            verify_claim_binding(&data.claims).ok()?;
            Some(data.claims.sub)
        });

        Ok(OptionalAuthUser { user_id })
    }
}

/// FDN-07 typed-role extractor — identical to [`AuthUser`] but exposes the
/// role as a strongly-typed [`UserRole`] instead of a raw `String`. Unknown
/// role strings are rejected with [`AppError::Unauthorized`]; the bearer
/// token is presenting a role the backend no longer recognizes (rolled
/// back migration, forged claim, etc.).
///
/// Subsequent handlers (Round 2b) will migrate from `AuthUser` to
/// [`RoleUser`] so they can call `state.policy.has(user.role, "…")`
/// without re-parsing the string each call.
pub struct RoleUser {
    pub user_id: Uuid,
    pub role: UserRole,
}

impl FromRequestParts<AppState> for RoleUser {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let auth_user = AuthUser::from_request_parts(parts, state).await?;
        let role = UserRole::from_str_lower(&auth_user.role).ok_or(AppError::Unauthorized)?;
        Ok(RoleUser {
            user_id: auth_user.user_id,
            role,
        })
    }
}

/// Per-request network context: source IP and User-Agent.
///
/// Resolved in the order:
///   1. `X-Forwarded-For` (first hop) — set by the production reverse
///      proxy and the `TestApp` harness for governor isolation.
///   2. `X-Real-IP` — set by some PaaS load balancers (Render, Fly).
///   3. `axum::extract::ConnectInfo<SocketAddr>` — direct peer.
///
/// All fields are optional; in particular background workers and tests
/// without `ConnectInfo` will see `None`. Audit writers must therefore
/// treat IP/UA as observability rather than authorisation evidence.
#[derive(Debug, Clone, Default)]
pub struct ClientInfo {
    pub ip: Option<IpAddr>,
    pub user_agent: Option<String>,
}

impl FromRequestParts<AppState> for ClientInfo {
    type Rejection = Infallible;

    async fn from_request_parts(
        parts: &mut Parts,
        _state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let ip = extract_ip(
            &parts.headers,
            parts.extensions.get::<ConnectInfo<std::net::SocketAddr>>(),
        );
        let user_agent = parts
            .headers
            .get(axum::http::header::USER_AGENT)
            .and_then(|v| v.to_str().ok())
            .map(str::to_owned);

        Ok(ClientInfo { ip, user_agent })
    }
}

/// Header → ConnectInfo IP resolution. Pulled out for test ergonomics.
fn extract_ip(
    headers: &HeaderMap,
    connect_info: Option<&ConnectInfo<std::net::SocketAddr>>,
) -> Option<IpAddr> {
    if let Some(forwarded) = headers.get("X-Forwarded-For").and_then(|v| v.to_str().ok()) {
        if let Some(first) = forwarded.split(',').next() {
            if let Ok(ip) = first.trim().parse::<IpAddr>() {
                return Some(ip);
            }
        }
    }
    if let Some(real_ip) = headers.get("X-Real-IP").and_then(|v| v.to_str().ok()) {
        if let Ok(ip) = real_ip.trim().parse::<IpAddr>() {
            return Some(ip);
        }
    }
    connect_info.map(|ci| ci.0.ip())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::authz::{Policy, PolicyHandle};

    fn handle(p: Policy) -> PolicyHandle {
        PolicyHandle::new(p)
    }

    #[test]
    fn admin_has_permission_uses_policy_cache() {
        let policy = handle(Policy::from_pairs([
            (UserRole::Admin, "admin.role.manage"),
            (UserRole::Support, "order.any.read"),
        ]));

        let admin = AdminUser {
            user_id: Uuid::new_v4(),
            role: "admin".into(),
        };

        assert!(admin.has_permission(&policy, "admin.role.manage"));
        assert!(!admin.has_permission(&policy, "blog.post.create"));

        // Forged / stale role string → no permission
        let ghost = AdminUser {
            user_id: Uuid::new_v4(),
            role: "root".into(),
        };
        assert!(!ghost.has_permission(&policy, "admin.role.manage"));
    }

    #[test]
    fn admin_require_ok_when_role_carries_permission() {
        let policy = handle(Policy::from_pairs([(UserRole::Admin, "admin.role.manage")]));
        let admin = AdminUser {
            user_id: Uuid::new_v4(),
            role: "admin".into(),
        };
        let role = admin
            .require(&policy, "admin.role.manage")
            .expect("admin should have admin.role.manage");
        assert_eq!(role, UserRole::Admin);
    }

    #[test]
    fn admin_require_forbidden_when_role_lacks_permission() {
        let policy = handle(Policy::from_pairs([(UserRole::Admin, "admin.role.manage")]));
        let support = AdminUser {
            user_id: Uuid::new_v4(),
            role: "support".into(),
        };
        match support.require(&policy, "admin.role.manage") {
            Err(AppError::Forbidden) => {}
            other => panic!("expected Forbidden, got {other:?}"),
        }
    }

    #[test]
    fn admin_require_unauthorized_when_role_unknown() {
        let policy = handle(Policy::from_pairs([(UserRole::Admin, "x")]));
        let ghost = AdminUser {
            user_id: Uuid::new_v4(),
            role: "root".into(),
        };
        match ghost.require(&policy, "x") {
            Err(AppError::Unauthorized) => {}
            other => panic!("expected Unauthorized, got {other:?}"),
        }
    }

    #[test]
    fn extract_ip_prefers_x_forwarded_for() {
        let mut headers = HeaderMap::new();
        headers.insert(
            "X-Forwarded-For",
            axum::http::HeaderValue::from_static("203.0.113.7, 10.0.0.1"),
        );
        let ip = extract_ip(&headers, None);
        assert_eq!(ip, Some("203.0.113.7".parse().unwrap()));
    }

    #[test]
    fn extract_ip_falls_back_to_x_real_ip() {
        let mut headers = HeaderMap::new();
        headers.insert(
            "X-Real-IP",
            axum::http::HeaderValue::from_static("198.51.100.42"),
        );
        let ip = extract_ip(&headers, None);
        assert_eq!(ip, Some("198.51.100.42".parse().unwrap()));
    }

    #[test]
    fn extract_ip_returns_none_without_signals() {
        let headers = HeaderMap::new();
        let ip = extract_ip(&headers, None);
        assert!(ip.is_none());
    }

    // ── BFF cookie / bearer dual extraction ────────────────────────────────

    #[test]
    fn extract_access_token_prefers_cookie() {
        let mut headers = HeaderMap::new();
        headers.insert(
            axum::http::header::COOKIE,
            axum::http::HeaderValue::from_static(
                "other=foo; swings_access=COOKIE_TOKEN; trailing=bar",
            ),
        );
        headers.insert(
            axum::http::header::AUTHORIZATION,
            axum::http::HeaderValue::from_static("Bearer BEARER_TOKEN"),
        );
        assert_eq!(
            extract_access_token(&headers),
            Some("COOKIE_TOKEN".to_string())
        );
    }

    #[test]
    fn extract_access_token_falls_back_to_bearer_when_cookie_absent() {
        let mut headers = HeaderMap::new();
        headers.insert(
            axum::http::header::AUTHORIZATION,
            axum::http::HeaderValue::from_static("Bearer BEARER_TOKEN"),
        );
        assert_eq!(
            extract_access_token(&headers),
            Some("BEARER_TOKEN".to_string())
        );
    }

    #[test]
    fn extract_access_token_falls_back_when_only_unrelated_cookies_present() {
        let mut headers = HeaderMap::new();
        headers.insert(
            axum::http::header::COOKIE,
            axum::http::HeaderValue::from_static("session=abc; theme=dark"),
        );
        headers.insert(
            axum::http::header::AUTHORIZATION,
            axum::http::HeaderValue::from_static("Bearer BEARER_TOKEN"),
        );
        assert_eq!(
            extract_access_token(&headers),
            Some("BEARER_TOKEN".to_string())
        );
    }

    #[test]
    fn extract_access_token_returns_none_without_any_carrier() {
        let headers = HeaderMap::new();
        assert!(extract_access_token(&headers).is_none());
    }

    // ── verify_claim_binding (strict iss/aud) ─────────────────────────────

    fn claims_with(iss: Option<&str>, aud: Option<&str>) -> Claims {
        Claims {
            sub: Uuid::new_v4(),
            role: "member".into(),
            exp: 0,
            iat: 0,
            iss: iss.map(str::to_owned),
            aud: aud.map(str::to_owned),
            imp_actor: None,
            imp_actor_role: None,
            imp_session: None,
        }
    }

    #[test]
    fn verify_claim_binding_ok_when_both_claims_match() {
        let c = claims_with(Some(JWT_ISSUER), Some(JWT_AUDIENCE));
        assert!(verify_claim_binding(&c).is_ok());
    }

    #[test]
    fn verify_claim_binding_rejects_missing_iss() {
        let c = claims_with(None, Some(JWT_AUDIENCE));
        match verify_claim_binding(&c) {
            Err(AppError::Unauthorized) => {}
            other => panic!("expected Unauthorized, got {other:?}"),
        }
    }

    #[test]
    fn verify_claim_binding_rejects_missing_aud() {
        let c = claims_with(Some(JWT_ISSUER), None);
        match verify_claim_binding(&c) {
            Err(AppError::Unauthorized) => {}
            other => panic!("expected Unauthorized, got {other:?}"),
        }
    }

    #[test]
    fn verify_claim_binding_rejects_missing_both_claims() {
        let c = claims_with(None, None);
        match verify_claim_binding(&c) {
            Err(AppError::Unauthorized) => {}
            other => panic!("expected Unauthorized, got {other:?}"),
        }
    }

    #[test]
    fn verify_claim_binding_rejects_wrong_iss() {
        let c = claims_with(Some("evil.example.com"), Some(JWT_AUDIENCE));
        match verify_claim_binding(&c) {
            Err(AppError::Unauthorized) => {}
            other => panic!("expected Unauthorized, got {other:?}"),
        }
    }

    #[test]
    fn verify_claim_binding_rejects_wrong_aud() {
        let c = claims_with(Some(JWT_ISSUER), Some("evil.example.com/app"));
        match verify_claim_binding(&c) {
            Err(AppError::Unauthorized) => {}
            other => panic!("expected Unauthorized, got {other:?}"),
        }
    }

    #[test]
    fn extract_access_token_ignores_empty_cookie_value() {
        let mut headers = HeaderMap::new();
        headers.insert(
            axum::http::header::COOKIE,
            axum::http::HeaderValue::from_static("swings_access=; other=x"),
        );
        // Empty cookie should be treated as absent, falling back to bearer
        // (also absent here, so None).
        assert!(extract_access_token(&headers).is_none());
    }
}
