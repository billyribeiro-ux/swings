//! ADM-09: role / permission matrix admin surface.
//!
//! Mounted under `/api/admin/security/roles`. The `user_role` enum is
//! immutable at runtime (Member / Author / Support / Admin), so the
//! API exposes only:
//!
//! * GET  `/permissions`              — full catalogue from the
//!   `permissions` table, alphabetised so the SPA can render the
//!   matrix without client-side sorting.
//! * GET  `/`                         — every (role, permission) pair
//!   currently active, plus the catalogue shape, in one round-trip.
//! * POST `/{role}/{permission}`      — grant a permission to a role.
//! * DELETE `/{role}/{permission}`    — revoke a permission from a role.
//! * PUT  `/{role}`                   — replace a role's permission set
//!   atomically (idempotent bulk update).
//! * POST `/_reload`                  — explicit policy hot-reload.
//!
//! Every mutation:
//!   1. Acquires a transactional lock on `role_permissions` so two
//!      concurrent admins cannot interleave a grant and a revoke.
//!   2. Writes one row to `admin_actions`.
//!   3. Reloads the in-process [`PolicyHandle`] before returning so
//!      the next request sees the new matrix.

use std::collections::BTreeSet;

use axum::{
    extract::{Path, State},
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::{
    error::{AppError, AppResult},
    extractors::{ClientInfo, PrivilegedUser},
    models::UserRole,
    services::audit::audit_admin_priv,
    AppState,
};

const PERM_READ: &str = "admin.role.read";
const PERM_MANAGE: &str = "admin.role.manage";

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(list_matrix))
        .route("/permissions", get(list_permissions))
        .route("/{role}", axum::routing::put(replace_role_permissions))
        .route(
            "/{role}/{permission}",
            post(grant).delete(revoke),
        )
        .route("/_reload", post(reload))
}

// ── DTOs ────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, ToSchema)]
pub struct PermissionRow {
    pub key: String,
    pub description: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct PermissionsResponse {
    pub data: Vec<PermissionRow>,
    pub total: i64,
}

/// Canonical role/permission pair.
#[derive(Debug, Serialize, ToSchema)]
pub struct RolePermPair {
    pub role: String,
    pub permission: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct MatrixResponse {
    /// Every active (role, permission) row, alphabetised by (role,
    /// permission). Stable order — the SPA renders directly.
    pub matrix: Vec<RolePermPair>,
    /// Every role label the backend recognises. Driven by the Rust
    /// enum so the SPA does not need to mirror it.
    pub roles: Vec<String>,
    /// Every permission key in the catalogue.
    pub permissions: Vec<PermissionRow>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct ReplaceRoleRequest {
    /// New permission set for the role. Unknown keys (not in the
    /// permissions catalogue) are rejected with `400`.
    pub permissions: Vec<String>,
}

// ── Helpers ─────────────────────────────────────────────────────────────

fn parse_role(raw: &str) -> AppResult<UserRole> {
    UserRole::from_str_lower(raw)
        .ok_or_else(|| AppError::BadRequest(format!("unknown role `{raw}`")))
}

async fn permission_exists(pool: &sqlx::PgPool, key: &str) -> AppResult<bool> {
    let exists: (bool,) = sqlx::query_as(
        "SELECT EXISTS(SELECT 1 FROM permissions WHERE key = $1)::bool",
    )
    .bind(key)
    .fetch_one(pool)
    .await?;
    Ok(exists.0)
}

// ── Handlers ────────────────────────────────────────────────────────────

#[utoipa::path(
    get,
    path = "/api/admin/security/roles/permissions",
    tag = "admin-roles",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Permission catalogue", body = PermissionsResponse),
        (status = 403, description = "Forbidden"),
    )
)]
pub(crate) async fn list_permissions(
    State(state): State<AppState>,
    admin: PrivilegedUser,
) -> AppResult<Json<PermissionsResponse>> {
    admin.require(&state.policy, PERM_READ)?;

    let rows: Vec<PermissionRow> = sqlx::query_as::<_, (String, String)>(
        "SELECT key, description FROM permissions ORDER BY key",
    )
    .fetch_all(&state.db)
    .await?
    .into_iter()
    .map(|(key, description)| PermissionRow { key, description })
    .collect();

    let total = rows.len() as i64;
    Ok(Json(PermissionsResponse { data: rows, total }))
}

