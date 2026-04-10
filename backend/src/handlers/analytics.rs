use axum::{extract::State, routing::post, Json, Router};

use crate::{
    db,
    error::{AppError, AppResult},
    extractors::OptionalAuthUser,
    models::AnalyticsIngestRequest,
    AppState,
};

pub fn router() -> Router<AppState> {
    Router::new().route("/events", post(ingest_events))
}

const MAX_EVENTS_PER_REQUEST: usize = 64;
const MAX_PATH_LEN: usize = 2048;

/// Public ingest for SPA analytics (optional Bearer links session to logged-in user).
async fn ingest_events(
    State(state): State<AppState>,
    opt: OptionalAuthUser,
    Json(req): Json<AnalyticsIngestRequest>,
) -> AppResult<Json<serde_json::Value>> {
    if req.events.is_empty() {
        return Err(AppError::BadRequest("events cannot be empty".to_string()));
    }
    if req.events.len() > MAX_EVENTS_PER_REQUEST {
        return Err(AppError::BadRequest(format!(
            "at most {MAX_EVENTS_PER_REQUEST} events per request"
        )));
    }

    let mut batch: Vec<(String, String, Option<String>, serde_json::Value)> =
        Vec::with_capacity(req.events.len());

    for ev in req.events {
        let et = ev.event_type.to_lowercase();
        if et != "page_view" && et != "impression" && et != "click" {
            return Err(AppError::BadRequest(
                "event_type must be page_view, impression, or click".to_string(),
            ));
        }
        let path = ev.path.trim().to_string();
        if path.is_empty() {
            return Err(AppError::BadRequest("path is required".to_string()));
        }
        if path.len() > MAX_PATH_LEN {
            return Err(AppError::BadRequest("path too long".to_string()));
        }
        let referrer = ev
            .referrer
            .map(|r| r.trim().to_string())
            .filter(|r| !r.is_empty());

        let metadata = if ev.metadata.is_null() {
            serde_json::json!({})
        } else {
            ev.metadata
        };

        batch.push((et, path, referrer, metadata));
    }

    db::ingest_analytics_events(&state.db, req.session_id, opt.user_id, batch).await?;

    Ok(Json(serde_json::json!({ "ok": true })))
}
