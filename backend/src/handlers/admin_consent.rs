//! CONSENT-07: admin CRUD for banner configs, categories, services, and
//! policies. Every mutating endpoint emits a `consent.admin.mutated` outbox
//! event so the audit trail captures who changed what and when.
//!
//! Routes (all admin-gated via the `AdminUser` extractor):
//!
//!   * GET  /api/admin/consent/banners
//!   * POST /api/admin/consent/banners
//!   * GET  /api/admin/consent/banners/{id}
//!   * PUT  /api/admin/consent/banners/{id}
//!   * GET  /api/admin/consent/categories
//!   * POST /api/admin/consent/categories
//!   * PUT  /api/admin/consent/categories/{key}
//!   * GET  /api/admin/consent/services
//!   * POST /api/admin/consent/services
//!   * PUT  /api/admin/consent/services/{id}
//!   * GET  /api/admin/consent/policies
//!   * POST /api/admin/consent/policies        — bumps the version implicitly
//!   * GET  /api/admin/consent/log            — CONSENT-03 `consent_records` view
//!   * GET  /api/admin/consent/integrity      — CONSENT-07 anchor list
//!
//! Delete is intentionally NOT exposed for any of these surfaces — the data
//! is append-only / version-bumped. Deactivation (setting `is_active = false`)
//! is the closest thing to "delete" the admin has.

use axum::{
    extract::{Path, Query, State},
    routing::{get, put},
    Json, Router,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::Row;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{
    consent::{integrity, repo},
    error::{AppError, AppResult},
    events::{self, outbox::Event},
    extractors::AdminUser,
    AppState,
};

pub fn router() -> Router<AppState> {
    Router::new()
        // Banner configs
        .route("/banners", get(list_banners).post(create_banner))
        .route("/banners/{id}", get(get_banner).put(update_banner))
        // Categories
        .route("/categories", get(list_categories).post(create_category))
        .route("/categories/{key}", put(update_category))
        // Services
        .route("/services", get(list_services).post(create_service))
        .route("/services/{id}", put(update_service))
        // Policies — versioned, append-only.
        .route("/policies", get(list_policies).post(create_policy))
        // Consent log (CONSENT-03) — read-only.
        .route("/log", get(list_log))
        // Integrity anchors (CONSENT-07).
        .route("/integrity", get(list_integrity))
}

// ── Outbox helpers ──────────────────────────────────────────────────────

/// Emit a `consent.admin.mutated` outbox event. Shaped identically for
/// every CRUD mutation so the admin log is a uniform stream.
async fn emit_audit_event(
    pool: &sqlx::PgPool,
    actor_id: Uuid,
    aggregate: &str,
    aggregate_id: String,
    action: &str,
    payload: serde_json::Value,
) -> AppResult<()> {
    let mut tx = pool.begin().await?;
    let evt = Event {
        aggregate_type: format!("consent.{aggregate}"),
        aggregate_id,
        event_type: "consent.admin.mutated".to_string(),
        payload: serde_json::json!({
            "action": action,
            "actor_id": actor_id,
            "detail": payload,
        }),
        headers: events::outbox::EventHeaders::default(),
    };
    events::outbox::publish_in_tx(&mut tx, &evt)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("outbox publish failed: {e}")))?;
    tx.commit().await?;
    Ok(())
}

// ── Banner configs ──────────────────────────────────────────────────────

#[derive(Debug, Deserialize, ToSchema)]
pub struct BannerUpsertRequest {
    pub region: String,
    pub locale: String,
    pub layout: String,
    pub position: String,
    pub copy_json: serde_json::Value,
    #[serde(default)]
    pub theme_json: serde_json::Value,
    #[serde(default = "default_true")]
    pub is_active: bool,
}

fn default_true() -> bool {
    true
}

async fn list_banners(
    State(state): State<AppState>,
    _admin: AdminUser,
) -> AppResult<Json<Vec<repo::BannerConfigRow>>> {
    let rows = sqlx::query_as::<_, repo::BannerConfigRow>(
        r#"
        SELECT id, region, locale, version, layout, position,
               theme_json, copy_json, is_active, updated_at
        FROM consent_banner_configs
        ORDER BY region ASC, locale ASC
        "#,
    )
    .fetch_all(&state.db)
    .await?;
    Ok(Json(rows))
}

async fn get_banner(
    State(state): State<AppState>,
    _admin: AdminUser,
    Path(id): Path<Uuid>,
) -> AppResult<Json<repo::BannerConfigRow>> {
    let row = sqlx::query_as::<_, repo::BannerConfigRow>(
        r#"
        SELECT id, region, locale, version, layout, position,
               theme_json, copy_json, is_active, updated_at
        FROM consent_banner_configs
        WHERE id = $1
        "#,
    )
    .bind(id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| AppError::NotFound("banner config not found".to_string()))?;
    Ok(Json(row))
}

