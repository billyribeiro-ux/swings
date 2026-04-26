//! EC-01 product + variant + downloadable-asset + bundle HTTP handlers.
//!
//! Admin surface: `/api/admin/products` — full CRUD for products, variants,
//! downloadable asset metadata, and bundle-item composition.
//!
//! Public surface: `/api/products` — `GET` list (published only) + `GET` by
//! slug. List response carries `Cache-Control: public, max-age=60` so the
//! edge cache can short-circuit repeat hits.
//!
//! Every mutating admin handler publishes a `commerce.product.*` event via
//! the FDN-04 outbox so later EC-subsystems (catalog denormalization, Stripe
//! sync, search index refresh) can subscribe without changing these handlers.

use axum::{
    extract::{Path, Query, State},
    http::{header, HeaderMap, HeaderValue},
    response::IntoResponse,
    routing::{delete, get, post, put},
    Json, Router,
};
use serde::Serialize;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{
    commerce::{
        products::{
            BundleItemInput, CreateAssetRequest, CreateProductRequest, CreateVariantRequest,
            ProductListParams, ProductStatus, ProductType, SetBundleItemsRequest, SetStatusRequest,
            UpdateProductRequest, UpdateVariantRequest,
        },
        repo,
    },
    error::{AppError, AppResult},
    events,
    extractors::{AdminUser, ClientInfo},
    models::PaginatedResponse,
    services::audit::audit_admin,
    AppState,
};

use crate::commerce::products::{BundleItem, DownloadableAsset, Product, ProductVariant};

// ══════════════════════════════════════════════════════════════════════
// ROUTERS
// ══════════════════════════════════════════════════════════════════════

pub fn admin_router() -> Router<AppState> {
    Router::new()
        .route(
            "/products",
            get(admin_list_products).post(admin_create_product),
        )
        .route(
            "/products/{id}",
            get(admin_get_product)
                .put(admin_update_product)
                .delete(admin_delete_product),
        )
        .route("/products/{id}/status", post(admin_set_status))
        // Variants
        .route(
            "/products/{id}/variants",
            get(admin_list_variants).post(admin_add_variant),
        )
        .route(
            "/products/{id}/variants/{variant_id}",
            put(admin_update_variant).delete(admin_delete_variant),
        )
        // Downloadable assets
        .route(
            "/products/{id}/assets",
            get(admin_list_assets).post(admin_add_asset),
        )
        .route(
            "/products/{id}/assets/{asset_id}",
            delete(admin_delete_asset),
        )
        // Bundle items
        .route(
            "/products/{id}/bundle-items",
            get(admin_list_bundle_items).put(admin_set_bundle_items),
        )
}

pub fn public_router() -> Router<AppState> {
    Router::new()
        .route("/products", get(public_list_products))
        .route("/products/{slug}", get(public_get_product))
}

// ══════════════════════════════════════════════════════════════════════
// RESPONSE TYPES
// ══════════════════════════════════════════════════════════════════════

/// Bundle of a product + its first-class children. Returned by `GET
/// /api/admin/products/{id}` and `GET /api/products/{slug}` so the admin UI
/// and the public PDP both receive a single hydrated document.
#[derive(Debug, Serialize, ToSchema)]
pub struct ProductDetail {
    #[serde(flatten)]
    pub product: Product,
    pub variants: Vec<ProductVariant>,
    pub assets: Vec<DownloadableAsset>,
    pub bundle_items: Vec<BundleItem>,
}

// ══════════════════════════════════════════════════════════════════════
// OUTBOX HELPER
// ══════════════════════════════════════════════════════════════════════

/// Wrap [`events::publish_in_tx`] in the common `AppError` conversion the rest
/// of the crate uses. Keeps the handler code readable.
async fn publish_event(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    event_type: &str,
    product: &Product,
) -> AppResult<()> {
    let event = events::Event {
        aggregate_type: "product".into(),
        aggregate_id: product.id.to_string(),
        event_type: event_type.into(),
        payload: serde_json::json!({
            "id": product.id,
            "slug": product.slug,
            "name": product.name,
            "product_type": product.product_type,
            "status": product.status,
        }),
        headers: events::EventHeaders::default(),
    };
    events::publish_in_tx(tx, &event)
        .await
        .map_err(|e| match e {
            events::OutboxError::Database(err) => AppError::Database(err),
            events::OutboxError::Serialize(err) => {
                AppError::Internal(anyhow::anyhow!("outbox serialize: {err}"))
            }
        })?;
    Ok(())
}

