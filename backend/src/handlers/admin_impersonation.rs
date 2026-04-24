//! ADM-07: admin impersonation endpoints.
//!
//! ## Routes
//!
//! Mounted by `main.rs` under `/api/admin/security/impersonation`:
//!
//!   * `POST   /`              — [`mint`]   (gated `user.impersonate`)
//!   * `GET    /`              — [`list`]   (gated `user.impersonate`)
//!   * `GET    /{id}`          — [`get_one`]   (gated `user.impersonate`)
//!   * `POST   /{id}/revoke`   — [`revoke`] (gated `user.impersonate`)
//!
//! And the self-exit route lives on the auth router (`main.rs` mounts
//! it at `/api/auth/impersonation/exit`) and is called by the
//! impersonated session itself with the impersonation JWT in `Authorization`.
//!
//! ## Token shape
//!
//! [`mint`] returns an HS256 JWT with the standard claims plus the
//! ADM-07 impersonation triplet:
//!
//! ```json
//! { "sub": "<target>", "role": "<target_role>", "exp": ..., "iat": ...,
//!   "imp_actor": "<admin>", "imp_actor_role": "admin",
//!   "imp_session": "<row id>" }
//! ```
//!
//! The `exp` claim mirrors the row's `expires_at`. The row is the
//! authoritative TTL — see `security::impersonation` for why.
//!
//! ## RBAC matrix
//!
//! All four admin-side routes require `user.impersonate`, which the
//! `058_admin_lifecycle_perms.sql` migration grants only to the `admin`
//! role. Support staff cannot mint, list, view, or revoke impersonation
//! sessions. The exit route requires `AuthUser` only (any role) — but
//! the JWT's `imp_session` claim must point at a still-active row.

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use chrono::{DateTime, Utc};
use jsonwebtoken::{encode, EncodingKey, Header};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

use crate::{
    db,
    error::{AppError, AppResult},
    extractors::{AuthUser, Claims, ClientInfo, PrivilegedUser, JWT_AUDIENCE, JWT_ISSUER},
    models::UserRole,
    notifications::send::{send_notification, Recipient, SendOptions},
    security::impersonation::{
        self, CreateImpersonationInput, ImpersonationSession, MAX_MINTS_PER_MINUTE,
    },
    services::audit::audit_admin_priv,
    AppState,
};

const IMPERSONATE_PERMISSION: &str = "user.impersonate";
const MINT_RATE_WINDOW_SECS: i64 = 60;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(list).post(mint))
        .route("/{id}", get(get_one))
        .route("/{id}/revoke", post(revoke))
}

/// Standalone router for the self-exit endpoint. Mounted by `main.rs`
/// under `/api/auth/impersonation` so the impersonated session reaches
/// it without traversing the IP-allowlist-gated `/api/admin/*` tree.
pub fn auth_router() -> Router<AppState> {
    Router::new().route("/exit", post(exit))
}

// ── DTOs ────────────────────────────────────────────────────────────────

