use axum::{
    extract::{Path, Query, State},
    routing::{delete, get, post, put},
    Json, Router,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

use crate::{
    error::{AppError, AppResult},
    extractors::{AdminUser, OptionalAuthUser},
    models::*,
    AppState,
};

// ══════════════════════════════════════════════════════════════════════
// ROUTERS
// ══════════════════════════════════════════════════════════════════════

pub fn admin_router() -> Router<AppState> {
    Router::new()
        .route("/popups", get(admin_list_popups))
        .route("/popups", post(admin_create_popup))
        .route("/popups/{id}", get(admin_get_popup))
        .route("/popups/{id}", put(admin_update_popup))
        .route("/popups/{id}", delete(admin_delete_popup))
        .route("/popups/{id}/toggle", post(admin_toggle_popup))
        .route("/popups/{id}/duplicate", post(admin_duplicate_popup))
        .route("/popups/{id}/submissions", get(admin_list_submissions))
        .route("/popups/{id}/analytics", get(admin_get_analytics))
}

pub fn public_router() -> Router<AppState> {
    Router::new()
        .route("/popups/active", get(public_active_popups))
        .route("/popups/event", post(public_track_event))
        .route("/popups/submit", post(public_submit_form))
}

// ══════════════════════════════════════════════════════════════════════
// REQUEST / RESPONSE TYPES
// ══════════════════════════════════════════════════════════════════════

#[derive(Debug, Deserialize)]
pub struct ActivePopupsQuery {
    pub page: Option<String>,
    pub device: Option<String>,
    pub user_status: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct TrackEventRequest {
    pub popup_id: Uuid,
    pub event_type: String,
    pub session_id: Option<Uuid>,
}

#[derive(Debug, Serialize)]
pub struct PopupDetailResponse {
    #[serde(flatten)]
    pub popup: Popup,
    pub analytics: PopupAnalytics,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct AnalyticsTimeBucket {
    pub bucket: DateTime<Utc>,
    pub impressions: i64,
    pub closes: i64,
    pub submits: i64,
}

#[derive(Debug, Deserialize)]
pub struct AnalyticsQuery {
    pub from: Option<DateTime<Utc>>,
    pub to: Option<DateTime<Utc>>,
    pub granularity: Option<String>,
}

// ══════════════════════════════════════════════════════════════════════
// ADMIN HANDLERS
// ══════════════════════════════════════════════════════════════════════

async fn admin_list_popups(
    State(state): State<AppState>,
    _admin: AdminUser,
    Query(params): Query<PaginationParams>,
) -> AppResult<Json<PaginatedResponse<Popup>>> {
    let per_page = params.per_page();
    let offset = params.offset();
    let page = params.page.unwrap_or(1).max(1);

    let popups = sqlx::query_as::<_, Popup>(
        r#"
        SELECT id, name, popup_type, trigger_type, trigger_config, content_json,
               style_json, targeting_rules, display_frequency, frequency_config,
               success_message, redirect_url, is_active, starts_at, expires_at,
               priority, created_by, created_at, updated_at
        FROM popups
        ORDER BY created_at DESC
        LIMIT $1 OFFSET $2
        "#,
    )
    .bind(per_page)
    .bind(offset)
    .fetch_all(&state.db)
    .await?;

    let total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM popups")
        .fetch_one(&state.db)
        .await?;

    let total_pages = (total as f64 / per_page as f64).ceil() as i64;

    Ok(Json(PaginatedResponse {
        data: popups,
        total,
        page,
        per_page,
        total_pages,
    }))
}

async fn admin_create_popup(
    State(state): State<AppState>,
    admin: AdminUser,
    Json(req): Json<CreatePopupRequest>,
) -> AppResult<Json<Popup>> {
    req.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;

    let popup = sqlx::query_as::<_, Popup>(
        r#"
        INSERT INTO popups (
            name, popup_type, trigger_type, trigger_config, content_json,
            style_json, targeting_rules, display_frequency, frequency_config,
            success_message, redirect_url, is_active, starts_at, expires_at,
            priority, created_by
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16)
        RETURNING id, name, popup_type, trigger_type, trigger_config, content_json,
                  style_json, targeting_rules, display_frequency, frequency_config,
                  success_message, redirect_url, is_active, starts_at, expires_at,
                  priority, created_by, created_at, updated_at
        "#,
    )
    .bind(&req.name)
    .bind(req.popup_type.as_deref().unwrap_or("modal"))
    .bind(req.trigger_type.as_deref().unwrap_or("time_delay"))
    .bind(
        req.trigger_config
            .as_ref()
            .unwrap_or(&serde_json::json!({"delay_ms": 3000})),
    )
    .bind(
        req.content_json
            .as_ref()
            .unwrap_or(&serde_json::json!({"elements": []})),
    )
    .bind(req.style_json.as_ref().unwrap_or(&serde_json::json!({
        "background": "#1a1a2e",
        "textColor": "#ffffff",
        "accentColor": "#0fa4af",
        "borderRadius": "16px",
        "maxWidth": "480px",
        "animation": "fade",
        "backdrop": true,
        "backdropColor": "rgba(0,0,0,0.6)"
    })))
    .bind(req.targeting_rules.as_ref().unwrap_or(&serde_json::json!({
        "pages": ["*"],
        "devices": ["desktop", "mobile", "tablet"],
        "userStatus": ["all"]
    })))
    .bind(
        req.display_frequency
            .as_deref()
            .unwrap_or("once_per_session"),
    )
    .bind(
        req.frequency_config
            .as_ref()
            .unwrap_or(&serde_json::json!({})),
    )
    .bind(&req.success_message)
    .bind(&req.redirect_url)
    .bind(req.is_active.unwrap_or(false))
    .bind(req.starts_at)
    .bind(req.expires_at)
    .bind(req.priority.unwrap_or(0))
    .bind(admin.user_id)
    .fetch_one(&state.db)
    .await?;

    Ok(Json(popup))
}

async fn admin_get_popup(
    State(state): State<AppState>,
    _admin: AdminUser,
    Path(id): Path<Uuid>,
) -> AppResult<Json<PopupDetailResponse>> {
    let popup = sqlx::query_as::<_, Popup>(
        r#"
        SELECT id, name, popup_type, trigger_type, trigger_config, content_json,
               style_json, targeting_rules, display_frequency, frequency_config,
               success_message, redirect_url, is_active, starts_at, expires_at,
               priority, created_by, created_at, updated_at
        FROM popups
        WHERE id = $1
        "#,
    )
    .bind(id)
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NotFound("Popup not found".to_string()))?;

    let analytics = build_popup_analytics(&state.db, &popup).await?;

    Ok(Json(PopupDetailResponse { popup, analytics }))
}

async fn admin_update_popup(
    State(state): State<AppState>,
    _admin: AdminUser,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdatePopupRequest>,
) -> AppResult<Json<Popup>> {
    // Ensure popup exists
    let existing = sqlx::query_as::<_, Popup>("SELECT * FROM popups WHERE id = $1")
        .bind(id)
        .fetch_optional(&state.db)
        .await?
        .ok_or(AppError::NotFound("Popup not found".to_string()))?;

    let popup = sqlx::query_as::<_, Popup>(
        r#"
        UPDATE popups SET
            name = COALESCE($1, name),
            popup_type = COALESCE($2, popup_type),
            trigger_type = COALESCE($3, trigger_type),
            trigger_config = COALESCE($4, trigger_config),
            content_json = COALESCE($5, content_json),
            style_json = COALESCE($6, style_json),
            targeting_rules = COALESCE($7, targeting_rules),
            display_frequency = COALESCE($8, display_frequency),
            frequency_config = COALESCE($9, frequency_config),
            success_message = COALESCE($10, success_message),
            redirect_url = COALESCE($11, redirect_url),
            is_active = COALESCE($12, is_active),
            starts_at = COALESCE($13, starts_at),
            expires_at = COALESCE($14, expires_at),
            priority = COALESCE($15, priority),
            updated_at = NOW()
        WHERE id = $16
        RETURNING id, name, popup_type, trigger_type, trigger_config, content_json,
                  style_json, targeting_rules, display_frequency, frequency_config,
                  success_message, redirect_url, is_active, starts_at, expires_at,
                  priority, created_by, created_at, updated_at
        "#,
    )
    .bind(&req.name)
    .bind(&req.popup_type)
    .bind(&req.trigger_type)
    .bind(&req.trigger_config)
    .bind(&req.content_json)
    .bind(&req.style_json)
    .bind(&req.targeting_rules)
    .bind(&req.display_frequency)
    .bind(&req.frequency_config)
    .bind(&req.success_message)
    .bind(&req.redirect_url)
    .bind(req.is_active)
    .bind(req.starts_at)
    .bind(req.expires_at)
    .bind(req.priority)
    .bind(id)
    .fetch_one(&state.db)
    .await?;

    // Suppress unused variable warning — we read existing to confirm it exists
    let _ = existing;

    Ok(Json(popup))
}

async fn admin_delete_popup(
    State(state): State<AppState>,
    _admin: AdminUser,
    Path(id): Path<Uuid>,
) -> AppResult<Json<serde_json::Value>> {
    let result = sqlx::query("DELETE FROM popups WHERE id = $1")
        .bind(id)
        .execute(&state.db)
        .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound("Popup not found".to_string()));
    }

    Ok(Json(serde_json::json!({ "message": "Popup deleted" })))
}

