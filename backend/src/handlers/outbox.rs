//! Admin ops HTTP surface for the outbox (FDN-04).
//!
//! Three routes, all admin-gated:
//!
//! * `GET  /api/admin/outbox?status=<state>` — paginated list, optional status filter.
//! * `GET  /api/admin/outbox/{id}` — single row detail.
//! * `POST /api/admin/outbox/{id}/retry` — force-schedule a failed / dead-letter
//!   row for immediate re-dispatch.
//!
//! The UI layer around these endpoints ships in a later subsystem; this file
//! only exposes the JSON API.

use axum::{
    extract::{Path, Query, State},
    routing::{get, post},
    Json, Router,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::Row;
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

use crate::{
    error::{AppError, AppResult},
    events::outbox::{OutboxRecord, OutboxStatus},
    extractors::AdminUser,
    models::PaginatedResponse,
    AppState,
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(list_outbox))
        .route("/{id}", get(get_outbox))
        .route("/{id}/retry", post(retry_outbox))
}

// ── Query / response DTOs ───────────────────────────────────────────────

/// Query parameters for [`list_outbox`]. Accepts an optional status filter and
/// page/per-page pagination.
#[derive(Debug, Deserialize, IntoParams, ToSchema)]
pub struct OutboxListQuery {
    /// Filter by lifecycle state. Omit to return every row.
    pub status: Option<String>,
    pub page: Option<i64>,
    pub per_page: Option<i64>,
}

impl OutboxListQuery {
    fn per_page(&self) -> i64 {
        self.per_page.unwrap_or(25).clamp(1, 200)
    }

    fn page(&self) -> i64 {
        self.page.unwrap_or(1).max(1)
    }

    fn offset(&self) -> i64 {
        (self.page() - 1) * self.per_page()
    }
}

/// Paginated outbox row. Kept in sync with [`OutboxRecord`] but serialises the
/// status as a lowercase string for frontend friendliness.
#[derive(Debug, Serialize, ToSchema)]
pub struct OutboxRowDto {
    pub id: Uuid,
    pub aggregate_type: String,
    pub aggregate_id: String,
    pub event_type: String,
    pub payload: serde_json::Value,
    pub headers: serde_json::Value,
    pub status: OutboxStatus,
    pub attempts: i32,
    pub max_attempts: i32,
    pub next_attempt_at: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_error: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<OutboxRecord> for OutboxRowDto {
    fn from(r: OutboxRecord) -> Self {
        OutboxRowDto {
            id: r.id,
            aggregate_type: r.aggregate_type,
            aggregate_id: r.aggregate_id,
            event_type: r.event_type,
            payload: r.payload,
            headers: r.headers,
            status: r.status,
            attempts: r.attempts,
            max_attempts: r.max_attempts,
            next_attempt_at: r.next_attempt_at,
            last_error: r.last_error,
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}

/// Response body for `POST /api/admin/outbox/{id}/retry`.
#[derive(Debug, Serialize, ToSchema)]
pub struct OutboxRetryResponse {
    pub id: Uuid,
    pub status: OutboxStatus,
    pub next_attempt_at: DateTime<Utc>,
}

// ── Handlers ────────────────────────────────────────────────────────────

#[utoipa::path(
    get,
    path = "/api/admin/outbox",
    tag = "admin",
    security(("bearer_auth" = [])),
    params(OutboxListQuery),
    responses(
        (status = 200, description = "Paginated outbox rows", body = PaginatedOutboxResponse),
        (status = 400, description = "Invalid status filter"),
        (status = 403, description = "Forbidden")
    )
)]
pub async fn list_outbox(
    State(state): State<AppState>,
    _admin: AdminUser,
    Query(q): Query<OutboxListQuery>,
) -> AppResult<Json<PaginatedResponse<OutboxRowDto>>> {
    let per_page = q.per_page();
    let offset = q.offset();
    let page = q.page();

    if let Some(raw) = &q.status {
        if OutboxStatus::from_db(raw).is_none() {
            return Err(AppError::BadRequest(format!(
                "invalid status filter `{raw}`; allowed: pending, in_flight, delivered, failed, dead_letter"
            )));
        }
    }

    let (rows, total) = match &q.status {
        Some(status) => {
            let rows = sqlx::query_as::<_, OutboxRecord>(
                r#"
                SELECT id, aggregate_type, aggregate_id, event_type, payload, headers,
                       status, attempts, max_attempts, next_attempt_at, last_error,
                       created_at, updated_at
                FROM outbox_events
                WHERE status = $1
                ORDER BY updated_at DESC, id DESC
                LIMIT $2 OFFSET $3
                "#,
            )
            .bind(status)
            .bind(per_page)
            .bind(offset)
            .fetch_all(&state.db)
            .await?;

            let total_row =
                sqlx::query("SELECT COUNT(*) AS total FROM outbox_events WHERE status = $1")
                    .bind(status)
                    .fetch_one(&state.db)
                    .await?;
            let total: i64 = total_row.try_get("total")?;
            (rows, total)
        }
        None => {
            let rows = sqlx::query_as::<_, OutboxRecord>(
                r#"
                SELECT id, aggregate_type, aggregate_id, event_type, payload, headers,
                       status, attempts, max_attempts, next_attempt_at, last_error,
                       created_at, updated_at
                FROM outbox_events
                ORDER BY updated_at DESC, id DESC
                LIMIT $1 OFFSET $2
                "#,
            )
            .bind(per_page)
            .bind(offset)
            .fetch_all(&state.db)
            .await?;

            let total_row = sqlx::query("SELECT COUNT(*) AS total FROM outbox_events")
                .fetch_one(&state.db)
                .await?;
            let total: i64 = total_row.try_get("total")?;
            (rows, total)
        }
    };

    let total_pages = if per_page > 0 {
        (total as f64 / per_page as f64).ceil() as i64
    } else {
        0
    };

    Ok(Json(PaginatedResponse {
        data: rows.into_iter().map(OutboxRowDto::from).collect(),
        total,
        page,
        per_page,
        total_pages,
    }))
}

#[utoipa::path(
    get,
    path = "/api/admin/outbox/{id}",
    tag = "admin",
    security(("bearer_auth" = [])),
    params(("id" = Uuid, Path, description = "Outbox event id")),
    responses(
        (status = 200, description = "Outbox row", body = OutboxRowDto),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Not found")
    )
)]
pub async fn get_outbox(
    State(state): State<AppState>,
    _admin: AdminUser,
    Path(id): Path<Uuid>,
) -> AppResult<Json<OutboxRowDto>> {
    let row = sqlx::query_as::<_, OutboxRecord>(
        r#"
        SELECT id, aggregate_type, aggregate_id, event_type, payload, headers,
               status, attempts, max_attempts, next_attempt_at, last_error,
               created_at, updated_at
        FROM outbox_events
        WHERE id = $1
        "#,
    )
    .bind(id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| AppError::NotFound(format!("outbox event {id}")))?;

    Ok(Json(OutboxRowDto::from(row)))
}

