//! Read-side queries for the consent tables seeded in migration `024_consent.sql`.
//!
//! Write-side CRUD (create/update/delete banner configs, categories, services,
//! policies) is bundled with CONSENT-07's admin UI — these read helpers are
//! what the public banner endpoint and the CONSENT-02 script-blocker gate rely
//! on, and they are the stable surface across subsystem boundaries.
//!
//! Queries use the runtime-checked form (`sqlx::query_as::<_, Row>`) to match
//! the pattern already established by `handlers::popups` and
//! `notifications::templates` — the compile-time macro is avoided crate-wide
//! wherever a row contains `JSONB` or `TEXT[]`, both of which appear here.

use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::{FromRow, PgPool};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::error::AppResult;

#[derive(Debug, Clone, Serialize, FromRow, ToSchema)]
pub struct CategoryRow {
    pub key: String,
    pub label: String,
    pub description: String,
    pub is_required: bool,
    pub sort_order: i32,
}

#[derive(Debug, Clone, Serialize, FromRow, ToSchema)]
pub struct ServiceRow {
    pub id: Uuid,
    pub slug: String,
    pub name: String,
    pub vendor: String,
    pub category: String,
    pub domains: Vec<String>,
    pub cookies: serde_json::Value,
    pub privacy_url: Option<String>,
    pub description: Option<String>,
    pub is_active: bool,
}

#[derive(Debug, Clone, Serialize, FromRow, ToSchema)]
pub struct BannerConfigRow {
    pub id: Uuid,
    pub region: String,
    pub locale: String,
    pub version: i32,
    pub layout: String,
    pub position: String,
    pub theme_json: serde_json::Value,
    pub copy_json: serde_json::Value,
    pub is_active: bool,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, FromRow, ToSchema)]
pub struct PolicyRow {
    pub version: i32,
    pub locale: String,
    pub effective_at: DateTime<Utc>,
}

/// Fetch every category sorted by `sort_order`, then `key` as a tiebreaker.
pub async fn list_categories(pool: &PgPool) -> AppResult<Vec<CategoryRow>> {
    let rows = sqlx::query_as::<_, CategoryRow>(
        r#"
        SELECT key, label, description, is_required, sort_order
        FROM consent_categories
        ORDER BY sort_order ASC, key ASC
        "#,
    )
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

/// Fetch every active service, ordered so client-side grouping is stable.
pub async fn list_active_services(pool: &PgPool) -> AppResult<Vec<ServiceRow>> {
    let rows = sqlx::query_as::<_, ServiceRow>(
        r#"
        SELECT id, slug, name, vendor, category, domains, cookies,
               privacy_url, description, is_active
        FROM consent_services
        WHERE is_active = TRUE
        ORDER BY category ASC, name ASC
        "#,
    )
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

/// Latest active policy for a given locale, falling back to `en` when the
/// exact locale is missing.
pub async fn latest_policy(pool: &PgPool, locale: &str) -> AppResult<Option<PolicyRow>> {
    let row = sqlx::query_as::<_, PolicyRow>(
        r#"
        SELECT version, locale, effective_at
        FROM consent_policies
        WHERE locale = $1 OR locale = 'en'
        ORDER BY (locale = $1) DESC, effective_at DESC
        LIMIT 1
        "#,
    )
    .bind(locale)
    .fetch_optional(pool)
    .await?;
    Ok(row)
}

/// Resolve a banner config for `(region, locale)` with graceful fallbacks:
///   1. Exact (region, locale).
///   2. (region, 'en').
///   3. ('default', locale).
///   4. ('default', 'en') — always seeded by `024_consent.sql`.
///
/// Inactive configs are skipped at every step.
pub async fn resolve_banner(
    pool: &PgPool,
    region: &str,
    locale: &str,
) -> AppResult<Option<BannerConfigRow>> {
    let row = sqlx::query_as::<_, BannerConfigRow>(
        r#"
        SELECT id, region, locale, version, layout, position,
               theme_json, copy_json, is_active, updated_at
        FROM consent_banner_configs
        WHERE is_active = TRUE
          AND (
              (region = $1 AND locale = $2) OR
              (region = $1 AND locale = 'en') OR
              (region = 'default' AND locale = $2) OR
              (region = 'default' AND locale = 'en')
          )
        ORDER BY
            (region = $1 AND locale = $2)   DESC,
            (region = $1 AND locale = 'en') DESC,
            (region = 'default' AND locale = $2) DESC,
            (region = 'default' AND locale = 'en') DESC
        LIMIT 1
        "#,
    )
    .bind(region)
    .bind(locale)
    .fetch_optional(pool)
    .await?;
    Ok(row)
}
