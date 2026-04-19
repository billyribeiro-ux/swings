//! FDN-05: admin + public HTTP surface for the notifications subsystem.
//!
//! Admin routes (all `AdminUser`-gated) cover template management, delivery
//! log, and suppression list ops. The public `/u/unsubscribe` route consumes
//! one-shot tokens and renders a success page.

use axum::{
    extract::{Path, Query, State},
    http::{header, StatusCode},
    response::{Html, IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::Row;
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

use crate::{
    common::html::sanitize_rich_text,
    error::{AppError, AppResult},
    extractors::{AdminUser, ClientInfo},
    models::PaginatedResponse,
    notifications::{
        channels::DeliveryRequest,
        preferences::{self, NotificationPreference, PreferenceUpdate},
        send::Recipient,
        suppression::{self, Suppression},
        templates::{RenderedTemplate, Template},
        unsubscribe::{self, UnsubscribeAction, UnsubscribeError},
        ChannelError,
    },
    services::audit::audit_admin,
    AppState,
};

/// Admin router — mounted under `/api/admin/notifications`.
pub fn admin_router() -> Router<AppState> {
    Router::new()
        .route("/templates", get(list_templates).post(create_template))
        .route("/templates/{id}", get(get_template).put(update_template))
        .route("/templates/{id}/preview", post(preview_template))
        .route("/templates/{id}/test-send", post(test_send_template))
        .route("/deliveries", get(list_deliveries))
        .route("/suppression", get(list_suppression).post(add_suppression))
        .route("/suppression/remove", post(remove_suppression))
}

/// Public router — mounted under `/u`.
pub fn public_router() -> Router<AppState> {
    Router::new().route("/unsubscribe", get(unsubscribe_page))
}

/// Member router — mounted under `/api/member`. Extends the existing member
/// surface with notification-preference self-service.
pub fn member_router() -> Router<AppState> {
    Router::new().route(
        "/notification-preferences",
        get(get_member_preferences).put(update_member_preferences),
    )
}

// ── Template DTOs ───────────────────────────────────────────────────────

#[derive(Debug, Serialize, ToSchema)]
pub struct PaginatedTemplatesResponse {
    pub data: Vec<Template>,
    pub total: i64,
    pub page: i64,
    pub per_page: i64,
    pub total_pages: i64,
}

#[derive(Debug, Deserialize, IntoParams, ToSchema)]
pub struct TemplateListQuery {
    pub key: Option<String>,
    pub channel: Option<String>,
    pub locale: Option<String>,
    pub active_only: Option<bool>,
    pub page: Option<i64>,
    pub per_page: Option<i64>,
}

impl TemplateListQuery {
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

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateTemplateRequest {
    pub key: String,
    pub channel: String,
    pub locale: Option<String>,
    pub subject: Option<String>,
    pub body_source: String,
    pub variables: Option<serde_json::Value>,
    pub is_active: Option<bool>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateTemplateRequest {
    pub subject: Option<String>,
    pub body_source: String,
    pub variables: Option<serde_json::Value>,
    pub is_active: Option<bool>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct PreviewRequest {
    pub context: serde_json::Value,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct TestSendRequest {
    pub to: String,
    pub context: serde_json::Value,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct TestSendResponse {
    pub provider_id: String,
    pub subject: Option<String>,
}

// ── Template handlers ───────────────────────────────────────────────────

#[utoipa::path(
    get,
    path = "/api/admin/notifications/templates",
    tag = "admin-notifications",
    security(("bearer_auth" = [])),
    params(TemplateListQuery),
    responses(
        (status = 200, description = "Paginated template rows", body = PaginatedTemplatesResponse),
        (status = 403, description = "Forbidden")
    )
)]
pub async fn list_templates(
    State(state): State<AppState>,
    _admin: AdminUser,
    Query(q): Query<TemplateListQuery>,
) -> AppResult<Json<PaginatedResponse<Template>>> {
    let per_page = q.per_page();
    let offset = q.offset();
    let page = q.page();
    let active_only = q.active_only.unwrap_or(false);

    let rows = sqlx::query_as::<_, Template>(
        r#"
        SELECT id, key, channel, locale, subject, body_source, body_compiled,
               variables, version, is_active, created_at, updated_at
        FROM notification_templates
        WHERE ($1::text IS NULL OR key = $1)
          AND ($2::text IS NULL OR channel = $2)
          AND ($3::text IS NULL OR locale = $3)
          AND (NOT $4 OR is_active = TRUE)
        ORDER BY key, channel, locale, version DESC
        LIMIT $5 OFFSET $6
        "#,
    )
    .bind(q.key.as_deref())
    .bind(q.channel.as_deref())
    .bind(q.locale.as_deref())
    .bind(active_only)
    .bind(per_page)
    .bind(offset)
    .fetch_all(&state.db)
    .await?;

    let total: i64 = sqlx::query_scalar::<_, i64>(
        r#"
        SELECT COUNT(*)
        FROM notification_templates
        WHERE ($1::text IS NULL OR key = $1)
          AND ($2::text IS NULL OR channel = $2)
          AND ($3::text IS NULL OR locale = $3)
          AND (NOT $4 OR is_active = TRUE)
        "#,
    )
    .bind(q.key.as_deref())
    .bind(q.channel.as_deref())
    .bind(q.locale.as_deref())
    .bind(active_only)
    .fetch_one(&state.db)
    .await?;

    let total_pages = if per_page > 0 {
        (total as f64 / per_page as f64).ceil() as i64
    } else {
        0
    };

    Ok(Json(PaginatedResponse {
        data: rows,
        total,
        page,
        per_page,
        total_pages,
    }))
}

#[utoipa::path(
    post,
    path = "/api/admin/notifications/templates",
    tag = "admin-notifications",
    security(("bearer_auth" = [])),
    request_body = CreateTemplateRequest,
    responses(
        (status = 200, description = "New template version", body = Template),
        (status = 400, description = "Invalid payload"),
        (status = 403, description = "Forbidden")
    )
)]
pub async fn create_template(
    State(state): State<AppState>,
    admin: AdminUser,
    client: ClientInfo,
    Json(req): Json<CreateTemplateRequest>,
) -> AppResult<Json<Template>> {
    if req.key.trim().is_empty() {
        return Err(AppError::BadRequest("key is required".into()));
    }
    if !is_allowed_channel(&req.channel) {
        return Err(AppError::BadRequest(format!(
            "channel `{}` is not allowed",
            req.channel
        )));
    }
    let locale = req.locale.unwrap_or_else(|| "en".to_string());
    let version = Template::next_version(&state.db, &req.key, &req.channel, &locale).await?;
    let body_source = sanitize_rich_text(&req.body_source);
    let body_compiled = body_source.clone();
    let variables = req.variables.unwrap_or_else(|| serde_json::json!([]));
    let is_active = req.is_active.unwrap_or(true);

    let row = sqlx::query_as::<_, Template>(
        r#"
        INSERT INTO notification_templates
            (key, channel, locale, subject, body_source, body_compiled,
             variables, version, is_active)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
        RETURNING id, key, channel, locale, subject, body_source, body_compiled,
                  variables, version, is_active, created_at, updated_at
        "#,
    )
    .bind(&req.key)
    .bind(&req.channel)
    .bind(&locale)
    .bind(req.subject.as_deref())
    .bind(&body_source)
    .bind(&body_compiled)
    .bind(&variables)
    .bind(version)
    .bind(is_active)
    .fetch_one(&state.db)
    .await?;

    audit_admin(
        &state.db,
        &admin,
        &client,
        "notification_template.create",
        "notification_template",
        row.id,
        serde_json::json!({
            "key": row.key,
            "channel": row.channel,
            "locale": row.locale,
            "version": row.version,
        }),
    )
    .await;

    Ok(Json(row))
}

#[utoipa::path(
    get,
    path = "/api/admin/notifications/templates/{id}",
    tag = "admin-notifications",
    security(("bearer_auth" = [])),
    params(("id" = Uuid, Path, description = "Template id")),
    responses(
        (status = 200, description = "Template row", body = Template),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Not found")
    )
)]
pub async fn get_template(
    State(state): State<AppState>,
    _admin: AdminUser,
    Path(id): Path<Uuid>,
) -> AppResult<Json<Template>> {
    let row = sqlx::query_as::<_, Template>(
        r#"
        SELECT id, key, channel, locale, subject, body_source, body_compiled,
               variables, version, is_active, created_at, updated_at
        FROM notification_templates
        WHERE id = $1
        "#,
    )
    .bind(id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| AppError::NotFound(format!("template {id}")))?;
    Ok(Json(row))
}

#[utoipa::path(
    put,
    path = "/api/admin/notifications/templates/{id}",
    tag = "admin-notifications",
    security(("bearer_auth" = [])),
    params(("id" = Uuid, Path, description = "Base template id (read for key/channel/locale lookup)")),
    request_body = UpdateTemplateRequest,
    responses(
        (status = 200, description = "New version row", body = Template),
        (status = 400, description = "Invalid payload"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Not found")
    )
)]
pub async fn update_template(
    State(state): State<AppState>,
    admin: AdminUser,
    client: ClientInfo,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateTemplateRequest>,
) -> AppResult<Json<Template>> {
    let base = sqlx::query_as::<_, Template>(
        r#"
        SELECT id, key, channel, locale, subject, body_source, body_compiled,
               variables, version, is_active, created_at, updated_at
        FROM notification_templates
        WHERE id = $1
        "#,
    )
    .bind(id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| AppError::NotFound(format!("template {id}")))?;

    let version = Template::next_version(&state.db, &base.key, &base.channel, &base.locale).await?;
    let body_source = sanitize_rich_text(&req.body_source);
    let body_compiled = body_source.clone();
    let variables = req.variables.unwrap_or(base.variables);
    let is_active = req.is_active.unwrap_or(base.is_active);
    let subject = req.subject.as_deref().or(base.subject.as_deref());

    let row = sqlx::query_as::<_, Template>(
        r#"
        INSERT INTO notification_templates
            (key, channel, locale, subject, body_source, body_compiled,
             variables, version, is_active)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
        RETURNING id, key, channel, locale, subject, body_source, body_compiled,
                  variables, version, is_active, created_at, updated_at
        "#,
    )
    .bind(&base.key)
    .bind(&base.channel)
    .bind(&base.locale)
    .bind(subject)
    .bind(&body_source)
    .bind(&body_compiled)
    .bind(&variables)
    .bind(version)
    .bind(is_active)
    .fetch_one(&state.db)
    .await?;

    audit_admin(
        &state.db,
        &admin,
        &client,
        "notification_template.update",
        "notification_template",
        row.id,
        serde_json::json!({
            "base_id": id,
            "key": row.key,
            "channel": row.channel,
            "locale": row.locale,
            "version": row.version,
        }),
    )
    .await;

    Ok(Json(row))
}

