//! EC-03: Public cart handlers.
//!
//! Endpoints:
//!   * `GET /api/cart`                  → current cart + totals.
//!   * `POST /api/cart/items`           → add an item.
//!   * `PUT /api/cart/items/{item_id}`  → update quantity.
//!   * `DELETE /api/cart/items/{item_id}` → remove an item.
//!   * `DELETE /api/cart`               → clear.
//!   * `POST /api/cart/merge`           → move anonymous → authed cart at login.
//!
//! Auth is optional — the anonymous caller passes an `X-Anonymous-Id` UUID
//! header, the authenticated caller rides their bearer token. The response
//! always carries `Cart` + `CartTotals` so the UI never needs a second round
//! trip.

use axum::{
    extract::{Path, State},
    http::HeaderMap,
    routing::{delete, get, post, put},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{
    commerce::{
        cart::{self, Cart, CartIdentity, CartItem, CartTotals},
        repo as product_repo,
    },
    error::{AppError, AppResult},
    extractors::{AuthUser, OptionalAuthUser},
    AppState,
};

// ── Router ─────────────────────────────────────────────────────────────

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(get_cart))
        .route("/", delete(clear_cart))
        .route("/items", post(add_item))
        .route("/items/{item_id}", put(update_item))
        .route("/items/{item_id}", delete(remove_item))
        .route("/merge", post(merge_cart))
}

// ── DTOs ───────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, ToSchema)]
pub struct CartResponse {
    pub cart: Cart,
    pub items: Vec<CartItem>,
    pub totals: CartTotals,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct AddItemRequest {
    pub product_id: Uuid,
    pub variant_id: Option<Uuid>,
    pub quantity: Option<i32>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateItemRequest {
    pub quantity: i32,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct MergeCartRequest {
    pub anonymous_id: Uuid,
}

// ── Identity resolution ────────────────────────────────────────────────

/// Resolve the caller's cart identity. Prefers the bearer token; falls back
/// to the `X-Anonymous-Id` header. Returns 400 if neither is provided so
/// the frontend is forced to set + persist a client-side UUID on first use.
const ANON_HEADER: &str = "X-Anonymous-Id";

fn resolve_identity(auth: &OptionalAuthUser, headers: &HeaderMap) -> AppResult<CartIdentity> {
    if let Some(uid) = auth.user_id {
        return Ok(CartIdentity::Subject(uid));
    }
    let raw = headers
        .get(ANON_HEADER)
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| {
            AppError::BadRequest(format!(
                "Missing {ANON_HEADER} header (or Authorization bearer token)"
            ))
        })?;
    let aid = Uuid::parse_str(raw)
        .map_err(|_| AppError::BadRequest(format!("{ANON_HEADER} is not a valid UUID")))?;
    Ok(CartIdentity::Anonymous(aid))
}

// ── Handlers ───────────────────────────────────────────────────────────

#[utoipa::path(
    get,
    path = "/api/cart",
    tag = "cart",
    responses((status = 200, description = "Current cart", body = CartResponse))
)]
pub(crate) async fn get_cart(
    State(state): State<AppState>,
    auth: OptionalAuthUser,
    headers: HeaderMap,
) -> AppResult<Json<CartResponse>> {
    let identity = resolve_identity(&auth, &headers)?;
    let cart_row = cart::get_or_create_cart(&state.db, identity).await?;
    let items = cart::list_items(&state.db, cart_row.id).await?;
    let totals = cart::compute_totals(&items, None);
    Ok(Json(CartResponse {
        cart: cart_row,
        items,
        totals,
    }))
}

