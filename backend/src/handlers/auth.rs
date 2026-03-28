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
    extractors::{AuthUser, Claims},
    models::*,
    AppState,
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/register", post(register))
        .route("/login", post(login))
        .route("/refresh", post(refresh))
        .route("/me", axum::routing::get(me))
        .route("/logout", post(logout))
}

async fn register(
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

    Ok(Json(AuthResponse {
        user: user.into(),
        access_token,
        refresh_token,
    }))
}

async fn login(
    State(state): State<AppState>,
    Json(req): Json<LoginRequest>,
) -> AppResult<Json<AuthResponse>> {
    req.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;

    let user = db::find_user_by_email(&state.db, &req.email)
        .await?
        .ok_or(AppError::Unauthorized)?;

    let parsed_hash =
        PasswordHash::new(&user.password_hash).map_err(|_| AppError::Unauthorized)?;

    Argon2::default()
        .verify_password(req.password.as_bytes(), &parsed_hash)
        .map_err(|_| AppError::Unauthorized)?;

    let (access_token, refresh_token) = generate_tokens(&state, &user).await?;

    Ok(Json(AuthResponse {
        user: user.into(),
        access_token,
        refresh_token,
    }))
}

async fn refresh(
    State(state): State<AppState>,
    Json(req): Json<RefreshRequest>,
) -> AppResult<Json<TokenResponse>> {
    let token_hash = hash_token(&req.refresh_token);

    let stored = db::find_refresh_token(&state.db, &token_hash)
        .await?
        .ok_or(AppError::Unauthorized)?;

    // Delete the used refresh token (rotation)
    db::delete_refresh_token(&state.db, &token_hash).await?;

    let user = db::find_user_by_id(&state.db, stored.user_id)
        .await?
        .ok_or(AppError::Unauthorized)?;

    let (access_token, refresh_token) = generate_tokens(&state, &user).await?;

    Ok(Json(TokenResponse {
        access_token,
        refresh_token,
    }))
}

async fn me(State(state): State<AppState>, auth: AuthUser) -> AppResult<Json<UserResponse>> {
    let user = db::find_user_by_id(&state.db, auth.user_id)
        .await?
        .ok_or(AppError::NotFound("User not found".to_string()))?;

    Ok(Json(user.into()))
}

async fn logout(State(state): State<AppState>, auth: AuthUser) -> AppResult<Json<serde_json::Value>> {
    db::delete_user_refresh_tokens(&state.db, auth.user_id).await?;
    Ok(Json(serde_json::json!({ "message": "Logged out" })))
}

// ── Helpers ─────────────────────────────────────────────────────────────

async fn generate_tokens(state: &AppState, user: &User) -> AppResult<(String, String)> {
    let now = Utc::now();

    let claims = Claims {
        sub: user.id,
        role: format!("{:?}", user.role).to_lowercase(),
        iat: now.timestamp() as usize,
        exp: (now + Duration::hours(state.config.jwt_expiration_hours)).timestamp() as usize,
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

    db::store_refresh_token(&state.db, user.id, &token_hash, expires_at).await?;

    Ok((access_token, refresh_token))
}

fn hash_token(token: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(token.as_bytes());
    format!("{:x}", hasher.finalize())
}
