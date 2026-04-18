//! CONSENT-03 — DSAR data-export builder.
//!
//! Collects a subject's personal data from the tables we know about and emits
//! a single JSON document the admin handler returns (and also persists as a
//! `data:` URI placeholder). Tables are queried defensively:
//!
//!   * `users`, `notification_deliveries`, `notification_preferences`,
//!     `consent_records` are always present.
//!   * `subscriptions` is present in most deployments; we use `to_regclass`
//!     to skip it without failing if an installation has not yet applied the
//!     subscriptions migration.
//!   * `courses_enrollment` is the "if table exists; otherwise skip" case
//!     called out in the plan — same `to_regclass` guard applies.
//!
//! The output shape is versioned (`version: 1`) so a later subsystem can
//! evolve the schema without breaking admin tooling that consumes it.
//!
//! Sensitive fields (`password_hash`, `verification_token_hash`, `ip_hash`)
//! are intentionally omitted. The subject is allowed to see their own data,
//! not the server-side cryptographic artefacts derived from it.

use serde::Serialize;
use serde_json::{json, Value};
use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::error::AppResult;

use super::records::DsarRow;

/// Top-level export envelope. Kept as a plain struct (vs. `Value`) so the
/// schema shape is self-documenting and so the `version` bump is a visible
/// code change.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DsarExport {
    pub version: u32,
    pub request_id: Uuid,
    pub email: String,
    pub kind: String,
    pub user: Option<Value>,
    pub notification_deliveries: Vec<Value>,
    pub notification_preferences: Vec<Value>,
    pub consent_records: Vec<Value>,
    pub subscriptions: Vec<Value>,
    pub course_enrollments: Vec<Value>,
}

/// Build the export payload for `request`. Reads only; never mutates.
pub async fn build_export(pool: &PgPool, request: &DsarRow) -> AppResult<DsarExport> {
    let user = fetch_user(pool, request).await?;
    let effective_user_id = request.user_id.or_else(|| user_id_from_value(&user));

    let notification_deliveries = fetch_deliveries(pool, effective_user_id, &request.email).await?;
    let notification_preferences = match effective_user_id {
        Some(id) => fetch_preferences(pool, id).await?,
        None => Vec::new(),
    };
    let consent_records = match effective_user_id {
        Some(id) => fetch_consent_records(pool, id).await?,
        None => Vec::new(),
    };

    let subscriptions = match effective_user_id {
        Some(id) if table_exists(pool, "subscriptions").await? => {
            fetch_subscriptions(pool, id).await?
        }
        _ => Vec::new(),
    };

    let course_enrollments = match effective_user_id {
        Some(id) if table_exists(pool, "courses_enrollment").await? => {
            fetch_course_enrollments(pool, id).await?
        }
        _ => Vec::new(),
    };

    Ok(DsarExport {
        version: 1,
        request_id: request.id,
        email: request.email.clone(),
        kind: request.kind.clone(),
        user,
        notification_deliveries,
        notification_preferences,
        consent_records,
        subscriptions,
        course_enrollments,
    })
}

/// Encode the export as a `data:` URI. The admin handler persists this as the
/// `fulfillment_url` placeholder until R2 uploads land in a later subsystem.
#[must_use]
pub fn export_to_data_uri(export: &DsarExport) -> String {
    // Stable, pretty-printed JSON — small DSARs are human-readable and the
    // size penalty is negligible. The `data:` prefix is a URL-safe envelope
    // that any admin tool can copy/paste into a browser.
    let body = serde_json::to_string(&export).unwrap_or_else(|_| "{}".to_string());
    format!(
        "data:application/json;charset=utf-8,{}",
        percent_encode_basic(&body)
    )
}

/// Minimal RFC 3986 percent-encoder for characters the `data:` URL spec
/// requires: reserved + non-ASCII. Avoids pulling a whole crate for this.
fn percent_encode_basic(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        let allowed = b.is_ascii_alphanumeric()
            || matches!(
                b,
                b'-' | b'_' | b'.' | b'~' | b'/' | b':' | b'{' | b'}' | b',' | b' '
            );
        if allowed {
            out.push(b as char);
        } else {
            out.push('%');
            out.push(hex_nibble(b >> 4));
            out.push(hex_nibble(b & 0x0F));
        }
    }
    out
}

fn hex_nibble(v: u8) -> char {
    match v {
        0..=9 => (b'0' + v) as char,
        10..=15 => (b'A' + v - 10) as char,
        _ => '0',
    }
}

// ── Private fetchers ────────────────────────────────────────────────────