#[utoipa::path(
    post,
    path = "/api/admin/outbox/{id}/retry",
    tag = "admin",
    security(("bearer_auth" = [])),
    params(("id" = Uuid, Path, description = "Outbox event id to retry")),
    responses(
        (status = 200, description = "Event re-queued", body = OutboxRetryResponse),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Not found"),
        (status = 409, description = "Event already delivered")
    )
)]
pub async fn retry_outbox(
    State(state): State<AppState>,
    _admin: AdminUser,
    Path(id): Path<Uuid>,
) -> AppResult<Json<OutboxRetryResponse>> {
    // Delivered rows are final — surfacing a conflict here avoids silently
    // re-firing an event that may have already been consumed.
    let row = sqlx::query_as::<_, OutboxRecord>(
        r#"
        UPDATE outbox_events
        SET status = 'pending',
            next_attempt_at = NOW(),
            last_error = NULL,
            updated_at = NOW()
        WHERE id = $1
          AND status <> 'delivered'
        RETURNING id, aggregate_type, aggregate_id, event_type, payload, headers,
                  status, attempts, max_attempts, next_attempt_at, last_error,
                  created_at, updated_at
        "#,
    )
    .bind(id)
    .fetch_optional(&state.db)
    .await?;

    let row = match row {
        Some(r) => r,
        None => {
            let exists: Option<(String,)> =
                sqlx::query_as("SELECT status FROM outbox_events WHERE id = $1")
                    .bind(id)
                    .fetch_optional(&state.db)
                    .await?;
            return match exists {
                Some((s,)) if s == "delivered" => Err(AppError::Conflict(format!(
                    "outbox event {id} already delivered"
                ))),
                Some(_) => Err(AppError::NotFound(format!("outbox event {id}"))),
                None => Err(AppError::NotFound(format!("outbox event {id}"))),
            };
        }
    };

    Ok(Json(OutboxRetryResponse {
        id: row.id,
        status: row.status,
        next_attempt_at: row.next_attempt_at,
    }))
}

/// OpenAPI wrapper so the snapshot carries a concrete `PaginatedResponse<OutboxRowDto>`
/// schema without bleeding the generic into `ApiDoc`.
#[derive(Debug, Serialize, ToSchema)]
pub struct PaginatedOutboxResponse {
    pub data: Vec<OutboxRowDto>,
    pub total: i64,
    pub page: i64,
    pub per_page: i64,
    pub total_pages: i64,
}
