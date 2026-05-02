//! ADM-10: admin members surface — search + manual create.
//!
//! Mounted at `/api/admin/members` *alongside* the legacy member
//! routes that live in [`crate::handlers::admin`]. The split keeps
//! the new permission-checked surface (`admin.member.read|create`)
//! isolated from the historical AdminUser-only routes that the
//! frontend already consumes; future hardening rounds can migrate
//! the legacy routes here without breaking the wire shape.
//!
//! Endpoints:
//!
//! * `GET  /search?q=&role=&status=&limit=&offset=`
//!   Indexed substring search over email + name with optional role
//!   and lifecycle-status filters. Backed by `pg_trgm` GIN indexes
//!   from migration `065_users_search.sql`.
//!
//! * `POST /`
//!   Manually create a member account with an explicit role and an
//!   optional one-shot temporary password. When no password is set,
//!   the account is created in a "disabled" state and the operator
//!   can send a reset link via `POST .../force-password-reset`.
//!   Seeding `role: admin` is allowed only when the actor also holds
//!   `admin.role.manage` (same gate as assigning roles elsewhere).
//!   The created row is audited under `admin.member.create`.

use axum::{
    extract::{Query, State},
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use validator::Validate;

use crate::{
    db::{self, UserStatusFilter},
    error::{AppError, AppResult},
    extractors::{ClientInfo, PrivilegedUser},
    models::{PaginatedResponse, UserResponse, UserRole},
    services::audit::audit_admin_priv,
    AppState,
};

const PERM_READ: &str = "admin.member.read";
const PERM_CREATE: &str = "admin.member.create";
const PERM_ROLE_MANAGE: &str = "admin.role.manage";

const DEFAULT_LIMIT: i64 = 25;
const MAX_LIMIT: i64 = 200;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/search", get(search))
        .route("/", post(create))
}

// ── DTOs ───────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct SearchQuery {
    /// Free-text substring matched against `email` and `name`.
    /// Empty / missing → unfiltered list.
    #[serde(default)]
    pub q: Option<String>,
    /// Role filter — must match the wire-form (`member`, `author`,
    /// `support`, `admin`). Unknown values are rejected with `400`.
    #[serde(default)]
    pub role: Option<String>,
    /// Lifecycle filter — `active`, `suspended`, `banned`, `unverified`.
    /// Unknown values are rejected with `400`.
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub limit: Option<i64>,
    #[serde(default)]
    pub offset: Option<i64>,
}

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct CreateMemberRequest {
    #[validate(email(message = "Invalid email address"))]
    pub email: String,
    #[validate(length(min = 1, max = 200, message = "Name is required (1-200 chars)"))]
    pub name: String,
    /// Role to seed the account with. `admin` requires
    /// `admin.role.manage` in addition to `admin.member.create`.
    pub role: UserRole,
    /// Optional one-shot temporary password. When `None` the account
    /// is created in a disabled state (no login until the user
    /// completes the password-reset / invite flow).
    #[validate(length(min = 12, message = "Temp password must be ≥ 12 characters"))]
    pub temp_password: Option<String>,
    /// Mark the email as already verified (operator vouches that
    /// they typed the address themselves, e.g. seeding a colleague).
    #[serde(default)]
    pub email_verified: bool,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct CreateMemberResponse {
    pub user: UserResponse,
    /// `true` when no temp_password was supplied and the operator
    /// must follow up with a password-reset / invite link out of band.
    pub requires_password_setup: bool,
}

// ── Handlers ───────────────────────────────────────────────────────────

#[utoipa::path(
    get,
    path = "/api/admin/members/search",
    tag = "admin-members",
    operation_id = "admin_members_search",
    security(("bearer_auth" = [])),
    params(
        ("q"      = Option<String>, Query, description = "Free-text substring across email + name"),
        ("role"   = Option<String>, Query, description = "Role filter (member|author|support|admin)"),
        ("status" = Option<String>, Query, description = "Status filter (active|suspended|banned|unverified)"),
        ("limit"  = Option<i64>,    Query, description = "Page size (1-200, default 25)"),
        ("offset" = Option<i64>,    Query, description = "Cursor offset"),
    ),
    responses(
        (status = 200, description = "Paginated member rows", body = PaginatedResponse<UserResponse>),
        (status = 400, description = "Invalid filter"),
        (status = 401, description = "Unauthenticated"),
        (status = 403, description = "Forbidden")
    )
)]
pub async fn search(
    State(state): State<AppState>,
    privileged: PrivilegedUser,
    Query(q): Query<SearchQuery>,
) -> AppResult<Json<PaginatedResponse<UserResponse>>> {
    privileged.require(&state.policy, PERM_READ)?;

    let limit = q.limit.unwrap_or(DEFAULT_LIMIT).clamp(1, MAX_LIMIT);
    let offset = q.offset.unwrap_or(0).max(0);

    let role_filter = match q.role.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        None => None,
        Some(r) => Some(
            UserRole::from_str_lower(&r.to_lowercase())
                .ok_or_else(|| AppError::BadRequest(format!("unknown role: {r}")))?,
        ),
    };

    let status_filter = match q.status.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        None => None,
        Some(s) => Some(
            UserStatusFilter::from_wire(&s.to_lowercase())
                .ok_or_else(|| AppError::BadRequest(format!("unknown status: {s}")))?,
        ),
    };

    let (users, total) = db::search_users(
        &state.db,
        q.q.as_deref(),
        role_filter.as_ref(),
        status_filter,
        limit,
        offset,
    )
    .await?;

    let per_page = limit;
    let page = (offset / per_page.max(1)) + 1;
    let total_pages = (total as f64 / per_page as f64).ceil() as i64;

    Ok(Json(PaginatedResponse {
        data: users.into_iter().map(UserResponse::from).collect(),
        total,
        page,
        per_page,
        total_pages,
    }))
}

