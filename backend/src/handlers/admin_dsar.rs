//! ADM-13: admin-initiated DSAR jobs (export + right-to-erasure).
//!
//! Mounted at `/api/admin/dsar`. The pre-existing
//! `/api/admin/consent/dsar` surface fulfils *subject-initiated*
//! requests with an email-verification gate; this surface is the
//! *operator-initiated* counterpart for legal-hold exports and
//! right-to-erasure tombstones.
//!
//! Endpoints:
//!
//!   * `GET    /jobs                                ` — paginated list.
//!   * `GET    /jobs/{id}                           ` — single job.
//!   * `POST   /jobs/export                         ` — single-control;
//!     synchronously composes the export, writes it as a `data:` URI
//!     to `dsar_jobs.artifact_url`, returns the artefact and the row.
//!   * `POST   /jobs/erase/request                  ` — first half of
//!     dual control. Refuses to queue a second pending erasure for
//!     the same subject (DB unique partial index enforces it too).
//!   * `POST   /jobs/{id}/erase/approve             ` — second admin
//!     approves; refuses self-approval, refuses non-pending jobs,
//!     runs the tombstone in a TX, marks the job `completed`.
//!   * `POST   /jobs/{id}/cancel                    ` — cancel a
//!     pending erasure (e.g. requester realised they targeted the
//!     wrong account).
//!
//! Every state change writes to `admin_actions` so the regulator-
//! facing audit log shows a complete dual-control chain:
//!
//!   ```text
//!   admin.dsar.erase.requested       (actor = requester)
//!   admin.dsar.erase.approved        (actor = approver)  ← different user
//!   admin.dsar.erase.completed       (actor = approver)  ← tombstone done
//!   ```