async fn create_banner(
    State(state): State<AppState>,
    admin: AdminUser,
    Json(req): Json<BannerUpsertRequest>,
) -> AppResult<Json<repo::BannerConfigRow>> {
    let row = sqlx::query_as::<_, repo::BannerConfigRow>(
        r#"
        INSERT INTO consent_banner_configs
            (region, locale, layout, position, copy_json, theme_json, is_active)
        VALUES ($1, $2, $3, $4, $5, $6, $7)
        RETURNING id, region, locale, version, layout, position,
                  theme_json, copy_json, is_active, updated_at
        "#,
    )
    .bind(&req.region)
    .bind(&req.locale)
    .bind(&req.layout)
    .bind(&req.position)
    .bind(&req.copy_json)
    .bind(&req.theme_json)
    .bind(req.is_active)
    .fetch_one(&state.db)
    .await?;

    emit_audit_event(
        &state.db,
        admin.user_id,
        "banner",
        row.id.to_string(),
        "create",
        serde_json::json!({ "region": row.region, "locale": row.locale }),
    )
    .await?;

    Ok(Json(row))
}

async fn update_banner(
    State(state): State<AppState>,
    admin: AdminUser,
    Path(id): Path<Uuid>,
    Json(req): Json<BannerUpsertRequest>,
) -> AppResult<Json<repo::BannerConfigRow>> {
    let row = sqlx::query_as::<_, repo::BannerConfigRow>(
        r#"
        UPDATE consent_banner_configs
        SET region = $2,
            locale = $3,
            layout = $4,
            position = $5,
            copy_json = $6,
            theme_json = $7,
            is_active = $8,
            version = version + 1,
            updated_at = NOW()
        WHERE id = $1
        RETURNING id, region, locale, version, layout, position,
                  theme_json, copy_json, is_active, updated_at
        "#,
    )
    .bind(id)
    .bind(&req.region)
    .bind(&req.locale)
    .bind(&req.layout)
    .bind(&req.position)
    .bind(&req.copy_json)
    .bind(&req.theme_json)
    .bind(req.is_active)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| AppError::NotFound("banner config not found".to_string()))?;

    emit_audit_event(
        &state.db,
        admin.user_id,
        "banner",
        row.id.to_string(),
        "update",
        serde_json::json!({ "region": row.region, "locale": row.locale, "version": row.version }),
    )
    .await?;

    Ok(Json(row))
}

// ── Categories ──────────────────────────────────────────────────────────

#[derive(Debug, Deserialize, ToSchema)]
pub struct CategoryUpsertRequest {
    pub key: String,
    pub label: String,
    pub description: String,
    #[serde(default)]
    pub is_required: bool,
    #[serde(default)]
    pub sort_order: i32,
}

async fn list_categories(
    State(state): State<AppState>,
    _admin: AdminUser,
) -> AppResult<Json<Vec<repo::CategoryRow>>> {
    let rows = repo::list_categories(&state.db).await?;
    Ok(Json(rows))
}

async fn create_category(
    State(state): State<AppState>,
    admin: AdminUser,
    Json(req): Json<CategoryUpsertRequest>,
) -> AppResult<Json<repo::CategoryRow>> {
    if req.key == "necessary" {
        return Err(AppError::BadRequest(
            "the `necessary` category is protected and cannot be mutated via the admin API"
                .to_string(),
        ));
    }
    let row = sqlx::query_as::<_, repo::CategoryRow>(
        r#"
        INSERT INTO consent_categories (key, label, description, is_required, sort_order)
        VALUES ($1, $2, $3, $4, $5)
        RETURNING key, label, description, is_required, sort_order
        "#,
    )
    .bind(&req.key)
    .bind(&req.label)
    .bind(&req.description)
    .bind(req.is_required)
    .bind(req.sort_order)
    .fetch_one(&state.db)
    .await?;

    emit_audit_event(
        &state.db,
        admin.user_id,
        "category",
        row.key.clone(),
        "create",
        serde_json::json!({ "label": row.label }),
    )
    .await?;

    Ok(Json(row))
}

async fn update_category(
    State(state): State<AppState>,
    admin: AdminUser,
    Path(key): Path<String>,
    Json(req): Json<CategoryUpsertRequest>,
) -> AppResult<Json<repo::CategoryRow>> {
    if key == "necessary" {
        return Err(AppError::BadRequest(
            "the `necessary` category is protected and cannot be mutated via the admin API"
                .to_string(),
        ));
    }
    let row = sqlx::query_as::<_, repo::CategoryRow>(
        r#"
        UPDATE consent_categories
        SET label = $2,
            description = $3,
            is_required = $4,
            sort_order = $5,
            updated_at = NOW()
        WHERE key = $1
        RETURNING key, label, description, is_required, sort_order
        "#,
    )
    .bind(&key)
    .bind(&req.label)
    .bind(&req.description)
    .bind(req.is_required)
    .bind(req.sort_order)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| AppError::NotFound("category not found".to_string()))?;

    emit_audit_event(
        &state.db,
        admin.user_id,
        "category",
        row.key.clone(),
        "update",
        serde_json::json!({ "label": row.label, "sort_order": row.sort_order }),
    )
    .await?;

    Ok(Json(row))
}