#[utoipa::path(
    post,
    path = "/api/cart/items",
    tag = "cart",
    request_body = AddItemRequest,
    responses(
        (status = 200, description = "Item added", body = CartResponse),
        (status = 404, description = "Product or variant not found")
    )
)]
pub(crate) async fn add_item(
    State(state): State<AppState>,
    auth: OptionalAuthUser,
    headers: HeaderMap,
    Json(req): Json<AddItemRequest>,
) -> AppResult<Json<CartResponse>> {
    let identity = resolve_identity(&auth, &headers)?;
    let cart_row = cart::get_or_create_cart(&state.db, identity).await?;
    let qty = req.quantity.unwrap_or(1);
    if qty <= 0 {
        return Err(AppError::BadRequest("quantity must be positive".into()));
    }

    // Resolve the unit price at add-time so later price edits don't
    // retroactively change a visitor's open cart. Variant price overrides
    // product price when set.
    let product =
        product_repo::get_product(&state.db, product_repo::ProductLookup::Id(req.product_id))
            .await?;
    let unit_price = if let Some(var_id) = req.variant_id {
        let variants = product_repo::list_variants(&state.db, product.id).await?;
        let variant = variants
            .into_iter()
            .find(|v| v.id == var_id)
            .ok_or_else(|| AppError::NotFound("Variant not found".to_string()))?;
        variant.price_cents.or(product.price_cents).unwrap_or(0)
    } else {
        product.price_cents.unwrap_or(0)
    };

    cart::add_item(
        &state.db,
        cart_row.id,
        req.product_id,
        req.variant_id,
        qty,
        unit_price,
    )
    .await?;

    let items = cart::list_items(&state.db, cart_row.id).await?;
    let totals = cart::compute_totals(&items, None);
    Ok(Json(CartResponse {
        cart: cart_row,
        items,
        totals,
    }))
}

#[utoipa::path(
    put,
    path = "/api/cart/items/{item_id}",
    tag = "cart",
    params(("item_id" = Uuid, Path, description = "Cart item id")),
    request_body = UpdateItemRequest,
    responses(
        (status = 200, description = "Updated", body = CartResponse),
        (status = 404, description = "Item not found")
    )
)]
pub(crate) async fn update_item(
    State(state): State<AppState>,
    auth: OptionalAuthUser,
    headers: HeaderMap,
    Path(item_id): Path<Uuid>,
    Json(req): Json<UpdateItemRequest>,
) -> AppResult<Json<CartResponse>> {
    let identity = resolve_identity(&auth, &headers)?;
    let cart_row = cart::get_or_create_cart(&state.db, identity).await?;
    cart::update_item_qty(&state.db, cart_row.id, item_id, req.quantity)
        .await?
        .ok_or_else(|| AppError::NotFound("Cart item not found".to_string()))?;
    let items = cart::list_items(&state.db, cart_row.id).await?;
    let totals = cart::compute_totals(&items, None);
    Ok(Json(CartResponse {
        cart: cart_row,
        items,
        totals,
    }))
}

#[utoipa::path(
    delete,
    path = "/api/cart/items/{item_id}",
    tag = "cart",
    params(("item_id" = Uuid, Path, description = "Cart item id")),
    responses((status = 200, description = "Removed", body = CartResponse))
)]
pub(crate) async fn remove_item(
    State(state): State<AppState>,
    auth: OptionalAuthUser,
    headers: HeaderMap,
    Path(item_id): Path<Uuid>,
) -> AppResult<Json<CartResponse>> {
    let identity = resolve_identity(&auth, &headers)?;
    let cart_row = cart::get_or_create_cart(&state.db, identity).await?;
    cart::remove_item(&state.db, cart_row.id, item_id).await?;
    let items = cart::list_items(&state.db, cart_row.id).await?;
    let totals = cart::compute_totals(&items, None);
    Ok(Json(CartResponse {
        cart: cart_row,
        items,
        totals,
    }))
}

#[utoipa::path(
    delete,
    path = "/api/cart",
    tag = "cart",
    responses((status = 200, description = "Cleared", body = CartResponse))
)]
pub(crate) async fn clear_cart(
    State(state): State<AppState>,
    auth: OptionalAuthUser,
    headers: HeaderMap,
) -> AppResult<Json<CartResponse>> {
    let identity = resolve_identity(&auth, &headers)?;
    let cart_row = cart::get_or_create_cart(&state.db, identity).await?;
    cart::clear_cart(&state.db, cart_row.id).await?;
    let items: Vec<CartItem> = Vec::new();
    let totals = cart::compute_totals(&items, None);
    Ok(Json(CartResponse {
        cart: cart_row,
        items,
        totals,
    }))
}

#[utoipa::path(
    post,
    path = "/api/cart/merge",
    tag = "cart",
    security(("bearer_auth" = [])),
    request_body = MergeCartRequest,
    responses(
        (status = 200, description = "Merged", body = CartResponse),
        (status = 401, description = "Unauthorized")
    )
)]
pub(crate) async fn merge_cart(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<MergeCartRequest>,
) -> AppResult<Json<CartResponse>> {
    let merged = cart::merge_carts(&state.db, req.anonymous_id, auth.user_id).await?;
    let items = cart::list_items(&state.db, merged.id).await?;
    let totals = cart::compute_totals(&items, None);
    Ok(Json(CartResponse {
        cart: merged,
        items,
        totals,
    }))
}
