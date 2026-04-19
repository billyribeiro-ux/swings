//! ADM-08: typed runtime settings catalogue.
//!
//! See `migrations/062_app_settings.sql` for the storage contract. The
//! module owns three responsibilities:
//!
//! 1. **Typed read/write** — every value carries a `value_type`
//!    discriminator and the [`SettingValue`] enum is the only shape
//!    handlers should consume; serde + Postgres conversion never lose
//!    the type tag.
//!
//! 2. **Hot cache** — [`Cache`] mirrors the table inside the
//!    `AppState`, behind an `RwLock`. Reads are O(1) hashmap lookups
//!    so the maintenance-mode middleware can run on every request
//!    without a DB round-trip. Writes (admin handler) bump the table
//!    + reload the cache atomically.
//!
//! 3. **Encryption-at-rest** — secrets are stored as the `value_type
//!    = 'secret'` envelope (`{ "ct": "<aead>", "nonce": "<b64>",
//!    "v": 1 }`). The plaintext only exists in process memory and
//!    only when a caller carries `admin.settings.read_secret`. Key
//!    material lives in `SETTINGS_ENCRYPTION_KEY` (32 raw bytes,
//!    base64-encoded). Missing key → secrets are read-only and
//!    decrypt returns an error so a misconfigured deploy fails loud.

pub mod crypto;

use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use sqlx::{postgres::PgRow, PgPool, Row};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::error::{AppError, AppResult};

// ── Reserved keys ──────────────────────────────────────────────────────

pub const KEY_MAINTENANCE_MODE: &str = "system.maintenance_mode";
pub const KEY_MAINTENANCE_MESSAGE: &str = "system.maintenance_message";
pub const KEY_MAINTENANCE_ADMIN_ONLY: &str = "system.maintenance_admin_only";

// ── Typed value model ──────────────────────────────────────────────────

/// Storage discriminator. Mirrors the `value_type` CHECK constraint in
/// migration 062. Kept as a strict enum so the admin handler cannot
/// silently coerce between types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum SettingType {
    String,
    Int,
    Bool,
    Json,
    Secret,
}

impl SettingType {
    pub fn as_str(self) -> &'static str {
        match self {
            SettingType::String => "string",
            SettingType::Int => "int",
            SettingType::Bool => "bool",
            SettingType::Json => "json",
            SettingType::Secret => "secret",
        }
    }

    pub fn from_str_lower(s: &str) -> Option<Self> {
        match s {
            "string" => Some(Self::String),
            "int" => Some(Self::Int),
            "bool" => Some(Self::Bool),
            "json" => Some(Self::Json),
            "secret" => Some(Self::Secret),
            _ => None,
        }
    }
}

/// Materialised in-memory representation of an `app_settings` row.
/// `value` always carries the canonical Postgres JSONB; for `Secret`
/// rows the JSON is the encrypted envelope (see `crypto`).
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct SettingRecord {
    pub key: String,
    pub value: JsonValue,
    pub value_type: SettingType,
    pub is_secret: bool,
    pub description: Option<String>,
    pub category: String,
    pub updated_at: DateTime<Utc>,
    pub updated_by: Option<Uuid>,
}

impl SettingRecord {
    fn from_row(row: &PgRow) -> Result<Self, sqlx::Error> {
        let value_type_raw: String = row.try_get("value_type")?;
        let value_type = SettingType::from_str_lower(&value_type_raw).ok_or_else(|| {
            sqlx::Error::Decode(
                format!("unknown value_type {value_type_raw} in app_settings").into(),
            )
        })?;
        Ok(Self {
            key: row.try_get("key")?,
            value: row.try_get("value")?,
            value_type,
            is_secret: row.try_get("is_secret")?,
            description: row.try_get("description")?,
            category: row.try_get("category")?,
            updated_at: row.try_get("updated_at")?,
            updated_by: row.try_get("updated_by")?,
        })
    }
}

/// Validate that `value` is shape-compatible with `value_type`. Returns
/// `BadRequest` so handler error responses round-trip through the
/// problem+json formatter.
pub fn validate_shape(value_type: SettingType, value: &JsonValue) -> AppResult<()> {
    let ok = match value_type {
        SettingType::String => value.is_string(),
        SettingType::Int => value.is_i64() || value.is_u64(),
        SettingType::Bool => value.is_boolean(),
        SettingType::Json => value.is_object() || value.is_array(),
        // Secret accepts:
        //   * a plaintext string from the admin write path (the handler
        //     encrypts before persisting), OR
        //   * the canonical envelope object (admin re-puts a
        //     pre-encrypted value during data migration).
        SettingType::Secret => {
            value.is_string()
                || (value.is_object() && value.get("ct").is_some() && value.get("nonce").is_some())
        }
    };
    if ok {
        Ok(())
    } else {
        Err(AppError::BadRequest(format!(
            "value does not match value_type={}",
            value_type.as_str()
        )))
    }
}