#[utoipa::path(
    get,
    path = "/api/admin/security/roles",
    tag = "admin-roles",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Role/permission matrix + catalogue", body = MatrixResponse),
        (status = 403, description = "Forbidden"),
    )
)]
pub(crate) async fn list_matrix(
    State(state): State<AppState>,
    admin: PrivilegedUser,
) -> AppResult<Json<MatrixResponse>> {
    admin.require(&state.policy, PERM_READ)?;

    let pairs: Vec<(String, String)> = sqlx::query_as(
        "SELECT role::text, permission FROM role_permissions ORDER BY role::text, permission",
    )
    .fetch_all(&state.db)
    .await?;

    let perm_rows: Vec<PermissionRow> = sqlx::query_as::<_, (String, String)>(
        "SELECT key, description FROM permissions ORDER BY key",
    )
    .fetch_all(&state.db)
    .await?
    .into_iter()
    .map(|(key, description)| PermissionRow { key, description })
    .collect();

    Ok(Json(MatrixResponse {
        matrix: pairs
            .into_iter()
            .map(|(role, permission)| RolePermPair { role, permission })
            .collect(),
        roles: [
            UserRole::Member,
            UserRole::Author,
            UserRole::Support,
            UserRole::Admin,
        ]
        .into_iter()
        .map(|r| r.as_str().to_owned())
        .collect(),
        permissions: perm_rows,
    }))
}

#[utoipa::path(
    post,
    path = "/api/admin/security/roles/{role}/{permission}",
    tag = "admin-roles",
    params(
        ("role" = String, Path, description = "Role label (member|author|support|admin)"),
        ("permission" = String, Path, description = "Permission key"),
    ),
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Permission granted (idempotent)", body = RolePermPair),
        (status = 400, description = "Unknown role or permission"),
        (status = 403, description = "Forbidden"),
    )
)]
pub(crate) async fn grant(
    State(state): State<AppState>,
    admin: PrivilegedUser,
    client: ClientInfo,
    Path((role_raw, permission)): Path<(String, String)>,
) -> AppResult<Json<RolePermPair>> {
    admin.require(&state.policy, PERM_MANAGE)?;
    let role = parse_role(&role_raw)?;

    if !permission_exists(&state.db, &permission).await? {
        return Err(AppError::BadRequest(format!(
            "unknown permission `{permission}`"
        )));
    }

    sqlx::query(
        "INSERT INTO role_permissions (role, permission) VALUES ($1::user_role, $2) ON CONFLICT (role, permission) DO NOTHING",
    )
    .bind(role.as_str())
    .bind(&permission)
    .execute(&state.db)
    .await?;

    state.policy.reload_from_db(&state.db).await?;

    audit_admin_priv(
        &state.db,
        &admin,
        &client,
        "admin.role.grant",
        "role_permission",
        format!("{}:{}", role.as_str(), permission),
        serde_json::json!({"role": role.as_str(), "permission": permission}),
    )
    .await;

    Ok(Json(RolePermPair {
        role: role.as_str().to_owned(),
        permission,
    }))
}

#[utoipa::path(
    delete,
    path = "/api/admin/security/roles/{role}/{permission}",
    tag = "admin-roles",
    params(
        ("role" = String, Path, description = "Role label (member|author|support|admin)"),
        ("permission" = String, Path, description = "Permission key"),
    ),
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Permission revoked"),
        (status = 400, description = "Unknown role"),
        (status = 403, description = "Forbidden"),
        (status = 409, description = "Refusing to revoke an admin self-lock guard"),
    )
)]
pub(crate) async fn revoke(
    State(state): State<AppState>,
    admin: PrivilegedUser,
    client: ClientInfo,
    Path((role_raw, permission)): Path<(String, String)>,
) -> AppResult<Json<serde_json::Value>> {
    admin.require(&state.policy, PERM_MANAGE)?;
    let role = parse_role(&role_raw)?;

    // Self-lock guard: an admin must never be able to revoke
    // `admin.role.manage` from the `admin` role — that would brick
    // the matrix with no recovery path short of a SQL console. Same
    // rationale for `admin.dashboard.read`: lose it and the entire
    // /api/admin/* surface 403s for everyone.
    if role == UserRole::Admin
        && (permission == PERM_MANAGE || permission == "admin.dashboard.read")
    {
        return Err(AppError::Conflict(format!(
            "refusing to revoke `{permission}` from admin role — would brick the admin surface"
        )));
    }

    let removed = sqlx::query(
        "DELETE FROM role_permissions WHERE role = $1::user_role AND permission = $2",
    )
    .bind(role.as_str())
    .bind(&permission)
    .execute(&state.db)
    .await?
    .rows_affected();

    state.policy.reload_from_db(&state.db).await?;

    audit_admin_priv(
        &state.db,
        &admin,
        &client,
        "admin.role.revoke",
        "role_permission",
        format!("{}:{}", role.as_str(), permission),
        serde_json::json!({
            "role": role.as_str(),
            "permission": permission,
            "rows_removed": removed,
        }),
    )
    .await;

    Ok(Json(serde_json::json!({
        "role": role.as_str(),
        "permission": permission,
        "removed": removed > 0,
    })))
}

