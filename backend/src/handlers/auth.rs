use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use axum::{extract::State, routing::post, Json, Router};
use chrono::{Duration, Utc};
use jsonwebtoken::{encode, EncodingKey, Header};
use sha2::{Digest, Sha256};
use uuid::Uuid;
use validator::Validate;

use crate::{
    db,
    error::{AppError, AppResult},
    extractors::{AuthUser, Claims, ClientInfo},
    models::*,
    notifications::send::{send_notification, Recipient, SendOptions},
    AppState,
};

pub fn router() -> Router<AppState> {
    Router::new()
        .merge(
            Router::new()
                .route("/register", post(register))
                .layer(crate::middleware::rate_limit::register_layer()),
        )
        .merge(
            Router::new()
                .route("/login", post(login))
                .layer(crate::middleware::rate_limit::login_layer()),
        )
        .merge(
            Router::new()
                .route("/forgot-password", post(forgot_password))
                .layer(crate::middleware::rate_limit::forgot_password_layer()),
        )
        .route("/refresh", post(refresh))
        .route("/me", axum::routing::get(me))
        .route("/logout", post(logout))
        .route("/reset-password", post(reset_password))
}

#[utoipa::path(
    post,
    path = "/api/auth/register",
    tag = "auth",
    request_body = RegisterRequest,
    responses(
        (status = 200, description = "Account created and authenticated", body = AuthResponse),
        (status = 409, description = "Email already registered"),
        (status = 422, description = "Validation error")
    )
)]
pub(crate) async fn register(
    State(state): State<AppState>,
    Json(req): Json<RegisterRequest>,
) -> AppResult<Json<AuthResponse>> {
    req.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;

    if db::find_user_by_email(&state.db, &req.email)
        .await?
        .is_some()
    {
        return Err(AppError::Conflict("Email already registered".to_string()));
    }

    let salt = SaltString::generate(&mut OsRng);
    let password_hash = Argon2::default()
        .hash_password(req.password.as_bytes(), &salt)
        .map_err(|e| AppError::BadRequest(format!("Password hash error: {e}")))?
        .to_string();

    let user = db::create_user(&state.db, &req.email, &password_hash, &req.name).await?;

    let (access_token, refresh_token) = generate_tokens(&state, &user).await?;

    // FDN-05: send the welcome email via the notifications pipeline. Errors
    // here are logged but never block signup — the user must be able to
    // complete registration even if the provider is momentarily down.
    let ctx = serde_json::json!({
        "name": user.name,
        "app_url": state.config.app_url,
        "year": chrono::Utc::now().format("%Y").to_string(),
    });
    if let Err(e) = send_notification(
        &state.db,
        "user.welcome",
        &Recipient::User {
            user_id: user.id,
            email: user.email.clone(),
        },
        ctx,
        SendOptions::default(),
    )
    .await
    {
        tracing::warn!(user_id = %user.id, error = %e, "failed to enqueue welcome email");
    }

    Ok(Json(AuthResponse {
        user: user.into(),
        access_token,
        refresh_token,
    }))
}

#[utoipa::path(
    post,
    path = "/api/auth/login",
    tag = "auth",
    request_body = LoginRequest,
    responses(
        (status = 200, description = "Authenticated", body = AuthResponse),
        (status = 401, description = "Invalid credentials"),
        (status = 422, description = "Validation error")
    )
)]
pub(crate) async fn login(
    State(state): State<AppState>,
    client: ClientInfo,
    Json(req): Json<LoginRequest>,
) -> AppResult<Json<AuthResponse>> {
    req.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;

    let user = match db::find_user_by_email(&state.db, &req.email).await? {
        Some(u) => u,
        None => {
            // ADM-02: log unknown-email failures so brute force / enumeration
            // shows up in the security console. Best-effort — never blocks
            // the 401 response if the audit table is unreachable.
            log_failed_login(&state, &req.email, &client, "unknown_email").await;
            return Err(AppError::Unauthorized);
        }
    };

    // ADM-02: hard ban / suspension gate. Both states return 401 (not 403)
    // to avoid leaking account existence — the response is identical to a
    // bad-password failure.
    if user.banned_at.is_some() {
        log_failed_login(&state, &req.email, &client, "banned").await;
        return Err(AppError::Unauthorized);
    }
    if user.suspended_at.is_some() {
        log_failed_login(&state, &req.email, &client, "suspended").await;
        return Err(AppError::Unauthorized);
    }

    let parsed_hash = match PasswordHash::new(&user.password_hash) {
        Ok(h) => h,
        Err(_) => {
            log_failed_login(&state, &req.email, &client, "bad_password").await;
            return Err(AppError::Unauthorized);
        }
    };

    if Argon2::default()
        .verify_password(req.password.as_bytes(), &parsed_hash)
        .is_err()
    {
        log_failed_login(&state, &req.email, &client, "bad_password").await;
        return Err(AppError::Unauthorized);
    }

    let (access_token, refresh_token) = generate_tokens(&state, &user).await?;

    Ok(Json(AuthResponse {
        user: user.into(),
        access_token,
        refresh_token,
    }))
}