#[utoipa::path(
    post,
    path = "/api/admin/notifications/templates/{id}/preview",
    tag = "admin-notifications",
    security(("bearer_auth" = [])),
    params(("id" = Uuid, Path, description = "Template id")),
    request_body = PreviewRequest,
    responses(
        (status = 200, description = "Rendered template", body = RenderedTemplate),
        (status = 400, description = "Invalid template or context"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Not found")
    )
)]
pub async fn preview_template(
    State(state): State<AppState>,
    _admin: AdminUser,
    Path(id): Path<Uuid>,
    Json(req): Json<PreviewRequest>,
) -> AppResult<Json<RenderedTemplate>> {
    let row = sqlx::query_as::<_, Template>(
        r#"
        SELECT id, key, channel, locale, subject, body_source, body_compiled,
               variables, version, is_active, created_at, updated_at
        FROM notification_templates
        WHERE id = $1
        "#,
    )
    .bind(id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| AppError::NotFound(format!("template {id}")))?;

    let rendered = row.render(&req.context)?;
    Ok(Json(rendered))
}

#[utoipa::path(
    post,
    path = "/api/admin/notifications/templates/{id}/test-send",
    tag = "admin-notifications",
    security(("bearer_auth" = [])),
    params(("id" = Uuid, Path, description = "Template id")),
    request_body = TestSendRequest,
    responses(
        (status = 200, description = "Delivered via the live channel", body = TestSendResponse),
        (status = 400, description = "Invalid payload"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Not found"),
        (status = 503, description = "Channel provider unavailable")
    )
)]
pub async fn test_send_template(
    State(state): State<AppState>,
    admin: AdminUser,
    client: ClientInfo,
    Path(id): Path<Uuid>,
    Json(req): Json<TestSendRequest>,
) -> AppResult<Json<TestSendResponse>> {
    if req.to.trim().is_empty() {
        return Err(AppError::BadRequest("`to` is required".into()));
    }
    let row = sqlx::query_as::<_, Template>(
        r#"
        SELECT id, key, channel, locale, subject, body_source, body_compiled,
               variables, version, is_active, created_at, updated_at
        FROM notification_templates
        WHERE id = $1
        "#,
    )
    .bind(id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| AppError::NotFound(format!("template {id}")))?;

    let rendered = row.render(&req.context)?;
    let channel = state
        .notifications
        .channels()
        .get(&row.channel)
        .ok_or_else(|| AppError::BadRequest(format!("no channel wired for `{}`", row.channel)))?
        .clone();

    let delivery = DeliveryRequest {
        to: &req.to,
        to_name: None,
        template_key: &row.key,
        subject: rendered.subject.as_deref(),
        body: &rendered.body,
        locale: &row.locale,
        idempotency_key: None,
    };

    let provider_id = channel.send(&delivery).await.map_err(|e| match e {
        ChannelError::Permanent(msg) => AppError::BadRequest(msg),
        ChannelError::Transient(msg) => AppError::ServiceUnavailable(msg),
    })?;

    audit_admin(
        &state.db,
        &admin,
        &client,
        "notification_template.test_send",
        "notification_template",
        id,
        serde_json::json!({
            "key": row.key,
            "channel": row.channel,
            "to": req.to,
            "provider_id": provider_id,
        }),
    )
    .await;

    Ok(Json(TestSendResponse {
        provider_id,
        subject: rendered.subject,
    }))
}