use axum::{
    body::Body,
    extract::{Path, Query, State},
    http::{header, HeaderMap, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::Row;
use utoipa::ToSchema;
use uuid::Uuid;
use validator::Validate;

use crate::{
    error::{AppError, AppResult},
    extractors::{ClientInfo, PrivilegedUser},
    services::{
        audit::audit_admin_priv,
        dsar_admin::{self, DsarAdminError, TombstoneSummary},
    },
    AppState,
};

const PERM_READ: &str = "admin.dsar.read";
const PERM_EXPORT: &str = "admin.dsar.export";
const PERM_ERASE_REQUEST: &str = "admin.dsar.erase.request";
const PERM_ERASE_APPROVE: &str = "admin.dsar.erase.approve";

const DEFAULT_LIMIT: i64 = 25;
const MAX_LIMIT: i64 = 200;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/jobs", get(list_jobs))
        .route("/jobs/export", post(create_export))
        .route("/jobs/erase/request", post(request_erase))
        .route("/jobs/{id}", get(read_job))
        .route("/jobs/{id}/artifact", get(stream_artifact))
        .route("/jobs/{id}/erase/approve", post(approve_erase))
        .route("/jobs/{id}/cancel", post(cancel_job))
}

// ── DTOs ───────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, sqlx::FromRow, ToSchema)]
pub struct DsarJob {
    pub id: Uuid,
    pub target_user_id: Uuid,
    pub kind: String,
    pub status: String,
    pub requested_by: Uuid,
    pub request_reason: String,
    pub approved_by: Option<Uuid>,
    pub approval_reason: Option<String>,
    pub approved_at: Option<DateTime<Utc>>,
    pub artifact_url: Option<String>,
    /// ADM-17: storage transport for the artefact. `inline`
    /// (legacy synchronous path → `data:` URI on `artifact_url`),
    /// `r2` (presigned URL on `artifact_url`), or `local`
    /// (download via `/jobs/{id}/artifact` streamer with bearer
    /// auth).
    pub artifact_kind: Option<String>,
    /// ADM-17: opaque storage key (R2 object key, or `dsar/{id}.json`
    /// for local mode). Operator surface only — downloads use
    /// `artifact_url` or the streamer.
    pub artifact_storage_key: Option<String>,
    /// ADM-17 + ADM-19: TTL after which the TTL sweep deletes the
    /// underlying object and NULLs all three artefact columns.
    pub artifact_expires_at: Option<DateTime<Utc>>,
    pub erasure_summary: Option<Value>,
    pub completed_at: Option<DateTime<Utc>>,
    pub failure_reason: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct ListQuery {
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub kind: Option<String>,
    #[serde(default)]
    pub target_user_id: Option<Uuid>,
    #[serde(default)]
    pub limit: Option<i64>,
    #[serde(default)]
    pub offset: Option<i64>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct JobListEnvelope {
    pub data: Vec<DsarJob>,
    pub total: i64,
    pub page: i64,
    pub per_page: i64,
    pub total_pages: i64,
}

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct ExportRequest {
    pub target_user_id: Uuid,
    #[validate(length(min = 1, max = 1000, message = "reason required"))]
    pub reason: String,
    /// ADM-17: opt into the async pipeline. When `true` the handler
    /// queues a `pending` job for the background worker and returns
    /// `202 Accepted` with the row but no inline export. When `false`
    /// (default) the legacy synchronous compose runs inline and the
    /// response includes the full document — preserved for parity with
    /// pre-ADM-17 callers and for ergonomic small-export UX.
    #[serde(default)]
    pub r#async: bool,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ExportResponse {
    pub job: DsarJob,
    /// Inline JSON snapshot of the export. Operators usually consume
    /// `job.artifact_url` (a `data:` URI for inline mode, presigned R2
    /// URL or `/artifact` streamer route for async mode) for the
    /// round-trippable document; `export` is the same payload
    /// deserialised so admin UIs can render it without a second hop.
    /// `null` when the request opted into the async pipeline because
    /// the worker has not composed the artefact yet.
    pub export: Option<Value>,
}

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct EraseRequestBody {
    pub target_user_id: Uuid,
    #[validate(length(min = 10, max = 1000, message = "reason must be 10..=1000 chars"))]
    pub reason: String,
}

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct EraseApproveBody {
    #[validate(length(min = 10, max = 1000, message = "approval reason must be 10..=1000 chars"))]
    pub approval_reason: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct EraseApproveResponse {
    pub job: DsarJob,
    pub summary: TombstoneSummary,
}

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct CancelBody {
    #[validate(length(max = 500))]
    pub reason: Option<String>,
}

// ── Handlers ───────────────────────────────────────────────────────────

#[utoipa::path(
    get,
    path = "/api/admin/dsar/jobs",
    tag = "admin-dsar",
    operation_id = "admin_dsar_list_jobs",
    security(("bearer_auth" = [])),
    params(
        ("status"          = Option<String>, Query, description = "Status filter"),
        ("kind"            = Option<String>, Query, description = "Kind filter (export|erase)"),
        ("target_user_id"  = Option<Uuid>,   Query, description = "Filter by target user"),
        ("limit"           = Option<i64>,    Query, description = "Page size (1..=200)"),
        ("offset"          = Option<i64>,    Query, description = "Cursor offset"),
    ),
    responses(
        (status = 200, description = "Paginated DSAR jobs", body = JobListEnvelope),
        (status = 401, description = "Unauthenticated"),
        (status = 403, description = "Forbidden")
    )
)]
pub async fn list_jobs(
    State(state): State<AppState>,
    privileged: PrivilegedUser,
    Query(q): Query<ListQuery>,
) -> AppResult<Json<JobListEnvelope>> {
    privileged.require(&state.policy, PERM_READ)?;

    let limit = q.limit.unwrap_or(DEFAULT_LIMIT).clamp(1, MAX_LIMIT);
    let offset = q.offset.unwrap_or(0).max(0);

    if let Some(s) = q.status.as_deref() {
        if !VALID_STATUSES.contains(&s) {
            return Err(AppError::BadRequest(format!("unknown status: {s}")));
        }
    }
    if let Some(k) = q.kind.as_deref() {
        if !["export", "erase"].contains(&k) {
            return Err(AppError::BadRequest(format!("unknown kind: {k}")));
        }
    }

    let where_sql = r#"
        WHERE ($1::text IS NULL OR status = $1)
          AND ($2::text IS NULL OR kind   = $2)
          AND ($3::uuid IS NULL OR target_user_id = $3)
    "#;
    let list_sql = format!(
        r#"
        SELECT id, target_user_id, kind, status, requested_by, request_reason,
               approved_by, approval_reason, approved_at, artifact_url,
               artifact_kind, artifact_storage_key, artifact_expires_at,
               erasure_summary, completed_at, failure_reason, created_at, updated_at
          FROM dsar_jobs {where_sql}
         ORDER BY created_at DESC
         LIMIT $4 OFFSET $5
        "#,
    );
    let count_sql = format!("SELECT COUNT(*) FROM dsar_jobs {where_sql}");

    let data: Vec<DsarJob> = sqlx::query_as(&list_sql)
        .bind(q.status.as_deref())
        .bind(q.kind.as_deref())
        .bind(q.target_user_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(&state.db)
        .await?;
    let total: (i64,) = sqlx::query_as(&count_sql)
        .bind(q.status.as_deref())
        .bind(q.kind.as_deref())
        .bind(q.target_user_id)
        .fetch_one(&state.db)
        .await?;

    let per_page = limit;
    let page = (offset / per_page.max(1)) + 1;
    let total_pages = (total.0 as f64 / per_page as f64).ceil() as i64;

    Ok(Json(JobListEnvelope {
        data,
        total: total.0,
        page,
        per_page,
        total_pages,
    }))
}

#[utoipa::path(
    get,
    path = "/api/admin/dsar/jobs/{id}",
    tag = "admin-dsar",
    operation_id = "admin_dsar_read_job",
    security(("bearer_auth" = [])),
    params(("id" = Uuid, Path, description = "DSAR job id")),
    responses(
        (status = 200, description = "DSAR job", body = DsarJob),
        (status = 404, description = "Job not found")
    )
)]
pub async fn read_job(
    State(state): State<AppState>,
    privileged: PrivilegedUser,
    Path(id): Path<Uuid>,
) -> AppResult<Json<DsarJob>> {
    privileged.require(&state.policy, PERM_READ)?;

    let job = fetch_job(&state.db, id)
        .await?
        .ok_or_else(|| AppError::NotFound("DSAR job not found".into()))?;
    Ok(Json(job))
}

#[utoipa::path(
    post,
    path = "/api/admin/dsar/jobs/export",
    tag = "admin-dsar",
    operation_id = "admin_dsar_create_export",
    security(("bearer_auth" = [])),
    request_body = ExportRequest,
    responses(
        (status = 201, description = "Export composed", body = ExportResponse),
        (status = 400, description = "Validation failed"),
        (status = 401, description = "Unauthenticated"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Target user not found")
    )
)]
pub async fn create_export(
    State(state): State<AppState>,
    privileged: PrivilegedUser,
    client: ClientInfo,
    Json(req): Json<ExportRequest>,
) -> AppResult<(StatusCode, Json<ExportResponse>)> {
    privileged.require(&state.policy, PERM_EXPORT)?;
    req.validate()
        .map_err(|e| AppError::BadRequest(e.to_string()))?;

    if req.r#async {
        // ADM-17 async path: queue a pending row and let the worker
        // compose + upload off-request. Returns `202 Accepted` so the
        // UI knows the artefact is not yet available.
        let job: DsarJob = sqlx::query_as(
            r#"
            INSERT INTO dsar_jobs
                (target_user_id, kind, status, requested_by, request_reason,
                 approved_by, approved_at)
            VALUES ($1, 'export', 'pending', $2, $3, $2, NOW())
            RETURNING id, target_user_id, kind, status, requested_by, request_reason,
                      approved_by, approval_reason, approved_at, artifact_url,
                      artifact_kind, artifact_storage_key, artifact_expires_at,
                      erasure_summary, completed_at, failure_reason, created_at, updated_at
            "#,
        )
        .bind(req.target_user_id)
        .bind(privileged.user_id)
        .bind(&req.reason)
        .fetch_one(&state.db)
        .await?;

        audit_admin_priv(
            &state.db,
            &privileged,
            &client,
            "admin.dsar.export.queued",
            "user",
            req.target_user_id.to_string(),
            serde_json::json!({ "job_id": job.id, "reason": req.reason }),
        )
        .await;

        return Ok((
            StatusCode::ACCEPTED,
            Json(ExportResponse { job, export: None }),
        ));
    }

    let export = dsar_admin::build_admin_export(&state.db, req.target_user_id)
        .await
        .map_err(map_dsar_err)?
        .ok_or_else(|| AppError::NotFound(format!("user {} not found", req.target_user_id)))?;
    let export_value = serde_json::to_value(&export)
        .map_err(|e| AppError::Internal(anyhow::anyhow!("serialize export: {e}")))?;
    let artifact_uri = crate::consent::dsar_export::export_to_data_uri(&export);

    let job: DsarJob = sqlx::query_as(
        r#"
        INSERT INTO dsar_jobs
            (target_user_id, kind, status, requested_by, request_reason,
             artifact_url, artifact_kind, completed_at, approved_by, approved_at)
        VALUES ($1, 'export', 'completed', $2, $3, $4, 'inline', NOW(), $2, NOW())
        RETURNING id, target_user_id, kind, status, requested_by, request_reason,
                  approved_by, approval_reason, approved_at, artifact_url,
                  artifact_kind, artifact_storage_key, artifact_expires_at,
                  erasure_summary, completed_at, failure_reason, created_at, updated_at
        "#,
    )
    .bind(req.target_user_id)
    .bind(privileged.user_id)
    .bind(&req.reason)
    .bind(&artifact_uri)
    .fetch_one(&state.db)
    .await?;

    audit_admin_priv(
        &state.db,
        &privileged,
        &client,
        "admin.dsar.export.completed",
        "user",
        req.target_user_id.to_string(),
        serde_json::json!({
            "job_id":   job.id,
            "reason":   req.reason,
            "byte_len": artifact_uri.len(),
        }),
    )
    .await;

    Ok((
        StatusCode::CREATED,
        Json(ExportResponse {
            job,
            export: Some(export_value),
        }),
    ))
}

