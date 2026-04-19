//! ADM-14: audit log viewer with FTS over `admin_actions`.
//!
//! Mounted at `/api/admin/audit`. Migration `070` ships:
//!
//!   * A `tsvector` GIN over `(action, target_kind, target_id,
//!     jsonb_pretty(metadata))` for free-text scans.
//!   * A `jsonb_path_ops` GIN on `metadata` for `metadata @> $`
//!     containment probes.
//!   * A `pg_trgm` GIN on `target_id` for substring matches on
//!     UUIDs / order numbers.
//!
//! Filters expose every shape investigators need:
//!
//!   * `q` — free-text via `plainto_tsquery('simple', $)`.
//!   * `actor_id`, `action`, `target_kind` — exact matches.
//!   * `target_id` — substring (trigram-backed).
//!   * `metadata_contains` — JSON object via `metadata @> $::jsonb`.
//!   * `from` / `to` — inclusive `created_at` bounds.
//!
//! Pagination is offset-based with a hard `MAX_LIMIT` cap; CSV
//! export is gated on a separate permission and capped at
//! `MAX_EXPORT_ROWS` so a runaway filter cannot pin a connection.

use std::fmt::Write as _;

use axum::{
    extract::{Path, Query, State},
    http::{header, StatusCode},
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{
    error::{AppError, AppResult},
    extractors::{ClientInfo, PrivilegedUser},
    services::audit::audit_admin_priv_no_target,
    AppState,
};

const PERM_READ: &str = "admin.audit.read";
const PERM_EXPORT: &str = "admin.audit.export";

const DEFAULT_LIMIT: i64 = 50;
const MAX_LIMIT: i64 = 200;
const MAX_EXPORT_ROWS: i64 = 100_000;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(list))
        .route("/export.csv", get(export_csv))
        .route("/{id}", get(read_one))
}

// ── DTOs ───────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, sqlx::FromRow, ToSchema)]
pub struct AuditRow {
    pub id: Uuid,
    pub actor_id: Uuid,
    pub actor_role: String,
    pub action: String,
    pub target_kind: String,
    pub target_id: Option<String>,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub metadata: Value,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct ListQuery {
    #[serde(default)]
    pub q: Option<String>,
    #[serde(default)]
    pub actor_id: Option<Uuid>,
    #[serde(default)]
    pub action: Option<String>,
    #[serde(default)]
    pub target_kind: Option<String>,
    #[serde(default)]
    pub target_id: Option<String>,
    /// JSON object — applied via `metadata @> $::jsonb`. Operators
    /// pass `metadata_contains={"reason":"fraud"}` etc.
    #[serde(default)]
    pub metadata_contains: Option<String>,
    #[serde(default)]
    pub from: Option<DateTime<Utc>>,
    #[serde(default)]
    pub to: Option<DateTime<Utc>>,
    #[serde(default)]
    pub limit: Option<i64>,
    #[serde(default)]
    pub offset: Option<i64>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct AuditListEnvelope {
    pub data: Vec<AuditRow>,
    pub total: i64,
    pub page: i64,
    pub per_page: i64,
    pub total_pages: i64,
}

// Stable WHERE snippet shared by list + count + export. Slots:
//   $1 = q (text, NULL = skip)             — full-text
//   $2 = actor_id (uuid, NULL = skip)
//   $3 = action (text, NULL = skip)
//   $4 = target_kind (text, NULL = skip)
//   $5 = target_id (text substring, NULL = skip)
//   $6 = metadata_contains (jsonb, NULL = skip)
//   $7 = from (timestamptz, NULL = skip)
//   $8 = to   (timestamptz, NULL = skip)
const WHERE_SQL: &str = r#"
    WHERE ($1::text  IS NULL OR search_tsv @@ plainto_tsquery('simple', $1))
      AND ($2::uuid  IS NULL OR actor_id    = $2)
      AND ($3::text  IS NULL OR action      = $3)
      AND ($4::text  IS NULL OR target_kind = $4)
      AND ($5::text  IS NULL OR target_id ILIKE '%' || $5 || '%')
      AND ($6::jsonb IS NULL OR metadata @> $6)
      AND ($7::timestamptz IS NULL OR created_at >= $7)
      AND ($8::timestamptz IS NULL OR created_at <= $8)
"#;

const SELECT_COLS: &str = r#"
    SELECT id, actor_id, actor_role::text AS actor_role, action,
           target_kind, target_id, host(ip_address) AS ip_address,
           user_agent, metadata, created_at
"#;

// ── Handlers ───────────────────────────────────────────────────────────

#[utoipa::path(
    get,
    path = "/api/admin/audit",
    tag = "admin-audit",
    security(("bearer_auth" = [])),
    params(
        ("q"                 = Option<String>,        Query, description = "Full-text query"),
        ("actor_id"          = Option<Uuid>,          Query, description = "Filter by actor"),
        ("action"            = Option<String>,        Query, description = "Exact action key"),
        ("target_kind"       = Option<String>,        Query, description = "Exact target kind"),
        ("target_id"         = Option<String>,        Query, description = "Substring on target_id"),
        ("metadata_contains" = Option<String>,        Query, description = "JSON object — metadata @> $"),
        ("from"              = Option<DateTime<Utc>>, Query, description = "Inclusive lower bound"),
        ("to"                = Option<DateTime<Utc>>, Query, description = "Inclusive upper bound"),
        ("limit"             = Option<i64>,           Query, description = "Page size (1..=200)"),
        ("offset"            = Option<i64>,           Query, description = "Cursor offset"),
    ),
    responses(
        (status = 200, description = "Paginated audit rows", body = AuditListEnvelope),
        (status = 400, description = "Invalid filter"),
        (status = 401, description = "Unauthenticated"),
        (status = 403, description = "Forbidden")
    )
)]
pub async fn list(
    State(state): State<AppState>,
    privileged: PrivilegedUser,
    Query(q): Query<ListQuery>,
) -> AppResult<Json<AuditListEnvelope>> {
    privileged.require(&state.policy, PERM_READ)?;

    let f = Filters::from(&q)?;
    let limit = q.limit.unwrap_or(DEFAULT_LIMIT).clamp(1, MAX_LIMIT);
    let offset = q.offset.unwrap_or(0).max(0);

    let list_sql = format!(
        "{SELECT_COLS} FROM admin_actions {WHERE_SQL} \
         ORDER BY created_at DESC, id DESC LIMIT $9 OFFSET $10",
    );
    let count_sql = format!("SELECT COUNT(*) FROM admin_actions {WHERE_SQL}");

    let data: Vec<AuditRow> = f
        .bind_into(sqlx::query_as::<_, AuditRow>(&list_sql))
        .bind(limit)
        .bind(offset)
        .fetch_all(&state.db)
        .await?;
    let total: (i64,) = f
        .bind_into(sqlx::query_as::<_, (i64,)>(&count_sql))
        .fetch_one(&state.db)
        .await?;

    let per_page = limit;
    let page = (offset / per_page.max(1)) + 1;
    let total_pages = (total.0 as f64 / per_page as f64).ceil() as i64;

    Ok(Json(AuditListEnvelope {
        data,
        total: total.0,
        page,
        per_page,
        total_pages,
    }))
}

