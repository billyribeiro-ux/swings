//! EC-01 row types for the product model.
//!
//! These are the strongly-typed wire + DB shapes. Money columns are `i64`
//! because that is what the DB encodes; the service layer lifts them into
//! [`crate::common::money::Money`] when arithmetic is needed (see `coupons.rs`
//! for the pattern). Keeping the row layer `i64` preserves the transparent
//! sqlx FromRow integration.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;
use uuid::Uuid;

/// Top-level product archetype. Matches the CHECK constraint in
/// `migrations/040_products.sql`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type, ToSchema)]
#[serde(rename_all = "snake_case")]
#[sqlx(type_name = "TEXT", rename_all = "lowercase")]
pub enum ProductType {
    Simple,
    Subscription,
    Downloadable,
    Bundle,
}

impl ProductType {
    /// Stable lowercase tag used in DB CHECK and outbox payloads.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            ProductType::Simple => "simple",
            ProductType::Subscription => "subscription",
            ProductType::Downloadable => "downloadable",
            ProductType::Bundle => "bundle",
        }
    }

    /// Inverse of [`as_str`](Self::as_str). Used when reading the TEXT column
    /// back into a typed enum for outbox payload rendering.
    pub fn from_str_lower(s: &str) -> Option<Self> {
        Some(match s {
            "simple" => ProductType::Simple,
            "subscription" => ProductType::Subscription,
            "downloadable" => ProductType::Downloadable,
            "bundle" => ProductType::Bundle,
            _ => return None,
        })
    }
}

/// Publication lifecycle. `draft` is the default on insert; only `published`
/// rows surface through the public router. `archived` is terminal and hides
/// the product from every lookup except admin list views.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type, ToSchema)]
#[serde(rename_all = "snake_case")]
#[sqlx(type_name = "TEXT", rename_all = "lowercase")]
pub enum ProductStatus {
    Draft,
    Published,
    Archived,
}

impl ProductStatus {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            ProductStatus::Draft => "draft",
            ProductStatus::Published => "published",
            ProductStatus::Archived => "archived",
        }
    }

    pub fn from_str_lower(s: &str) -> Option<Self> {
        Some(match s {
            "draft" => ProductStatus::Draft,
            "published" => ProductStatus::Published,
            "archived" => ProductStatus::Archived,
            _ => return None,
        })
    }
}

/// `products` row. Money fields are `BIGINT` cents; translate via
/// [`crate::common::money::Money::cents`] when arithmetic is needed.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct Product {
    pub id: Uuid,
    pub slug: String,
    pub name: String,
    pub description: Option<String>,
    pub product_type: String,
    pub status: String,
    pub price_cents: Option<i64>,
    pub compare_at_cents: Option<i64>,
    pub currency: String,
    pub tax_class: String,
    pub stripe_product_id: Option<String>,
    pub stripe_price_id: Option<String>,
    pub gallery_media_ids: Vec<Uuid>,
    pub featured_media_id: Option<Uuid>,
    pub seo_title: Option<String>,
    pub seo_description: Option<String>,
    pub metadata: serde_json::Value,
    pub created_by: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// `product_variants` row. A variant overrides product pricing when
/// `price_cents` is set; otherwise it inherits.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct ProductVariant {
    pub id: Uuid,
    pub product_id: Uuid,
    pub sku: Option<String>,
    pub name: Option<String>,
    pub price_cents: Option<i64>,
    pub currency: Option<String>,
    pub attributes: serde_json::Value,
    pub stripe_price_id: Option<String>,
    pub position: i32,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
}

/// `downloadable_assets` row. `storage_key` is the R2 object key; EC-07 will
/// wire signed-URL issuance against it. `access_policy` gates who can request
/// a fresh signed URL.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct DownloadableAsset {
    pub id: Uuid,
    pub product_id: Uuid,
    pub variant_id: Option<Uuid>,
    pub storage_key: String,
    pub filename: String,
    pub mime_type: String,
    pub size_bytes: i64,
    pub sha256: String,
    pub access_policy: String,
    pub required_tier: Option<String>,
    pub download_limit: Option<i32>,
    pub expires_after_hours: Option<i32>,
    pub created_at: DateTime<Utc>,
}

/// `bundle_items` row. `quantity` is always positive (CHECK-enforced).
/// `child_variant_id` is optional; when set, the bundle includes that specific
/// variant, otherwise it includes the default/any variant of the child product.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct BundleItem {
    pub id: Uuid,
    pub bundle_product_id: Uuid,
    pub child_product_id: Uuid,
    pub child_variant_id: Option<Uuid>,
    pub quantity: i32,
    pub position: i32,
}

// ── Admin request/response DTOs ────────────────────────────────────────