/// Response body for `POST /` (mint). Contains the freshly-issued
/// impersonation JWT plus the row metadata so the SPA does not need a
/// follow-up GET to render the banner.
#[derive(Debug, Serialize, ToSchema)]
pub struct MintResponse {
    pub access_token: String,
    /// Echoed `expires_at` — same value carried in the JWT's `exp`.
    pub expires_at: DateTime<Utc>,
    pub session: ImpersonationSession,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ListResponse {
    pub data: Vec<ImpersonationSession>,
    pub total: i64,
    /// Cursor (`issued_at` of the last row) to pass back via `?after=`
    /// to fetch the next page; `None` when the page is not full.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<DateTime<Utc>>,
}

/// Pagination + filter inputs for `GET /api/admin/security/impersonation`.
#[derive(Debug, Deserialize, IntoParams, Default)]
#[into_params(parameter_in = Query)]
pub struct ListQuery {
    /// Cursor — `issued_at` of the last row from the previous page.
    /// Returns the newest page when omitted.
    #[serde(default)]
    pub after: Option<DateTime<Utc>>,
    /// Page size, clamped to 1..=100. Defaults to 25 when omitted.
    #[serde(default)]
    pub limit: Option<i64>,
}

#[derive(Debug, Deserialize, ToSchema, Default)]
pub struct RevokeRequest {
    /// Optional free-text justification for the revocation. Stored on
    /// the row and surfaced in the audit-log metadata.
    #[serde(default)]
    pub reason: Option<String>,
}

// ── Handlers ────────────────────────────────────────────────────────────

#[utoipa::path(
    post,
    path = "/api/admin/security/impersonation",
    tag = "admin-impersonation",
    operation_id = "admin_impersonation_mint",
    security(("bearer_auth" = [])),
    request_body = CreateImpersonationInput,
    responses(
        (status = 200, description = "Impersonation token", body = MintResponse),
        (status = 400, description = "Invalid input or unsafe target"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Target user not found")
    )
)]
pub(crate) async fn mint(
    State(state): State<AppState>,
    admin: PrivilegedUser,
    client: ClientInfo,
    Json(input): Json<CreateImpersonationInput>,
) -> AppResult<Json<MintResponse>> {
    admin.require(&state.policy, IMPERSONATE_PERMISSION)?;
    impersonation::validate_input(&input)?;

    // Per-actor burst cap (defence-in-depth on top of admin auth + IP
    // allowlist + permission gate). Counts the canonical session rows
    // so the cap is portable across instances and survives restart.
    let recent =
        impersonation::count_recent_for_actor(&state.db, admin.user_id, MINT_RATE_WINDOW_SECS)
            .await?;
    if recent >= MAX_MINTS_PER_MINUTE {
        tracing::warn!(
            actor_id = %admin.user_id,
            recent_mints = recent,
            "impersonation mint blocked: per-actor rate limit"
        );
        metrics::counter!("impersonation_mint_rate_limited_total").increment(1);
        return Err(AppError::TooManyRequests);
    }

    let target_role =
        impersonation::assert_target_safe(&state.db, admin.user_id, input.target_user_id).await?;

    let ttl = impersonation::resolve_ttl(input.ttl_minutes);

    let session = impersonation::create(
        &state.db,
        admin.user_id,
        admin.role,
        input.target_user_id,
        &input.reason,
        ttl,
        client.ip,
        client.user_agent.as_deref(),
    )
    .await?;

    let access_token = sign_impersonation_jwt(
        &state.config.jwt_secret,
        input.target_user_id,
        target_role,
        admin.user_id,
        admin.role,
        session.id,
        session.issued_at,
        session.expires_at,
    )?;

    audit_admin_priv(
        &state.db,
        &admin,
        &client,
        "admin.impersonation.start",
        "impersonation_session",
        session.id,
        serde_json::json!({
            "target_user_id": session.target_user_id,
            "target_role":    target_role.as_str(),
            "expires_at":     session.expires_at,
            "ttl_minutes":    ttl.num_minutes(),
            "reason":         session.reason,
        }),
    )
    .await;

    // GDPR Art. 32 contemporaneous notification. Best-effort: a
    // notification failure must never block the support flow because
    // the audit row is already persisted. We dispatch in the
    // background so the admin response latency is unaffected.
    notify_target_session_started(&state, &session, target_role).await;

    Ok(Json(MintResponse {
        access_token,
        expires_at: session.expires_at,
        session,
    }))
}

/// Fire-and-forget the "support session started" email to the target
/// user. Errors are logged at WARN — never propagated.
async fn notify_target_session_started(
    state: &AppState,
    session: &ImpersonationSession,
    _target_role: UserRole,
) {
    let target = match db::find_user_by_id(&state.db, session.target_user_id).await {
        Ok(Some(u)) => u,
        Ok(None) => {
            tracing::warn!(
                target_user_id = %session.target_user_id,
                "impersonation notify skipped: target user vanished between mint and notify"
            );
            return;
        }
        Err(err) => {
            tracing::warn!(
                error = %err,
                target_user_id = %session.target_user_id,
                "impersonation notify skipped: lookup error"
            );
            return;
        }
    };

    let ttl_minutes = (session.expires_at - session.issued_at).num_minutes();
    let ctx = serde_json::json!({
        "name":         target.name,
        "reason":       session.reason,
        "issued_at":    session.issued_at.to_rfc3339(),
        "expires_at":   session.expires_at.to_rfc3339(),
        "ttl_minutes":  ttl_minutes,
        "app_url":      state.config.app_url,
        "year":         chrono::Utc::now().format("%Y").to_string(),
    });
    if let Err(err) = send_notification(
        &state.db,
        "admin.impersonation_started",
        &Recipient::User {
            user_id: target.id,
            email: target.email.clone(),
        },
        ctx,
        SendOptions::default(),
    )
    .await
    {
        tracing::warn!(
            error = %err,
            target_user_id = %target.id,
            "failed to enqueue impersonation-started email"
        );
    }
}

#[utoipa::path(
    get,
    path = "/api/admin/security/impersonation",
    tag = "admin-impersonation",
    operation_id = "admin_impersonation_list",
    params(ListQuery),
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Active impersonation sessions", body = ListResponse),
        (status = 403, description = "Forbidden")
    )
)]
pub(crate) async fn list(
    State(state): State<AppState>,
    admin: PrivilegedUser,
    Query(q): Query<ListQuery>,
) -> AppResult<Json<ListResponse>> {
    admin.require(&state.policy, IMPERSONATE_PERMISSION)?;
    let limit = q
        .limit
        .unwrap_or(impersonation::LIST_DEFAULT_LIMIT)
        .clamp(1, impersonation::LIST_MAX_LIMIT);
    let data = impersonation::list_active_paginated(&state.db, q.after, limit).await?;
    let total = data.len() as i64;
    let next_cursor = if total == limit {
        data.last().map(|s| s.issued_at)
    } else {
        None
    };
    Ok(Json(ListResponse {
        data,
        total,
        next_cursor,
    }))
}