#[utoipa::path(
    post,
    path = "/api/admin/dsar/jobs/erase/request",
    tag = "admin-dsar",
    operation_id = "admin_dsar_request_erase",
    security(("bearer_auth" = [])),
    request_body = EraseRequestBody,
    responses(
        (status = 201, description = "Erasure pending approval", body = DsarJob),
        (status = 400, description = "Validation failed"),
        (status = 401, description = "Unauthenticated"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Target user not found"),
        (status = 409, description = "Pending erasure already exists, or user already erased")
    )
)]
pub async fn request_erase(
    State(state): State<AppState>,
    privileged: PrivilegedUser,
    client: ClientInfo,
    Json(req): Json<EraseRequestBody>,
) -> AppResult<(StatusCode, Json<DsarJob>)> {
    privileged.require(&state.policy, PERM_ERASE_REQUEST)?;
    req.validate()
        .map_err(|e| AppError::BadRequest(e.to_string()))?;

    let user_state: Option<(Option<DateTime<Utc>>,)> = sqlx::query_as(
        "SELECT erased_at FROM users WHERE id = $1",
    )
    .bind(req.target_user_id)
    .fetch_optional(&state.db)
    .await?;
    match user_state {
        None => {
            return Err(AppError::NotFound(format!(
                "user {} not found",
                req.target_user_id
            )))
        }
        Some((Some(_),)) => {
            return Err(AppError::Conflict(format!(
                "user {} is already tombstoned",
                req.target_user_id
            )))
        }
        Some((None,)) => {}
    }

    let job: DsarJob = match sqlx::query_as(
        r#"
        INSERT INTO dsar_jobs (target_user_id, kind, status, requested_by, request_reason)
        VALUES ($1, 'erase', 'pending', $2, $3)
        RETURNING id, target_user_id, kind, status, requested_by, request_reason,
                  approved_by, approval_reason, approved_at, artifact_url,
                  artifact_kind, artifact_storage_key, artifact_expires_at,
                  erasure_summary, completed_at, failure_reason, created_at, updated_at
        "#,
    )
    .bind(req.target_user_id)
    .bind(privileged.user_id)
    .bind(&req.reason)
    .fetch_one(&state.db)
    .await
    {
        Ok(j) => j,
        Err(sqlx::Error::Database(db_err))
            if db_err.code().as_deref() == Some("23505")
                && db_err.constraint() == Some("dsar_jobs_one_pending_erase") =>
        {
            return Err(AppError::Conflict(
                "a pending erasure already exists for this user".into(),
            ))
        }
        Err(e) => return Err(AppError::Database(e)),
    };

    audit_admin_priv(
        &state.db,
        &privileged,
        &client,
        "admin.dsar.erase.requested",
        "user",
        req.target_user_id.to_string(),
        serde_json::json!({
            "job_id": job.id,
            "reason": req.reason,
        }),
    )
    .await;

    Ok((StatusCode::CREATED, Json(job)))
}

