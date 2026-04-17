//! FORM-01: Runtime-checked sqlx queries for forms / versions / submissions /
//! partials. Follows the `handlers::popups` + `consent::repo` convention —
//! compile-time macros are avoided so `DATABASE_URL` is never required at
//! build time.
//!
//! The DB-dependent tests belong to the integration-test suite (future
//! `tests/forms_repo.rs` once `tests/support/` grows a forms fixture). The
//! helpers here are unit-tested indirectly via the validation + handler
//! tests, and via the openapi snapshot for schema stability.

use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::{FromRow, PgPool, Postgres, Transaction};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::error::AppResult;

// ── Row shapes ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, FromRow, ToSchema)]
pub struct FormRow {
    pub id: Uuid,
    pub slug: String,
    pub name: String,
    pub description: Option<String>,
    pub is_active: bool,
    pub settings: serde_json::Value,
    pub created_by: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, FromRow, ToSchema)]
pub struct FormVersionRow {
    pub id: Uuid,
    pub form_id: Uuid,
    pub version: i32,
    pub schema_json: serde_json::Value,
    pub logic_json: serde_json::Value,
    pub is_published: bool,
    pub published_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, FromRow, ToSchema)]
pub struct SubmissionRow {
    pub id: Uuid,
    pub form_id: Uuid,
    pub form_version_id: Uuid,
    pub subject_id: Option<Uuid>,
    pub anonymous_id: Option<Uuid>,
    pub status: String,
    pub data_json: serde_json::Value,
    pub files_json: serde_json::Value,
    pub ip_hash: String,
    pub user_agent: String,
    pub referrer: Option<String>,
    pub utm: serde_json::Value,
    pub validation_errors: Option<serde_json::Value>,
    pub submitted_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, FromRow, ToSchema)]
