//! ADM-06: CRUD endpoints for the admin IP allowlist.
//!
//! All four routes mount under `/api/admin/security/ip-allowlist` and are
//! gated by [`PrivilegedUser`] + an explicit per-action permission check
//! against the FDN-07 policy. Reads require `admin.ip_allowlist.read`,
//! mutations require `admin.ip_allowlist.manage`. Both are seeded for
//! the `admin` role only — support staff intentionally cannot touch the
//! allowlist (see migration `059_admin_ip_allowlist.sql`).
//!
//! Every mutation writes one row to `admin_actions` via
//! [`crate::services::audit::audit_admin`] so the change is recoverable
//! from the audit log alone.

use axum::{
    extract::{Path, State},
    routing::{delete, get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{
    error::{AppError, AppResult},
    extractors::{ClientInfo, PrivilegedUser},
    security::ip_allowlist::{self, AllowlistEntry, CreateAllowlistInput},
    services::audit::audit_admin_priv,
    AppState,
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(list_entries).post(create_entry))
        .route("/{id}", delete(delete_entry))
        .route("/{id}/toggle", post(toggle_entry))
}

// ── DTOs ────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, ToSchema)]
pub struct AllowlistResponse {
    pub data: Vec<AllowlistEntry>,
    pub total: i64,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct ToggleRequest {
    pub is_active: bool,
}

// ── Handlers ────────────────────────────────────────────────────────────

#[utoipa::path(
    get,
    path = "/api/admin/security/ip-allowlist",
    tag = "admin-security",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Allowlist entries", body = AllowlistResponse),
        (status = 403, description = "Forbidden")
    )
)]
pub(crate) async fn list_entries(
    State(state): State<AppState>,
    admin: PrivilegedUser,
) -> AppResult<Json<AllowlistResponse>> {
    admin.require(&state.policy, "admin.ip_allowlist.read")?;
    let data = ip_allowlist::list_all(&state.db).await?;
    let total = data.len() as i64;
    Ok(Json(AllowlistResponse { data, total }))
}

#[utoipa::path(
    post,
    path = "/api/admin/security/ip-allowlist",
    tag = "admin-security",
    security(("bearer_auth" = [])),
    request_body = CreateAllowlistInput,
    responses(
        (status = 200, description = "New allowlist entry", body = AllowlistEntry),
        (status = 400, description = "Invalid CIDR or label"),
        (status = 403, description = "Forbidden"),
        (status = 409, description = "CIDR already on the list")
    )
)]
pub(crate) async fn create_entry(
    State(state): State<AppState>,
    admin: PrivilegedUser,
    client: ClientInfo,
    Json(input): Json<CreateAllowlistInput>,
) -> AppResult<Json<AllowlistEntry>> {
    admin.require(&state.policy, "admin.ip_allowlist.manage")?;

    let entry = ip_allowlist::create(&state.db, admin.user_id, &input).await?;

    audit_admin_priv(
        &state.db,
        &admin,
        &client,
        "admin.ip_allowlist.create",
        "admin_ip_allowlist",
        entry.id,
        serde_json::json!({
            "cidr": entry.cidr,
            "label": entry.label,
            "is_active": entry.is_active,
        }),
    )
    .await;

    Ok(Json(entry))
}

#[utoipa::path(
    delete,
    path = "/api/admin/security/ip-allowlist/{id}",
    tag = "admin-security",
    security(("bearer_auth" = [])),
    params(("id" = Uuid, Path, description = "Allowlist entry id")),
    responses(
        (status = 200, description = "Entry removed"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Entry not found")
    )
)]
pub(crate) async fn delete_entry(
    State(state): State<AppState>,
    admin: PrivilegedUser,
    client: ClientInfo,
    Path(id): Path<Uuid>,
) -> AppResult<Json<serde_json::Value>> {
    admin.require(&state.policy, "admin.ip_allowlist.manage")?;

    // Snapshot for audit metadata BEFORE deletion so the row contents
    // survive the operation.
    let snapshot = ip_allowlist::get(&state.db, id).await?;
    let removed = ip_allowlist::delete(&state.db, id).await?;
    if !removed {
        return Err(AppError::NotFound("Allowlist entry not found".to_string()));
    }

    let metadata = match &snapshot {
        Some(entry) => serde_json::json!({
            "cidr": entry.cidr,
            "label": entry.label,
            "was_active": entry.is_active,
        }),
        None => serde_json::json!({}),
    };

    audit_admin_priv(
        &state.db,
        &admin,
        &client,
        "admin.ip_allowlist.delete",
        "admin_ip_allowlist",
        id,
        metadata,
    )
    .await;

    Ok(Json(serde_json::json!({
        "id": id,
        "deleted": true,
    })))
}

#[utoipa::path(
    post,
    path = "/api/admin/security/ip-allowlist/{id}/toggle",
    tag = "admin-security",
    security(("bearer_auth" = [])),
    params(("id" = Uuid, Path, description = "Allowlist entry id")),
    request_body = ToggleRequest,
    responses(
        (status = 200, description = "Updated entry", body = AllowlistEntry),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Entry not found")
    )
)]
pub(crate) async fn toggle_entry(
    State(state): State<AppState>,
    admin: PrivilegedUser,
    client: ClientInfo,
    Path(id): Path<Uuid>,
    Json(req): Json<ToggleRequest>,
) -> AppResult<Json<AllowlistEntry>> {
    admin.require(&state.policy, "admin.ip_allowlist.manage")?;

    let entry = ip_allowlist::set_active(&state.db, id, req.is_active)
        .await?
        .ok_or_else(|| AppError::NotFound("Allowlist entry not found".to_string()))?;

    audit_admin_priv(
        &state.db,
        &admin,
        &client,
        "admin.ip_allowlist.toggle",
        "admin_ip_allowlist",
        entry.id,
        serde_json::json!({
            "cidr": entry.cidr,
            "label": entry.label,
            "is_active": entry.is_active,
        }),
    )
    .await;

    Ok(Json(entry))
}