async fn admin_toggle_popup(
    State(state): State<AppState>,
    _admin: AdminUser,
    Path(id): Path<Uuid>,
) -> AppResult<Json<Popup>> {
    let popup = sqlx::query_as::<_, Popup>(
        r#"
        UPDATE popups SET
            is_active = NOT is_active,
            updated_at = NOW()
        WHERE id = $1
        RETURNING id, name, popup_type, trigger_type, trigger_config, content_json,
                  style_json, targeting_rules, display_frequency, frequency_config,
                  success_message, redirect_url, is_active, starts_at, expires_at,
                  priority, created_by, created_at, updated_at
        "#,
    )
    .bind(id)
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NotFound("Popup not found".to_string()))?;

    Ok(Json(popup))
}

async fn admin_duplicate_popup(
    State(state): State<AppState>,
    admin: AdminUser,
    Path(id): Path<Uuid>,
) -> AppResult<Json<Popup>> {
    let source = sqlx::query_as::<_, Popup>("SELECT * FROM popups WHERE id = $1")
        .bind(id)
        .fetch_optional(&state.db)
        .await?
        .ok_or(AppError::NotFound("Popup not found".to_string()))?;

    let new_name = format!("{} (Copy)", source.name);

    let popup = sqlx::query_as::<_, Popup>(
        r#"
        INSERT INTO popups (
            name, popup_type, trigger_type, trigger_config, content_json,
            style_json, targeting_rules, display_frequency, frequency_config,
            success_message, redirect_url, is_active, starts_at, expires_at,
            priority, created_by
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, FALSE, $12, $13, $14, $15)
        RETURNING id, name, popup_type, trigger_type, trigger_config, content_json,
                  style_json, targeting_rules, display_frequency, frequency_config,
                  success_message, redirect_url, is_active, starts_at, expires_at,
                  priority, created_by, created_at, updated_at
        "#,
    )
    .bind(&new_name)
    .bind(&source.popup_type)
    .bind(&source.trigger_type)
    .bind(&source.trigger_config)
    .bind(&source.content_json)
    .bind(&source.style_json)
    .bind(&source.targeting_rules)
    .bind(&source.display_frequency)
    .bind(&source.frequency_config)
    .bind(&source.success_message)
    .bind(&source.redirect_url)
    .bind(source.starts_at)
    .bind(source.expires_at)
    .bind(source.priority)
    .bind(admin.user_id)
    .fetch_one(&state.db)
    .await?;

    Ok(Json(popup))
}