#[utoipa::path(
    get,
    path = "/api/admin/security/impersonation/{id}",
    tag = "admin-impersonation",
    operation_id = "admin_impersonation_get_one",
    security(("bearer_auth" = [])),
    params(("id" = Uuid, Path, description = "Impersonation session id")),
    responses(
        (status = 200, description = "Session row", body = ImpersonationSession),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Session not found")
    )
)]
pub(crate) async fn get_one(
    State(state): State<AppState>,
    admin: PrivilegedUser,
    Path(id): Path<Uuid>,
) -> AppResult<Json<ImpersonationSession>> {
    admin.require(&state.policy, IMPERSONATE_PERMISSION)?;
    let session = impersonation::get(&state.db, id)
        .await?
        .ok_or_else(|| AppError::NotFound("Impersonation session not found".into()))?;
    Ok(Json(session))
}

#[utoipa::path(
    post,
    path = "/api/admin/security/impersonation/{id}/revoke",
    tag = "admin-impersonation",
    operation_id = "admin_impersonation_revoke",
    security(("bearer_auth" = [])),
    params(("id" = Uuid, Path, description = "Impersonation session id")),
    request_body = RevokeRequest,
    responses(
        (status = 200, description = "Session revoked", body = ImpersonationSession),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Session not found or already revoked")
    )
)]
pub(crate) async fn revoke(
    State(state): State<AppState>,
    admin: PrivilegedUser,
    client: ClientInfo,
    Path(id): Path<Uuid>,
    Json(req): Json<RevokeRequest>,
) -> AppResult<Json<ImpersonationSession>> {
    admin.require(&state.policy, IMPERSONATE_PERMISSION)?;

    let session = impersonation::revoke(&state.db, id, admin.user_id, req.reason.as_deref())
        .await?
        .ok_or_else(|| {
            AppError::NotFound("Impersonation session not found or already revoked".into())
        })?;

    audit_admin_priv(
        &state.db,
        &admin,
        &client,
        "admin.impersonation.revoke",
        "impersonation_session",
        session.id,
        serde_json::json!({
            "target_user_id": session.target_user_id,
            "actor_user_id":  session.actor_user_id,
            "reason":         req.reason,
        }),
    )
    .await;

    Ok(Json(session))
}

#[utoipa::path(
    post,
    path = "/api/auth/impersonation/exit",
    tag = "auth",
    operation_id = "auth_impersonation_exit",
    security(("bearer_auth" = [])),
    responses(
        (status = 204, description = "Impersonation session ended"),
        (status = 400, description = "Caller is not currently impersonating"),
        (status = 401, description = "Unauthenticated")
    )
)]
pub(crate) async fn exit(
    State(state): State<AppState>,
    auth: AuthUser,
    client: ClientInfo,
) -> AppResult<StatusCode> {
    // Only meaningful when the caller's JWT carries an impersonation
    // session — bare access tokens have nothing to revoke.
    let session_id = auth
        .impersonation_session_id
        .ok_or_else(|| AppError::BadRequest("Not currently impersonating.".into()))?;
    let actor_id = auth.impersonator_id.ok_or(AppError::Unauthorized)?;

    let revoked = impersonation::revoke(&state.db, session_id, actor_id, Some("self-exit")).await?;

    // Best-effort audit row recorded against the real admin actor. We
    // build the AdminAction by hand because the impersonated session's
    // AuthUser is not a PrivilegedUser, so the audit_admin_priv helper
    // does not fit the call site.
    let actor_role = db::find_user_by_id(&state.db, actor_id)
        .await
        .ok()
        .flatten()
        .map(|u| u.role)
        .unwrap_or(UserRole::Admin);

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
        })),
    )
    .await;

    Ok(StatusCode::NO_CONTENT)
}

// ── Helpers ─────────────────────────────────────────────────────────────

#[allow(clippy::too_many_arguments)]
fn sign_impersonation_jwt(
    secret: &str,
    target_user_id: Uuid,
    target_role: UserRole,
    actor_user_id: Uuid,
    actor_role: UserRole,
    session_id: Uuid,
    issued_at: DateTime<Utc>,
    expires_at: DateTime<Utc>,
) -> AppResult<String> {
    let claims = Claims {
        sub: target_user_id,
        role: target_role.as_str().to_string(),
        iat: issued_at.timestamp() as usize,
        exp: expires_at.timestamp() as usize,
        iss: Some(JWT_ISSUER.to_string()),
        aud: Some(JWT_AUDIENCE.to_string()),
        imp_actor: Some(actor_user_id),
        imp_actor_role: Some(actor_role.as_str().to_string()),
        imp_session: Some(session_id),
    };
    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .map_err(|err| AppError::Internal(anyhow::anyhow!("impersonation token sign failed: {err}")))
}