// ── Delivery log ────────────────────────────────────────────────────────

#[derive(Debug, Deserialize, IntoParams, ToSchema)]
pub struct DeliveryListQuery {
    pub status: Option<String>,
    pub user_id: Option<Uuid>,
    pub page: Option<i64>,
    pub per_page: Option<i64>,
}

impl DeliveryListQuery {
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

#[derive(Debug, sqlx::FromRow, Serialize, ToSchema)]
pub struct DeliveryRow {
    pub id: Uuid,
    pub user_id: Option<Uuid>,
    pub anonymous_email: Option<String>,
    pub template_key: String,
    pub channel: String,
    pub provider_id: Option<String>,
    pub status: String,
    pub subject: Option<String>,
    pub rendered_body: String,
    pub metadata: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct PaginatedDeliveriesResponse {
    pub data: Vec<DeliveryRow>,
    pub total: i64,
    pub page: i64,
    pub per_page: i64,
    pub total_pages: i64,
}

#[utoipa::path(
    get,
    path = "/api/admin/notifications/deliveries",
    tag = "admin-notifications",
    security(("bearer_auth" = [])),
    params(DeliveryListQuery),
    responses(
        (status = 200, description = "Paginated delivery log", body = PaginatedDeliveriesResponse),
        (status = 403, description = "Forbidden")
    )
)]
pub async fn list_deliveries(
    State(state): State<AppState>,
    _admin: AdminUser,
    Query(q): Query<DeliveryListQuery>,
) -> AppResult<Json<PaginatedResponse<DeliveryRow>>> {
    let per_page = q.per_page();
    let page = q.page();
    let offset = q.offset();

    let rows = sqlx::query_as::<_, DeliveryRow>(
        r#"
        SELECT id, user_id, anonymous_email, template_key, channel, provider_id,
               status, subject, rendered_body, metadata, created_at, updated_at
        FROM notification_deliveries
        WHERE ($1::text IS NULL OR status = $1)
          AND ($2::uuid IS NULL OR user_id = $2)
        ORDER BY created_at DESC
        LIMIT $3 OFFSET $4
        "#,
    )
    .bind(q.status.as_deref())
    .bind(q.user_id)
    .bind(per_page)
    .bind(offset)
    .fetch_all(&state.db)
    .await?;

    let total_row = sqlx::query(
        r#"
        SELECT COUNT(*) AS total
        FROM notification_deliveries
        WHERE ($1::text IS NULL OR status = $1)
          AND ($2::uuid IS NULL OR user_id = $2)
        "#,
    )
    .bind(q.status.as_deref())
    .bind(q.user_id)
    .fetch_one(&state.db)
    .await?;
    let total: i64 = total_row.try_get("total")?;

    let total_pages = if per_page > 0 {
        (total as f64 / per_page as f64).ceil() as i64
    } else {
        0
    };

    Ok(Json(PaginatedResponse {
        data: rows,
        total,
        page,
        per_page,
        total_pages,
    }))
}

