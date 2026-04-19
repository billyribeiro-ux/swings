//! ADM-06: admin IP allowlist repo + middleware-facing query helpers.
//!
//! This module owns the SQL-side contract for the `admin_ip_allowlist`
//! table introduced by migration `059_admin_ip_allowlist.sql`. The
//! middleware in [`crate::middleware::admin_ip_allowlist`] consults
//! [`is_ip_allowed`] on every privileged request; the CRUD handlers in
//! [`crate::handlers::admin_ip_allowlist`] consume the [`list_*`],
//! [`create`], [`set_active`], and [`delete`] helpers.
//!
//! CIDR ranges are stored using the native Postgres `cidr` type so
//! containment checks happen server-side via the `>>=` operator. The
//! Rust side reads/writes the column with explicit `::text` / `::cidr`
//! casts which keeps `sqlx` on its default feature set (no
//! `ipnetwork` / `cidr` crate dependency required at compile time).

use std::net::IpAddr;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::error::{AppError, AppResult};

/// Materialised allowlist entry as returned by the admin CRUD endpoints.
#[derive(Debug, Clone, Serialize, ToSchema, sqlx::FromRow)]
pub struct AllowlistEntry {
    pub id: Uuid,
    /// CIDR range in canonical text form (e.g. `203.0.113.0/24`,
    /// `2001:db8::/32`). Always returned via `cidr::text` cast on read.
    pub cidr: String,
    /// Human label so operators can recall what each entry covers
    /// (e.g. "office VPN", "Cloudflare egress").
    pub label: String,
    pub is_active: bool,
    pub created_by: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Inputs accepted by `POST /api/admin/security/ip-allowlist`.
#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateAllowlistInput {
    /// CIDR range. Anything Postgres' `cidr` type accepts is valid here
    /// (`192.0.2.42`, `198.51.100.0/24`, `::1/128`, …).
    pub cidr: String,
    pub label: String,
    /// Optional initial active flag — defaults to `true`.
    #[serde(default = "default_active")]
    pub is_active: bool,
}

fn default_active() -> bool {
    true
}

/// Validate user-supplied input before it reaches Postgres. Most of the
/// CIDR validation is delegated to Postgres (`cidr` cast errors bubble up
/// as `AppError::BadRequest`), but we trim & length-check the human-facing
/// fields first so the database CHECK isn't the friendliest error message.
pub fn validate_input(input: &CreateAllowlistInput) -> AppResult<(String, String)> {
    let cidr = input.cidr.trim();
    if cidr.is_empty() {
        return Err(AppError::BadRequest("cidr is required".to_string()));
    }
    let label = input.label.trim();
    if label.is_empty() {
        return Err(AppError::BadRequest("label is required".to_string()));
    }
    if label.len() > 200 {
        return Err(AppError::BadRequest(
            "label must be 200 characters or fewer".to_string(),
        ));
    }
    Ok((cidr.to_string(), label.to_string()))
}

/// `true` if the supplied IP falls inside any active CIDR row, OR if the
/// allowlist contains zero active rows (open-mode default).
///
/// Returning `Ok(true)` on database errors is intentional: an outage of
/// the allowlist table must never lock operators out of the admin UI.
/// The error is logged so the on-call still sees it.
pub async fn is_ip_allowed(pool: &PgPool, ip: IpAddr) -> bool {
    // First branch: empty list → pass-through. Cheap COUNT-style query so
    // the typical (no rows) case finishes in microseconds.
    let active_count: i64 =
        match sqlx::query_scalar("SELECT COUNT(*)::bigint FROM admin_ip_allowlist WHERE is_active")
            .fetch_one(pool)
            .await
        {
            Ok(c) => c,
            Err(err) => {
                tracing::warn!(error = %err, "ip_allowlist: count query failed; failing open");
                return true;
            }
        };

    if active_count == 0 {
        return true;
    }

    let ip_str = ip.to_string();
    match sqlx::query_scalar::<_, bool>(
        r#"
        SELECT EXISTS (
            SELECT 1
              FROM admin_ip_allowlist
             WHERE is_active
               AND cidr >>= $1::inet
        )
        "#,
    )
    .bind(&ip_str)
    .fetch_one(pool)
    .await
    {
        Ok(matched) => matched,
        Err(err) => {
            tracing::warn!(error = %err, ip = %ip_str, "ip_allowlist: containment query failed; failing open");
            true
        }
    }
}