async fn admin_list_submissions(
    State(state): State<AppState>,
    _admin: AdminUser,
    Path(id): Path<Uuid>,
    Query(params): Query<PaginationParams>,
) -> AppResult<Json<PaginatedResponse<PopupSubmission>>> {
    let per_page = params.per_page();
    let offset = params.offset();
    let page = params.page.unwrap_or(1).max(1);

    // Verify popup exists
    let exists: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM popups WHERE id = $1)")
        .bind(id)
        .fetch_one(&state.db)
        .await?;

    if !exists {
        return Err(AppError::NotFound("Popup not found".to_string()));
    }

    let submissions = sqlx::query_as::<_, PopupSubmission>(
        r#"
        SELECT id, popup_id, user_id, session_id, form_data, ip_address,
               user_agent, page_url, submitted_at
        FROM popup_submissions
        WHERE popup_id = $1
        ORDER BY submitted_at DESC
        LIMIT $2 OFFSET $3
        "#,
    )
    .bind(id)
    .bind(per_page)
    .bind(offset)
    .fetch_all(&state.db)
    .await?;

    let total: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM popup_submissions WHERE popup_id = $1")
            .bind(id)
            .fetch_one(&state.db)
            .await?;

    let total_pages = (total as f64 / per_page as f64).ceil() as i64;

    Ok(Json(PaginatedResponse {
        data: submissions,
        total,
        page,
        per_page,
        total_pages,
    }))
}