/// ADM-02: best-effort write to `failed_login_attempts`.
///
/// Failures here log a warning and continue — the user-facing 401 response
/// must never depend on observability succeeding.
async fn log_failed_login(state: &AppState, email: &str, client: &ClientInfo, reason: &str) {
    if let Err(e) = db::record_failed_login(
        &state.db,
        email,
        client.ip,
        client.user_agent.as_deref(),
        reason,
    )
    .await
    {
        tracing::warn!(error = %e, reason, "failed to record failed_login_attempt");
    }
}

#[utoipa::path(
    post,
    path = "/api/auth/refresh",
    tag = "auth",
    request_body = RefreshRequest,
    responses(
        (status = 200, description = "Token rotated", body = TokenResponse),
        (status = 401, description = "Invalid or reused refresh token")
    )
)]
pub(crate) async fn refresh(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Json(req): Json<RefreshRequest>,
) -> AppResult<Json<TokenResponse>> {
    // ADM-07-α: refuse refresh attempts whose Authorization header
    // carries an active impersonation token. Impersonation JWTs are
    // intentionally not paired with a refresh token (see
    // `admin_impersonation::mint`); a refresh attempt under an
    // impersonation context can only be the result of a buggy or
    // malicious client trying to extend a support session beyond
    // its TTL. Failing closed here is cheap and removes the entire
    // class of "silent re-elevation via refresh" attacks.
    if bearer_is_impersonation(&headers) {
        tracing::warn!("refresh blocked: bearer token is an impersonation token");
        return Err(AppError::Forbidden);
    }

    let token_hash = hash_token(&req.refresh_token);

    let stored = db::find_refresh_token(&state.db, &token_hash)
        .await?
        .ok_or(AppError::Unauthorized)?;

    if stored.used {
        tracing::warn!(
            user_id = %stored.user_id,
            family_id = %stored.family_id,
            "Refresh token reuse detected — revoking token family"
        );
        db::delete_refresh_tokens_by_family(&state.db, stored.family_id).await?;
        return Err(AppError::TokenReuseDetected(
            "Session invalidated due to token reuse".to_string(),
        ));
    }

    db::mark_refresh_token_used(&state.db, stored.id).await?;

    let user = db::find_user_by_id(&state.db, stored.user_id)
        .await?
        .ok_or(AppError::Unauthorized)?;

    let now = Utc::now();
    let claims = Claims {
        sub: user.id,
        role: format!("{:?}", user.role).to_lowercase(),
        iat: now.timestamp() as usize,
        exp: (now + Duration::hours(state.config.jwt_expiration_hours)).timestamp() as usize,
        imp_actor: None,
        imp_actor_role: None,
        imp_session: None,
    };

    let access_token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(state.config.jwt_secret.as_bytes()),
    )
    .map_err(|e| AppError::BadRequest(format!("Token generation failed: {e}")))?;

    let new_refresh = Uuid::new_v4().to_string();
    let new_hash = hash_token(&new_refresh);
    let expires_at = now + Duration::days(state.config.refresh_token_expiration_days);

    db::store_refresh_token(
        &state.db,
        stored.user_id,
        &new_hash,
        expires_at,
        stored.family_id,
        false,
    )
    .await?;

    Ok(Json(TokenResponse {
        access_token,
        refresh_token: new_refresh,
    }))
}

async fn me(State(state): State<AppState>, auth: AuthUser) -> AppResult<Json<UserResponse>> {
    let user = db::find_user_by_id(&state.db, auth.user_id)
        .await?
        .ok_or(AppError::NotFound("User not found".to_string()))?;

    Ok(Json(user.into()))
}

