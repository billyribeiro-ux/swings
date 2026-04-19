//! ADM-08: admin REST surface for the typed settings catalogue.
//!
//! Mounted under `/api/admin/settings`. Reads, writes, and the
//! single-key reveal endpoint are gated by three distinct
//! permissions so the role matrix can model "lifecycle" vs "deploy"
//! staff:
//!
//! * `admin.settings.read`        — list + get (redacted secrets)
//! * `admin.settings.read_secret` — get with `?reveal=true` (decrypt)
//! * `admin.settings.write`       — PUT / create (incl. encryption)
//!
//! All mutations write a row to `admin_actions` via
//! [`audit_admin_priv`] and reload the in-memory snapshot in
//! [`crate::AppState::settings`] so the maintenance middleware (and
//! every other consumer) sees the change on the next request.

use axum::{
    extract::{Path, Query, State},
    routing::get,
    Json, Router,
};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use utoipa::{IntoParams, ToSchema};

use crate::{
    error::{AppError, AppResult},
    extractors::{ClientInfo, PrivilegedUser},
    services::audit::audit_admin_priv,
    settings::{self, crypto, SettingType, SettingView},
    AppState,
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(list))
        .route("/{key}", get(get_one).put(upsert))
        .route("/_reload", axum::routing::post(reload))
}

// ── DTOs ────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, ToSchema)]
pub struct SettingListResponse {
    pub data: Vec<SettingView>,
    pub total: i64,
}

#[derive(Debug, Deserialize, IntoParams)]
pub struct GetQuery {
    /// When `true` and the caller carries `admin.settings.read_secret`,
    /// the response includes the decrypted plaintext under
    /// `revealed_value`. Otherwise the value is redacted to `"***"`.
    #[serde(default)]
    pub reveal: bool,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct SettingGetResponse {
    #[serde(flatten)]
    pub view: SettingView,
    /// Set only when `?reveal=true` succeeded. JSON value (string for
    /// `secret`, native shape otherwise).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub revealed_value: Option<JsonValue>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct SettingUpsertRequest {
    /// Required when creating a new key. Ignored on update of an
    /// existing key (the stored type is canonical).
    pub value_type: Option<SettingType>,
    /// Defaults to `false` on create. Ignored on update.
    #[serde(default)]
    pub is_secret: bool,
    /// Optional admin-facing description; only persisted on create.
    pub description: Option<String>,
    /// Defaults to `"general"` on create. Ignored on update.
    pub category: Option<String>,
    /// The raw value. For `secret` types, pass the cleartext as a JSON
    /// string — the server encrypts before persisting.
    pub value: JsonValue,
}

// ── Handlers ────────────────────────────────────────────────────────────

#[utoipa::path(
    get,
    path = "/api/admin/settings",
    tag = "admin-settings",
    operation_id = "admin_settings_list",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "All settings (secrets redacted)", body = SettingListResponse),
        (status = 403, description = "Forbidden"),
    )
)]
pub(crate) async fn list(
    State(state): State<AppState>,
    admin: PrivilegedUser,
) -> AppResult<Json<SettingListResponse>> {
    admin.require(&state.policy, "admin.settings.read")?;
    // Snapshot from cache. The cache is reloaded on every mutation,
    // so reads never hit the database. This keeps the admin list
    // page responsive even with hundreds of keys.
    let mut rows = state.settings.snapshot();
    rows.sort_by(|a, b| a.key.cmp(&b.key));
    let total = rows.len() as i64;
    let data: Vec<SettingView> = rows.iter().map(SettingView::from_record_redacted).collect();
    Ok(Json(SettingListResponse { data, total }))
}

#[utoipa::path(
    get,
    path = "/api/admin/settings/{key}",
    tag = "admin-settings",
    operation_id = "admin_settings_get_one",
    params(
        ("key" = String, Path, description = "Setting key (e.g. system.maintenance_mode)"),
        GetQuery,
    ),
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Single setting", body = SettingGetResponse),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Unknown key"),
    )
)]
pub(crate) async fn get_one(
    State(state): State<AppState>,
    admin: PrivilegedUser,
    client: ClientInfo,
    Path(key): Path<String>,
    Query(q): Query<GetQuery>,
) -> AppResult<Json<SettingGetResponse>> {
    admin.require(&state.policy, "admin.settings.read")?;

    // Bypass cache: a freshly-rotated secret should appear immediately
    // even if the cache reload is racing with the read.
    let rec = settings::fetch(&state.db, &key)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("setting `{key}` not found")))?;

    let view = SettingView::from_record_redacted(&rec);

    let revealed = if q.reveal && (rec.is_secret || matches!(rec.value_type, SettingType::Secret)) {
        admin.require(&state.policy, "admin.settings.read_secret")?;

        let key_b64 = crypto::key_from_env().map_err(|err| {
            AppError::ServiceUnavailable(format!("settings encryption key unavailable: {err}"))
        })?;
        let plaintext = settings::reveal_secret(&rec, &key_b64)?;

        // Reveal is sensitive — write a dedicated audit row even though
        // it does not mutate state. This is how SOC2 evidence is
        // generated for "who saw which credential, when".
        audit_admin_priv(
            &state.db,
            &admin,
            &client,
            "admin.settings.reveal",
            "app_setting",
            rec.key.clone(),
            serde_json::json!({"category": rec.category}),
        )
        .await;

        Some(plaintext)
    } else {
        None
    };

    Ok(Json(SettingGetResponse {
        view,
        revealed_value: revealed,
    }))
}