/// Return every row, newest first. Used by the admin GET endpoint.
pub async fn list_all(pool: &PgPool) -> AppResult<Vec<AllowlistEntry>> {
    let rows = sqlx::query_as::<_, AllowlistEntry>(
        r#"
        SELECT id,
               cidr::text AS cidr,
               label,
               is_active,
               created_by,
               created_at,
               updated_at
          FROM admin_ip_allowlist
         ORDER BY created_at DESC
        "#,
    )
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

/// Insert a new entry. Returns `AppError::Conflict` when the CIDR already
/// exists (UNIQUE constraint violation) and `AppError::BadRequest` when
/// the supplied string is not parseable by Postgres' `cidr` type.
pub async fn create(
    pool: &PgPool,
    actor_id: Uuid,
    input: &CreateAllowlistInput,
) -> AppResult<AllowlistEntry> {
    let (cidr, label) = validate_input(input)?;

    let row = sqlx::query_as::<_, AllowlistEntry>(
        r#"
        INSERT INTO admin_ip_allowlist (cidr, label, is_active, created_by)
        VALUES ($1::cidr, $2, $3, $4)
        RETURNING id,
                  cidr::text AS cidr,
                  label,
                  is_active,
                  created_by,
                  created_at,
                  updated_at
        "#,
    )
    .bind(&cidr)
    .bind(&label)
    .bind(input.is_active)
    .bind(actor_id)
    .fetch_one(pool)
    .await
    .map_err(map_create_err)?;

    Ok(row)
}

fn map_create_err(err: sqlx::Error) -> AppError {
    match &err {
        sqlx::Error::Database(db_err) => {
            // 23505 = unique_violation (duplicate CIDR).
            if db_err.code().as_deref() == Some("23505") {
                return AppError::Conflict(
                    "An allowlist entry already exists for that CIDR".to_string(),
                );
            }
            // 22P02 = invalid_text_representation (bad CIDR cast). Postgres
            // surfaces this when the string fails the cidr parser.
            if db_err.code().as_deref() == Some("22P02") {
                return AppError::BadRequest(
                    "cidr must be a valid IPv4 / IPv6 CIDR range".to_string(),
                );
            }
            err.into()
        }
        _ => err.into(),
    }
}

/// Toggle `is_active` to a specific value. Returns the updated row.
pub async fn set_active(
    pool: &PgPool,
    id: Uuid,
    is_active: bool,
) -> AppResult<Option<AllowlistEntry>> {
    let row = sqlx::query_as::<_, AllowlistEntry>(
        r#"
        UPDATE admin_ip_allowlist
           SET is_active  = $2,
               updated_at = NOW()
         WHERE id = $1
         RETURNING id,
                   cidr::text AS cidr,
                   label,
                   is_active,
                   created_by,
                   created_at,
                   updated_at
        "#,
    )
    .bind(id)
    .bind(is_active)
    .fetch_optional(pool)
    .await?;
    Ok(row)
}

/// Remove an entry. Returns `Ok(true)` if a row was deleted, `Ok(false)`
/// when nothing matched.
pub async fn delete(pool: &PgPool, id: Uuid) -> AppResult<bool> {
    let result = sqlx::query("DELETE FROM admin_ip_allowlist WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(result.rows_affected() > 0)
}

/// Snapshot lookup — `Some(entry)` when present, used by handlers that
/// want pre-mutation state for audit metadata.
pub async fn get(pool: &PgPool, id: Uuid) -> AppResult<Option<AllowlistEntry>> {
    let row = sqlx::query_as::<_, AllowlistEntry>(
        r#"
        SELECT id,
               cidr::text AS cidr,
               label,
               is_active,
               created_by,
               created_at,
               updated_at
          FROM admin_ip_allowlist
         WHERE id = $1
        "#,
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;
    Ok(row)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_input_trims_and_rejects_blanks() {
        let ok = CreateAllowlistInput {
            cidr: "  10.0.0.0/8  ".to_string(),
            label: "  office  ".to_string(),
            is_active: true,
        };
        let (cidr, label) = validate_input(&ok).unwrap();
        assert_eq!(cidr, "10.0.0.0/8");
        assert_eq!(label, "office");

        let blank_cidr = CreateAllowlistInput {
            cidr: "   ".to_string(),
            label: "ok".to_string(),
            is_active: true,
        };
        assert!(matches!(
            validate_input(&blank_cidr).unwrap_err(),
            AppError::BadRequest(_)
        ));

        let blank_label = CreateAllowlistInput {
            cidr: "1.1.1.1/32".to_string(),
            label: "".to_string(),
            is_active: true,
        };
        assert!(matches!(
            validate_input(&blank_label).unwrap_err(),
            AppError::BadRequest(_)
        ));

        let too_long = CreateAllowlistInput {
            cidr: "1.1.1.1/32".to_string(),
            label: "x".repeat(201),
            is_active: true,
        };
        assert!(matches!(
            validate_input(&too_long).unwrap_err(),
            AppError::BadRequest(_)
        ));
    }
}