// ══════════════════════════════════════════════════════════════════════
// ADMIN HANDLERS
// ══════════════════════════════════════════════════════════════════════

async fn admin_list_products(
    State(state): State<AppState>,
    _admin: AdminUser,
    Query(params): Query<ProductListParams>,
) -> AppResult<Json<PaginatedResponse<Product>>> {
    let per_page = params.per_page.unwrap_or(20).clamp(1, 100);
    let page = params.page.unwrap_or(1).max(1);
    let (data, total) = repo::list_products(&state.db, &params, false).await?;
    let total_pages = if per_page > 0 {
        (total as f64 / per_page as f64).ceil() as i64
    } else {
        0
    };
    Ok(Json(PaginatedResponse {
        data,
        total,
        page,
        per_page,
        total_pages,
    }))
}

async fn admin_get_product(
    State(state): State<AppState>,
    _admin: AdminUser,
    Path(id): Path<Uuid>,
) -> AppResult<Json<ProductDetail>> {
    let product = repo::get_product(&state.db, repo::ProductLookup::Id(id)).await?;
    let variants = repo::list_variants(&state.db, product.id).await?;
    let assets = repo::list_assets(&state.db, product.id).await?;
    let bundle_items = repo::list_bundle_items(&state.db, product.id).await?;
    Ok(Json(ProductDetail {
        product,
        variants,
        assets,
        bundle_items,
    }))
}

#[utoipa::path(
    post,
    path = "/api/admin/products",
    tag = "products",
    security(("bearer_auth" = [])),
    request_body = CreateProductRequest,
    responses(
        (status = 200, description = "Product created", body = Product),
        (status = 403, description = "Forbidden"),
        (status = 409, description = "Slug conflict")
    )
)]
pub(crate) async fn admin_create_product(
    State(state): State<AppState>,
    admin: AdminUser,
    client: ClientInfo,
    Json(req): Json<CreateProductRequest>,
) -> AppResult<Json<Product>> {
    admin.require(&state.policy, "product.manage")?;
    // Slug uniqueness is also enforced at the DB via the UNIQUE constraint;
    // we check up-front for a human-friendly 409.
    let existing: Option<(Uuid,)> = sqlx::query_as("SELECT id FROM products WHERE slug = $1")
        .bind(&req.slug)
        .fetch_optional(&state.db)
        .await?;
    if existing.is_some() {
        return Err(AppError::Conflict(format!(
            "Product with slug '{}' already exists",
            req.slug
        )));
    }

    let product_type = req.product_type.as_str();
    let status = req.status.unwrap_or(ProductStatus::Draft).as_str();
    let currency = req.currency.as_deref().unwrap_or("USD");
    let tax_class = req.tax_class.as_deref().unwrap_or("standard");
    let gallery_ids = req.gallery_media_ids.clone().unwrap_or_default();
    let metadata = req
        .metadata
        .clone()
        .unwrap_or_else(|| serde_json::json!({}));

    let mut tx = state.db.begin().await?;
    let product = repo::create_product(
        &mut tx,
        &req.slug,
        &req.name,
        req.description.as_deref(),
        product_type,
        status,
        req.price_cents,
        req.compare_at_cents,
        currency,
        tax_class,
        req.stripe_product_id.as_deref(),
        req.stripe_price_id.as_deref(),
        &gallery_ids,
        req.featured_media_id,
        req.seo_title.as_deref(),
        req.seo_description.as_deref(),
        &metadata,
        Some(admin.user_id),
    )
    .await?;
    publish_event(&mut tx, "commerce.product.created", &product).await?;
    tx.commit().await?;

    audit_admin(
        &state.db,
        &admin,
        &client,
        "product.create",
        "product",
        product.id,
        serde_json::json!({
            "slug": product.slug,
            "product_type": product.product_type,
            "status": product.status,
        }),
    )
    .await;

    Ok(Json(product))
}