async fn admin_get_analytics(
    State(state): State<AppState>,
    _admin: AdminUser,
    Path(id): Path<Uuid>,
    Query(query): Query<AnalyticsQuery>,
) -> AppResult<Json<serde_json::Value>> {
    // Verify popup exists
    let popup = sqlx::query_as::<_, Popup>("SELECT * FROM popups WHERE id = $1")
        .bind(id)
        .fetch_optional(&state.db)
        .await?
        .ok_or(AppError::NotFound("Popup not found".to_string()))?;

    let summary = build_popup_analytics(&state.db, &popup).await?;

    let from = query
        .from
        .unwrap_or_else(|| Utc::now() - chrono::Duration::days(30));
    let to = query.to.unwrap_or_else(Utc::now);
    let granularity = query.granularity.as_deref().unwrap_or("day");

    let time_series = sqlx::query_as::<_, AnalyticsTimeBucket>(
        r#"
        SELECT
            time_bucket AS bucket,
            COALESCE(SUM(CASE WHEN event_type = 'impression' THEN 1 ELSE 0 END), 0) AS impressions,
            COALESCE(SUM(CASE WHEN event_type = 'close' THEN 1 ELSE 0 END), 0) AS closes,
            COALESCE(SUM(CASE WHEN event_type = 'submit' THEN 1 ELSE 0 END), 0) AS submits
        FROM (
            SELECT
                date_trunc($1, created_at) AS time_bucket,
                event_type
            FROM popup_events
            WHERE popup_id = $2
              AND created_at >= $3
              AND created_at <= $4
        ) sub
        GROUP BY time_bucket
        ORDER BY time_bucket ASC
        "#,
    )
    .bind(granularity_to_trunc(granularity))
    .bind(id)
    .bind(from)
    .bind(to)
    .fetch_all(&state.db)
    .await?;

    Ok(Json(serde_json::json!({
        "summary": summary,
        "time_series": time_series,
        "from": from,
        "to": to,
        "granularity": granularity,
    })))
}

// ══════════════════════════════════════════════════════════════════════
// PUBLIC HANDLERS
// ══════════════════════════════════════════════════════════════════════

async fn public_active_popups(
    State(state): State<AppState>,
    Query(query): Query<ActivePopupsQuery>,
) -> AppResult<Json<Vec<Popup>>> {
    // Fetch all active popups within their date window
    let popups = sqlx::query_as::<_, Popup>(
        r#"
        SELECT id, name, popup_type, trigger_type, trigger_config, content_json,
               style_json, targeting_rules, display_frequency, frequency_config,
               success_message, redirect_url, is_active, starts_at, expires_at,
               priority, created_by, created_at, updated_at
        FROM popups
        WHERE is_active = TRUE
          AND (starts_at IS NULL OR starts_at <= NOW())
          AND (expires_at IS NULL OR expires_at >= NOW())
        ORDER BY priority DESC, created_at DESC
        "#,
    )
    .fetch_all(&state.db)
    .await?;

    let page_path = query.page.as_deref().unwrap_or("*");
    let device = query.device.as_deref().unwrap_or("desktop");
    let user_status = query.user_status.as_deref().unwrap_or("all");

    let filtered: Vec<Popup> = popups
        .into_iter()
        .filter(|popup| {
            matches_targeting_rules(&popup.targeting_rules, page_path, device, user_status)
        })
        .collect();

    Ok(Json(filtered))
}

async fn public_track_event(
    State(state): State<AppState>,
    Json(req): Json<TrackEventRequest>,
) -> AppResult<Json<serde_json::Value>> {
    let event_type = req.event_type.to_lowercase();
    if !["impression", "close", "submit", "click"].contains(&event_type.as_str()) {
        return Err(AppError::BadRequest(
            "event_type must be one of: impression, close, submit, click".to_string(),
        ));
    }

    // Verify popup exists
    let exists: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM popups WHERE id = $1)")
        .bind(req.popup_id)
        .fetch_one(&state.db)
        .await?;

    if !exists {
        return Err(AppError::NotFound("Popup not found".to_string()));
    }

    sqlx::query(
        r#"
        INSERT INTO popup_events (popup_id, event_type, session_id)
        VALUES ($1, $2, $3)
        "#,
    )
    .bind(req.popup_id)
    .bind(&event_type)
    .bind(req.session_id)
    .execute(&state.db)
    .await?;

    Ok(Json(serde_json::json!({ "ok": true })))
}