#[utoipa::path(
    post,
    path = "/api/admin/dsar/jobs/{id}/erase/approve",
    tag = "admin-dsar",
    operation_id = "admin_dsar_approve_erase",
    security(("bearer_auth" = [])),
    params(("id" = Uuid, Path, description = "DSAR job id")),
    request_body = EraseApproveBody,
    responses(
        (status = 200, description = "Erasure executed", body = EraseApproveResponse),
        (status = 400, description = "Validation failed"),
        (status = 401, description = "Unauthenticated"),
        (status = 403, description = "Forbidden — self-approval is not permitted"),
        (status = 404, description = "Job not found"),
        (status = 409, description = "Job is not pending")
    )
)]
pub async fn approve_erase(
    State(state): State<AppState>,
    privileged: PrivilegedUser,
    client: ClientInfo,
    Path(id): Path<Uuid>,
    Json(req): Json<EraseApproveBody>,
) -> AppResult<Json<EraseApproveResponse>> {
    privileged.require(&state.policy, PERM_ERASE_APPROVE)?;
    req.validate()
        .map_err(|e| AppError::BadRequest(e.to_string()))?;

    let job = fetch_job(&state.db, id)
        .await?
        .ok_or_else(|| AppError::NotFound("DSAR job not found".into()))?;

    if job.kind != "erase" {
        return Err(AppError::BadRequest(format!(
            "job kind {} is not erasable",
            job.kind
        )));
    }
    if job.status != "pending" {
        return Err(AppError::Conflict(format!(
            "job is in state {} (must be pending)",
            job.status
        )));
    }
    if job.requested_by == privileged.user_id {
        return Err(AppError::Forbidden);
    }

    // Mark approved first so audit-log readers see the dual-control
    // step even if the tombstone fails. We commit the approval, then
    // execute the tombstone, then mark `completed` — if the tombstone
    // fails the row stays in `approved` and the failure reason is
    // recorded for triage.
    let approved_job: DsarJob = sqlx::query_as(
        r#"
        UPDATE dsar_jobs
           SET status          = 'approved',
               approved_by     = $1,
               approval_reason = $2,
               approved_at     = NOW(),
               updated_at      = NOW()
         WHERE id = $3
         RETURNING id, target_user_id, kind, status, requested_by, request_reason,
                   approved_by, approval_reason, approved_at, artifact_url,
                   artifact_kind, artifact_storage_key, artifact_expires_at,
                   erasure_summary, completed_at, failure_reason, created_at, updated_at
        "#,
    )
    .bind(privileged.user_id)
    .bind(&req.approval_reason)
    .bind(id)
    .fetch_one(&state.db)
    .await?;

    audit_admin_priv(
        &state.db,
        &privileged,
        &client,
        "admin.dsar.erase.approved",
        "user",
        approved_job.target_user_id.to_string(),
        serde_json::json!({
            "job_id":          approved_job.id,
            "approval_reason": req.approval_reason,
            "requested_by":    approved_job.requested_by,
        }),
    )
    .await;

    let summary =
        match dsar_admin::tombstone_user(&state.db, approved_job.target_user_id, approved_job.id)
            .await
        {
            Ok(s) => s,
            Err(e) => {
                let msg = e.to_string();
                let _ = sqlx::query(
                    "UPDATE dsar_jobs
                        SET status = 'failed', failure_reason = $1, updated_at = NOW()
                      WHERE id = $2",
                )
                .bind(&msg)
                .bind(id)
                .execute(&state.db)
                .await;
                return Err(map_dsar_err(e));
            }
        };

    let summary_value = serde_json::to_value(&summary)
        .map_err(|e| AppError::Internal(anyhow::anyhow!("serialize summary: {e}")))?;
    let job: DsarJob = sqlx::query_as(
        r#"
        UPDATE dsar_jobs
           SET status         = 'completed',
               erasure_summary = $1,
               completed_at   = NOW(),
               updated_at     = NOW()
         WHERE id = $2
         RETURNING id, target_user_id, kind, status, requested_by, request_reason,
                   approved_by, approval_reason, approved_at, artifact_url,
                   artifact_kind, artifact_storage_key, artifact_expires_at,
                   erasure_summary, completed_at, failure_reason, created_at, updated_at
        "#,
    )
    .bind(&summary_value)
    .bind(id)
    .fetch_one(&state.db)
    .await?;

    audit_admin_priv(
        &state.db,
        &privileged,
        &client,
        "admin.dsar.erase.completed",
        "user",
        job.target_user_id.to_string(),
        serde_json::json!({
            "job_id":  job.id,
            "summary": summary_value,
        }),
    )
    .await;

    Ok(Json(EraseApproveResponse { job, summary }))
}