#[utoipa::path(
    post,
    path = "/api/admin/members",
    tag = "admin-members",
    operation_id = "admin_members_create",
    security(("bearer_auth" = [])),
    request_body = CreateMemberRequest,
    responses(
        (status = 201, description = "Member created", body = CreateMemberResponse),
        (status = 400, description = "Validation failed"),
        (status = 401, description = "Unauthenticated"),
        (status = 403, description = "Forbidden"),
        (status = 409, description = "Email already exists")
    )
)]
pub async fn create(
    State(state): State<AppState>,
    privileged: PrivilegedUser,
    client: ClientInfo,
    Json(req): Json<CreateMemberRequest>,
) -> AppResult<(StatusCode, Json<CreateMemberResponse>)> {
    privileged.require(&state.policy, PERM_CREATE)?;
    req.validate()
        .map_err(|e| AppError::BadRequest(e.to_string()))?;

    // Privilege escalation: only operators who may assign roles (same
    // catalogue key as the role-picker UI) may seed an admin directly.
    if matches!(req.role, UserRole::Admin) {
        privileged.require(&state.policy, PERM_ROLE_MANAGE)?;
    }

    let normalized = db::normalize_email(&req.email);
    if db::find_user_by_email(&state.db, &normalized)
        .await?
        .is_some()
    {
        return Err(AppError::Conflict(
            "A user with that email already exists".to_string(),
        ));
    }

    // Hash the optional temp password on the blocking pool (W3-2). The
    // credential format mirrors /api/auth/register so login verification
    // stays a single code path.
    let password_hash = match req.temp_password.clone() {
        Some(pw) => Some(crate::common::password::hash_password(pw).await?),
        None => None,
    };
    let requires_password_setup = password_hash.is_none();

    let user = db::admin_create_user(
        &state.db,
        &req.email,
        &req.name,
        &req.role,
        password_hash.as_deref(),
        req.email_verified,
    )
    .await?;

    audit_admin_priv(
        &state.db,
        &privileged,
        &client,
        "admin.member.create",
        "user",
        user.id.to_string(),
        serde_json::json!({
            "email": user.email,
            "role": user.role,
            "email_verified": req.email_verified,
            "requires_password_setup": requires_password_setup,
        }),
    )
    .await;

    Ok((
        StatusCode::CREATED,
        Json(CreateMemberResponse {
            user: user.into(),
            requires_password_setup,
        }),
    ))
}