async fn public_submit_form(
    State(state): State<AppState>,
    opt: OptionalAuthUser,
    Json(req): Json<PopupSubmitRequest>,
) -> AppResult<Json<PopupSubmission>> {
    // Verify popup exists
    let exists: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM popups WHERE id = $1)")
        .bind(req.popup_id)
        .fetch_one(&state.db)
        .await?;

    if !exists {
        return Err(AppError::NotFound("Popup not found".to_string()));
    }

    let submission = sqlx::query_as::<_, PopupSubmission>(
        r#"
        INSERT INTO popup_submissions (popup_id, user_id, session_id, form_data, page_url)
        VALUES ($1, $2, $3, $4, $5)
        RETURNING id, popup_id, user_id, session_id, form_data, ip_address,
                  user_agent, page_url, submitted_at
        "#,
    )
    .bind(req.popup_id)
    .bind(opt.user_id)
    .bind(req.session_id)
    .bind(&req.form_data)
    .bind(&req.page_url)
    .fetch_one(&state.db)
    .await?;

    // Also record a submit event
    sqlx::query(
        r#"
        INSERT INTO popup_events (popup_id, event_type, session_id)
        VALUES ($1, 'submit', $2)
        "#,
    )
    .bind(req.popup_id)
    .bind(req.session_id)
    .execute(&state.db)
    .await?;

    Ok(Json(submission))
}

// ══════════════════════════════════════════════════════════════════════
// HELPERS
// ══════════════════════════════════════════════════════════════════════

async fn build_popup_analytics(pool: &sqlx::PgPool, popup: &Popup) -> AppResult<PopupAnalytics> {
    let impressions: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM popup_events WHERE popup_id = $1 AND event_type = 'impression'",
    )
    .bind(popup.id)
    .fetch_one(pool)
    .await?;

    let closes: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM popup_events WHERE popup_id = $1 AND event_type = 'close'",
    )
    .bind(popup.id)
    .fetch_one(pool)
    .await?;

    let submissions: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM popup_events WHERE popup_id = $1 AND event_type = 'submit'",
    )
    .bind(popup.id)
    .fetch_one(pool)
    .await?;

    let conversion_rate = if impressions > 0 {
        (submissions as f64 / impressions as f64) * 100.0
    } else {
        0.0
    };

    Ok(PopupAnalytics {
        popup_id: popup.id,
        popup_name: popup.name.clone(),
        total_impressions: impressions,
        total_closes: closes,
        total_submissions: submissions,
        conversion_rate,
    })
}

/// Match a popup's targeting_rules JSON against the requested page, device, and user status.
fn matches_targeting_rules(
    rules: &serde_json::Value,
    page_path: &str,
    device: &str,
    user_status: &str,
) -> bool {
    // Check page patterns
    if let Some(pages) = rules.get("pages").and_then(|v| v.as_array()) {
        let page_match = pages.iter().any(|p| {
            if let Some(pattern) = p.as_str() {
                matches_page_pattern(pattern, page_path)
            } else {
                false
            }
        });
        if !page_match {
            return false;
        }
    }

    // Check device targeting
    if let Some(devices) = rules.get("devices").and_then(|v| v.as_array()) {
        let device_match = devices.iter().any(|d| {
            d.as_str()
                .map(|s| s.eq_ignore_ascii_case(device))
                .unwrap_or(false)
        });
        if !device_match {
            return false;
        }
    }

    // Check user status targeting
    if let Some(statuses) = rules.get("userStatus").and_then(|v| v.as_array()) {
        let status_match = statuses.iter().any(|s| {
            if let Some(st) = s.as_str() {
                st == "all" || st.eq_ignore_ascii_case(user_status)
            } else {
                false
            }
        });
        if !status_match {
            return false;
        }
    }

    true
}

/// Simple glob-like page pattern matching.
/// Supports:
///   "*"         -> matches everything
///   "/blog/*"   -> matches /blog/ and anything under it
///   "/pricing"  -> exact match
fn matches_page_pattern(pattern: &str, path: &str) -> bool {
    if pattern == "*" {
        return true;
    }

    if pattern.ends_with('*') {
        let prefix = &pattern[..pattern.len() - 1];
        return path.starts_with(prefix);
    }

    pattern == path
}

/// Convert user-facing granularity to a Postgres date_trunc argument.
fn granularity_to_trunc(granularity: &str) -> &str {
    match granularity {
        "hour" => "hour",
        "day" => "day",
        "week" => "week",
        "month" => "month",
        _ => "day",
    }
}