// ── Suppression ─────────────────────────────────────────────────────────

#[derive(Debug, Deserialize, IntoParams, ToSchema)]
pub struct SuppressionListQuery {
    pub page: Option<i64>,
    pub per_page: Option<i64>,
}

impl SuppressionListQuery {
    fn per_page(&self) -> i64 {
        self.per_page.unwrap_or(50).clamp(1, 200)
    }
    fn page(&self) -> i64 {
        self.page.unwrap_or(1).max(1)
    }
    fn offset(&self) -> i64 {
        (self.page() - 1) * self.per_page()
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct PaginatedSuppressionResponse {
    pub data: Vec<Suppression>,
    pub total: i64,
    pub page: i64,
    pub per_page: i64,
    pub total_pages: i64,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct AddSuppressionRequest {
    pub email: String,
    pub reason: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct RemoveSuppressionRequest {
    pub email: String,
}

#[utoipa::path(
    get,
    path = "/api/admin/notifications/suppression",
    tag = "admin-notifications",
    security(("bearer_auth" = [])),
    params(SuppressionListQuery),
    responses(
        (status = 200, description = "Paginated suppression entries", body = PaginatedSuppressionResponse),
        (status = 403, description = "Forbidden")
    )
)]
pub async fn list_suppression(
    State(state): State<AppState>,
    _admin: AdminUser,
    Query(q): Query<SuppressionListQuery>,
) -> AppResult<Json<PaginatedResponse<Suppression>>> {
    let per_page = q.per_page();
    let page = q.page();
    let offset = q.offset();

    let rows = suppression::list(&state.db, per_page, offset).await?;
    let total = suppression::count(&state.db).await?;
    let total_pages = if per_page > 0 {
        (total as f64 / per_page as f64).ceil() as i64
    } else {
        0
    };
    Ok(Json(PaginatedResponse {
        data: rows,
        total,
        page,
        per_page,
        total_pages,
    }))
}

#[utoipa::path(
    post,
    path = "/api/admin/notifications/suppression",
    tag = "admin-notifications",
    security(("bearer_auth" = [])),
    request_body = AddSuppressionRequest,
    responses(
        (status = 200, description = "Address suppressed", body = Suppression),
        (status = 400, description = "Invalid payload"),
        (status = 403, description = "Forbidden")
    )
)]
pub async fn add_suppression(
    State(state): State<AppState>,
    admin: AdminUser,
    client: ClientInfo,
    Json(req): Json<AddSuppressionRequest>,
) -> AppResult<Json<Suppression>> {
    if req.email.trim().is_empty() {
        return Err(AppError::BadRequest("email is required".into()));
    }
    if req.reason.trim().is_empty() {
        return Err(AppError::BadRequest("reason is required".into()));
    }
    let row = suppression::suppress(&state.db, &req.email, &req.reason).await?;

    audit_admin(
        &state.db,
        &admin,
        &client,
        "notification.suppression.add",
        "suppression",
        row.email.clone(),
        serde_json::json!({
            "email": row.email,
            "reason": req.reason,
        }),
    )
    .await;

    Ok(Json(row))
}

