use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use axum::{
    extract::State,
    http::{header, HeaderMap, HeaderValue},
    routing::post,
    Json, Router,
};
use axum_extra::extract::cookie::{Cookie, CookieJar, SameSite};
use chrono::{Duration, Utc};
use jsonwebtoken::{encode, EncodingKey, Header};
use time::Duration as CookieDuration;
use uuid::Uuid;
use validator::Validate;

use crate::{
    crypto::hash_token,
    db,
    error::{AppError, AppResult},
    extractors::{AuthUser, Claims, ClientInfo, MaybeAuthUser, JWT_AUDIENCE, JWT_ISSUER},
    models::*,
    notifications::send::{send_notification, Recipient, SendOptions},
    AppState,
};

/// SECURITY: cookie name for the access token half of the BFF session.
///
/// HttpOnly + Secure (in production) + SameSite=Lax. JS cannot read it, so an
/// XSS sink can no longer exfiltrate the bearer token. The browser attaches it
/// automatically on every same-origin request to `/api/*`.
pub(crate) const COOKIE_ACCESS: &str = "swings_access";

/// SECURITY: cookie name for the refresh token half of the BFF session.
///
/// Same hardening as [`COOKIE_ACCESS`]. Read by [`refresh`] when no JSON
/// `refresh_token` body is present so the SPA never has to handle the value.
pub(crate) const COOKIE_REFRESH: &str = "swings_refresh";

/// Build a hardened auth cookie carrying `value` for `name`.
///
/// Attributes:
/// * `HttpOnly` — opaque to JS, defeats XSS exfiltration.
/// * `Secure` — only when `Config::is_production` is true; the dev path
///   must work over plaintext `http://localhost`.
/// * `SameSite=Lax` — sent on top-level same-site navigations (e.g. the
///   Stripe-checkout redirect back to `/dashboard?...`) while blocking the
///   worst CSRF vectors. We do NOT use `Strict` because the Stripe
///   success-URL return is a top-level navigation cross-site → same-site
///   that `Strict` would strip.
/// * `Path=/` — every API + page route gets the cookie.
/// * `Domain` unset — defaults to the request host. Works for localhost,
///   Vercel preview URLs, and the apex domain without re-deploying for
///   each environment.
/// * `Max-Age` — caller-supplied lifetime; matches the JWT expiration.
fn build_session_cookie(
    name: &'static str,
    value: String,
    max_age_secs: i64,
    is_production: bool,
) -> Cookie<'static> {
    let mut cookie = Cookie::new(name, value);
    cookie.set_http_only(true);
    cookie.set_secure(is_production);
    cookie.set_same_site(SameSite::Lax);
    cookie.set_path("/");
    cookie.set_max_age(CookieDuration::seconds(max_age_secs));
    cookie
}