pub struct PartialRow {
    pub id: Uuid,
    pub form_id: Uuid,
    pub resume_token_hash: String,
    pub data_json: serde_json::Value,
    pub subject_id: Option<Uuid>,
    pub expires_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

// ── Forms ──────────────────────────────────────────────────────────────

pub async fn create_form(
    pool: &PgPool,
    slug: &str,
    name: &str,
    description: Option<&str>,
    settings: &serde_json::Value,
    created_by: Option<Uuid>,
) -> AppResult<FormRow> {
    let row = sqlx::query_as::<_, FormRow>(
        r#"
        INSERT INTO forms (slug, name, description, settings, created_by)
        VALUES ($1, $2, $3, $4, $5)
        RETURNING id, slug, name, description, is_active, settings,
                  created_by, created_at, updated_at
        "#,
    )
    .bind(slug)
    .bind(name)
    .bind(description)
    .bind(settings)
    .bind(created_by)
    .fetch_one(pool)
    .await?;
    Ok(row)
}

pub async fn update_form(
    pool: &PgPool,
    id: Uuid,
    name: Option<&str>,
    description: Option<&str>,
    is_active: Option<bool>,
    settings: Option<&serde_json::Value>,
) -> AppResult<Option<FormRow>> {
    let row = sqlx::query_as::<_, FormRow>(
        r#"
        UPDATE forms SET
            name        = COALESCE($2, name),
            description = COALESCE($3, description),
            is_active   = COALESCE($4, is_active),
            settings    = COALESCE($5, settings),
            updated_at  = NOW()
        WHERE id = $1
        RETURNING id, slug, name, description, is_active, settings,
                  created_by, created_at, updated_at
        "#,
    )
    .bind(id)
    .bind(name)
    .bind(description)
    .bind(is_active)
    .bind(settings)
    .fetch_optional(pool)
    .await?;
    Ok(row)
}

pub async fn get_form_by_slug(pool: &PgPool, slug: &str) -> AppResult<Option<FormRow>> {
    let row = sqlx::query_as::<_, FormRow>(
        r#"
        SELECT id, slug, name, description, is_active, settings,
               created_by, created_at, updated_at
        FROM forms
        WHERE slug = $1
        "#,
    )
    .bind(slug)
    .fetch_optional(pool)
    .await?;
    Ok(row)
}

pub async fn get_form_by_id(pool: &PgPool, id: Uuid) -> AppResult<Option<FormRow>> {
    let row = sqlx::query_as::<_, FormRow>(
        r#"
        SELECT id, slug, name, description, is_active, settings,
               created_by, created_at, updated_at
        FROM forms
        WHERE id = $1
        "#,
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;
    Ok(row)
}

// ── Versions ───────────────────────────────────────────────────────────

pub async fn create_form_version(
    pool: &PgPool,
    form_id: Uuid,
    version: i32,
    schema_json: &serde_json::Value,
    logic_json: &serde_json::Value,
) -> AppResult<FormVersionRow> {
    let row = sqlx::query_as::<_, FormVersionRow>(
        r#"
        INSERT INTO form_versions (form_id, version, schema_json, logic_json)
        VALUES ($1, $2, $3, $4)
        RETURNING id, form_id, version, schema_json, logic_json,
                  is_published, published_at, created_at
        "#,
    )
    .bind(form_id)
    .bind(version)
    .bind(schema_json)
    .bind(logic_json)
    .fetch_one(pool)
    .await?;
    Ok(row)
}

/// Publish a version — atomically flips the previously-published row off and
/// the requested version on. `is_published = TRUE` is a soft singleton
/// (enforced by the partial index `idx_form_versions_active`).
pub async fn publish_form_version(
    pool: &PgPool,
    form_id: Uuid,
    version: i32,
) -> AppResult<Option<FormVersionRow>> {
    let mut tx: Transaction<'_, Postgres> = pool.begin().await?;

    sqlx::query(
        r#"
        UPDATE form_versions SET is_published = FALSE, published_at = published_at
        WHERE form_id = $1 AND is_published = TRUE
        "#,
    )
    .bind(form_id)
    .execute(&mut *tx)
    .await?;

    let row = sqlx::query_as::<_, FormVersionRow>(
        r#"
        UPDATE form_versions
        SET is_published = TRUE, published_at = NOW()
        WHERE form_id = $1 AND version = $2
        RETURNING id, form_id, version, schema_json, logic_json,
                  is_published, published_at, created_at
        "#,
    )
    .bind(form_id)
    .bind(version)
    .fetch_optional(&mut *tx)
    .await?;

    tx.commit().await?;
    Ok(row)
}

pub async fn get_active_version(
    pool: &PgPool,
    form_id: Uuid,
) -> AppResult<Option<FormVersionRow>> {
    let row = sqlx::query_as::<_, FormVersionRow>(
        r#"
        SELECT id, form_id, version, schema_json, logic_json,
               is_published, published_at, created_at
        FROM form_versions
        WHERE form_id = $1 AND is_published = TRUE
        ORDER BY published_at DESC
        LIMIT 1
        "#,
    )
    .bind(form_id)
    .fetch_optional(pool)
    .await?;
    Ok(row)
}

// ── Submissions ────────────────────────────────────────────────────────

/// Arguments for [`insert_submission`]; bundled so the handler doesn't carry
/// a 12-arg call site.
pub struct InsertSubmission<'a> {
    pub form_id: Uuid,
    pub form_version_id: Uuid,
    pub subject_id: Option<Uuid>,
    pub anonymous_id: Option<Uuid>,
    pub status: &'a str,
    pub data_json: &'a serde_json::Value,
    pub files_json: &'a serde_json::Value,
    pub ip_hash: &'a str,
    pub user_agent: &'a str,
    pub referrer: Option<&'a str>,
    pub utm: &'a serde_json::Value,
    pub validation_errors: Option<&'a serde_json::Value>,
}

pub async fn insert_submission<'a>(
    tx: &mut Transaction<'_, Postgres>,
    args: InsertSubmission<'a>,
) -> AppResult<SubmissionRow> {
    let row = sqlx::query_as::<_, SubmissionRow>(
        r#"
        INSERT INTO form_submissions (
            form_id, form_version_id, subject_id, anonymous_id, status,
            data_json, files_json, ip_hash, user_agent, referrer, utm,
            validation_errors
        ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
        RETURNING id, form_id, form_version_id, subject_id, anonymous_id,
                  status, data_json, files_json, ip_hash, user_agent,
                  referrer, utm, validation_errors, submitted_at
        "#,
    )
    .bind(args.form_id)
    .bind(args.form_version_id)
    .bind(args.subject_id)
    .bind(args.anonymous_id)
    .bind(args.status)
    .bind(args.data_json)
    .bind(args.files_json)
    .bind(args.ip_hash)
    .bind(args.user_agent)
    .bind(args.referrer)
    .bind(args.utm)
    .bind(args.validation_errors)
    .fetch_one(&mut **tx)
    .await?;
    Ok(row)
}

/// List submissions for an admin view. Supports status + date-range filtering
/// and is paginated.
pub struct SubmissionListFilter<'a> {
    pub form_id: Uuid,
    pub status: Option<&'a str>,
    pub from: Option<DateTime<Utc>>,
    pub to: Option<DateTime<Utc>>,
    pub limit: i64,
    pub offset: i64,
}

