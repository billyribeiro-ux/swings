//! EC-01 runtime-checked sqlx queries for the product model.
//!
//! Every call uses the runtime form `sqlx::query_as::<_, Row>(...)` per the
//! crate-wide convention — the codebase does not ship the offline macro cache,
//! and several rows here embed `JSONB` / `UUID[]` columns that the macro
//! struggles with anyway.
//!
//! The functions live under `commerce::repo` so handler code can `use
//! crate::commerce::repo` and reach for the read/write helpers without threading
//! raw SQL through the HTTP surface.

use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

use crate::error::{AppError, AppResult};

use super::products::{
    BundleItem, BundleItemInput, DownloadableAsset, Product, ProductListParams, ProductVariant,
};

/// Either a UUID or a slug — supports the "`get_product(id_or_slug)`" pattern
/// the plan calls for without exposing two entry points to the caller.
pub enum ProductLookup<'a> {
    Id(Uuid),
    Slug(&'a str),
}

/// Paginate `products` with optional filters. Admin-only callers pass every
/// status; the public handler forces `status = 'published'` regardless of
/// `filter.status` to avoid an accidental admin leak.
pub async fn list_products(
    pool: &PgPool,
    filter: &ProductListParams,
    force_published: bool,
) -> AppResult<(Vec<Product>, i64)> {
    let per_page = filter.per_page.unwrap_or(20).clamp(1, 100);
    let page = filter.page.unwrap_or(1).max(1);
    let offset = (page - 1) * per_page;

    let status_filter: Option<String> = if force_published {
        Some("published".to_string())
    } else {
        filter.status.clone()
    };
    let type_filter = filter.product_type.clone();
    let search_pattern = filter.search.as_deref().map(|s| format!("%{s}%"));

    let rows = sqlx::query_as::<_, Product>(
        r#"
        SELECT id, slug, name, description, product_type, status,
               price_cents, compare_at_cents, currency, tax_class,
               stripe_product_id, stripe_price_id, gallery_media_ids,
               featured_media_id, seo_title, seo_description, metadata,
               created_by, created_at, updated_at
        FROM products
        WHERE ($1::text IS NULL OR status = $1)
          AND ($2::text IS NULL OR product_type = $2)
          AND ($3::text IS NULL OR name ILIKE $3 OR slug ILIKE $3 OR description ILIKE $3)
        ORDER BY updated_at DESC
        LIMIT $4 OFFSET $5
        "#,
    )
    .bind(&status_filter)
    .bind(&type_filter)
    .bind(&search_pattern)
    .bind(per_page)
    .bind(offset)
    .fetch_all(pool)
    .await?;

    let total: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*) FROM products
        WHERE ($1::text IS NULL OR status = $1)
          AND ($2::text IS NULL OR product_type = $2)
          AND ($3::text IS NULL OR name ILIKE $3 OR slug ILIKE $3 OR description ILIKE $3)
        "#,
    )
    .bind(&status_filter)
    .bind(&type_filter)
    .bind(&search_pattern)
    .fetch_one(pool)
    .await?;

    Ok((rows, total))
}

/// Look up a single product by id or slug. Returns `Err(NotFound)` when no
/// row matches so the handler can surface a 404.
pub async fn get_product(pool: &PgPool, lookup: ProductLookup<'_>) -> AppResult<Product> {
    let row: Option<Product> = match lookup {
        ProductLookup::Id(id) => {
            sqlx::query_as::<_, Product>(
                r#"SELECT * FROM products WHERE id = $1"#,
            )
            .bind(id)
            .fetch_optional(pool)
            .await?
        }
        ProductLookup::Slug(slug) => {
            sqlx::query_as::<_, Product>(
                r#"SELECT * FROM products WHERE slug = $1"#,
            )
            .bind(slug)
            .fetch_optional(pool)
            .await?
        }
    };
    row.ok_or_else(|| AppError::NotFound("Product not found".to_string()))
}