#[utoipa::path(
    put,
    path = "/api/admin/products/{id}",
    tag = "products",
    security(("bearer_auth" = [])),
    params(("id" = Uuid, Path, description = "Product id")),
    request_body = UpdateProductRequest,
    responses(
        (status = 200, description = "Product updated", body = Product),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Product not found")
    )
)]
pub(crate) async fn admin_update_product(
    State(state): State<AppState>,
    admin: AdminUser,
    client: ClientInfo,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateProductRequest>,
) -> AppResult<Json<Product>> {
    admin.require(&state.policy, "product.manage")?;
    let mut tx = state.db.begin().await?;

    // Double-Option pattern: caller distinguishes "leave alone" (None) vs
    // "set to NULL" (Some(None)) vs "set to value" (Some(Some(x))). Most
    // fields here are plain Option<_> which means callers cannot clear them;
    // that is deliberate — the UI treats missing fields as no-op.
    let product = repo::update_product(
        &mut tx,
        id,
        req.slug.as_deref(),
        req.name.as_deref(),
        req.description.as_ref().map(|s| Some(s.as_str())),
        req.price_cents.map(Some),
        req.compare_at_cents.map(Some),
        req.currency.as_deref(),
        req.tax_class.as_deref(),
        req.stripe_product_id.as_ref().map(|s| Some(s.as_str())),
        req.stripe_price_id.as_ref().map(|s| Some(s.as_str())),
        req.gallery_media_ids.as_deref(),
        req.featured_media_id.map(Some),
        req.seo_title.as_ref().map(|s| Some(s.as_str())),
        req.seo_description.as_ref().map(|s| Some(s.as_str())),
        req.metadata.as_ref(),
    )
    .await?;
    publish_event(&mut tx, "commerce.product.updated", &product).await?;
    tx.commit().await?;

    audit_admin(
        &state.db,
        &admin,
        &client,
        "product.update",
        "product",
        product.id,
        serde_json::json!({
            "slug": product.slug,
            "status": product.status,
        }),
    )
    .await;

    Ok(Json(product))
}

#[utoipa::path(
    post,
    path = "/api/admin/products/{id}/status",
    tag = "products",
    security(("bearer_auth" = [])),
    params(("id" = Uuid, Path, description = "Product id")),
    request_body = SetStatusRequest,
    responses(
        (status = 200, description = "Product status updated", body = Product),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Product not found")
    )
)]
pub(crate) async fn admin_set_status(
    State(state): State<AppState>,
    admin: AdminUser,
    client: ClientInfo,
    Path(id): Path<Uuid>,
    Json(req): Json<SetStatusRequest>,
) -> AppResult<Json<Product>> {
    admin.require(&state.policy, "product.manage")?;
    let mut tx = state.db.begin().await?;
    let product = repo::set_status(&mut tx, id, req.status.as_str()).await?;
    publish_event(&mut tx, "commerce.product.status_changed", &product).await?;
    tx.commit().await?;

    audit_admin(
        &state.db,
        &admin,
        &client,
        "product.status.update",
        "product",
        product.id,
        serde_json::json!({
            "slug": product.slug,
            "status": product.status,
        }),
    )
    .await;

    Ok(Json(product))
}

#[utoipa::path(
    delete,
    path = "/api/admin/products/{id}",
    tag = "products",
    security(("bearer_auth" = [])),
    params(("id" = Uuid, Path, description = "Product id")),
    responses(
        (status = 200, description = "Product deleted"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Product not found")
    )
)]
pub(crate) async fn admin_delete_product(
    State(state): State<AppState>,
    admin: AdminUser,
    client: ClientInfo,
    Path(id): Path<Uuid>,
) -> AppResult<Json<serde_json::Value>> {
    admin.require(&state.policy, "product.manage")?;
    // Snapshot for audit metadata before the hard delete.
    let snapshot: Option<(String, String)> =
        sqlx::query_as("SELECT slug, product_type FROM products WHERE id = $1")
            .bind(id)
            .fetch_optional(&state.db)
            .await?;
    let (slug, product_type) =
        snapshot.ok_or(AppError::NotFound("Product not found".to_string()))?;

    // Hard delete — CASCADEs cover variants / assets / bundle_items the row
    // composes. Orders / line-items referencing the product live under EC-04
    // and will add a PROTECT-style guard at that time.
    let res = sqlx::query("DELETE FROM products WHERE id = $1")
        .bind(id)
        .execute(&state.db)
        .await?;
    if res.rows_affected() == 0 {
        return Err(AppError::NotFound("Product not found".to_string()));
    }

    audit_admin(
        &state.db,
        &admin,
        &client,
        "product.delete",
        "product",
        id,
        serde_json::json!({
            "slug": slug,
            "product_type": product_type,
        }),
    )
    .await;

    Ok(Json(serde_json::json!({ "deleted": true })))
}