/// Admin-only payload for `POST /api/admin/products`. `product_type` + `slug`
/// are mandatory; everything else is optional on create and can be updated
/// later.
#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateProductRequest {
    pub slug: String,
    pub name: String,
    pub product_type: ProductType,
    pub description: Option<String>,
    pub status: Option<ProductStatus>,
    pub price_cents: Option<i64>,
    pub compare_at_cents: Option<i64>,
    pub currency: Option<String>,
    pub tax_class: Option<String>,
    pub stripe_product_id: Option<String>,
    pub stripe_price_id: Option<String>,
    pub gallery_media_ids: Option<Vec<Uuid>>,
    pub featured_media_id: Option<Uuid>,
    pub seo_title: Option<String>,
    pub seo_description: Option<String>,
    pub metadata: Option<serde_json::Value>,
}

/// Admin-only payload for `PUT /api/admin/products/{id}`. Every field is
/// optional — `None` means "leave the existing value alone" (COALESCE in the
/// repo layer). `product_type` is deliberately not update-able because
/// changing it post-creation would invalidate variant + asset + bundle rows.
#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateProductRequest {
    pub slug: Option<String>,
    pub name: Option<String>,
    pub description: Option<String>,
    pub price_cents: Option<i64>,
    pub compare_at_cents: Option<i64>,
    pub currency: Option<String>,
    pub tax_class: Option<String>,
    pub stripe_product_id: Option<String>,
    pub stripe_price_id: Option<String>,
    pub gallery_media_ids: Option<Vec<Uuid>>,
    pub featured_media_id: Option<Uuid>,
    pub seo_title: Option<String>,
    pub seo_description: Option<String>,
    pub metadata: Option<serde_json::Value>,
}

/// Admin-only payload for `POST /api/admin/products/{id}/status`.
#[derive(Debug, Deserialize, ToSchema)]
pub struct SetStatusRequest {
    pub status: ProductStatus,
}

/// Admin-only payload for creating a variant.
#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateVariantRequest {
    pub sku: Option<String>,
    pub name: Option<String>,
    pub price_cents: Option<i64>,
    pub currency: Option<String>,
    pub attributes: Option<serde_json::Value>,
    pub stripe_price_id: Option<String>,
    pub position: Option<i32>,
    pub is_active: Option<bool>,
}

/// Admin-only payload for updating a variant.
#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateVariantRequest {
    pub sku: Option<String>,
    pub name: Option<String>,
    pub price_cents: Option<i64>,
    pub currency: Option<String>,
    pub attributes: Option<serde_json::Value>,
    pub stripe_price_id: Option<String>,
    pub position: Option<i32>,
    pub is_active: Option<bool>,
}

/// Admin-only payload for adding a downloadable asset.
#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateAssetRequest {
    pub variant_id: Option<Uuid>,
    pub storage_key: String,
    pub filename: String,
    pub mime_type: String,
    pub size_bytes: i64,
    pub sha256: String,
    pub access_policy: Option<String>,
    pub required_tier: Option<String>,
    pub download_limit: Option<i32>,
    pub expires_after_hours: Option<i32>,
}

/// Admin-only payload for a single bundle component.
#[derive(Debug, Deserialize, Serialize, Clone, ToSchema)]
pub struct BundleItemInput {
    pub child_product_id: Uuid,
    pub child_variant_id: Option<Uuid>,
    pub quantity: i32,
    pub position: i32,
}

/// Admin-only payload for `PUT /api/admin/products/{id}/bundle-items`.
#[derive(Debug, Deserialize, ToSchema)]
pub struct SetBundleItemsRequest {
    pub items: Vec<BundleItemInput>,
}

/// List filter — admin-only. `status` defaults to None (all statuses); the
/// public router always filters to `published` regardless of the query.
#[derive(Debug, Deserialize)]
pub struct ProductListParams {
    pub page: Option<i64>,
    pub per_page: Option<i64>,
    pub status: Option<String>,
    pub product_type: Option<String>,
    pub search: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn product_type_round_trip() {
        for t in [
            ProductType::Simple,
            ProductType::Subscription,
            ProductType::Downloadable,
            ProductType::Bundle,
        ] {
            assert_eq!(ProductType::from_str_lower(t.as_str()), Some(t));
        }
        assert_eq!(ProductType::from_str_lower("nonsense"), None);
    }

    #[test]
    fn product_status_round_trip() {
        for s in [
            ProductStatus::Draft,
            ProductStatus::Published,
            ProductStatus::Archived,
        ] {
            assert_eq!(ProductStatus::from_str_lower(s.as_str()), Some(s));
        }
        assert_eq!(ProductStatus::from_str_lower("nonsense"), None);
    }

    #[test]
    fn product_type_serde_is_snake_case() {
        let json = serde_json::to_string(&ProductType::Subscription).unwrap();
        assert_eq!(json, "\"subscription\"");
        let parsed: ProductType = serde_json::from_str("\"downloadable\"").unwrap();
        assert_eq!(parsed, ProductType::Downloadable);
    }

    #[test]
    fn product_status_serde_is_snake_case() {
        let json = serde_json::to_string(&ProductStatus::Archived).unwrap();
        assert_eq!(json, "\"archived\"");
        let parsed: ProductStatus = serde_json::from_str("\"published\"").unwrap();
        assert_eq!(parsed, ProductStatus::Published);
    }
}
