//! EC-02 public catalog search + category tree HTTP handlers.
//!
//! Two endpoints:
//!
//! * `GET /api/catalog/search?q=&category=&tag=&min_price=&max_price=&page=&per_page=`
//!   — full-text search, filters, pagination. Responds with
//!   `{products, total, facets, page, per_page}`.
//! * `GET /api/catalog/categories` — the full category tree (flat, with
//!   `parent_id` for client-side tree rendering).
//!
//! Both are public; admin mutation of categories arrives in a follow-up
//! ticket.

use axum::{
    extract::{Path, Query, State},
    http::{header, HeaderMap, HeaderValue},
    response::IntoResponse,
    routing::get,
    Json, Router,
};
use serde::Deserialize;

use crate::commerce::catalog::{
    self, Category, SearchParams, SearchResponse,
};
use crate::error::AppResult;
use crate::AppState;

pub fn public_router() -> Router<AppState> {
    Router::new()
        .route("/search", get(search))
        .route("/categories", get(list_categories))
        .route("/categories/{slug}", get(category_by_slug))
}

/// Query params accepted on `/api/catalog/search`. The `category` alias
/// maps to `category_id` on the service layer — the handler form-level
/// name is shorter on the URL.
#[derive(Debug, Deserialize)]
pub struct SearchQuery {
    #[serde(default)]
    pub q: Option<String>,
    #[serde(default, rename = "category")]
    pub category_id: Option<uuid::Uuid>,
    #[serde(default)]
    pub tag: Option<String>,
    #[serde(default, rename = "min_price")]
    pub min_price_cents: Option<i64>,
    #[serde(default, rename = "max_price")]
    pub max_price_cents: Option<i64>,
    #[serde(default)]
    pub page: Option<i64>,
    #[serde(default)]
    pub per_page: Option<i64>,
}

impl From<SearchQuery> for SearchParams {
    fn from(q: SearchQuery) -> Self {
        SearchParams {
            q: q.q,
            category_id: q.category_id,
            tag: q.tag,
            min_price_cents: q.min_price_cents,
            max_price_cents: q.max_price_cents,
            page: q.page,
            per_page: q.per_page,
        }
    }
}

async fn search(
    State(state): State<AppState>,
    Query(params): Query<SearchQuery>,
) -> AppResult<impl IntoResponse> {
    let resp: SearchResponse = catalog::search_products(&state.db, &params.into()).await?;
    let mut headers = HeaderMap::new();
    headers.insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static("public, max-age=30"),
    );
    Ok((headers, Json(resp)))
}

async fn list_categories(State(state): State<AppState>) -> AppResult<impl IntoResponse> {
    let categories: Vec<Category> = catalog::list_categories(&state.db).await?;
    let mut headers = HeaderMap::new();
    headers.insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static("public, max-age=300"),
    );
    Ok((headers, Json(categories)))
}

async fn category_by_slug(
    State(state): State<AppState>,
    Path(slug): Path<String>,
) -> AppResult<impl IntoResponse> {
    match catalog::get_category_by_slug(&state.db, &slug).await? {
        Some(c) => Ok(Json(c)),
        None => Err(crate::error::AppError::NotFound(
            "Category not found".into(),
        )),
    }
}