#[utoipa::path(
    get,
    path = "/api/admin/audit/{id}",
    tag = "admin-audit",
    security(("bearer_auth" = [])),
    params(("id" = Uuid, Path, description = "Audit row id")),
    responses(
        (status = 200, description = "Audit row", body = AuditRow),
        (status = 404, description = "Row not found")
    )
)]
pub async fn read_one(
    State(state): State<AppState>,
    privileged: PrivilegedUser,
    Path(id): Path<Uuid>,
) -> AppResult<Json<AuditRow>> {
    privileged.require(&state.policy, PERM_READ)?;

    let sql = format!("{SELECT_COLS} FROM admin_actions WHERE id = $1");
    let row = sqlx::query_as::<_, AuditRow>(&sql)
        .bind(id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| AppError::NotFound("audit row not found".into()))?;
    Ok(Json(row))
}

#[utoipa::path(
    get,
    path = "/api/admin/audit/export.csv",
    tag = "admin-audit",
    security(("bearer_auth" = [])),
    params(
        ("q"                 = Option<String>,        Query, description = "Full-text query"),
        ("actor_id"          = Option<Uuid>,          Query, description = "Filter by actor"),
        ("action"            = Option<String>,        Query, description = "Exact action"),
        ("target_kind"       = Option<String>,        Query, description = "Exact target_kind"),
        ("target_id"         = Option<String>,        Query, description = "Substring target_id"),
        ("metadata_contains" = Option<String>,        Query, description = "JSON containment"),
        ("from"              = Option<DateTime<Utc>>, Query, description = "Inclusive lower bound"),
        ("to"                = Option<DateTime<Utc>>, Query, description = "Inclusive upper bound"),
    ),
    responses(
        (status = 200, description = "CSV stream", content_type = "text/csv"),
        (status = 401, description = "Unauthenticated"),
        (status = 403, description = "Forbidden")
    )
)]
pub async fn export_csv(
    State(state): State<AppState>,
    privileged: PrivilegedUser,
    client: ClientInfo,
    Query(q): Query<ListQuery>,
) -> AppResult<Response> {
    privileged.require(&state.policy, PERM_EXPORT)?;

    let f = Filters::from(&q)?;
    let sql = format!(
        "{SELECT_COLS} FROM admin_actions {WHERE_SQL} \
         ORDER BY created_at DESC, id DESC LIMIT $9",
    );
    let rows: Vec<AuditRow> = f
        .bind_into(sqlx::query_as::<_, AuditRow>(&sql))
        .bind(MAX_EXPORT_ROWS)
        .fetch_all(&state.db)
        .await?;

    let mut buf = String::with_capacity(4096 + rows.len() * 256);
    buf.push_str(
        "id,created_at,actor_id,actor_role,action,target_kind,target_id,ip_address,metadata\n",
    );
    for r in &rows {
        let _ = writeln!(
            buf,
            "{id},{created},{actor},{role},{action},{kind},{tid},{ip},{meta}",
            id = r.id,
            created = r.created_at.to_rfc3339(),
            actor = r.actor_id,
            role = r.actor_role,
            action = csv_escape(&r.action),
            kind = csv_escape(&r.target_kind),
            tid = r.target_id.as_deref().map(csv_escape).unwrap_or_default(),
            ip = r.ip_address.as_deref().unwrap_or_default(),
            meta = csv_escape(&r.metadata.to_string()),
        );
    }

    audit_admin_priv_no_target(
        &state.db,
        &privileged,
        &client,
        "admin.audit.export",
        "audit",
        serde_json::json!({
            "rows": rows.len(),
            "filter": {
                "q":             q.q,
                "actor_id":      q.actor_id,
                "action":        q.action,
                "target_kind":   q.target_kind,
                "target_id":     q.target_id,
                "metadata_contains": q.metadata_contains,
                "from":          q.from,
                "to":            q.to,
            },
        }),
    )
    .await;

    let body = buf.into_bytes();
    Ok((
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, "text/csv; charset=utf-8".to_string()),
            (
                header::CONTENT_DISPOSITION,
                "attachment; filename=\"audit.csv\"".to_string(),
            ),
        ],
        body,
    )
        .into_response())
}

