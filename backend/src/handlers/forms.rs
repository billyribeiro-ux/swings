//! FORM-03: Submission store + audit.
//!
//! Public endpoints:
//!   * `GET  /api/forms/{slug}`          — active schema + logic + settings.
//!   * `POST /api/forms/{slug}/submit`   — validated submission (rate-limited).
//!   * `POST /api/forms/{slug}/partial`  — save-and-resume draft (rate-limited).
//!   * `GET  /api/forms/{slug}/partial?token=…` — resume a saved draft.
//!
//! Admin endpoints (AdminUser gate):
//!   * `GET  /api/admin/forms/{id}/submissions`      — list + filter + CSV.
//!   * `POST /api/admin/forms/{id}/submissions/bulk` — mark-spam / delete / restore.
//!
//! Every successful submit emits a `form.submitted` event to the outbox so
//! FDN-05's `NotifyHandler` can fan notifications out later.

use async_trait::async_trait;
use axum::{
    body::Body,
    extract::{Path, Query, State},
    http::{header, HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use chrono::{DateTime, Duration, Utc};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{
    error::{AppError, AppResult},
    events::{publish_in_tx, Event},
    extractors::{AdminUser, ClientInfo, OptionalAuthUser},
    forms::{
        repo::{self, InsertSubmission, SubmissionListFilter, SubmissionRow},
        schema::FieldSchema,
        validation::{validate, AsyncRuleRunner, ValidationError},
    },
    services::audit::audit_admin,
    AppState,
};

// ── Routers ────────────────────────────────────────────────────────────

pub fn public_router() -> Router<AppState> {
    // FDN-08: separate rate-limit layers for submit vs partial. The schema
    // GET stays on the global governor only.
    Router::new()
        // FORM-10 CountryStateField backing data — must be declared
        // before the catch-all `/{slug}` route below to avoid the slug
        // matcher swallowing the literal "geo" segment.
        .route("/geo/countries", get(public_geo_countries))
        .route("/geo/states", get(public_geo_states))
        .route("/{slug}", get(public_get_form))
        .merge(
            Router::new()
                .route("/{slug}/submit", post(public_submit))
                .layer(crate::middleware::rate_limit::form_submit_layer()),
        )
        .merge(
            Router::new()
                .route("/{slug}/partial", post(public_save_partial))
                .route("/{slug}/partial", get(public_load_partial))
                .layer(crate::middleware::rate_limit::form_partial_layer()),
        )
        // FORM-08: payment-intent endpoint shares the submit rate-limit
        // layer — it costs a Stripe round-trip on every call.
        .merge(
            Router::new()
                .route("/{slug}/payment-intent", post(public_create_payment_intent))
                .layer(crate::middleware::rate_limit::form_submit_layer()),
        )
}

pub fn admin_router() -> Router<AppState> {
    Router::new()
        .route("/{id}/submissions", get(admin_list_submissions))
        .route(
            "/{id}/submissions/bulk",
            post(admin_bulk_update_submissions),
        )
}

// ── DTOs ───────────────────────────────────────────────────────────────

/// Payload of `GET /api/forms/{slug}` — the shape FORM-10's renderer hydrates.
#[derive(Debug, Serialize, ToSchema)]
pub struct FormDefinition {
    pub id: Uuid,
    pub slug: String,
    pub name: String,
    pub description: Option<String>,
    pub settings: serde_json::Value,
    /// Active version's `schema_json` — a JSON array of `FieldSchema` variants.
    pub schema: serde_json::Value,
    /// Active version's `logic_json` — a JSON array of `LogicRule`.
    pub logic: serde_json::Value,
    pub version: i32,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct SubmitRequest {
    pub data: serde_json::Value,
    /// Optional UTM block captured from the query-string at render time.
    #[serde(default)]
    pub utm: serde_json::Value,
    /// Optional file descriptors from FORM-05. Shape `[{ field_key, file_id,
    /// filename, size, sha256, mime_type }]`.
    #[serde(default)]
    pub files: serde_json::Value,
    /// Optional anonymous id cookie — the frontend generates one for
    /// unauthenticated sessions so repeat submissions tie back together.
    #[serde(default)]
    pub anonymous_id: Option<Uuid>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct SubmitResponse {
    pub id: Uuid,
    pub status: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct PartialRequest {
    pub data: serde_json::Value,
    /// Optional: extend an existing resume token in place. When set + the
    /// token is valid, the existing row is updated and the same token is
    /// returned; otherwise a fresh token is minted.
    #[serde(default)]
    pub resume_token: Option<String>,
    /// Zero-based page-break index the draft was on when saved. Lets the
    /// renderer jump straight back to the right step on resume.
    #[serde(default)]
    pub current_step: Option<i32>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct PartialResponse {
    pub resume_token: String,
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct PartialLoadQuery {
    pub token: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct PaymentIntentRequest {
    /// FieldSchema `key` of the payment field on the form.
    pub field_key: String,
    /// Donor-supplied amount for `payment_kind = donation`. Ignored for
    /// fixed-amount one-time payments — the schema's `amount_cents` wins.
    #[serde(default)]
    pub amount_cents: Option<i64>,
    /// Donor email — receipts are sent to this address.
    pub email: String,
    /// Optional resume token if the field is on a draft (so we can
    /// later cross-link `form_payment_intents.partial_id`).
    #[serde(default)]
    pub resume_token: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct PaymentIntentClientResponse {
    pub intent_id: String,
    pub client_secret: String,
    pub amount_cents: i64,
    pub currency: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct PartialLoadResponse {
    pub data: serde_json::Value,
    pub current_step: i32,
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct SubmissionListQuery {
    pub page: Option<i64>,
    pub per_page: Option<i64>,
    pub status: Option<String>,
    pub from: Option<DateTime<Utc>>,
    pub to: Option<DateTime<Utc>>,
    pub format: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct PaginatedSubmissions {
    pub data: Vec<SubmissionRow>,
    pub total: i64,
    pub page: i64,
    pub per_page: i64,
    pub total_pages: i64,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct BulkActionRequest {
    pub ids: Vec<Uuid>,
    /// `delete` | `mark_spam` | `restore`.
    pub action: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct BulkActionResponse {
    pub updated: u64,
}

// ── Public handlers ────────────────────────────────────────────────────

#[utoipa::path(
    get,
    path = "/api/forms/{slug}",
    tag = "forms",
    params(("slug" = String, Path, description = "Form URL slug")),
    responses(
        (status = 200, description = "Active form definition", body = FormDefinition),
        (status = 404, description = "Form not found or has no published version")
    )
)]
pub async fn public_get_form(
    State(state): State<AppState>,
    Path(slug): Path<String>,
) -> AppResult<Json<FormDefinition>> {
    let form = repo::get_form_by_slug(&state.db, &slug)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("form `{slug}` not found")))?;
    if !form.is_active {
        return Err(AppError::NotFound(format!("form `{slug}` is not active")));
    }
    let version = repo::get_active_version(&state.db, form.id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("form `{slug}` has no published version")))?;

    Ok(Json(FormDefinition {
        id: form.id,
        slug: form.slug,
        name: form.name,
        description: form.description,
        settings: form.settings,
        schema: version.schema_json,
        logic: version.logic_json,
        version: version.version,
    }))
}

#[utoipa::path(
    post,
    path = "/api/forms/{slug}/submit",
    tag = "forms",
    params(("slug" = String, Path, description = "Form URL slug")),
    request_body = SubmitRequest,
    responses(
        (status = 200, description = "Submission accepted", body = SubmitResponse),
        (status = 404, description = "Form not found"),
        (status = 422, description = "Validation failed; response body carries per-field errors")
    )
)]
pub async fn public_submit(
    State(state): State<AppState>,
    opt: OptionalAuthUser,
    Path(slug): Path<String>,
    headers: HeaderMap,
    Json(req): Json<SubmitRequest>,
) -> AppResult<Json<SubmitResponse>> {
    let form = repo::get_form_by_slug(&state.db, &slug)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("form `{slug}` not found")))?;
    if !form.is_active {
        return Err(AppError::NotFound(format!("form `{slug}` is not active")));
    }
    let version = repo::get_active_version(&state.db, form.id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("form `{slug}` has no published version")))?;

    // Decode the schema.
    let schema: Vec<FieldSchema> =
        serde_json::from_value(version.schema_json.clone()).map_err(|e| {
            AppError::Internal(anyhow::anyhow!(
                "form `{slug}` version {} has malformed schema_json: {}",
                version.version,
                e
            ))
        })?;

    // Run validation.
    let runner = UniqueEmailRunner::new(state.db.clone());
    let errors = validate(&schema, &req.data, &runner).await;
    if !errors.is_empty() {
        let body =
            serde_json::to_value(&errors).map_err(|e| AppError::Internal(anyhow::anyhow!(e)))?;
        return Err(AppError::ValidationBody(body));
    }

    // Enrich audit fields. We rely on proxy-supplied headers for the IP —
    // Railway/Vercel set `x-forwarded-for` and strip client-supplied values,
    // so trusting them behind the reverse proxy is safe. See the "Trust note"
    // in `middleware::rate_limit`.
    let ip_hash = ip_hash_daily(&client_ip_str(&headers));
    let user_agent = header_str(&headers, header::USER_AGENT)
        .unwrap_or("")
        .to_string();
    let referrer = header_str(&headers, header::REFERER).map(|s| s.to_string());
    let utm = if req.utm.is_object() {
        req.utm.clone()
    } else {
        serde_json::json!({})
    };

    let mut tx = state.db.begin().await?;
    let submission = repo::insert_submission(
        &mut tx,
        InsertSubmission {
            form_id: form.id,
            form_version_id: version.id,
            subject_id: opt.user_id,
            anonymous_id: req.anonymous_id,
            status: "complete",
            data_json: &req.data,
            files_json: &req.files,
            ip_hash: &ip_hash,
            user_agent: &user_agent,
            referrer: referrer.as_deref(),
            utm: &utm,
            validation_errors: None,
        },
    )
    .await?;

    // FORM-03: emit `form.submitted` event — FDN-05 NotifyHandler consumes it.
    let event = Event {
        aggregate_type: "form".into(),
        aggregate_id: form.id.to_string(),
        event_type: "form.submitted".into(),
        payload: serde_json::json!({
            "form_id": form.id,
            "form_slug": form.slug,
            "submission_id": submission.id,
            "subject_id": submission.subject_id,
        }),
        headers: Default::default(),
    };
    publish_in_tx(&mut tx, &event)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!(e)))?;

    tx.commit().await?;

    Ok(Json(SubmitResponse {
        id: submission.id,
        status: submission.status,
    }))
}

#[utoipa::path(
    post,
    path = "/api/forms/{slug}/partial",
    tag = "forms",
    params(("slug" = String, Path, description = "Form URL slug")),
    request_body = PartialRequest,
    responses(
        (status = 200, description = "Partial saved", body = PartialResponse),
        (status = 404, description = "Form not found")
    )
)]
pub async fn public_save_partial(
    State(state): State<AppState>,
    opt: OptionalAuthUser,
    Path(slug): Path<String>,
    Json(req): Json<PartialRequest>,
) -> AppResult<Json<PartialResponse>> {
    let form = repo::get_form_by_slug(&state.db, &slug)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("form `{slug}` not found")))?;

    // 24h default; CONSENT-07 may expose this on form.settings later.
    let ttl = Duration::hours(24);
    let expires_at = Utc::now() + ttl;

    // Mint a fresh token (32 random bytes, hex-encoded for the URL) — only
    // the SHA-256 hash is stored server-side.
    // FORM-04: if the client supplies an existing resume token, extend the
    // same row in place; the plaintext never re-leaves the server. Otherwise
    // mint a fresh random token, hash it, insert, and return the plaintext
    // exactly once.
    if let Some(existing) = req.resume_token.as_deref() {
        let existing_hash = hash_bytes(existing);
        if let Some(row) = repo::update_partial_by_hash(
            &state.db,
            form.id,
            &existing_hash,
            &req.data,
            req.current_step.unwrap_or(0),
            expires_at,
        )
        .await?
        {
            return Ok(Json(PartialResponse {
                resume_token: existing.to_string(),
                expires_at: row.expires_at,
            }));
        }
        // Token unknown or expired — fall through and mint a fresh row.
    }

    let token = mint_token_hex();
    let token_hash = hash_bytes(&token);

    repo::insert_partial(
        &state.db,
        form.id,
        &token_hash,
        &req.data,
        req.current_step.unwrap_or(0),
        opt.user_id,
        expires_at,
    )
    .await?;

    Ok(Json(PartialResponse {
        resume_token: token,
        expires_at,
    }))
}