// ── Services ────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize, ToSchema)]
pub struct ServiceUpsertRequest {
    pub slug: String,
    pub name: String,
    pub vendor: String,
    pub category: String,
    #[serde(default)]
    pub domains: Vec<String>,
    #[serde(default = "default_empty_json_array")]
    pub cookies: serde_json::Value,
    pub privacy_url: Option<String>,
    pub description: Option<String>,
    #[serde(default = "default_true")]
    pub is_active: bool,
}

fn default_empty_json_array() -> serde_json::Value {
    serde_json::json!([])
}

async fn list_services(
    State(state): State<AppState>,
    _admin: AdminUser,
) -> AppResult<Json<Vec<repo::ServiceRow>>> {
    let rows = sqlx::query_as::<_, repo::ServiceRow>(
        r#"
        SELECT id, slug, name, vendor, category, domains, cookies,
               privacy_url, description, is_active
        FROM consent_services
        ORDER BY category ASC, name ASC
        "#,
    )
    .fetch_all(&state.db)
    .await?;
    Ok(Json(rows))
}

async fn create_service(
    State(state): State<AppState>,
    admin: AdminUser,
    Json(req): Json<ServiceUpsertRequest>,
) -> AppResult<Json<repo::ServiceRow>> {
    let row = sqlx::query_as::<_, repo::ServiceRow>(
        r#"
        INSERT INTO consent_services
            (slug, name, vendor, category, domains, cookies, privacy_url, description, is_active)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
        RETURNING id, slug, name, vendor, category, domains, cookies,
                  privacy_url, description, is_active
        "#,
    )
    .bind(&req.slug)
    .bind(&req.name)
    .bind(&req.vendor)
    .bind(&req.category)
    .bind(&req.domains)
    .bind(&req.cookies)
    .bind(&req.privacy_url)
    .bind(&req.description)
    .bind(req.is_active)
    .fetch_one(&state.db)
    .await?;

    emit_audit_event(
        &state.db,
        admin.user_id,
        "service",
        row.id.to_string(),
        "create",
        serde_json::json!({ "slug": row.slug, "category": row.category }),
    )
    .await?;

    Ok(Json(row))
}

async fn update_service(
    State(state): State<AppState>,
    admin: AdminUser,
    Path(id): Path<Uuid>,
    Json(req): Json<ServiceUpsertRequest>,
) -> AppResult<Json<repo::ServiceRow>> {
    let row = sqlx::query_as::<_, repo::ServiceRow>(
        r#"
        UPDATE consent_services
        SET slug = $2,
            name = $3,
            vendor = $4,
            category = $5,
            domains = $6,
            cookies = $7,
            privacy_url = $8,
            description = $9,
            is_active = $10,
            updated_at = NOW()
        WHERE id = $1
        RETURNING id, slug, name, vendor, category, domains, cookies,
                  privacy_url, description, is_active
        "#,
    )
    .bind(id)
    .bind(&req.slug)
    .bind(&req.name)
    .bind(&req.vendor)
    .bind(&req.category)
    .bind(&req.domains)
    .bind(&req.cookies)
    .bind(&req.privacy_url)
    .bind(&req.description)
    .bind(req.is_active)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| AppError::NotFound("service not found".to_string()))?;

    emit_audit_event(
        &state.db,
        admin.user_id,
        "service",
        row.id.to_string(),
        "update",
        serde_json::json!({ "slug": row.slug }),
    )
    .await?;

    Ok(Json(row))
}