#[utoipa::path(
    post,
    path = "/api/admin/dsar/jobs/{id}/cancel",
    tag = "admin-dsar",
    operation_id = "admin_dsar_cancel_job",
    security(("bearer_auth" = [])),
    params(("id" = Uuid, Path, description = "DSAR job id")),
    request_body = CancelBody,
    responses(
        (status = 200, description = "Job cancelled", body = DsarJob),
        (status = 401, description = "Unauthenticated"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Job not found"),
        (status = 409, description = "Job is not in a cancellable state")
    )
)]
pub async fn cancel_job(
    State(state): State<AppState>,
    privileged: PrivilegedUser,
    client: ClientInfo,
    Path(id): Path<Uuid>,
    Json(req): Json<CancelBody>,
) -> AppResult<Json<DsarJob>> {
    // Either gate works — the original requester can cancel (request
    // perm) and the approver can also abort (approve perm).
    let can_cancel = state.policy.has(privileged.role, PERM_ERASE_REQUEST)
        || state.policy.has(privileged.role, PERM_ERASE_APPROVE);
    if !can_cancel {
        return Err(AppError::Forbidden);
    }
    req.validate()
        .map_err(|e| AppError::BadRequest(e.to_string()))?;

    let job = fetch_job(&state.db, id)
        .await?
        .ok_or_else(|| AppError::NotFound("DSAR job not found".into()))?;

    if !matches!(job.status.as_str(), "pending" | "approved") {
        return Err(AppError::Conflict(format!(
            "job is in state {} (cannot cancel)",
            job.status
        )));
    }

    let updated: DsarJob = sqlx::query_as(
        r#"
        UPDATE dsar_jobs
           SET status         = 'cancelled',
               failure_reason = $1,
               updated_at     = NOW()
         WHERE id = $2
         RETURNING id, target_user_id, kind, status, requested_by, request_reason,
                   approved_by, approval_reason, approved_at, artifact_url,
                   artifact_kind, artifact_storage_key, artifact_expires_at,
                   erasure_summary, completed_at, failure_reason, created_at, updated_at
        "#,
    )
    .bind(req.reason.as_deref())
    .bind(id)
    .fetch_one(&state.db)
    .await?;

    audit_admin_priv(
        &state.db,
        &privileged,
        &client,
        "admin.dsar.cancelled",
        "user",
        updated.target_user_id.to_string(),
        serde_json::json!({
            "job_id":      updated.id,
            "reason":      req.reason,
            "prior_state": job.status,
        }),
    )
    .await;

    Ok(Json(updated))
}