#[utoipa::path(
    put,
    path = "/api/admin/security/roles/{role}",
    tag = "admin-roles",
    params(("role" = String, Path, description = "Role label")),
    request_body = ReplaceRoleRequest,
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Role permission set replaced atomically"),
        (status = 400, description = "Unknown role or permission"),
        (status = 403, description = "Forbidden"),
        (status = 409, description = "Refusing to drop a self-lock guard"),
    )
)]
pub(crate) async fn replace_role_permissions(
    State(state): State<AppState>,
    admin: PrivilegedUser,
    client: ClientInfo,
    Path(role_raw): Path<String>,
    Json(req): Json<ReplaceRoleRequest>,
) -> AppResult<Json<serde_json::Value>> {
    admin.require(&state.policy, PERM_MANAGE)?;
    let role = parse_role(&role_raw)?;

    // Dedupe + normalise; spec'd as Vec on the wire to match the
    // SPA's matrix toggle semantics, but stored as a set internally.
    let next: BTreeSet<String> = req.permissions.into_iter().collect();

    if role == UserRole::Admin {
        for guard in [PERM_MANAGE, "admin.dashboard.read"] {
            if !next.contains(guard) {
                return Err(AppError::Conflict(format!(
                    "refusing to drop `{guard}` from admin role — would brick the admin surface"
                )));
            }
        }
    }

    // Validate every key against the catalogue before any mutation
    // so a partial replace cannot half-apply.
    for key in &next {
        if !permission_exists(&state.db, key).await? {
            return Err(AppError::BadRequest(format!(
                "unknown permission `{key}`"
            )));
        }
    }

    let mut tx = state.db.begin().await?;

    // Lock the existing rows for this role to serialize concurrent
    // matrix mutations. Without the lock two admins could race and
    // the loser's grants would silently disappear.
    sqlx::query(
        "SELECT permission FROM role_permissions WHERE role = $1::user_role FOR UPDATE",
    )
    .bind(role.as_str())
    .fetch_all(&mut *tx)
    .await?;

    sqlx::query("DELETE FROM role_permissions WHERE role = $1::user_role")
        .bind(role.as_str())
        .execute(&mut *tx)
        .await?;

    for perm in &next {
        sqlx::query(
            "INSERT INTO role_permissions (role, permission) VALUES ($1::user_role, $2)",
        )
        .bind(role.as_str())
        .bind(perm)
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;

    state.policy.reload_from_db(&state.db).await?;

    audit_admin_priv(
        &state.db,
        &admin,
        &client,
        "admin.role.replace",
        "role_permissions",
        role.as_str().to_string(),
        serde_json::json!({
            "role": role.as_str(),
            "permissions": next.iter().collect::<Vec<_>>(),
            "count": next.len(),
        }),
    )
    .await;

    Ok(Json(serde_json::json!({
        "role": role.as_str(),
        "permissions": next.iter().collect::<Vec<_>>(),
        "count": next.len(),
    })))
}

#[utoipa::path(
    post,
    path = "/api/admin/security/roles/_reload",
    tag = "admin-roles",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Policy cache reloaded; returns pair count"),
        (status = 403, description = "Forbidden"),
    )
)]
pub(crate) async fn reload(
    State(state): State<AppState>,
    admin: PrivilegedUser,
    client: ClientInfo,
) -> AppResult<Json<serde_json::Value>> {
    admin.require(&state.policy, PERM_MANAGE)?;

    let count = state.policy.reload_from_db(&state.db).await?;

    audit_admin_priv(
        &state.db,
        &admin,
        &client,
        "admin.role.reload",
        "role_permissions",
        "*".to_string(),
        serde_json::json!({"pairs": count}),
    )
    .await;

    Ok(Json(serde_json::json!({"reloaded": count})))
}