/// Insert a new product row. Returns the hydrated `Product` on success so the
/// caller can echo the server-assigned id + timestamps.
#[allow(clippy::too_many_arguments)]
pub async fn create_product(
    tx: &mut Transaction<'_, Postgres>,
    slug: &str,
    name: &str,
    description: Option<&str>,
    product_type: &str,
    status: &str,
    price_cents: Option<i64>,
    compare_at_cents: Option<i64>,
    currency: &str,
    tax_class: &str,
    stripe_product_id: Option<&str>,
    stripe_price_id: Option<&str>,
    gallery_media_ids: &[Uuid],
    featured_media_id: Option<Uuid>,
    seo_title: Option<&str>,
    seo_description: Option<&str>,
    metadata: &serde_json::Value,
    created_by: Option<Uuid>,
) -> AppResult<Product> {
    let product = sqlx::query_as::<_, Product>(
        r#"
        INSERT INTO products (
            slug, name, description, product_type, status,
            price_cents, compare_at_cents, currency, tax_class,
            stripe_product_id, stripe_price_id, gallery_media_ids,
            featured_media_id, seo_title, seo_description, metadata, created_by
        )
        VALUES ($1, $2, $3, $4, $5,
                $6, $7, $8, $9,
                $10, $11, $12,
                $13, $14, $15, $16, $17)
        RETURNING id, slug, name, description, product_type, status,
                  price_cents, compare_at_cents, currency, tax_class,
                  stripe_product_id, stripe_price_id, gallery_media_ids,
                  featured_media_id, seo_title, seo_description, metadata,
                  created_by, created_at, updated_at
        "#,
    )
    .bind(slug)
    .bind(name)
    .bind(description)
    .bind(product_type)
    .bind(status)
    .bind(price_cents)
    .bind(compare_at_cents)
    .bind(currency)
    .bind(tax_class)
    .bind(stripe_product_id)
    .bind(stripe_price_id)
    .bind(gallery_media_ids)
    .bind(featured_media_id)
    .bind(seo_title)
    .bind(seo_description)
    .bind(metadata)
    .bind(created_by)
    .fetch_one(&mut **tx)
    .await?;
    Ok(product)
}

/// Partial-update helper — every `Option::Some` field is written; `None`
/// preserves the current value via COALESCE. `updated_at` is always bumped.
#[allow(clippy::too_many_arguments)]
pub async fn update_product(
    tx: &mut Transaction<'_, Postgres>,
    id: Uuid,
    slug: Option<&str>,
    name: Option<&str>,
    description: Option<Option<&str>>,
    price_cents: Option<Option<i64>>,
    compare_at_cents: Option<Option<i64>>,
    currency: Option<&str>,
    tax_class: Option<&str>,
    stripe_product_id: Option<Option<&str>>,
    stripe_price_id: Option<Option<&str>>,
    gallery_media_ids: Option<&[Uuid]>,
    featured_media_id: Option<Option<Uuid>>,
    seo_title: Option<Option<&str>>,
    seo_description: Option<Option<&str>>,
    metadata: Option<&serde_json::Value>,
) -> AppResult<Product> {
    // sqlx COALESCE pattern — pass NULL for fields the caller left as None, and
    // the SQL keeps the existing row value. Double-`Option` is used to let the
    // caller explicitly set a field to NULL (outer Some, inner None) vs. leave
    // it alone (outer None).
    let product = sqlx::query_as::<_, Product>(
        r#"
        UPDATE products SET
            slug             = COALESCE($1, slug),
            name             = COALESCE($2, name),
            description      = CASE WHEN $18::bool THEN $3 ELSE description END,
            price_cents      = CASE WHEN $19::bool THEN $4 ELSE price_cents END,
            compare_at_cents = CASE WHEN $20::bool THEN $5 ELSE compare_at_cents END,
            currency         = COALESCE($6, currency),
            tax_class        = COALESCE($7, tax_class),
            stripe_product_id= CASE WHEN $21::bool THEN $8 ELSE stripe_product_id END,
            stripe_price_id  = CASE WHEN $22::bool THEN $9 ELSE stripe_price_id END,
            gallery_media_ids= COALESCE($10, gallery_media_ids),
            featured_media_id= CASE WHEN $23::bool THEN $11 ELSE featured_media_id END,
            seo_title        = CASE WHEN $24::bool THEN $12 ELSE seo_title END,
            seo_description  = CASE WHEN $25::bool THEN $13 ELSE seo_description END,
            metadata         = COALESCE($14, metadata),
            updated_at       = NOW()
        WHERE id = $15
        RETURNING id, slug, name, description, product_type, status,
                  price_cents, compare_at_cents, currency, tax_class,
                  stripe_product_id, stripe_price_id, gallery_media_ids,
                  featured_media_id, seo_title, seo_description, metadata,
                  created_by, created_at, updated_at
        "#,
    )
    .bind(slug)
    .bind(name)
    .bind(description.and_then(|o| o.map(|s| s.to_string())))
    .bind(price_cents.flatten())
    .bind(compare_at_cents.flatten())
    .bind(currency)
    .bind(tax_class)
    .bind(stripe_product_id.and_then(|o| o.map(|s| s.to_string())))
    .bind(stripe_price_id.and_then(|o| o.map(|s| s.to_string())))
    .bind(gallery_media_ids)
    .bind(featured_media_id.flatten())
    .bind(seo_title.and_then(|o| o.map(|s| s.to_string())))
    .bind(seo_description.and_then(|o| o.map(|s| s.to_string())))
    .bind(metadata)
    .bind(id)
    // explicit-null flags (`$18..$25`): drive the CASE branches above so a
    // None outer-Option leaves the column alone and a Some(None) writes NULL.
    .bind(description.is_some())
    .bind(price_cents.is_some())
    .bind(compare_at_cents.is_some())
    .bind(stripe_product_id.is_some())
    .bind(stripe_price_id.is_some())
    .bind(featured_media_id.is_some())
    .bind(seo_title.is_some())
    .bind(seo_description.is_some())
    .fetch_optional(&mut **tx)
    .await?
    .ok_or_else(|| AppError::NotFound("Product not found".to_string()))?;
    Ok(product)
}

