//! EC-03: Persistent cart service.
//!
//! A cart is keyed by either an authenticated `user_id` or an anonymous
//! `anonymous_id` (UUID the frontend mints on first visit). The service
//! layer here owns:
//!
//!   * identity-keyed cart lookup + on-demand creation ([`get_or_create_cart`]),
//!   * line item CRUD with automatic (product, variant) de-duplication
//!     (cart_items carries a UNIQUE index on the triple),
//!   * total computation in-memory from a list of items plus an optional
//!     applied coupon ([`compute_totals`]),
//!   * anonymous → authed merge at login ([`merge_carts`]).
//!
//! Coupon / tax / shipping math lives outside this module — EC-08 / EC-11
//! plug into [`compute_totals`] via an `AppliedAdjustment` so the cart
//! stays ignorant of regional VAT rules.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, PgPool};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::error::AppResult;

// ── Types ──────────────────────────────────────────────────────────────

/// Resolved identity of a cart's owner. The resolver in `handlers::cart`
/// returns this enum after looking at the bearer token + `X-Anonymous-Id`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CartIdentity {
    Subject(Uuid),
    Anonymous(Uuid),
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct Cart {
    pub id: Uuid,
    pub user_id: Option<Uuid>,
    pub anonymous_id: Option<Uuid>,
    pub currency: String,
    pub subtotal_cents: i64,
    pub discount_cents: i64,
    pub tax_cents: i64,
    pub total_cents: i64,
    pub applied_coupon_ids: Vec<Uuid>,
    pub metadata: serde_json::Value,
    pub expires_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct CartItem {
    pub id: Uuid,
    pub cart_id: Uuid,
    pub product_id: Uuid,
    pub variant_id: Option<Uuid>,
    pub quantity: i32,
    pub unit_price_cents: i64,
    pub line_total_cents: i64,
    pub metadata: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Returned to the client alongside the cart + items. Recomputed from scratch
/// on every write so the numbers are always reconciled with the items list.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CartTotals {
    pub subtotal_cents: i64,
    pub discount_cents: i64,
    pub tax_cents: i64,
    pub total_cents: i64,
}

/// Hook applied by EC-08 (tax) / EC-11 (coupons) when computing totals.
/// The cart module itself never manufactures these — callers pass an
/// `AppliedAdjustment` built from the relevant domain module.
#[derive(Debug, Clone, Default)]
pub struct AppliedAdjustment {
    pub discount_cents: i64,
    pub tax_cents: i64,
}

// ── Queries ────────────────────────────────────────────────────────────

/// Find a cart for the given identity; create one if none exists yet.
pub async fn get_or_create_cart(pool: &PgPool, identity: CartIdentity) -> AppResult<Cart> {
    if let Some(cart) = find_cart(pool, identity).await? {
        return Ok(cart);
    }
    let (user_id, anonymous_id) = match identity {
        CartIdentity::Subject(uid) => (Some(uid), None),
        CartIdentity::Anonymous(aid) => (None, Some(aid)),
    };
    let row = sqlx::query_as::<_, Cart>(
        r#"
        INSERT INTO carts (user_id, anonymous_id)
        VALUES ($1, $2)
        RETURNING id, user_id, anonymous_id, currency, subtotal_cents,
                  discount_cents, tax_cents, total_cents, applied_coupon_ids,
                  metadata, expires_at, created_at, updated_at
        "#,
    )
    .bind(user_id)
    .bind(anonymous_id)
    .fetch_one(pool)
    .await?;
    Ok(row)
}

async fn find_cart(pool: &PgPool, identity: CartIdentity) -> AppResult<Option<Cart>> {
    let sql = match identity {
        CartIdentity::Subject(_) => {
            r#"
            SELECT id, user_id, anonymous_id, currency, subtotal_cents,
                   discount_cents, tax_cents, total_cents, applied_coupon_ids,
                   metadata, expires_at, created_at, updated_at
            FROM carts WHERE user_id = $1
            "#
        }
        CartIdentity::Anonymous(_) => {
            r#"
            SELECT id, user_id, anonymous_id, currency, subtotal_cents,
                   discount_cents, tax_cents, total_cents, applied_coupon_ids,
                   metadata, expires_at, created_at, updated_at
            FROM carts WHERE anonymous_id = $1
            "#
        }
    };
    let bind = match identity {
        CartIdentity::Subject(uid) => uid,
        CartIdentity::Anonymous(aid) => aid,
    };
    let row = sqlx::query_as::<_, Cart>(sql)
        .bind(bind)
        .fetch_optional(pool)
        .await?;
    Ok(row)
}

pub async fn list_items(pool: &PgPool, cart_id: Uuid) -> AppResult<Vec<CartItem>> {
    let rows = sqlx::query_as::<_, CartItem>(
        r#"
        SELECT id, cart_id, product_id, variant_id, quantity,
               unit_price_cents, line_total_cents, metadata,
               created_at, updated_at
        FROM cart_items
        WHERE cart_id = $1
        ORDER BY created_at ASC
        "#,
    )
    .bind(cart_id)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

/// Upsert a line. If a row already exists for `(cart_id, product_id, variant_id)`
/// the quantity is added and `line_total_cents` rebuilt; the price on an
/// existing line is preserved so later product-price edits do not retroactively
/// revalue an open cart (users see the price they saw at add-time).
pub async fn add_item(
    pool: &PgPool,
    cart_id: Uuid,
    product_id: Uuid,
    variant_id: Option<Uuid>,
    quantity: i32,
    unit_price_cents: i64,
) -> AppResult<CartItem> {
    let line_total = unit_price_cents.saturating_mul(quantity as i64);
    let row = sqlx::query_as::<_, CartItem>(
        r#"
        INSERT INTO cart_items
            (cart_id, product_id, variant_id, quantity,
             unit_price_cents, line_total_cents)
        VALUES ($1, $2, $3, $4, $5, $6)
        ON CONFLICT (cart_id, product_id, COALESCE(variant_id, '00000000-0000-0000-0000-000000000000'::uuid))
        DO UPDATE SET
            quantity         = cart_items.quantity + EXCLUDED.quantity,
            line_total_cents = cart_items.unit_price_cents
                               * (cart_items.quantity + EXCLUDED.quantity),
            updated_at       = NOW()
        RETURNING id, cart_id, product_id, variant_id, quantity,
                  unit_price_cents, line_total_cents, metadata,
                  created_at, updated_at
        "#,
    )
    .bind(cart_id)
    .bind(product_id)
    .bind(variant_id)
    .bind(quantity)
    .bind(unit_price_cents)
    .bind(line_total)
    .fetch_one(pool)
    .await?;
    Ok(row)
}

/// Replace the quantity on an existing line. Returns `None` when the line
/// doesn't belong to the supplied cart (defense against id-enumeration).
pub async fn update_item_qty(
    pool: &PgPool,
    cart_id: Uuid,
    item_id: Uuid,
    quantity: i32,
) -> AppResult<Option<CartItem>> {
    if quantity <= 0 {
        remove_item(pool, cart_id, item_id).await?;
        return Ok(None);
    }
    let row = sqlx::query_as::<_, CartItem>(
        r#"
        UPDATE cart_items
           SET quantity         = $3,
               line_total_cents = unit_price_cents * $3,
               updated_at       = NOW()
         WHERE id = $1 AND cart_id = $2
        RETURNING id, cart_id, product_id, variant_id, quantity,
                  unit_price_cents, line_total_cents, metadata,
                  created_at, updated_at
        "#,
    )
    .bind(item_id)
    .bind(cart_id)
    .bind(quantity)
    .fetch_optional(pool)
    .await?;
    Ok(row)
}

pub async fn remove_item(pool: &PgPool, cart_id: Uuid, item_id: Uuid) -> AppResult<()> {
    sqlx::query("DELETE FROM cart_items WHERE id = $1 AND cart_id = $2")
        .bind(item_id)
        .bind(cart_id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn clear_cart(pool: &PgPool, cart_id: Uuid) -> AppResult<()> {
    sqlx::query("DELETE FROM cart_items WHERE cart_id = $1")
        .bind(cart_id)
        .execute(pool)
        .await?;
    Ok(())
}

/// Move items from the anonymous cart onto the authenticated cart, summing
/// quantities on duplicate lines. The anonymous cart is deleted on success.
/// This is the only path that takes the `user_id` + `anonymous_id` pair —
/// every other API requires the user to have already merged.
pub async fn merge_carts(
    pool: &PgPool,
    anonymous_id: Uuid,
    user_id: Uuid,
) -> AppResult<Cart> {
    let mut tx = pool.begin().await?;

    let authed = sqlx::query_as::<_, Cart>(
        r#"
        INSERT INTO carts (user_id) VALUES ($1)
        ON CONFLICT (user_id) WHERE user_id IS NOT NULL DO UPDATE SET updated_at = NOW()
        RETURNING id, user_id, anonymous_id, currency, subtotal_cents,
                  discount_cents, tax_cents, total_cents, applied_coupon_ids,
                  metadata, expires_at, created_at, updated_at
        "#,
    )
    .bind(user_id)
    .fetch_one(&mut *tx)
    .await?;

    let anon: Option<Cart> = sqlx::query_as::<_, Cart>(
        r#"
        SELECT id, user_id, anonymous_id, currency, subtotal_cents,
               discount_cents, tax_cents, total_cents, applied_coupon_ids,
               metadata, expires_at, created_at, updated_at
        FROM carts WHERE anonymous_id = $1
        "#,
    )
    .bind(anonymous_id)
    .fetch_optional(&mut *tx)
    .await?;

    if let Some(anon_cart) = anon {
        if anon_cart.id != authed.id {
            sqlx::query(
                r#"
                INSERT INTO cart_items
                    (cart_id, product_id, variant_id, quantity,
                     unit_price_cents, line_total_cents, metadata)
                SELECT $1, product_id, variant_id, quantity,
                       unit_price_cents, line_total_cents, metadata
                FROM cart_items WHERE cart_id = $2
                ON CONFLICT (cart_id, product_id, COALESCE(variant_id, '00000000-0000-0000-0000-000000000000'::uuid))
                DO UPDATE SET
                    quantity         = cart_items.quantity + EXCLUDED.quantity,
                    line_total_cents = cart_items.unit_price_cents
                                       * (cart_items.quantity + EXCLUDED.quantity),
                    updated_at       = NOW()
                "#,
            )
            .bind(authed.id)
            .bind(anon_cart.id)
            .execute(&mut *tx)
            .await?;

            sqlx::query("DELETE FROM carts WHERE id = $1")
                .bind(anon_cart.id)
                .execute(&mut *tx)
                .await?;
        }
    }

    tx.commit().await?;
    Ok(authed)
}

// ── Totals ─────────────────────────────────────────────────────────────

/// Deterministic, in-memory total computation — no DB access. Coupon + tax
/// modules contribute an `AppliedAdjustment`; the cart itself only sums
/// line_total_cents and subtracts / adds accordingly.
pub fn compute_totals(items: &[CartItem], adjustment: Option<AppliedAdjustment>) -> CartTotals {
    let subtotal: i64 = items.iter().map(|i| i.line_total_cents).sum();
    let adj = adjustment.unwrap_or_default();
    let discount = adj.discount_cents.min(subtotal);
    let tax = adj.tax_cents.max(0);
    let total = (subtotal - discount + tax).max(0);
    CartTotals {
        subtotal_cents: subtotal,
        discount_cents: discount,
        tax_cents: tax,
        total_cents: total,
    }
}

// ── Unit tests ─────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn item(qty: i32, unit: i64) -> CartItem {
        CartItem {
            id: Uuid::new_v4(),
            cart_id: Uuid::new_v4(),
            product_id: Uuid::new_v4(),
            variant_id: None,
            quantity: qty,
            unit_price_cents: unit,
            line_total_cents: unit * qty as i64,
            metadata: serde_json::json!({}),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    #[test]
    fn totals_sum_line_items() {
        let items = vec![item(2, 1_000), item(1, 500)];
        let t = compute_totals(&items, None);
        assert_eq!(t.subtotal_cents, 2_500);
        assert_eq!(t.discount_cents, 0);
        assert_eq!(t.tax_cents, 0);
        assert_eq!(t.total_cents, 2_500);
    }

    #[test]
    fn discount_is_capped_at_subtotal() {
        let items = vec![item(1, 100)];
        let t = compute_totals(
            &items,
            Some(AppliedAdjustment {
                discount_cents: 9_999,
                tax_cents: 0,
            }),
        );
        assert_eq!(t.discount_cents, 100);
        assert_eq!(t.total_cents, 0);
    }

    #[test]
    fn tax_is_floored_at_zero() {
        let items = vec![item(1, 100)];
        let t = compute_totals(
            &items,
            Some(AppliedAdjustment {
                discount_cents: 0,
                tax_cents: -50,
            }),
        );
        assert_eq!(t.tax_cents, 0);
        assert_eq!(t.total_cents, 100);
    }

    #[test]
    fn total_never_goes_negative() {
        let items = vec![item(1, 100)];
        let t = compute_totals(
            &items,
            Some(AppliedAdjustment {
                discount_cents: 500,
                tax_cents: 0,
            }),
        );
        assert_eq!(t.total_cents, 0);
    }
}