async fn admin_list_variants(
    State(state): State<AppState>,
    _admin: AdminUser,
    Path(id): Path<Uuid>,
) -> AppResult<Json<Vec<ProductVariant>>> {
    Ok(Json(repo::list_variants(&state.db, id).await?))
}

#[utoipa::path(
    post,
    path = "/api/admin/products/{id}/variants",
    tag = "products",
    security(("bearer_auth" = [])),
    params(("id" = Uuid, Path, description = "Product id")),
    request_body = CreateVariantRequest,
    responses(
        (status = 200, description = "Variant created", body = ProductVariant),
        (status = 403, description = "Forbidden")
    )
)]
pub(crate) async fn admin_add_variant(
    State(state): State<AppState>,
    admin: AdminUser,
    client: ClientInfo,
    Path(id): Path<Uuid>,
    Json(req): Json<CreateVariantRequest>,
) -> AppResult<Json<ProductVariant>> {
    admin.require(&state.policy, "product.manage")?;
    let mut tx = state.db.begin().await?;
    let product = repo::get_product(&state.db, repo::ProductLookup::Id(id)).await?;
    let attributes = req
        .attributes
        .clone()
        .unwrap_or_else(|| serde_json::json!({}));
    let variant = repo::add_variant(
        &mut tx,
        id,
        req.sku.as_deref(),
        req.name.as_deref(),
        req.price_cents,
        req.currency.as_deref(),
        &attributes,
        req.stripe_price_id.as_deref(),
        req.position.unwrap_or(0),
        req.is_active.unwrap_or(true),
    )
    .await?;
    publish_event(&mut tx, "commerce.product.updated", &product).await?;
    tx.commit().await?;

    audit_admin(
        &state.db,
        &admin,
        &client,
        "product.variant.create",
        "product_variant",
        variant.id,
        serde_json::json!({
            "product_id": id,
            "sku": variant.sku,
        }),
    )
    .await;

    Ok(Json(variant))
}

#[utoipa::path(
    put,
    path = "/api/admin/products/{id}/variants/{variant_id}",
    tag = "products",
    security(("bearer_auth" = [])),
    params(
        ("id" = Uuid, Path, description = "Product id"),
        ("variant_id" = Uuid, Path, description = "Variant id")
    ),
    request_body = UpdateVariantRequest,
    responses(
        (status = 200, description = "Variant updated", body = ProductVariant),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Variant not found")
    )
)]
pub(crate) async fn admin_update_variant(
    State(state): State<AppState>,
    admin: AdminUser,
    client: ClientInfo,
    Path((id, variant_id)): Path<(Uuid, Uuid)>,
    Json(req): Json<UpdateVariantRequest>,
) -> AppResult<Json<ProductVariant>> {
    admin.require(&state.policy, "product.manage")?;
    let mut tx = state.db.begin().await?;
    let product = repo::get_product(&state.db, repo::ProductLookup::Id(id)).await?;
    let variant = repo::update_variant(
        &mut tx,
        variant_id,
        req.sku.as_ref().map(|s| Some(s.as_str())),
        req.name.as_ref().map(|s| Some(s.as_str())),
        req.price_cents.map(Some),
        req.currency.as_ref().map(|s| Some(s.as_str())),
        req.attributes.as_ref(),
        req.stripe_price_id.as_ref().map(|s| Some(s.as_str())),
        req.position,
        req.is_active,
    )
    .await?;
    publish_event(&mut tx, "commerce.product.updated", &product).await?;
    tx.commit().await?;

    audit_admin(
        &state.db,
        &admin,
        &client,
        "product.variant.update",
        "product_variant",
        variant.id,
        serde_json::json!({
            "product_id": id,
            "sku": variant.sku,
        }),
    )
    .await;

    Ok(Json(variant))
}