#[derive(Debug, Deserialize)]
pub struct GeoStatesQuery {
    pub country: String,
}

/// FORM-10: ISO 3166-1 alpha-2 country list for the chained dropdown.
#[utoipa::path(
    get,
    path = "/api/forms/geo/countries",
    tag = "forms",
    responses((status = 200, description = "Country list", body = [crate::forms::geo::Country]))
)]
pub async fn public_geo_countries() -> Json<Vec<crate::forms::geo::Country>> {
    Json(crate::forms::geo::countries())
}

/// FORM-10: ISO 3166-2 state / province list for the supplied alpha-2
/// country. Returns an empty array for uncovered countries — the
/// renderer falls back to a free-text input in that case.
#[utoipa::path(
    get,
    path = "/api/forms/geo/states",
    tag = "forms",
    params(("country" = String, Query, description = "ISO 3166-1 alpha-2")),
    responses((status = 200, description = "State list", body = [crate::forms::geo::State]))
)]
pub async fn public_geo_states(
    Query(q): Query<GeoStatesQuery>,
) -> Json<Vec<crate::forms::geo::State>> {
    Json(crate::forms::geo::states_for(&q.country.to_uppercase()))
}

/// FORM-08: mint a Stripe PaymentIntent for a payment / donation field.
///
/// Required header: `Idempotency-Key` (UUID-ish opaque token). Replays
/// short-circuit at the DB lookup before we ever round-trip Stripe.
#[utoipa::path(
    post,
    path = "/api/forms/{slug}/payment-intent",
    tag = "forms",
    params(("slug" = String, Path, description = "Form URL slug")),
    request_body = PaymentIntentRequest,
    responses(
        (status = 200, description = "Intent created", body = PaymentIntentClientResponse),
        (status = 400, description = "Missing Idempotency-Key or invalid amount"),
        (status = 404, description = "Form / field not found")
    )
)]
pub async fn public_create_payment_intent(
    State(state): State<AppState>,
    Path(slug): Path<String>,
    headers: HeaderMap,
    Json(req): Json<PaymentIntentRequest>,
) -> AppResult<Json<PaymentIntentClientResponse>> {
    let idempotency_key = headers
        .get("idempotency-key")
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| AppError::BadRequest("Missing Idempotency-Key header".into()))?;

    if idempotency_key.len() < 8 || idempotency_key.len() > 128 {
        return Err(AppError::BadRequest(
            "Idempotency-Key must be 8..128 chars".into(),
        ));
    }

    // Replay short-circuit — return the stored secret without touching Stripe.
    if let Some(existing) =
        crate::forms::find_payment_intent_by_key(&state.db, idempotency_key).await?
    {
        return Ok(Json(PaymentIntentClientResponse {
            intent_id: existing.stripe_payment_intent_id,
            client_secret: existing.stripe_client_secret,
            amount_cents: existing.amount_cents,
            currency: existing.currency,
        }));
    }

    let form = repo::get_form_by_slug(&state.db, &slug)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("form `{slug}` not found")))?;
    if !form.is_active {
        return Err(AppError::NotFound(format!("form `{slug}` is not active")));
    }
    let version = repo::get_active_version(&state.db, form.id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("form `{slug}` has no published version")))?;

    // Locate the named field + extract its payment config from schema_json.
    let fields: Vec<FieldSchema> =
        serde_json::from_value(version.schema_json.clone()).map_err(|e| {
            AppError::Internal(anyhow::anyhow!(
                "form schema is not a FieldSchema array: {e}"
            ))
        })?;
    let field = fields
        .into_iter()
        .find(|f| f.meta().key == req.field_key)
        .ok_or_else(|| {
            AppError::NotFound(format!("field `{}` not on form `{}`", req.field_key, slug))
        })?;

    let (kind, amount_cents, currency) = match field {
        FieldSchema::Payment {
            amount_cents: schema_amount,
            currency,
            payment_kind,
            suggested_amounts,
            allow_custom,
            ..
        } => {
            let kind = crate::forms::PaymentKind::parse(&payment_kind).ok_or_else(|| {
                AppError::BadRequest(format!("unknown payment_kind `{payment_kind}`"))
            })?;
            let resolved = match kind {
                crate::forms::PaymentKind::Donation => {
                    let req_amount = req.amount_cents.unwrap_or(schema_amount);
                    crate::forms::validate_donation_amount(
                        req_amount,
                        &suggested_amounts,
                        allow_custom,
                    )
                    .map_err(|e| AppError::BadRequest(format!("{e}")))?;
                    req_amount
                }
                _ => schema_amount,
            };
            (kind, resolved, currency)
        }
        FieldSchema::Subscription { .. } => {
            return Err(AppError::BadRequest(
                "subscription fields require the dedicated subscription endpoint".into(),
            ));
        }
        _ => {
            return Err(AppError::BadRequest(format!(
                "field `{}` is not a payment field",
                req.field_key
            )));
        }
    };

    let metadata_id = format!("form:{}", form.id);
    let (pi_id, client_secret) = crate::stripe_api::create_form_payment_intent(
        &state,
        amount_cents,
        &currency,
        &req.email,
        &metadata_id,
        Some(idempotency_key),
    )
    .await?;

    crate::forms::insert_payment_intent(
        &state.db,
        form.id,
        &req.field_key,
        None,
        &pi_id,
        &client_secret,
        None,
        None,
        amount_cents,
        &currency,
        kind,
        idempotency_key,
        "requires_payment_method",
    )
    .await?;

    Ok(Json(PaymentIntentClientResponse {
        intent_id: pi_id,
        client_secret,
        amount_cents,
        currency,
    }))
}