/// Build a deletion cookie for `name` — empty value, `Max-Age=0`, same path
/// the original was set on. Browsers honour the deletion immediately.
fn clear_session_cookie(name: &'static str, is_production: bool) -> Cookie<'static> {
    let mut cookie = Cookie::new(name, "");
    cookie.set_http_only(true);
    cookie.set_secure(is_production);
    cookie.set_same_site(SameSite::Lax);
    cookie.set_path("/");
    cookie.set_max_age(CookieDuration::ZERO);
    cookie
}

/// Append `Set-Cookie: ...` headers for the access + refresh pair.
///
/// Returns a populated [`HeaderMap`] that handler responses can chain into
/// the axum response tuple. We append (rather than insert) because a single
/// HTTP response may carry multiple `Set-Cookie` lines and `insert` would
/// silently replace.
pub(crate) fn auth_cookie_headers(
    state: &AppState,
    access_token: &str,
    refresh_token: &str,
) -> AppResult<HeaderMap> {
    let prod = state.config.is_production();
    let access = build_session_cookie(
        COOKIE_ACCESS,
        access_token.to_string(),
        state.config.jwt_expiration_hours.saturating_mul(3600),
        prod,
    );
    let refresh = build_session_cookie(
        COOKIE_REFRESH,
        refresh_token.to_string(),
        state
            .config
            .refresh_token_expiration_days
            .saturating_mul(86_400),
        prod,
    );
    let mut headers = HeaderMap::new();
    headers.append(
        header::SET_COOKIE,
        HeaderValue::from_str(&access.to_string())
            .map_err(|e| AppError::BadRequest(format!("set-cookie encode: {e}")))?,
    );
    headers.append(
        header::SET_COOKIE,
        HeaderValue::from_str(&refresh.to_string())
            .map_err(|e| AppError::BadRequest(format!("set-cookie encode: {e}")))?,
    );
    Ok(headers)
}

/// Build deletion `Set-Cookie` headers for both halves of the session.
pub(crate) fn clear_auth_cookie_headers(state: &AppState) -> AppResult<HeaderMap> {
    let prod = state.config.is_production();
    let access = clear_session_cookie(COOKIE_ACCESS, prod);
    let refresh = clear_session_cookie(COOKIE_REFRESH, prod);
    let mut headers = HeaderMap::new();
    headers.append(
        header::SET_COOKIE,
        HeaderValue::from_str(&access.to_string())
            .map_err(|e| AppError::BadRequest(format!("set-cookie encode: {e}")))?,
    );
    headers.append(
        header::SET_COOKIE,
        HeaderValue::from_str(&refresh.to_string())
            .map_err(|e| AppError::BadRequest(format!("set-cookie encode: {e}")))?,
    );
    Ok(headers)
}

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
        .route("/verify-email", post(verify_email))
        .route("/resend-verification", post(resend_verification))
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
) -> AppResult<(HeaderMap, Json<AuthResponse>)> {
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

    if let Err(e) = issue_email_verification(&state, &user).await {
        tracing::warn!(user_id = %user.id, error = %e, "failed to enqueue verification email");
    }

    let cookies = auth_cookie_headers(&state, &access_token, &refresh_token)?;
    Ok((
        cookies,
        Json(AuthResponse {
            user: user.into(),
            access_token,
            refresh_token,
        }),
    ))
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
) -> AppResult<(HeaderMap, Json<AuthResponse>)> {
    req.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;

    let user = match db::find_user_by_email(&state.db, &req.email).await? {
        Some(u) => u,
        None => {
            // ADM-02: log unknown-email failures so brute force / enumeration
            // shows up in the security console. Best-effort — never blocks
            // the 401 response if the audit table is unreachable.
            log_failed_login(&state, &req.email, &client, "unknown_email").await;
            // SECURITY: run Argon2 against a fixed dummy hash so the
            // unknown-email branch takes roughly the same wall-clock time
            // as a valid-email-with-bad-password branch. Prevents the
            // timing side-channel that would otherwise let an attacker
            // enumerate registered accounts by measuring 401 latency.
            consume_login_timing_budget();
            return Err(AppError::Unauthorized);
        }
    };

    // ADM-15: lift an expired temporary suspension (timeout) lazily on
    // login. The `suspended_until` column is set when an operator issues
    // a time-boxed suspension via POST /api/admin/members/{id}/suspend
    // with `until`. Doing this here keeps us off a background sweeper
    // while still giving the user back their account at the right moment.
    let user = match db::lift_expired_suspension(&state.db, user.id).await {
        Ok(Some(refreshed)) => refreshed,
        Ok(None) => user,
        Err(e) => {
            // Best-effort: a failed lazy-clear must not allow login when
            // the underlying row is still suspended. Fall through with
            // the original row so the suspension gate below still fires.
            tracing::warn!(error = %e, user_id = %user.id, "lift_expired_suspension failed; using stale row");
            user
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

    let cookies = auth_cookie_headers(&state, &access_token, &refresh_token)?;
    Ok((
        cookies,
        Json(AuthResponse {
            user: user.into(),
            access_token,
            refresh_token,
        }),
    ))
}

/// SECURITY: burn an Argon2 verification budget against a fixed dummy hash
/// so the unknown-email branch of `/api/auth/login` takes a comparable amount
/// of wall-clock time to the valid-email + bad-password branch. This closes
/// the timing side-channel an attacker would otherwise use to enumerate
/// registered accounts by measuring 401 response latency.
///
/// We generate the hash exactly once on first call (lazy), then run
/// `verify_password` on every subsequent invocation so the cost matches
/// a real `verify_password` call.
fn consume_login_timing_budget() {
    static DUMMY_HASH: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    let encoded = DUMMY_HASH.get_or_init(|| {
        let salt = SaltString::generate(&mut OsRng);
        Argon2::default()
            .hash_password(b"timing-equalisation-dummy", &salt)
            .map(|h| h.to_string())
            // If Argon2 ever refuses to hash the fixed input on this build,
            // fall through with an empty string; `PasswordHash::new` below
            // will error and we skip the verify. A best-effort defence is
            // still strictly better than none.
            .unwrap_or_default()
    });
    if let Ok(parsed) = PasswordHash::new(encoded) {
        let _ = Argon2::default().verify_password(b"not-a-real-password", &parsed);
    }
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
    jar: CookieJar,
    body: Option<Json<RefreshRequest>>,
) -> AppResult<(HeaderMap, Json<TokenResponse>)> {
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

    // BFF: prefer the httpOnly cookie that the SPA never sees. Fall back to
    // the legacy JSON body for the rollout window — once every live client
    // round-trips through cookie-based login the body branch can go away.
    let supplied_token = jar
        .get(COOKIE_REFRESH)
        .map(|c| c.value().to_string())
        .or_else(|| body.and_then(|Json(req)| req.refresh_token));
    let raw_refresh = supplied_token.ok_or(AppError::Unauthorized)?;

    let token_hash = hash_token(&raw_refresh);

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
        iss: Some(JWT_ISSUER.to_string()),
        aud: Some(JWT_AUDIENCE.to_string()),
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

    let cookies = auth_cookie_headers(&state, &access_token, &new_refresh)?;
    Ok((
        cookies,
        Json(TokenResponse {
            access_token,
            refresh_token: new_refresh,
        }),
    ))
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
    MaybeAuthUser(auth): MaybeAuthUser,
    client: ClientInfo,
) -> AppResult<(HeaderMap, Json<serde_json::Value>)> {
    // BFF: logout must be idempotent. A user with an expired/missing cookie
    // (or one that was already cleared in another tab) still needs a clean
    // way to wipe whatever session state is left in the browser. When there
    // is no authenticated session we just emit the clear-cookie headers and
    // return 200 — no DB work, no audit row, nothing to revoke.
    let Some(auth) = auth else {
        let cookies = clear_auth_cookie_headers(&state)?;
        return Ok((
            cookies,
            Json(serde_json::json!({ "message": "Logged out" })),
        ));
    };

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

        // BFF: deliberately do NOT clear the auth cookies on the
        // impersonation-end branch — the admin's own session lives on the
        // same browser and must keep its cookies. Only the impersonation row
        // is revoked above.
        return Ok((
            HeaderMap::new(),
            Json(serde_json::json!({
                "message": "Impersonation ended",
                "impersonation_ended": true
            })),
        ));
    }

    db::delete_user_refresh_tokens(&state.db, auth.user_id).await?;
    let cookies = clear_auth_cookie_headers(&state)?;
    Ok((
        cookies,
        Json(serde_json::json!({ "message": "Logged out" })),
    ))
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

        let reset_url = format!(
            "{}/reset-password?token={}",
            state.config.frontend_url, raw_token
        );

        // FDN-05: send the reset email through the notifications pipeline.
        // Failures are logged but not surfaced to the client — the response
        // is identical on success or soft-failure to avoid email enumeration.
        //
        // SECURITY: never log the raw token, the reset URL, or the email
        // address. Anyone with log access would otherwise be able to take
        // over the account. Pivot back to the user via `user_id` only.
        tracing::info!(user_id = %user.id, "password reset token issued");
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

#[utoipa::path(
    post,
    path = "/api/auth/verify-email",
    tag = "auth",
    request_body = VerifyEmailRequest,
    responses(
        (status = 200, description = "Email verified"),
        (status = 400, description = "Invalid or expired verification token"),
        (status = 422, description = "Validation error")
    )
)]
pub(crate) async fn verify_email(
    State(state): State<AppState>,
    Json(req): Json<VerifyEmailRequest>,
) -> AppResult<Json<serde_json::Value>> {
    req.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;

    let token_hash = hash_token(&req.token);
    let verification = db::find_email_verification_token(&state.db, &token_hash)
        .await?
        .ok_or_else(|| AppError::BadRequest("Invalid or expired verification token".to_string()))?;

    db::mark_user_email_verified(&state.db, verification.user_id).await?;
    db::mark_email_verification_token_used(&state.db, verification.id).await?;

    Ok(Json(serde_json::json!({
        "message": "Email verified successfully.",
        "verified": true
    })))
}