#[utoipa::path(
    post,
    path = "/api/auth/logout",
    tag = "auth",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Logged out; refresh tokens revoked. \
            When called under an impersonation token, ends the impersonation \
            session instead and returns `{ \"message\": \"Impersonation ended\" }`."),
        (status = 401, description = "Unauthorized")
    )
)]
pub(crate) async fn logout(
    State(state): State<AppState>,
    auth: AuthUser,
    client: ClientInfo,
) -> AppResult<Json<serde_json::Value>> {
    // ADM-07-α: when the caller's session is an impersonation token,
    // a naive `delete_user_refresh_tokens(auth.user_id)` would punt
    // every refresh token of the *target* user — i.e. the admin would
    // accidentally log the customer out of every device. Detect the
    // impersonation context and revoke the impersonation row instead;
    // the admin's own real session is unaffected because impersonation
    // tokens are never refresh-paired.
    if let (Some(session_id), Some(actor_id)) =
        (auth.impersonation_session_id, auth.impersonator_id)
    {
        let revoked = crate::security::impersonation::revoke(
            &state.db,
            session_id,
            actor_id,
            Some("logout-while-impersonating"),
        )
        .await?;

        // Mirror the audit shape of `admin_impersonation::exit` so the
        // audit-log viewer treats this as a single class of action.
        let actor_role = db::find_user_by_id(&state.db, actor_id)
            .await
            .ok()
            .flatten()
            .map(|u| u.role)
            .unwrap_or(crate::models::UserRole::Admin);
        crate::services::audit::record_admin_action_best_effort(
            &state.db,
            crate::services::audit::AdminAction::new(
                actor_id,
                actor_role,
                "admin.impersonation.exit",
                "impersonation_session",
            )
            .with_target_id(session_id)
            .with_client(&client)
            .with_metadata(serde_json::json!({
                "target_user_id":   auth.user_id,
                "session_was_live": revoked.is_some(),
                "via":              "logout",
            })),
        )
        .await;

        return Ok(Json(serde_json::json!({
            "message": "Impersonation ended",
            "impersonation_ended": true
        })));
    }

    db::delete_user_refresh_tokens(&state.db, auth.user_id).await?;
    Ok(Json(serde_json::json!({ "message": "Logged out" })))
}

// ── Forgot / Reset Password ─────────────────────────────────────────────

#[utoipa::path(
    post,
    path = "/api/auth/forgot-password",
    tag = "auth",
    request_body = ForgotPasswordRequest,
    responses(
        (status = 200, description = "Reset email dispatched if account exists"),
        (status = 422, description = "Validation error")
    )
)]
pub(crate) async fn forgot_password(
    State(state): State<AppState>,
    Json(req): Json<ForgotPasswordRequest>,
) -> AppResult<Json<serde_json::Value>> {
    req.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;

    // Always return success to prevent email enumeration
    let user = db::find_user_by_email(&state.db, &req.email).await?;

    if let Some(user) = user {
        let raw_token = Uuid::new_v4().to_string();
        let token_hash = hash_token(&raw_token);
        let expires_at = Utc::now() + Duration::hours(1);

        db::create_password_reset_token(&state.db, user.id, &token_hash, expires_at).await?;

        // Build reset URL
        let reset_url = format!(
            "{}/admin/reset-password?token={}",
            state.config.frontend_url, raw_token
        );

        // FDN-05: send the reset email through the notifications pipeline.
        // Failures are logged but not surfaced to the client — the response
        // is identical on success or soft-failure to avoid email enumeration.
        tracing::info!(
            "Password reset requested for {}. Reset URL: {}",
            req.email,
            reset_url
        );
        let ctx = serde_json::json!({
            "name": user.name,
            "reset_url": reset_url,
            "app_url": state.config.app_url,
            "year": chrono::Utc::now().format("%Y").to_string(),
        });
        if let Err(e) = send_notification(
            &state.db,
            "user.password_reset",
            &Recipient::User {
                user_id: user.id,
                email: user.email.clone(),
            },
            ctx,
            SendOptions::default(),
        )
        .await
        {
            tracing::warn!(user_id = %user.id, error = %e, "failed to enqueue password reset email");
        }
    }

    Ok(Json(serde_json::json!({
        "message": "If an account with that email exists, a password reset link has been sent."
    })))
}

#[utoipa::path(
    post,
    path = "/api/auth/reset-password",
    tag = "auth",
    request_body = ResetPasswordRequest,
    responses(
        (status = 200, description = "Password updated"),
        (status = 400, description = "Invalid or expired reset token"),
        (status = 422, description = "Validation error")
    )
)]
pub(crate) async fn reset_password(
    State(state): State<AppState>,
    Json(req): Json<ResetPasswordRequest>,
) -> AppResult<Json<serde_json::Value>> {
    req.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;

    let token_hash = hash_token(&req.token);

    let reset_token = db::find_password_reset_token(&state.db, &token_hash)
        .await?
        .ok_or(AppError::BadRequest(
            "Invalid or expired reset token".to_string(),
        ))?;

    // Hash new password
    let salt = SaltString::generate(&mut OsRng);
    let password_hash = Argon2::default()
        .hash_password(req.new_password.as_bytes(), &salt)
        .map_err(|e| AppError::BadRequest(format!("Password hash error: {e}")))?
        .to_string();

    // Update password and mark token as used
    db::update_user_password(&state.db, reset_token.user_id, &password_hash).await?;
    db::mark_reset_token_used(&state.db, reset_token.id).await?;

    // Invalidate all refresh tokens for security
    db::delete_user_refresh_tokens(&state.db, reset_token.user_id).await?;

    tracing::info!("Password reset completed for user {}", reset_token.user_id);

    Ok(Json(serde_json::json!({
        "message": "Password has been reset successfully. Please log in with your new password."
    })))
}