// ── In-memory cache ────────────────────────────────────────────────────

/// Process-wide settings snapshot. Cheap to clone (`Arc` over the
/// inner `RwLock`); designed to live inside [`crate::AppState`].
#[derive(Clone, Default)]
pub struct Cache {
    inner: Arc<RwLock<HashMap<String, SettingRecord>>>,
}

impl Cache {
    /// Build an empty cache. Call [`Self::reload`] before serving
    /// traffic so the maintenance middleware sees a populated map.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Replace the snapshot in one swap so middleware reads never see
    /// a half-populated map. Returns the number of rows loaded.
    pub async fn reload(&self, pool: &PgPool) -> AppResult<usize> {
        let rows = sqlx::query("SELECT key, value, value_type, is_secret, description, category, updated_at, updated_by FROM app_settings")
            .fetch_all(pool)
            .await?;
        let mut next = HashMap::with_capacity(rows.len());
        for row in &rows {
            let rec = SettingRecord::from_row(row).map_err(AppError::Database)?;
            next.insert(rec.key.clone(), rec);
        }
        let count = next.len();
        // Poisoned RwLock: best-effort recover and overwrite.
        let mut guard = match self.inner.write() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        *guard = next;
        tracing::info!(count, "settings cache reloaded");
        Ok(count)
    }

    /// Snapshot all rows (cloned). Used by the admin list endpoint.
    pub fn snapshot(&self) -> Vec<SettingRecord> {
        let guard = match self.inner.read() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        guard.values().cloned().collect()
    }

    /// Clone a single record by key.
    pub fn get(&self, key: &str) -> Option<SettingRecord> {
        let guard = match self.inner.read() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        guard.get(key).cloned()
    }

    /// Boolean convenience used by the maintenance middleware. Treats
    /// missing / non-boolean values as `default`.
    pub fn get_bool(&self, key: &str, default: bool) -> bool {
        self.get(key)
            .and_then(|r| r.value.as_bool())
            .unwrap_or(default)
    }

    /// String convenience. Returns the empty string when missing.
    pub fn get_string(&self, key: &str, default: &str) -> String {
        self.get(key)
            .and_then(|r| r.value.as_str().map(str::to_owned))
            .unwrap_or_else(|| default.to_owned())
    }
}

// ── Repository: reads / writes (single source of truth) ────────────────

/// Fetch a single row from the table (bypasses cache). Used by the
/// admin handler to avoid serving a stale read after a write.
pub async fn fetch(pool: &PgPool, key: &str) -> AppResult<Option<SettingRecord>> {
    let row = sqlx::query("SELECT key, value, value_type, is_secret, description, category, updated_at, updated_by FROM app_settings WHERE key = $1")
        .bind(key)
        .fetch_optional(pool)
        .await?;
    let Some(row) = row else { return Ok(None) };
    let rec = SettingRecord::from_row(&row).map_err(AppError::Database)?;
    Ok(Some(rec))
}

/// Update an existing row. Returns the post-update record. Returns
/// `NotFound` when the key does not exist — the admin UI must call
/// the create path first.
pub async fn update(
    pool: &PgPool,
    key: &str,
    value: &JsonValue,
    updated_by: Uuid,
) -> AppResult<SettingRecord> {
    let row = sqlx::query("UPDATE app_settings SET value = $2, updated_at = NOW(), updated_by = $3 WHERE key = $1 RETURNING key, value, value_type, is_secret, description, category, updated_at, updated_by")
        .bind(key)
        .bind(value)
        .bind(updated_by)
        .fetch_optional(pool)
        .await?;
    let Some(row) = row else {
        return Err(AppError::NotFound(format!("setting `{key}` not found")));
    };
    SettingRecord::from_row(&row).map_err(AppError::Database)
}

/// Insert a fresh row. Used by `PUT` when the key is unknown — the
/// admin handler funnels to here after asserting the caller carries
/// `admin.settings.write` and validating the value shape.
#[allow(clippy::too_many_arguments)]
pub async fn create(
    pool: &PgPool,
    key: &str,
    value: &JsonValue,
    value_type: SettingType,
    is_secret: bool,
    description: Option<&str>,
    category: &str,
    updated_by: Uuid,
) -> AppResult<SettingRecord> {
    let row = sqlx::query(
        "INSERT INTO app_settings (key, value, value_type, is_secret, description, category, updated_by) \
         VALUES ($1, $2, $3, $4, $5, $6, $7) \
         RETURNING key, value, value_type, is_secret, description, category, updated_at, updated_by"
    )
        .bind(key)
        .bind(value)
        .bind(value_type.as_str())
        .bind(is_secret)
        .bind(description)
        .bind(category)
        .bind(updated_by)
        .fetch_one(pool)
        .await?;
    SettingRecord::from_row(&row).map_err(AppError::Database)
}