pub async fn list_submissions<'a>(
    pool: &PgPool,
    f: SubmissionListFilter<'a>,
) -> AppResult<(Vec<SubmissionRow>, i64)> {
    let rows = sqlx::query_as::<_, SubmissionRow>(
        r#"
        SELECT id, form_id, form_version_id, subject_id, anonymous_id,
               status, data_json, files_json, ip_hash, user_agent,
               referrer, utm, validation_errors, submitted_at
        FROM form_submissions
        WHERE form_id = $1
          AND ($2::text IS NULL OR status = $2)
          AND ($3::timestamptz IS NULL OR submitted_at >= $3)
          AND ($4::timestamptz IS NULL OR submitted_at <= $4)
        ORDER BY submitted_at DESC
        LIMIT $5 OFFSET $6
        "#,
    )
    .bind(f.form_id)
    .bind(f.status)
    .bind(f.from)
    .bind(f.to)
    .bind(f.limit)
    .bind(f.offset)
    .fetch_all(pool)
    .await?;

    let total: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*) FROM form_submissions
        WHERE form_id = $1
          AND ($2::text IS NULL OR status = $2)
          AND ($3::timestamptz IS NULL OR submitted_at >= $3)
          AND ($4::timestamptz IS NULL OR submitted_at <= $4)
        "#,
    )
    .bind(f.form_id)
    .bind(f.status)
    .bind(f.from)
    .bind(f.to)
    .fetch_one(pool)
    .await?;

    Ok((rows, total))
}

/// Bulk-update submission statuses; returns the count of rows affected.
/// The caller has already validated that `action` maps to a valid status.
pub async fn bulk_update_submission_status(
    pool: &PgPool,
    form_id: Uuid,
    ids: &[Uuid],
    new_status: &str,
) -> AppResult<u64> {
    let result = sqlx::query(
        r#"
        UPDATE form_submissions
        SET status = $3
        WHERE form_id = $1 AND id = ANY($2)
        "#,
    )
    .bind(form_id)
    .bind(ids)
    .bind(new_status)
    .execute(pool)
    .await?;
    Ok(result.rows_affected())
}

// ── Partials ───────────────────────────────────────────────────────────

pub async fn insert_partial(
    pool: &PgPool,
    form_id: Uuid,
    resume_token_hash: &str,
    data_json: &serde_json::Value,
    subject_id: Option<Uuid>,
    expires_at: DateTime<Utc>,
) -> AppResult<PartialRow> {
    let row = sqlx::query_as::<_, PartialRow>(
        r#"
        INSERT INTO form_partials (form_id, resume_token_hash, data_json, subject_id, expires_at)
        VALUES ($1, $2, $3, $4, $5)
        RETURNING id, form_id, resume_token_hash, data_json, subject_id,
                  expires_at, created_at
        "#,
    )
    .bind(form_id)
    .bind(resume_token_hash)
    .bind(data_json)
    .bind(subject_id)
    .bind(expires_at)
    .fetch_one(pool)
    .await?;
    Ok(row)
}

/// Look up a partial draft by its SHA-256 token hash; returns `None` if the
/// token is unknown or the row has expired.
pub async fn resolve_partial(
    pool: &PgPool,
    form_id: Uuid,
    resume_token_hash: &str,
) -> AppResult<Option<PartialRow>> {
    let row = sqlx::query_as::<_, PartialRow>(
        r#"
        SELECT id, form_id, resume_token_hash, data_json, subject_id,
               expires_at, created_at
        FROM form_partials
        WHERE form_id = $1 AND resume_token_hash = $2 AND expires_at > NOW()
        "#,
    )
    .bind(form_id)
    .bind(resume_token_hash)
    .fetch_optional(pool)
    .await?;
    Ok(row)
}