pub async fn public_load_partial(
    State(state): State<AppState>,
    Path(slug): Path<String>,
    Query(q): Query<PartialLoadQuery>,
) -> AppResult<Json<PartialLoadResponse>> {
    let form = repo::get_form_by_slug(&state.db, &slug)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("form `{slug}` not found")))?;
    let token_hash = hash_bytes(&q.token);
    let row = repo::resolve_partial(&state.db, form.id, &token_hash)
        .await?
        .ok_or_else(|| AppError::NotFound("partial not found or expired".into()))?;
    Ok(Json(PartialLoadResponse {
        data: row.data_json,
        current_step: row.current_step,
        expires_at: row.expires_at,
    }))
}

// ── Admin handlers ─────────────────────────────────────────────────────

pub async fn admin_list_submissions(
    State(state): State<AppState>,
    _admin: AdminUser,
    Path(form_id): Path<Uuid>,
    Query(q): Query<SubmissionListQuery>,
) -> AppResult<Response> {
    let per_page = q.per_page.unwrap_or(25).clamp(1, 500);
    let page = q.page.unwrap_or(1).max(1);
    let offset = (page - 1) * per_page;

    let (rows, total) = repo::list_submissions(
        &state.db,
        SubmissionListFilter {
            form_id,
            status: q.status.as_deref(),
            from: q.from,
            to: q.to,
            limit: per_page,
            offset,
        },
    )
    .await?;

    if q.format.as_deref() == Some("csv") {
        let csv = submissions_to_csv(&rows);
        let resp = Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, "text/csv; charset=utf-8")
            .header(
                header::CONTENT_DISPOSITION,
                format!("attachment; filename=\"form_{form_id}_submissions.csv\""),
            )
            .body(Body::from(csv))
            .map_err(|e| AppError::Internal(anyhow::anyhow!(e)))?;
        return Ok(resp);
    }

    let total_pages = (total as f64 / per_page as f64).ceil() as i64;
    Ok(Json(PaginatedSubmissions {
        data: rows,
        total,
        page,
        per_page,
        total_pages,
    })
    .into_response())
}