// ── Filters helper ─────────────────────────────────────────────────────

/// Validated, normalised filter pack. Owns ordered `$1..$8` bindings
/// so list + count + export share one definition and never drift.
struct Filters {
    q: Option<String>,
    actor_id: Option<Uuid>,
    action: Option<String>,
    target_kind: Option<String>,
    target_id: Option<String>,
    metadata: Option<Value>,
    from: Option<DateTime<Utc>>,
    to: Option<DateTime<Utc>>,
}

impl Filters {
    fn from(q: &ListQuery) -> AppResult<Self> {
        let metadata = match q
            .metadata_contains
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
        {
            None => None,
            Some(s) => {
                let v: Value = serde_json::from_str(s).map_err(|e| {
                    AppError::BadRequest(format!("metadata_contains is not valid JSON: {e}"))
                })?;
                if !v.is_object() {
                    return Err(AppError::BadRequest(
                        "metadata_contains must be a JSON object".into(),
                    ));
                }
                Some(v)
            }
        };
        if let (Some(from), Some(to)) = (q.from, q.to) {
            if from > to {
                return Err(AppError::BadRequest(
                    "`from` must be <= `to`".to_string(),
                ));
            }
        }
        let trim = |s: &Option<String>| {
            s.as_deref()
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(str::to_string)
        };
        Ok(Self {
            q: trim(&q.q),
            actor_id: q.actor_id,
            action: trim(&q.action),
            target_kind: trim(&q.target_kind),
            target_id: trim(&q.target_id),
            metadata,
            from: q.from,
            to: q.to,
        })
    }

    fn bind_into<'a, O>(
        &'a self,
        q: sqlx::query::QueryAs<'a, sqlx::Postgres, O, sqlx::postgres::PgArguments>,
    ) -> sqlx::query::QueryAs<'a, sqlx::Postgres, O, sqlx::postgres::PgArguments>
    where
        O: for<'r> sqlx::FromRow<'r, sqlx::postgres::PgRow> + Send + Unpin,
    {
        q.bind(self.q.as_deref())
            .bind(self.actor_id)
            .bind(self.action.as_deref())
            .bind(self.target_kind.as_deref())
            .bind(self.target_id.as_deref())
            .bind(self.metadata.as_ref())
            .bind(self.from)
            .bind(self.to)
    }
}

fn csv_escape(s: &str) -> String {
    if s.contains(',') || s.contains('"') || s.contains('\n') || s.contains('\r') {
        let escaped = s.replace('"', "\"\"");
        format!("\"{escaped}\"")
    } else {
        s.to_string()
    }
}