// ── Policies ────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, ToSchema, sqlx::FromRow)]
pub struct PolicyDetail {
    pub id: Uuid,
    pub version: i32,
    pub markdown: String,
    pub locale: String,
    pub effective_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct PolicyCreateRequest {
    pub markdown: String,
    #[serde(default = "default_en")]
    pub locale: String,
}

fn default_en() -> String {
    "en".to_string()
}

async fn list_policies(
    State(state): State<AppState>,
    _admin: AdminUser,
) -> AppResult<Json<Vec<PolicyDetail>>> {
    let rows = sqlx::query_as::<_, PolicyDetail>(
        r#"
        SELECT id, version, markdown, locale, effective_at, created_at
        FROM consent_policies
        ORDER BY effective_at DESC, version DESC
        "#,
    )
    .fetch_all(&state.db)
    .await?;
    Ok(Json(rows))
}

async fn create_policy(
    State(state): State<AppState>,
    admin: AdminUser,
    Json(req): Json<PolicyCreateRequest>,
) -> AppResult<Json<PolicyDetail>> {
    // Next version = max(existing) + 1 for this locale. Default `en` catches
    // the common case where an admin doesn't specify a locale.
    let next_version: i32 = sqlx::query(
        r#"
        SELECT COALESCE(MAX(version), 0) + 1 AS v
        FROM consent_policies
        WHERE locale = $1
        "#,
    )
    .bind(&req.locale)
    .fetch_one(&state.db)
    .await?
    .try_get::<i32, _>("v")?;

    let row = sqlx::query_as::<_, PolicyDetail>(
        r#"
        INSERT INTO consent_policies (version, markdown, locale)
        VALUES ($1, $2, $3)
        RETURNING id, version, markdown, locale, effective_at, created_at
        "#,
    )
    .bind(next_version)
    .bind(&req.markdown)
    .bind(&req.locale)
    .fetch_one(&state.db)
    .await?;

    emit_audit_event(
        &state.db,
        admin.user_id,
        "policy",
        row.id.to_string(),
        "create",
        serde_json::json!({ "version": row.version, "locale": row.locale }),
    )
    .await?;

    Ok(Json(row))
}

// ── Consent log (CONSENT-03 read) ───────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct LogQuery {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ConsentLogRow {
    pub id: Uuid,
    pub subject_id: Option<String>,
    pub action: String,
    pub categories: serde_json::Value,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ConsentLogResponse {
    pub rows: Vec<ConsentLogRow>,
    pub table_present: bool,
    pub total: i64,
}

async fn list_log(
    State(state): State<AppState>,
    _admin: AdminUser,
    Query(q): Query<LogQuery>,
) -> AppResult<Json<ConsentLogResponse>> {
    // Table may not exist yet in the current DB (CONSENT-03 landed in a
    // sibling worktree). We return `table_present: false` + an empty rows
    // list so the admin UI can show a friendly "run the migration" notice
    // instead of failing with a 500.
    let table_exists: bool = sqlx::query_scalar::<_, bool>(
        r#"
        SELECT EXISTS (
            SELECT 1 FROM pg_catalog.pg_class c
            JOIN pg_catalog.pg_namespace n ON n.oid = c.relnamespace
            WHERE c.relname = 'consent_records' AND n.nspname = 'public'
        )
        "#,
    )
    .fetch_one(&state.db)
    .await?;

    if !table_exists {
        return Ok(Json(ConsentLogResponse {
            rows: vec![],
            table_present: false,
            total: 0,
        }));
    }

    let limit = q.limit.unwrap_or(50).clamp(1, 500);
    let offset = q.offset.unwrap_or(0).max(0);

    let rows = sqlx::query_as::<
        _,
        (
            Uuid,
            Option<String>,
            String,
            serde_json::Value,
            DateTime<Utc>,
        ),
    >(
        r#"
        SELECT id, subject_id::text, action::text, categories, created_at
        FROM consent_records
        ORDER BY created_at DESC
        LIMIT $1 OFFSET $2
        "#,
    )
    .bind(limit)
    .bind(offset)
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();

    let total: i64 = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM consent_records")
        .fetch_one(&state.db)
        .await
        .unwrap_or(0);

    let mapped: Vec<ConsentLogRow> = rows
        .into_iter()
        .map(
            |(id, subject_id, action, categories, created_at)| ConsentLogRow {
                id,
                subject_id,
                action,
                categories,
                created_at,
            },
        )
        .collect();

    Ok(Json(ConsentLogResponse {
        rows: mapped,
        table_present: true,
        total,
    }))
}

// ── Integrity anchors ───────────────────────────────────────────────────

#[derive(Debug, Serialize, ToSchema)]
pub struct IntegrityAnchorDto {
    pub id: Uuid,
    pub anchor_hash: String,
    pub record_count: i32,
    pub window_start_at: Option<DateTime<Utc>>,
    pub window_end_at: Option<DateTime<Utc>>,
    pub anchored_at: DateTime<Utc>,
}

impl From<integrity::IntegrityAnchor> for IntegrityAnchorDto {
    fn from(a: integrity::IntegrityAnchor) -> Self {
        IntegrityAnchorDto {
            id: a.id,
            anchor_hash: a.anchor_hash,
            record_count: a.record_count,
            window_start_at: a.window_start_at,
            window_end_at: a.window_end_at,
            anchored_at: a.anchored_at,
        }
    }
}

async fn list_integrity(
    State(state): State<AppState>,
    _admin: AdminUser,
) -> AppResult<Json<Vec<IntegrityAnchorDto>>> {
    let rows = integrity::list_anchors(&state.db, 100).await?;
    Ok(Json(
        rows.into_iter().map(IntegrityAnchorDto::from).collect(),
    ))
}