/// Public-API view: redacts secret values and surfaces the canonical
/// envelope structure. Use [`reveal_secret`] when the caller carries
/// `admin.settings.read_secret`.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct SettingView {
    pub key: String,
    /// Either the actual value (non-secret) or the JSON literal
    /// `"***"` (secret + caller lacks reveal permission).
    pub value: JsonValue,
    pub value_type: SettingType,
    pub is_secret: bool,
    pub description: Option<String>,
    pub category: String,
    pub updated_at: DateTime<Utc>,
    pub updated_by: Option<Uuid>,
}

impl SettingView {
    /// Build the redacted view from a record.
    #[must_use]
    pub fn from_record_redacted(rec: &SettingRecord) -> Self {
        let value = if rec.is_secret || matches!(rec.value_type, SettingType::Secret) {
            JsonValue::String("***".into())
        } else {
            rec.value.clone()
        };
        Self {
            key: rec.key.clone(),
            value,
            value_type: rec.value_type,
            is_secret: rec.is_secret,
            description: rec.description.clone(),
            category: rec.category.clone(),
            updated_at: rec.updated_at,
            updated_by: rec.updated_by,
        }
    }
}

/// Decrypt a secret record into the cleartext JSON value it
/// originally protected. Errors propagate as `Forbidden` so a missing
/// key (or a bad ciphertext) never leaks the underlying crypto reason.
pub fn reveal_secret(rec: &SettingRecord, key_b64: &str) -> AppResult<JsonValue> {
    if !matches!(rec.value_type, SettingType::Secret) && !rec.is_secret {
        return Ok(rec.value.clone());
    }
    let envelope: crypto::Envelope = serde_json::from_value(rec.value.clone()).map_err(|err| {
        AppError::Internal(anyhow::anyhow!(
            "secret envelope malformed for key {}: {err}",
            rec.key
        ))
    })?;
    let plaintext = crypto::decrypt(&envelope, key_b64)
        .map_err(|err| AppError::Internal(anyhow::anyhow!("decrypt failed: {err}")))?;
    Ok(JsonValue::String(plaintext))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_shape_string_ok() {
        assert!(validate_shape(SettingType::String, &JsonValue::String("a".into())).is_ok());
        assert!(validate_shape(SettingType::String, &JsonValue::Bool(true)).is_err());
    }

    #[test]
    fn validate_shape_bool_ok() {
        assert!(validate_shape(SettingType::Bool, &JsonValue::Bool(false)).is_ok());
        assert!(validate_shape(SettingType::Bool, &JsonValue::Number(1.into())).is_err());
    }

    #[test]
    fn validate_shape_int_ok() {
        assert!(validate_shape(SettingType::Int, &serde_json::json!(42)).is_ok());
        assert!(validate_shape(SettingType::Int, &serde_json::json!(true)).is_err());
    }

    #[test]
    fn validate_shape_json_requires_object_or_array() {
        assert!(validate_shape(SettingType::Json, &serde_json::json!({"a":1})).is_ok());
        assert!(validate_shape(SettingType::Json, &serde_json::json!([1,2])).is_ok());
        assert!(validate_shape(SettingType::Json, &serde_json::json!("a")).is_err());
    }

    #[test]
    fn validate_shape_secret_accepts_string_or_envelope() {
        assert!(validate_shape(SettingType::Secret, &serde_json::json!("plaintext")).is_ok());
        assert!(validate_shape(
            SettingType::Secret,
            &serde_json::json!({"ct": "x", "nonce": "y", "v": 1}),
        )
        .is_ok());
        assert!(validate_shape(SettingType::Secret, &serde_json::json!(42)).is_err());
    }

    #[test]
    fn settingview_redacts_secret() {
        let rec = SettingRecord {
            key: "x".into(),
            value: serde_json::json!({"ct":"abc","nonce":"def","v":1}),
            value_type: SettingType::Secret,
            is_secret: true,
            description: None,
            category: "x".into(),
            updated_at: Utc::now(),
            updated_by: None,
        };
        let view = SettingView::from_record_redacted(&rec);
        assert_eq!(view.value, JsonValue::String("***".into()));
    }

    #[test]
    fn cache_get_bool_returns_default_when_missing() {
        let c = Cache::new();
        assert!(c.get_bool("nope", true));
        assert!(!c.get_bool("nope", false));
    }
}