#[utoipa::path(
    delete,
    path = "/api/admin/products/{id}/variants/{variant_id}",
    tag = "products",
    security(("bearer_auth" = [])),
    params(
        ("id" = Uuid, Path, description = "Product id"),
        ("variant_id" = Uuid, Path, description = "Variant id")
    ),
    responses(
        (status = 200, description = "Variant deleted"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Variant not found")
    )
)]
pub(crate) async fn admin_delete_variant(
    State(state): State<AppState>,
    admin: AdminUser,
    client: ClientInfo,
    Path((id, variant_id)): Path<(Uuid, Uuid)>,
) -> AppResult<Json<serde_json::Value>> {
    admin.require(&state.policy, "product.manage")?;
    let mut tx = state.db.begin().await?;
    let product = repo::get_product(&state.db, repo::ProductLookup::Id(id)).await?;
    repo::delete_variant(&mut tx, variant_id).await?;
    publish_event(&mut tx, "commerce.product.updated", &product).await?;
    tx.commit().await?;

    audit_admin(
        &state.db,
        &admin,
        &client,
        "product.variant.delete",
        "product_variant",
        variant_id,
        serde_json::json!({ "product_id": id }),
    )
    .await;

    Ok(Json(serde_json::json!({ "deleted": true })))
}

async fn admin_list_assets(
    State(state): State<AppState>,
    _admin: AdminUser,
    Path(id): Path<Uuid>,
) -> AppResult<Json<Vec<DownloadableAsset>>> {
    Ok(Json(repo::list_assets(&state.db, id).await?))
}

#[utoipa::path(
    post,
    path = "/api/admin/products/{id}/assets",
    tag = "products",
    security(("bearer_auth" = [])),
    params(("id" = Uuid, Path, description = "Product id")),
    request_body = CreateAssetRequest,
    responses(
        (status = 200, description = "Asset created", body = DownloadableAsset),
        (status = 403, description = "Forbidden")
    )
)]
pub(crate) async fn admin_add_asset(
    State(state): State<AppState>,
    admin: AdminUser,
    client: ClientInfo,
    Path(id): Path<Uuid>,
    Json(req): Json<CreateAssetRequest>,
) -> AppResult<Json<DownloadableAsset>> {
    admin.require(&state.policy, "product.manage")?;
    let access_policy = req.access_policy.as_deref().unwrap_or("purchase_required");
    let mut tx = state.db.begin().await?;
    let product = repo::get_product(&state.db, repo::ProductLookup::Id(id)).await?;
    let asset = repo::add_asset(
        &mut tx,
        id,
        req.variant_id,
        &req.storage_key,
        &req.filename,
        &req.mime_type,
        req.size_bytes,
        &req.sha256,
        access_policy,
        req.required_tier.as_deref(),
        req.download_limit,
        req.expires_after_hours,
    )
    .await?;
    publish_event(&mut tx, "commerce.product.updated", &product).await?;
    tx.commit().await?;

    audit_admin(
        &state.db,
        &admin,
        &client,
        "product.asset.create",
        "downloadable_asset",
        asset.id,
        serde_json::json!({
            "product_id": id,
            "filename": asset.filename,
            "mime_type": asset.mime_type,
        }),
    )
    .await;

    Ok(Json(asset))
}

#[utoipa::path(
    delete,
    path = "/api/admin/products/{id}/assets/{asset_id}",
    tag = "products",
    security(("bearer_auth" = [])),
    params(
        ("id" = Uuid, Path, description = "Product id"),
        ("asset_id" = Uuid, Path, description = "Asset id")
    ),
    responses(
        (status = 200, description = "Asset deleted"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Asset not found")
    )
)]
pub(crate) async fn admin_delete_asset(
    State(state): State<AppState>,
    admin: AdminUser,
    client: ClientInfo,
    Path((id, asset_id)): Path<(Uuid, Uuid)>,
) -> AppResult<Json<serde_json::Value>> {
    admin.require(&state.policy, "product.manage")?;
    let mut tx = state.db.begin().await?;
    let product = repo::get_product(&state.db, repo::ProductLookup::Id(id)).await?;
    repo::delete_asset(&mut tx, asset_id).await?;
    publish_event(&mut tx, "commerce.product.updated", &product).await?;
    tx.commit().await?;

    audit_admin(
        &state.db,
        &admin,
        &client,
        "product.asset.delete",
        "downloadable_asset",
        asset_id,
        serde_json::json!({ "product_id": id }),
    )
    .await;
    // TODO: EC-07 — physically remove the R2 object (or enqueue a worker job)
    // once the asset no longer participates in any order's download
    // entitlement. Row delete alone is safe for now because signed-URL
    // issuance is still in the plan.
    Ok(Json(serde_json::json!({ "deleted": true })))
}