#[utoipa::path(
    post,
    path = "/api/admin/notifications/suppression/remove",
    tag = "admin-notifications",
    security(("bearer_auth" = [])),
    request_body = RemoveSuppressionRequest,
    responses(
        (status = 200, description = "Whether a row was removed"),
        (status = 403, description = "Forbidden")
    )
)]
pub async fn remove_suppression(
    State(state): State<AppState>,
    admin: AdminUser,
    client: ClientInfo,
    Json(req): Json<RemoveSuppressionRequest>,
) -> AppResult<Json<serde_json::Value>> {
    let removed = suppression::unsuppress(&state.db, &req.email).await?;

    audit_admin(
        &state.db,
        &admin,
        &client,
        "notification.suppression.remove",
        "suppression",
        req.email.clone(),
        serde_json::json!({
            "email": req.email,
            "removed": removed,
        }),
    )
    .await;

    Ok(Json(serde_json::json!({ "removed": removed })))
}

// ── Member preferences ──────────────────────────────────────────────────

#[derive(Debug, Deserialize, ToSchema)]
pub struct BulkPreferenceUpdate {
    pub items: Vec<PreferenceUpdate>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct MemberPreferencesResponse {
    pub preferences: Vec<NotificationPreference>,
}

#[utoipa::path(
    get,
    path = "/api/member/notification-preferences",
    tag = "member",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Current user's notification preferences", body = MemberPreferencesResponse),
        (status = 401, description = "Unauthorized")
    )
)]
pub async fn get_member_preferences(
    State(state): State<AppState>,
    auth: crate::extractors::AuthUser,
) -> AppResult<Json<MemberPreferencesResponse>> {
    let rows = preferences::get_preferences(&state.db, auth.user_id).await?;
    Ok(Json(MemberPreferencesResponse { preferences: rows }))
}