#[utoipa::path(
    post,
    path = "/api/admin/forms/{id}/submissions/bulk",
    tag = "forms",
    security(("bearer_auth" = [])),
    params(("id" = Uuid, Path, description = "Form id")),
    request_body = BulkActionRequest,
    responses(
        (status = 200, description = "Bulk action applied", body = BulkActionResponse),
        (status = 400, description = "Unknown action"),
        (status = 403, description = "Forbidden")
    )
)]
pub async fn admin_bulk_update_submissions(
    State(state): State<AppState>,
    admin: AdminUser,
    client: ClientInfo,
    Path(form_id): Path<Uuid>,
    Json(req): Json<BulkActionRequest>,
) -> AppResult<Json<BulkActionResponse>> {
    let status = match req.action.as_str() {
        "delete" => "deleted",
        "mark_spam" => "spam",
        "restore" => "complete",
        other => {
            return Err(AppError::BadRequest(format!(
                "unknown bulk action: {other}"
            )));
        }
    };
    let updated = repo::bulk_update_submission_status(&state.db, form_id, &req.ids, status).await?;

    audit_admin(
        &state.db,
        &admin,
        &client,
        "form.submissions.bulk_update",
        "form",
        form_id,
        serde_json::json!({
            "action": req.action,
            "status": status,
            "submission_ids": req.ids,
            "updated": updated,
        }),
    )
    .await;

    Ok(Json(BulkActionResponse { updated }))
}