#[utoipa::path(
    post,
    path = "/api/auth/resend-verification",
    tag = "auth",
    request_body = ResendVerificationRequest,
    responses(
        (status = 200, description = "Verification email queued if account is pending verification"),
        (status = 422, description = "Validation error")
    )
)]
pub(crate) async fn resend_verification(
    State(state): State<AppState>,
    Json(req): Json<ResendVerificationRequest>,
) -> AppResult<Json<serde_json::Value>> {
    req.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;

    if let Some(user) = db::find_user_by_email(&state.db, &req.email).await? {
        if user.email_verified_at.is_none() {
            if let Err(e) = issue_email_verification(&state, &user).await {
                tracing::warn!(
                    user_id = %user.id,
                    error = %e,
                    "failed to enqueue verification email on resend"
                );
            }
        }
    }

    Ok(Json(serde_json::json!({
        "message": "If your account exists and is unverified, a verification email has been sent."
    })))
}

// ── Helpers ─────────────────────────────────────────────────────────────

async fn issue_email_verification(state: &AppState, user: &User) -> AppResult<()> {
    let raw_token = Uuid::new_v4().to_string();
    let token_hash = hash_token(&raw_token);
    let expires_at = Utc::now() + Duration::hours(24);

    db::create_email_verification_token(&state.db, user.id, &token_hash, expires_at).await?;

    let verify_url = format!(
        "{}/verify-email?token={}",
        state.config.frontend_url, raw_token
    );
    let ctx = serde_json::json!({
        "name": user.name,
        "verify_url": verify_url,
        "app_url": state.config.app_url,
        "year": chrono::Utc::now().format("%Y").to_string(),
    });

    send_notification(
        &state.db,
        "user.email_verification",
        &Recipient::User {
            user_id: user.id,
            email: user.email.clone(),
        },
        ctx,
        SendOptions::default(),
    )
    .await
    .map_err(|e| AppError::ServiceUnavailable(e.to_string()))?;

    Ok(())
}