#[utoipa::path(
    put,
    path = "/api/member/notification-preferences",
    tag = "member",
    security(("bearer_auth" = [])),
    request_body = BulkPreferenceUpdate,
    responses(
        (status = 200, description = "Updated preferences", body = MemberPreferencesResponse),
        (status = 400, description = "Invalid payload"),
        (status = 401, description = "Unauthorized")
    )
)]
pub async fn update_member_preferences(
    State(state): State<AppState>,
    auth: crate::extractors::AuthUser,
    Json(body): Json<BulkPreferenceUpdate>,
) -> AppResult<Json<MemberPreferencesResponse>> {
    for item in &body.items {
        if item.category.trim().is_empty() {
            return Err(AppError::BadRequest(
                "category required for each item".into(),
            ));
        }
        if !is_allowed_channel(&item.channel) {
            return Err(AppError::BadRequest(format!(
                "channel `{}` is not allowed",
                item.channel
            )));
        }
    }

    let mut tx = state.db.begin().await?;
    for item in &body.items {
        preferences::set_preference(&mut *tx, auth.user_id, item).await?;
    }
    tx.commit().await?;

    let rows = preferences::get_preferences(&state.db, auth.user_id).await?;
    Ok(Json(MemberPreferencesResponse { preferences: rows }))
}

// ── Public unsubscribe ──────────────────────────────────────────────────

#[derive(Debug, Deserialize, IntoParams, ToSchema)]
pub struct UnsubscribeQuery {
    pub token: String,
}

/// Minimal HTML rendered for the success + failure states. Intentionally
/// inline to avoid wiring a whole template engine for a one-off page.
fn render_page(title: &str, message: &str) -> String {
    format!(
        r#"<!DOCTYPE html><html lang="en"><head><meta charset="UTF-8"><title>{title}</title>
<meta name="viewport" content="width=device-width, initial-scale=1.0">
<style>body{{margin:0;padding:40px 16px;background:#0a0f1c;color:#e2e8f0;font-family:-apple-system,BlinkMacSystemFont,Segoe UI,sans-serif;}}
.card{{max-width:480px;margin:0 auto;background:#111827;border:1px solid #1e293b;border-radius:12px;padding:32px;text-align:center;}}
h1{{color:#f1f5f9;font-size:22px;margin:0 0 12px 0;}}
p{{color:#94a3b8;line-height:1.6;margin:0;}}</style></head>
<body><div class="card"><h1>{title}</h1><p>{message}</p></div></body></html>"#
    )
}

pub async fn unsubscribe_page(
    State(state): State<AppState>,
    Query(q): Query<UnsubscribeQuery>,
) -> Response {
    match unsubscribe::consume_token(&state.db, &q.token).await {
        Ok(UnsubscribeAction::CategoryDisabled { category, .. }) => Html(render_page(
            "You are unsubscribed",
            &format!("You will no longer receive <strong>{category}</strong> emails."),
        ))
        .into_response(),
        Ok(UnsubscribeAction::AllMarketing { .. }) => Html(render_page(
            "You are unsubscribed",
            "You will no longer receive marketing emails from us.",
        ))
        .into_response(),
        Err(UnsubscribeError::AlreadyUsed) => {
            let mut resp = Html(render_page(
                "Already unsubscribed",
                "This link has already been used.",
            ))
            .into_response();
            *resp.status_mut() = StatusCode::OK;
            resp.headers_mut().insert(
                header::CONTENT_TYPE,
                axum::http::HeaderValue::from_static("text/html; charset=utf-8"),
            );
            resp
        }
        Err(UnsubscribeError::Expired | UnsubscribeError::NotFound) => {
            let mut resp = Html(render_page(
                "Invalid link",
                "This unsubscribe link is no longer valid. Update your preferences from your account dashboard.",
            ))
            .into_response();
            *resp.status_mut() = StatusCode::BAD_REQUEST;
            resp
        }
        Err(err) => {
            tracing::error!(error = %err, "unsubscribe token consume failed");
            let mut resp = Html(render_page(
                "Something went wrong",
                "We could not process this request. Please try again later.",
            ))
            .into_response();
            *resp.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
            resp
        }
    }
}

// ── Helpers ─────────────────────────────────────────────────────────────

fn is_allowed_channel(name: &str) -> bool {
    matches!(
        name,
        "email" | "sms" | "push" | "in_app" | "slack" | "discord" | "webhook"
    )
}

/// Non-test helper used by other crate modules + integration tests that need
/// to send directly to a concrete [`Recipient`]. Public so the handler can
/// stay slim.
#[allow(dead_code)]
pub(crate) fn recipient_for_user(user_id: Uuid, email: String) -> Recipient {
    Recipient::User { user_id, email }
}