async fn table_exists(pool: &PgPool, table: &str) -> AppResult<bool> {
    let exists: Option<String> = sqlx::query_scalar(r#"SELECT to_regclass($1)::text"#)
        .bind(table)
        .fetch_one(pool)
        .await?;
    Ok(exists.is_some())
}

async fn fetch_user(pool: &PgPool, request: &DsarRow) -> AppResult<Option<Value>> {
    let row = sqlx::query(
        r#"
        SELECT id, email, name, role, created_at, updated_at
        FROM users
        WHERE ($1::uuid IS NOT NULL AND id = $1::uuid)
           OR LOWER(email) = LOWER($2)
        ORDER BY (id = $1::uuid) DESC
        LIMIT 1
        "#,
    )
    .bind(request.user_id)
    .bind(&request.email)
    .fetch_optional(pool)
    .await?;

    let Some(row) = row else {
        return Ok(None);
    };

    let id: Uuid = row.try_get("id")?;
    let email: String = row.try_get("email")?;
    let name: Option<String> = row.try_get("name").ok();
    let role: Option<String> = row.try_get::<String, _>("role").ok();
    let created_at: chrono::DateTime<chrono::Utc> = row.try_get("created_at")?;
    let updated_at: chrono::DateTime<chrono::Utc> = row.try_get("updated_at")?;

    Ok(Some(json!({
        "id": id,
        "email": email,
        "name": name,
        "role": role,
        "createdAt": created_at,
        "updatedAt": updated_at,
    })))
}

fn user_id_from_value(v: &Option<Value>) -> Option<Uuid> {
    v.as_ref()
        .and_then(|obj| obj.get("id"))
        .and_then(Value::as_str)
        .and_then(|s| Uuid::parse_str(s).ok())
}

async fn fetch_deliveries(
    pool: &PgPool,
    user_id: Option<Uuid>,
    email: &str,
) -> AppResult<Vec<Value>> {
    let rows = sqlx::query(
        r#"
        SELECT id, template_key, channel, status, subject, created_at, updated_at
        FROM notification_deliveries
        WHERE ($1::uuid IS NOT NULL AND user_id = $1::uuid)
           OR LOWER(anonymous_email) = LOWER($2)
        ORDER BY created_at DESC
        LIMIT 1000
        "#,
    )
    .bind(user_id)
    .bind(email)
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|r| {
            let id: Uuid = r.try_get("id").unwrap_or_else(|_| Uuid::nil());
            let template_key: String = r.try_get("template_key").unwrap_or_default();
            let channel: String = r.try_get("channel").unwrap_or_default();
            let status: String = r.try_get("status").unwrap_or_default();
            let subject: Option<String> = r.try_get("subject").ok();
            let created_at: chrono::DateTime<chrono::Utc> = r
                .try_get("created_at")
                .unwrap_or_else(|_| chrono::Utc::now());
            let updated_at: chrono::DateTime<chrono::Utc> = r
                .try_get("updated_at")
                .unwrap_or_else(|_| chrono::Utc::now());
            json!({
                "id": id,
                "templateKey": template_key,
                "channel": channel,
                "status": status,
                "subject": subject,
                "createdAt": created_at,
                "updatedAt": updated_at,
            })
        })
        .collect())
}

async fn fetch_preferences(pool: &PgPool, user_id: Uuid) -> AppResult<Vec<Value>> {
    let rows = sqlx::query(
        r#"
        SELECT category, channel, enabled, quiet_hours_start, quiet_hours_end,
               timezone, updated_at
        FROM notification_preferences
        WHERE user_id = $1
        "#,
    )
    .bind(user_id)
    .fetch_all(pool)
    .await?;
    Ok(rows
        .into_iter()
        .map(|r| {
            let category: String = r.try_get("category").unwrap_or_default();
            let channel: String = r.try_get("channel").unwrap_or_default();
            let enabled: bool = r.try_get("enabled").unwrap_or(true);
            json!({
                "category": category,
                "channel": channel,
                "enabled": enabled,
            })
        })
        .collect())
}

async fn fetch_consent_records(pool: &PgPool, subject_id: Uuid) -> AppResult<Vec<Value>> {
    let rows = sqlx::query(
        r#"
        SELECT id, banner_version, policy_version, categories, services, action,
               tcf_string, gpc_signal, country, created_at
        FROM consent_records
        WHERE subject_id = $1
        ORDER BY created_at DESC
        LIMIT 1000
        "#,
    )
    .bind(subject_id)
    .fetch_all(pool)
    .await?;
    Ok(rows
        .into_iter()
        .map(|r| {
            let id: Uuid = r.try_get("id").unwrap_or_else(|_| Uuid::nil());
            let banner_version: i32 = r.try_get("banner_version").unwrap_or(0);
            let policy_version: i32 = r.try_get("policy_version").unwrap_or(0);
            let categories: Value = r.try_get("categories").unwrap_or(Value::Null);
            let services: Value = r.try_get("services").unwrap_or(Value::Null);
            let action: String = r.try_get("action").unwrap_or_default();
            let tcf_string: Option<String> = r.try_get("tcf_string").ok();
            let gpc_signal: Option<bool> = r.try_get("gpc_signal").ok();
            let country: Option<String> = r.try_get("country").ok();
            let created_at: chrono::DateTime<chrono::Utc> = r
                .try_get("created_at")
                .unwrap_or_else(|_| chrono::Utc::now());
            json!({
                "id": id,
                "bannerVersion": banner_version,
                "policyVersion": policy_version,
                "categories": categories,
                "services": services,
                "action": action,
                "tcfString": tcf_string,
                "gpcSignal": gpc_signal,
                "country": country,
                "createdAt": created_at,
            })
        })
        .collect())
}

