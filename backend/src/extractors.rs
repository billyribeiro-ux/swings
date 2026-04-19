use std::convert::Infallible;
use std::net::IpAddr;

use axum::{
    extract::{ConnectInfo, FromRequestParts},
    http::{request::Parts, HeaderMap},
};
use jsonwebtoken::{decode, DecodingKey, Validation};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{authz::PolicyHandle, error::AppError, models::UserRole, AppState};

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: Uuid,
    pub role: String,
    pub exp: usize,
    pub iat: usize,
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

impl FromRequestParts<AppState> for AuthUser {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let auth_header = parts
            .headers
            .get("Authorization")
            .and_then(|v| v.to_str().ok())
            .ok_or(AppError::Unauthorized)?;

        let token = auth_header
            .strip_prefix("Bearer ")
            .ok_or(AppError::Unauthorized)?;

        let token_data = decode::<Claims>(
            token,
            &DecodingKey::from_secret(state.config.jwt_secret.as_bytes()),
            &Validation::default(),
        )
        .map_err(|_| AppError::Unauthorized)?;

        let claims = token_data.claims;

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
        let user_id = parts
            .headers
            .get("Authorization")
            .and_then(|v| v.to_str().ok())
            .and_then(|h| h.strip_prefix("Bearer "))
            .and_then(|token| {
                decode::<Claims>(
                    token,
                    &DecodingKey::from_secret(state.config.jwt_secret.as_bytes()),
                    &Validation::default(),
                )
                .ok()
                .map(|t| t.claims.sub)
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
        let ip = extract_ip(&parts.headers, parts.extensions.get::<ConnectInfo<std::net::SocketAddr>>());
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
}