pub(crate) async fn generate_tokens(state: &AppState, user: &User) -> AppResult<(String, String)> {
    let now = Utc::now();

    let claims = Claims {
        sub: user.id,
        role: format!("{:?}", user.role).to_lowercase(),
        iat: now.timestamp() as usize,
        exp: (now + Duration::hours(state.config.jwt_expiration_hours)).timestamp() as usize,
        iss: Some(JWT_ISSUER.to_string()),
        aud: Some(JWT_AUDIENCE.to_string()),
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

    // ── Cookie attribute coverage ──────────────────────────────────────────

    #[test]
    fn session_cookie_carries_secure_flag_in_production() {
        let cookie = build_session_cookie(COOKIE_ACCESS, "tok".into(), 3600, /*prod*/ true);
        assert!(cookie.secure().unwrap_or(false));
        assert!(cookie.http_only().unwrap_or(false));
        assert_eq!(cookie.path(), Some("/"));
    }

    #[test]
    fn session_cookie_omits_secure_flag_outside_production() {
        let cookie = build_session_cookie(COOKIE_ACCESS, "tok".into(), 3600, /*prod*/ false);
        assert!(!cookie.secure().unwrap_or(false));
        assert!(cookie.http_only().unwrap_or(false));
    }

    #[test]
    fn session_cookie_uses_samesite_lax() {
        // SameSite=Lax — required so the Stripe-checkout return navigation
        // still ships the cookie. Strict would strip it on the cross-site →
        // same-site top-level navigation back to `/dashboard?...`.
        let cookie = build_session_cookie(COOKIE_ACCESS, "tok".into(), 3600, true);
        assert_eq!(cookie.same_site(), Some(SameSite::Lax));
    }

    #[test]
    fn clear_cookie_uses_zero_max_age() {
        let cookie = clear_session_cookie(COOKIE_ACCESS, true);
        assert_eq!(cookie.max_age(), Some(CookieDuration::ZERO));
        assert_eq!(cookie.value(), "");
        assert!(cookie.http_only().unwrap_or(false));
    }
}