// ── Helpers ────────────────────────────────────────────────────────────

const VALID_STATUSES: &[&str] = &[
    "pending",
    "approved",
    "composing",
    "completed",
    "rejected",
    "cancelled",
    "failed",
];

/// Stream the artefact behind a `local`-mode async export.
///
/// Async DSAR exports stored on R2 expose a presigned URL the operator
/// hits directly; for local-storage deployments (dev, single-node, or
/// air-gapped) we serve the JSON via this RBAC-gated route instead so
/// no anonymous filesystem access is required. Returns `404` for
/// inline jobs (the artefact is already in `artifact_url`) and for
/// jobs that have not yet been composed.
#[utoipa::path(
    get,
    path = "/api/admin/dsar/jobs/{id}/artifact",
    tag = "admin-dsar",
    operation_id = "admin_dsar_stream_artifact",
    security(("bearer_auth" = [])),
    params(("id" = Uuid, Path, description = "DSAR job id")),
    responses(
        (status = 200, description = "Streamed JSON artefact",
            content_type = "application/json"),
        (status = 400, description = "Artefact is not local-streamable"),
        (status = 404, description = "Artefact missing or expired"),
        (status = 409, description = "Job not yet completed")
    )
)]
pub async fn stream_artifact(
    State(state): State<AppState>,
    privileged: PrivilegedUser,
    client: ClientInfo,
    Path(id): Path<Uuid>,
) -> AppResult<Response> {
    privileged.require(&state.policy, PERM_READ)?;

    let row = sqlx::query(
        r#"
        SELECT target_user_id, status, artifact_kind, artifact_storage_key,
               artifact_expires_at
          FROM dsar_jobs
         WHERE id = $1
        "#,
    )
    .bind(id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| AppError::NotFound(format!("dsar job {id} not found")))?;

    let target_user_id: Uuid = row.try_get("target_user_id")?;
    let status: String = row.try_get("status")?;
    let kind: Option<String> = row.try_get("artifact_kind")?;
    let key: Option<String> = row.try_get("artifact_storage_key")?;
    let expires_at: Option<DateTime<Utc>> = row.try_get("artifact_expires_at")?;

    if status != "completed" {
        return Err(AppError::Conflict(format!(
            "job is in status `{status}`, not ready"
        )));
    }
    let kind = kind.ok_or_else(|| AppError::NotFound(format!("job {id} has no artefact")))?;
    if kind != "local" {
        return Err(AppError::BadRequest(format!(
            "artefact kind `{kind}` is not streamable; use the URL on the job"
        )));
    }
    let key = key.ok_or_else(|| AppError::NotFound(format!("job {id} missing storage key")))?;
    if let Some(exp) = expires_at {
        if exp < Utc::now() {
            return Err(AppError::NotFound(format!("artefact for job {id} expired")));
        }
    }

    let upload_dir = state.media_backend.upload_dir().ok_or_else(|| {
        AppError::Internal(anyhow::anyhow!(
            "stream_artifact called with non-local backend"
        ))
    })?;
    let path = std::path::Path::new(upload_dir).join(&key);
    let bytes = tokio::fs::read(&path)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("read artefact: {e}")))?;
    let len = bytes.len();

    audit_admin_priv(
        &state.db,
        &privileged,
        &client,
        "admin.dsar.export.downloaded",
        "user",
        target_user_id.to_string(),
        serde_json::json!({ "job_id": id, "bytes": len }),
    )
    .await;

    let mut headers = HeaderMap::new();
    headers.insert(header::CONTENT_TYPE, HeaderValue::from_static("application/json"));
    headers.insert(
        header::CONTENT_DISPOSITION,
        HeaderValue::from_str(&format!("attachment; filename=\"dsar-{id}.json\""))
            .map_err(|e| AppError::Internal(anyhow::anyhow!("disposition header: {e}")))?,
    );
    Ok((StatusCode::OK, headers, Body::from(bytes)).into_response())
}