async fn admin_list_bundle_items(
    State(state): State<AppState>,
    _admin: AdminUser,
    Path(id): Path<Uuid>,
) -> AppResult<Json<Vec<BundleItem>>> {
    Ok(Json(repo::list_bundle_items(&state.db, id).await?))
}

#[utoipa::path(
    put,
    path = "/api/admin/products/{id}/bundle-items",
    tag = "products",
    security(("bearer_auth" = [])),
    params(("id" = Uuid, Path, description = "Bundle product id")),
    request_body = SetBundleItemsRequest,
    responses(
        (status = 200, description = "Bundle items replaced", body = [BundleItem]),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Bundle not found")
    )
)]
pub(crate) async fn admin_set_bundle_items(
    State(state): State<AppState>,
    admin: AdminUser,
    client: ClientInfo,
    Path(id): Path<Uuid>,
    Json(req): Json<SetBundleItemsRequest>,
) -> AppResult<Json<Vec<BundleItem>>> {
    admin.require(&state.policy, "product.manage")?;
    let mut tx = state.db.begin().await?;
    let product = repo::get_product(&state.db, repo::ProductLookup::Id(id)).await?;
    if product.product_type != ProductType::Bundle.as_str() {
        return Err(AppError::Unprocessable(
            "Only bundle-typed products can have bundle items".to_string(),
        ));
    }
    let items: Vec<BundleItemInput> = req.items.clone();
    let rows = repo::set_bundle_items(&mut tx, id, &items).await?;
    publish_event(&mut tx, "commerce.product.updated", &product).await?;
    tx.commit().await?;

    let child_ids: Vec<Uuid> = rows.iter().map(|r| r.child_product_id).collect();
    audit_admin(
        &state.db,
        &admin,
        &client,
        "product.bundle_items.set",
        "product",
        id,
        serde_json::json!({
            "slug": product.slug,
            "child_product_ids": child_ids,
            "count": rows.len(),
        }),
    )
    .await;

    Ok(Json(rows))
}

// ══════════════════════════════════════════════════════════════════════
// PUBLIC HANDLERS
// ══════════════════════════════════════════════════════════════════════

async fn public_list_products(
    State(state): State<AppState>,
    Query(params): Query<ProductListParams>,
) -> AppResult<impl IntoResponse> {
    // Force published regardless of what the caller asked for.
    let (data, total) = repo::list_products(&state.db, &params, true).await?;
    let per_page = params.per_page.unwrap_or(20).clamp(1, 100);
    let page = params.page.unwrap_or(1).max(1);
    let total_pages = if per_page > 0 {
        (total as f64 / per_page as f64).ceil() as i64
    } else {
        0
    };
    let body = Json(PaginatedResponse {
        data,
        total,
        page,
        per_page,
        total_pages,
    });
    // Cache-friendly: PDP list pages change on an editorial, not per-request,
    // cadence. Edge / CDN can safely hold for 60s.
    let mut headers = HeaderMap::new();
    headers.insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static("public, max-age=60"),
    );
    Ok((headers, body))
}

async fn public_get_product(
    State(state): State<AppState>,
    Path(slug): Path<String>,
) -> AppResult<Json<ProductDetail>> {
    let product = repo::get_product(&state.db, repo::ProductLookup::Slug(&slug)).await?;
    if product.status != ProductStatus::Published.as_str() {
        return Err(AppError::NotFound("Product not found".to_string()));
    }
    let variants = repo::list_variants(&state.db, product.id).await?;
    let assets = repo::list_assets(&state.db, product.id).await?;
    let bundle_items = repo::list_bundle_items(&state.db, product.id).await?;
    Ok(Json(ProductDetail {
        product,
        variants,
        assets,
        bundle_items,
    }))
}