// ── Helpers ────────────────────────────────────────────────────────────

fn header_str(headers: &HeaderMap, key: header::HeaderName) -> Option<&str> {
    headers.get(key).and_then(|v| v.to_str().ok())
}

/// Best-effort client IP from proxy headers — matches the shape used by
/// `middleware::rate_limit::client_ip`. Falls back to `unknown` when both
/// headers are absent; `ConnectInfo`-based peer fallback is handled at the
/// rate-limit layer already.
fn client_ip_str(headers: &HeaderMap) -> String {
    if let Some(v) = headers.get("x-forwarded-for").and_then(|v| v.to_str().ok()) {
        if let Some(first) = v.split(',').next() {
            let t = first.trim();
            if !t.is_empty() {
                return t.to_owned();
            }
        }
    }
    if let Some(v) = headers.get("x-real-ip").and_then(|v| v.to_str().ok()) {
        let t = v.trim();
        if !t.is_empty() {
            return t.to_owned();
        }
    }
    "unknown".to_string()
}

/// Daily-rotated IP hash — `SHA-256(ip || YYYY-MM-DD)`. Coarse bucket prevents
/// trivial correlation across days while still letting ops de-dupe submissions
/// from a single attacker during a single incident window.
///
/// TODO: dedupe with the CONSENT-03 helper (`crate::consent::records::ip_hash_daily`)
/// once that subsystem lands — the shape MUST remain identical.
fn ip_hash_daily(ip: &str) -> String {
    let today = Utc::now().format("%Y-%m-%d").to_string();
    let mut hasher = Sha256::new();
    hasher.update(ip.as_bytes());
    hasher.update(b"|");
    hasher.update(today.as_bytes());
    hasher
        .finalize()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

/// FORM-04: the `form_partials.resume_token_hash` column is BYTEA, so the
/// handler hashes tokens straight to the 32-byte digest instead of taking
/// a round trip through hex.
fn hash_bytes(input: &str) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    hasher.finalize().into()
}