async fn fetch_job(pool: &sqlx::PgPool, id: Uuid) -> AppResult<Option<DsarJob>> {
    let row = sqlx::query_as::<_, DsarJob>(
        r#"
        SELECT id, target_user_id, kind, status, requested_by, request_reason,
               approved_by, approval_reason, approved_at, artifact_url,
               artifact_kind, artifact_storage_key, artifact_expires_at,
               erasure_summary, completed_at, failure_reason, created_at, updated_at
          FROM dsar_jobs
         WHERE id = $1
        "#,
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;
    Ok(row)
}

fn map_dsar_err(e: DsarAdminError) -> AppError {
    match e {
        DsarAdminError::UserNotFound(id) => AppError::NotFound(format!("user {id} not found")),
        DsarAdminError::AlreadyErased(id) => {
            AppError::Conflict(format!("user {id} is already tombstoned"))
        }
        DsarAdminError::Db(err) => AppError::Database(err),
        DsarAdminError::Compose(s) => AppError::Internal(anyhow::anyhow!(s)),
    }
}

// Suppress unused lint when the `Row` import is only used by some
// fetch paths (kept for future-proofing the manual scalar pulls).
#[allow(dead_code)]
fn _row_marker(r: &sqlx::postgres::PgRow) {
    let _: Result<i64, _> = r.try_get("id");
}