#[utoipa::path(
    put,
    path = "/api/admin/settings/{key}",
    tag = "admin-settings",
    operation_id = "admin_settings_upsert",
    params(("key" = String, Path, description = "Setting key")),
    request_body = SettingUpsertRequest,
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Persisted setting (redacted)", body = SettingView),
        (status = 400, description = "Value shape mismatched value_type"),
        (status = 403, description = "Forbidden"),
        (status = 503, description = "Encryption key missing"),
    )
)]
pub(crate) async fn upsert(
    State(state): State<AppState>,
    admin: PrivilegedUser,
    client: ClientInfo,
    Path(key): Path<String>,
    Json(req): Json<SettingUpsertRequest>,
) -> AppResult<Json<SettingView>> {
    admin.require(&state.policy, "admin.settings.write")?;

    if key.is_empty() || key.len() > 128 {
        return Err(AppError::BadRequest(
            "key must be 1..=128 characters".into(),
        ));
    }

    let existing = settings::fetch(&state.db, &key).await?;

    let (value_type, is_secret) = match &existing {
        // On update, the stored type is canonical — clients cannot
        // change a `bool` into a `string` without first deleting the
        // row. Avoids a class of "type drift" bugs in cached readers.
        Some(rec) => (rec.value_type, rec.is_secret),
        None => {
            let vt = req.value_type.ok_or_else(|| {
                AppError::BadRequest("value_type is required when creating a setting".into())
            })?;
            (vt, req.is_secret || matches!(vt, SettingType::Secret))
        }
    };

    settings::validate_shape(value_type, &req.value)?;

    // Materialise the on-disk payload. For secrets the request carries
    // the cleartext (string); we seal it before persisting so the
    // database never sees the plaintext.
    let stored_value = if matches!(value_type, SettingType::Secret) {
        let plaintext = req.value.as_str().ok_or_else(|| {
            AppError::BadRequest("secret value must be a JSON string (the cleartext)".into())
        })?;
        let key_b64 = crypto::key_from_env().map_err(|err| {
            AppError::ServiceUnavailable(format!("settings encryption key unavailable: {err}"))
        })?;
        let envelope = crypto::encrypt(plaintext, &key_b64)
            .map_err(|err| AppError::Internal(anyhow::anyhow!("encrypt failed: {err}")))?;
        serde_json::to_value(envelope)
            .map_err(|err| AppError::Internal(anyhow::anyhow!("envelope serialize: {err}")))?
    } else {
        req.value.clone()
    };

    let rec = if existing.is_some() {
        settings::update(&state.db, &key, &stored_value, admin.user_id).await?
    } else {
        settings::create(
            &state.db,
            &key,
            &stored_value,
            value_type,
            is_secret,
            req.description.as_deref(),
            req.category.as_deref().unwrap_or("general"),
            admin.user_id,
        )
        .await?
    };

    // Hot reload so the maintenance middleware sees the new value on
    // the next request without a deploy. Best-effort: a reload failure
    // is logged but the write succeeded.
    if let Err(err) = state.settings.reload(&state.db).await {
        tracing::warn!(error = %err, "settings cache reload failed; cache may be stale");
    }

    audit_admin_priv(
        &state.db,
        &admin,
        &client,
        if existing.is_some() {
            "admin.settings.update"
        } else {
            "admin.settings.create"
        },
        "app_setting",
        rec.key.clone(),
        serde_json::json!({
            "value_type": rec.value_type,
            "is_secret": rec.is_secret,
            "category": rec.category,
        }),
    )
    .await;

    Ok(Json(SettingView::from_record_redacted(&rec)))
}

#[utoipa::path(
    post,
    path = "/api/admin/settings/_reload",
    tag = "admin-settings",
    operation_id = "admin_settings_reload",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Cache reloaded; returns row count"),
        (status = 403, description = "Forbidden"),
    )
)]
pub(crate) async fn reload(
    State(state): State<AppState>,
    admin: PrivilegedUser,
    client: ClientInfo,
) -> AppResult<Json<JsonValue>> {
    admin.require(&state.policy, "admin.settings.write")?;
    let count = state.settings.reload(&state.db).await?;

    audit_admin_priv(
        &state.db,
        &admin,
        &client,
        "admin.settings.reload",
        "app_setting",
        "*".to_string(),
        serde_json::json!({"count": count}),
    )
    .await;

    Ok(Json(serde_json::json!({"reloaded": count})))
}