fn mint_token_hex() -> String {
    let mut bytes = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut bytes);
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn submissions_to_csv(rows: &[SubmissionRow]) -> String {
    let mut out = String::new();
    out.push_str(
        "id,form_id,form_version_id,subject_id,status,submitted_at,ip_hash,user_agent,referrer\n",
    );
    for r in rows {
        out.push_str(&csv_field(&r.id.to_string()));
        out.push(',');
        out.push_str(&csv_field(&r.form_id.to_string()));
        out.push(',');
        out.push_str(&csv_field(&r.form_version_id.to_string()));
        out.push(',');
        out.push_str(&csv_field(
            &r.subject_id.map(|u| u.to_string()).unwrap_or_default(),
        ));
        out.push(',');
        out.push_str(&csv_field(&r.status));
        out.push(',');
        out.push_str(&csv_field(&r.submitted_at.to_rfc3339()));
        out.push(',');
        out.push_str(&csv_field(&r.ip_hash));
        out.push(',');
        out.push_str(&csv_field(&r.user_agent));
        out.push(',');
        out.push_str(&csv_field(r.referrer.as_deref().unwrap_or("")));
        out.push('\n');
    }
    out
}

/// Minimal RFC 4180 field encoder. Quotes are doubled and the whole field
/// is quoted when it contains `,`, `"`, `\r`, or `\n`.
fn csv_field(s: &str) -> String {
    let needs_quote = s.contains(',') || s.contains('"') || s.contains('\n') || s.contains('\r');
    if !needs_quote {
        return s.to_owned();
    }
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        if c == '"' {
            out.push('"');
            out.push('"');
        } else {
            out.push(c);
        }
    }
    out.push('"');
    out
}