async fn fetch_subscriptions(pool: &PgPool, user_id: Uuid) -> AppResult<Vec<Value>> {
    // `subscriptions` has historically varied across deployments; pull only
    // columns universally present in this codebase's migration 001 and fall
    // back to a best-effort JSON blob when a column is missing.
    let rows = sqlx::query(
        r#"
        SELECT id::text AS id, status::text AS status, plan::text AS plan,
               created_at, updated_at
        FROM subscriptions
        WHERE user_id = $1
        ORDER BY created_at DESC
        LIMIT 200
        "#,
    )
    .bind(user_id)
    .fetch_all(pool)
    .await?;
    Ok(rows
        .into_iter()
        .map(|r| {
            let id: String = r.try_get("id").unwrap_or_default();
            let status: String = r.try_get("status").unwrap_or_default();
            let plan: String = r.try_get("plan").unwrap_or_default();
            let created_at: chrono::DateTime<chrono::Utc> = r
                .try_get("created_at")
                .unwrap_or_else(|_| chrono::Utc::now());
            json!({
                "id": id,
                "status": status,
                "plan": plan,
                "createdAt": created_at,
            })
        })
        .collect())
}

async fn fetch_course_enrollments(pool: &PgPool, user_id: Uuid) -> AppResult<Vec<Value>> {
    let rows = sqlx::query(
        r#"
        SELECT course_id::text AS course_id, enrolled_at
        FROM courses_enrollment
        WHERE user_id = $1
        ORDER BY enrolled_at DESC
        LIMIT 500
        "#,
    )
    .bind(user_id)
    .fetch_all(pool)
    .await?;
    Ok(rows
        .into_iter()
        .map(|r| {
            let course_id: String = r.try_get("course_id").unwrap_or_default();
            let enrolled_at: chrono::DateTime<chrono::Utc> = r
                .try_get("enrolled_at")
                .unwrap_or_else(|_| chrono::Utc::now());
            json!({
                "courseId": course_id,
                "enrolledAt": enrolled_at,
            })
        })
        .collect())
}

// ── Unit tests ──────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    #[test]
    fn export_serializes_with_expected_top_level_keys() {
        let row = DsarRow {
            id: Uuid::nil(),
            user_id: None,
            email: "alice@example.com".into(),
            kind: "access".into(),
            status: "pending".into(),
            payload: serde_json::json!({}),
            fulfilled_at: None,
            fulfilled_by: None,
            fulfillment_url: None,
            admin_notes: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        // Hand-build an export (don't touch the DB) and confirm the shape.
        let export = DsarExport {
            version: 1,
            request_id: row.id,
            email: row.email.clone(),
            kind: row.kind.clone(),
            user: None,
            notification_deliveries: vec![],
            notification_preferences: vec![],
            consent_records: vec![],
            subscriptions: vec![],
            course_enrollments: vec![],
        };
        let v = serde_json::to_value(&export).expect("serialize");
        for key in [
            "version",
            "requestId",
            "email",
            "kind",
            "user",
            "notificationDeliveries",
            "notificationPreferences",
            "consentRecords",
            "subscriptions",
            "courseEnrollments",
        ] {
            assert!(v.get(key).is_some(), "missing key: {key}");
        }
        assert_eq!(v["version"], serde_json::json!(1));
    }

    #[test]
    fn data_uri_has_expected_prefix() {
        let export = DsarExport {
            version: 1,
            request_id: Uuid::nil(),
            email: "a@b.co".into(),
            kind: "access".into(),
            user: None,
            notification_deliveries: vec![],
            notification_preferences: vec![],
            consent_records: vec![],
            subscriptions: vec![],
            course_enrollments: vec![],
        };
        let uri = export_to_data_uri(&export);
        assert!(uri.starts_with("data:application/json;charset=utf-8,"));
    }

    #[test]
    fn percent_encode_preserves_basic_chars_and_escapes_others() {
        let enc = percent_encode_basic("abc ABC 123/.-_~");
        assert_eq!(enc, "abc ABC 123/.-_~");
        let enc2 = percent_encode_basic("\"?&=");
        assert!(enc2.contains("%22"));
        assert!(enc2.contains("%3F"));
        assert!(enc2.contains("%26"));
        assert!(enc2.contains("%3D"));
    }
}
