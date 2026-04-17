use std::convert::Infallible;

use axum::{extract::FromRequestParts, http::request::Parts};
use jsonwebtoken::{decode, DecodingKey, Validation};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{authz::Policy, error::AppError, models::UserRole, AppState};

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: Uuid,
    pub role: String,
    pub exp: usize,
    pub iat: usize,
}

pub struct AuthUser {
    pub user_id: Uuid,
    pub role: String,
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

        Ok(AuthUser {
            user_id: token_data.claims.sub,
            role: token_data.claims.role,
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
    pub fn has_permission(&self, policy: &Policy, perm: &str) -> bool {
        UserRole::from_str_lower(&self.role).is_some_and(|role| policy.has(role, perm))
    }
}

impl FromRequestParts<AppState> for AdminUser {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let auth_user = AuthUser::from_request_parts(parts, state).await?;

        if auth_user.role != "admin" {
            return Err(AppError::Forbidden);
        }

        Ok(AdminUser {
            user_id: auth_user.user_id,
            role: auth_user.role,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::authz::Policy;

    #[test]
    fn admin_has_permission_uses_policy_cache() {
        let policy = Policy::from_pairs([
            (UserRole::Admin, "admin.role.manage"),
            (UserRole::Support, "order.any.read"),
        ]);

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
}