/// Flip `status` in isolation. Kept separate from [`update_product`] so the
/// outbox event emitted from the handler can carry a dedicated
/// `commerce.product.status_changed` event-type.
pub async fn set_status(
    tx: &mut Transaction<'_, Postgres>,
    id: Uuid,
    status: &str,
) -> AppResult<Product> {
    let product = sqlx::query_as::<_, Product>(
        r#"
        UPDATE products SET status = $1, updated_at = NOW()
        WHERE id = $2
        RETURNING id, slug, name, description, product_type, status,
                  price_cents, compare_at_cents, currency, tax_class,
                  stripe_product_id, stripe_price_id, gallery_media_ids,
                  featured_media_id, seo_title, seo_description, metadata,
                  created_by, created_at, updated_at
        "#,
    )
    .bind(status)
    .bind(id)
    .fetch_optional(&mut **tx)
    .await?
    .ok_or_else(|| AppError::NotFound("Product not found".to_string()))?;
    Ok(product)
}

// ── Variants ───────────────────────────────────────────────────────────

/// Fetch every variant of a product in `position` order. Handlers can filter
/// `is_active` client-side cheaply; keeping the repo call unfiltered keeps the
/// interface minimal.
pub async fn list_variants(pool: &PgPool, product_id: Uuid) -> AppResult<Vec<ProductVariant>> {
    let rows = sqlx::query_as::<_, ProductVariant>(
        r#"
        SELECT id, product_id, sku, name, price_cents, currency, attributes,
               stripe_price_id, position, is_active, created_at
        FROM product_variants
        WHERE product_id = $1
        ORDER BY position ASC, created_at ASC
        "#,
    )
    .bind(product_id)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

/// Insert a variant. `sku` uniqueness is enforced at the DB level.
#[allow(clippy::too_many_arguments)]
pub async fn add_variant(
    tx: &mut Transaction<'_, Postgres>,
    product_id: Uuid,
    sku: Option<&str>,
    name: Option<&str>,
    price_cents: Option<i64>,
    currency: Option<&str>,
    attributes: &serde_json::Value,
    stripe_price_id: Option<&str>,
    position: i32,
    is_active: bool,
) -> AppResult<ProductVariant> {
    let variant = sqlx::query_as::<_, ProductVariant>(
        r#"
        INSERT INTO product_variants (
            product_id, sku, name, price_cents, currency,
            attributes, stripe_price_id, position, is_active
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
        RETURNING id, product_id, sku, name, price_cents, currency, attributes,
                  stripe_price_id, position, is_active, created_at
        "#,
    )
    .bind(product_id)
    .bind(sku)
    .bind(name)
    .bind(price_cents)
    .bind(currency)
    .bind(attributes)
    .bind(stripe_price_id)
    .bind(position)
    .bind(is_active)
    .fetch_one(&mut **tx)
    .await?;
    Ok(variant)
}

/// Partial update — None leaves the value alone.
#[allow(clippy::too_many_arguments)]
pub async fn update_variant(
    tx: &mut Transaction<'_, Postgres>,
    id: Uuid,
    sku: Option<Option<&str>>,
    name: Option<Option<&str>>,
    price_cents: Option<Option<i64>>,
    currency: Option<Option<&str>>,
    attributes: Option<&serde_json::Value>,
    stripe_price_id: Option<Option<&str>>,
    position: Option<i32>,
    is_active: Option<bool>,
) -> AppResult<ProductVariant> {
    let variant = sqlx::query_as::<_, ProductVariant>(
        r#"
        UPDATE product_variants SET
            sku             = CASE WHEN $10::bool THEN $1 ELSE sku END,
            name            = CASE WHEN $11::bool THEN $2 ELSE name END,
            price_cents     = CASE WHEN $12::bool THEN $3 ELSE price_cents END,
            currency        = CASE WHEN $13::bool THEN $4 ELSE currency END,
            attributes      = COALESCE($5, attributes),
            stripe_price_id = CASE WHEN $14::bool THEN $6 ELSE stripe_price_id END,
            position        = COALESCE($7, position),
            is_active       = COALESCE($8, is_active)
        WHERE id = $9
        RETURNING id, product_id, sku, name, price_cents, currency, attributes,
                  stripe_price_id, position, is_active, created_at
        "#,
    )
    .bind(sku.and_then(|o| o.map(|s| s.to_string())))
    .bind(name.and_then(|o| o.map(|s| s.to_string())))
    .bind(price_cents.flatten())
    .bind(currency.and_then(|o| o.map(|s| s.to_string())))
    .bind(attributes)
    .bind(stripe_price_id.and_then(|o| o.map(|s| s.to_string())))
    .bind(position)
    .bind(is_active)
    .bind(id)
    .bind(sku.is_some())
    .bind(name.is_some())
    .bind(price_cents.is_some())
    .bind(currency.is_some())
    .bind(stripe_price_id.is_some())
    .fetch_optional(&mut **tx)
    .await?
    .ok_or_else(|| AppError::NotFound("Variant not found".to_string()))?;
    Ok(variant)
}

/// Hard-delete a variant row. Downloadable assets with a matching `variant_id`
/// cascade via the FK.
pub async fn delete_variant(tx: &mut Transaction<'_, Postgres>, id: Uuid) -> AppResult<()> {
    let res = sqlx::query("DELETE FROM product_variants WHERE id = $1")
        .bind(id)
        .execute(&mut **tx)
        .await?;
    if res.rows_affected() == 0 {
        return Err(AppError::NotFound("Variant not found".to_string()));
    }
    Ok(())
}

// ── Downloadable assets ───────────────────────────────────────────────

/// List every downloadable tied to a product. `variant_id` is returned on each
/// row so the UI can render the per-variant grouping.
pub async fn list_assets(pool: &PgPool, product_id: Uuid) -> AppResult<Vec<DownloadableAsset>> {
    let rows = sqlx::query_as::<_, DownloadableAsset>(
        r#"
        SELECT id, product_id, variant_id, storage_key, filename, mime_type,
               size_bytes, sha256, access_policy, required_tier, download_limit,
               expires_after_hours, created_at
        FROM downloadable_assets
        WHERE product_id = $1
        ORDER BY created_at ASC
        "#,
    )
    .bind(product_id)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

/// Insert a downloadable asset. `storage_key` is the R2 object key; EC-07 will
/// validate the object exists + populate the signed-URL issuance path against
/// it. At EC-01 we only record the metadata.
#[allow(clippy::too_many_arguments)]
pub async fn add_asset(
    tx: &mut Transaction<'_, Postgres>,
    product_id: Uuid,
    variant_id: Option<Uuid>,
    storage_key: &str,
    filename: &str,
    mime_type: &str,
    size_bytes: i64,
    sha256: &str,
    access_policy: &str,
    required_tier: Option<&str>,
    download_limit: Option<i32>,
    expires_after_hours: Option<i32>,
) -> AppResult<DownloadableAsset> {
    let asset = sqlx::query_as::<_, DownloadableAsset>(
        r#"
        INSERT INTO downloadable_assets (
            product_id, variant_id, storage_key, filename, mime_type,
            size_bytes, sha256, access_policy, required_tier,
            download_limit, expires_after_hours
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
        RETURNING id, product_id, variant_id, storage_key, filename, mime_type,
                  size_bytes, sha256, access_policy, required_tier,
                  download_limit, expires_after_hours, created_at
        "#,
    )
    .bind(product_id)
    .bind(variant_id)
    .bind(storage_key)
    .bind(filename)
    .bind(mime_type)
    .bind(size_bytes)
    .bind(sha256)
    .bind(access_policy)
    .bind(required_tier)
    .bind(download_limit)
    .bind(expires_after_hours)
    .fetch_one(&mut **tx)
    .await?;
    Ok(asset)
}

/// Delete an asset row. The storage object itself lives on R2; EC-07 is
/// responsible for the physical deletion flow (it may need to defer to a
/// background worker if the object is referenced by in-flight orders).
pub async fn delete_asset(tx: &mut Transaction<'_, Postgres>, id: Uuid) -> AppResult<()> {
    let res = sqlx::query("DELETE FROM downloadable_assets WHERE id = $1")
        .bind(id)
        .execute(&mut **tx)
        .await?;
    if res.rows_affected() == 0 {
        return Err(AppError::NotFound("Asset not found".to_string()));
    }
    Ok(())
}

// ── Bundle items ──────────────────────────────────────────────────────

/// List bundle items of a bundle-typed product, ordered by `position`.
pub async fn list_bundle_items(
    pool: &PgPool,
    bundle_product_id: Uuid,
) -> AppResult<Vec<BundleItem>> {
    let rows = sqlx::query_as::<_, BundleItem>(
        r#"
        SELECT id, bundle_product_id, child_product_id, child_variant_id,
               quantity, position
        FROM bundle_items
        WHERE bundle_product_id = $1
        ORDER BY position ASC
        "#,
    )
    .bind(bundle_product_id)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

/// Replace every bundle item on a product with `items`. Done in-transaction to
/// avoid a partial-state window; callers pass their own `&mut tx` so the
/// outbox event publishes atomically alongside.
pub async fn set_bundle_items(
    tx: &mut Transaction<'_, Postgres>,
    bundle_product_id: Uuid,
    items: &[BundleItemInput],
) -> AppResult<Vec<BundleItem>> {
    sqlx::query("DELETE FROM bundle_items WHERE bundle_product_id = $1")
        .bind(bundle_product_id)
        .execute(&mut **tx)
        .await?;

    let mut rows: Vec<BundleItem> = Vec::with_capacity(items.len());
    for item in items {
        let row = sqlx::query_as::<_, BundleItem>(
            r#"
            INSERT INTO bundle_items (
                bundle_product_id, child_product_id, child_variant_id,
                quantity, position
            )
            VALUES ($1, $2, $3, $4, $5)
            RETURNING id, bundle_product_id, child_product_id, child_variant_id,
                      quantity, position
            "#,
        )
        .bind(bundle_product_id)
        .bind(item.child_product_id)
        .bind(item.child_variant_id)
        .bind(item.quantity)
        .bind(item.position)
        .fetch_one(&mut **tx)
        .await?;
        rows.push(row);
    }
    Ok(rows)
}