// ── AsyncRuleRunner impl ───────────────────────────────────────────────

/// Resolves `unique_email` by checking `users.email` — the only async rule
/// currently wired into FORM-02.
struct UniqueEmailRunner {
    pool: sqlx::PgPool,
}

impl UniqueEmailRunner {
    fn new(pool: sqlx::PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl AsyncRuleRunner for UniqueEmailRunner {
    async fn run(
        &self,
        field_key: &str,
        rule: &crate::forms::schema::AsyncRule,
        value: &serde_json::Value,
    ) -> Option<ValidationError> {
        match rule {
            crate::forms::schema::AsyncRule::UniqueEmail => {
                let email = value.as_str()?;
                // Case-insensitive comparison — matches the existing
                // `UPPER(email) = UPPER($1)` pattern used in auth.
                let exists: Option<(Uuid,)> = sqlx::query_as(
                    r#"SELECT id FROM users WHERE LOWER(email) = LOWER($1) LIMIT 1"#,
                )
                .bind(email)
                .fetch_optional(&self.pool)
                .await
                .ok()
                .flatten();
                if exists.is_some() {
                    return Some(ValidationError {
                        field_key: field_key.to_string(),
                        code: "unique_email".to_string(),
                        message: "This email is already registered.".to_string(),
                    });
                }
                None
            }
        }
    }
}

// ── Unit tests ─────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn csv_field_escapes_quotes_and_commas() {
        assert_eq!(csv_field("hello"), "hello");
        assert_eq!(csv_field("a,b"), "\"a,b\"");
        assert_eq!(csv_field("she said \"hi\""), "\"she said \"\"hi\"\"\"");
        assert_eq!(csv_field("line1\nline2"), "\"line1\nline2\"");
    }

    #[test]
    fn ip_hash_daily_is_deterministic_within_a_day() {
        let a = ip_hash_daily("1.2.3.4");
        let b = ip_hash_daily("1.2.3.4");
        assert_eq!(a, b);
        let c = ip_hash_daily("1.2.3.5");
        assert_ne!(a, c);
        // 64 hex chars = 32 bytes of SHA-256.
        assert_eq!(a.len(), 64);
    }

    #[test]
    fn hash_bytes_is_stable() {
        let a = hash_bytes("token");
        let b = hash_bytes("token");
        assert_eq!(a, b);
        assert_eq!(a.len(), 32);
        // Known SHA-256("token") first byte.
        assert_eq!(a[0], 0x3c);
    }

    #[test]
    fn mint_token_hex_is_64_chars() {
        let t = mint_token_hex();
        assert_eq!(t.len(), 64);
        assert!(t.chars().all(|c| c.is_ascii_hexdigit()));
    }
}