// ── Helpers ─────────────────────────────────────────────────────────────

async fn generate_tokens(state: &AppState, user: &User) -> AppResult<(String, String)> {
    let now = Utc::now();

    let claims = Claims {
        sub: user.id,
        role: format!("{:?}", user.role).to_lowercase(),
        iat: now.timestamp() as usize,
        exp: (now + Duration::hours(state.config.jwt_expiration_hours)).timestamp() as usize,
        imp_actor: None,
        imp_actor_role: None,
        imp_session: None,
    };

    let access_token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(state.config.jwt_secret.as_bytes()),
    )
    .map_err(|e| AppError::BadRequest(format!("Token generation failed: {e}")))?;

    let refresh_token = Uuid::new_v4().to_string();
    let token_hash = hash_token(&refresh_token);
    let expires_at = now + Duration::days(state.config.refresh_token_expiration_days);
    let family_id = Uuid::new_v4();

    db::store_refresh_token(
        &state.db,
        user.id,
        &token_hash,
        expires_at,
        family_id,
        false,
    )
    .await?;

    Ok((access_token, refresh_token))
}

fn hash_token(token: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(token.as_bytes());
    hasher
        .finalize()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

/// Inspect the `Authorization` header for an impersonation JWT — without
/// validating signature or expiry. We only care about the *intent*
/// (presence of `imp_session`) so the refresh endpoint can fail closed
/// before the heavier refresh-token lookup runs. See [`refresh`].
fn bearer_is_impersonation(headers: &axum::http::HeaderMap) -> bool {
    let Some(token) = headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|h| h.strip_prefix("Bearer "))
    else {
        return false;
    };
    let Some(payload_b64) = token.split('.').nth(1) else {
        return false;
    };
    use base64::Engine;
    let Ok(bytes) = base64::engine::general_purpose::URL_SAFE_NO_PAD.decode(payload_b64) else {
        return false;
    };
    serde_json::from_slice::<serde_json::Value>(&bytes)
        .ok()
        .as_ref()
        .and_then(|v| v.get("imp_session"))
        .is_some_and(|v| !v.is_null())
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::{HeaderMap, HeaderValue};
    use base64::Engine;

    fn make_jwt_with_payload(payload: serde_json::Value) -> String {
        let header_b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(b"{}");
        let payload_b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .encode(serde_json::to_vec(&payload).unwrap());
        format!("{header_b64}.{payload_b64}.sig")
    }

    #[test]
    fn bearer_is_impersonation_detects_imp_session() {
        let mut headers = HeaderMap::new();
        let token = make_jwt_with_payload(
            serde_json::json!({"sub":"x","imp_session":"00000000-0000-0000-0000-000000000001"}),
        );
        headers.insert(
            axum::http::header::AUTHORIZATION,
            HeaderValue::from_str(&format!("Bearer {token}")).unwrap(),
        );
        assert!(bearer_is_impersonation(&headers));
    }

    #[test]
    fn bearer_is_impersonation_false_for_normal_token() {
        let mut headers = HeaderMap::new();
        let token = make_jwt_with_payload(serde_json::json!({"sub":"x"}));
        headers.insert(
            axum::http::header::AUTHORIZATION,
            HeaderValue::from_str(&format!("Bearer {token}")).unwrap(),
        );
        assert!(!bearer_is_impersonation(&headers));
    }

    #[test]
    fn bearer_is_impersonation_false_when_imp_session_null() {
        let mut headers = HeaderMap::new();
        let token = make_jwt_with_payload(serde_json::json!({"sub":"x","imp_session":null}));
        headers.insert(
            axum::http::header::AUTHORIZATION,
            HeaderValue::from_str(&format!("Bearer {token}")).unwrap(),
        );
        assert!(!bearer_is_impersonation(&headers));
    }

    #[test]
    fn bearer_is_impersonation_false_without_header() {
        let headers = HeaderMap::new();
        assert!(!bearer_is_impersonation(&headers));
    }
}
