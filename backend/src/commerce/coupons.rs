//! EC-11 coupon engine — stub bootstrapped here so the `commerce` module
//! compiles at EC-01 time. The real engine (Money-based pricing, BOGO,
//! category scope, recurring flagging) lands in EC-11 on the same branch.
//!
//! The module file exists so `commerce::mod.rs` can expose a stable module
//! path while EC-01 is the only landed ticket.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::common::money::Money;

/// Discount scope — where on the cart the discount applies.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum CouponScope {
    Cart,
    Product,
    Category,
    Subscription,
}

/// Subscription billing applicability for a coupon.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum RecurringMode {
    OneTime,
    Forever,
    Repeating,
}

/// Configuration for a "buy N, get M free" promotion. EC-11 fills in the
/// engine-side evaluation.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct BogoConfig {
    pub buy_qty: i32,
    pub get_qty: i32,
    #[serde(default)]
    pub applies_to_product_ids: Vec<Uuid>,
}

/// A single line in the pricing cart as seen by the engine. EC-11 fills in
/// the calculation paths; this shape is shared so the handler layer can
/// already build a request without waiting for the engine implementation.
#[derive(Debug, Clone)]
pub struct CartLine {
    pub product_id: Uuid,
    pub variant_id: Option<Uuid>,
    pub category_id: Option<Uuid>,
    pub unit_price: Money,
    pub quantity: i32,
    pub is_subscription: bool,
}

/// Coupon inputs the engine needs to decide applicability + compute a
/// discount amount. Mirrors the enriched `041_coupons_refactor.sql` columns.
#[derive(Debug, Clone)]
pub struct CouponInput {
    pub code: String,
    pub scope: CouponScope,
    pub discount_value_cents: Option<Money>,
    pub discount_percent_bps: Option<u32>,
    pub max_discount: Option<Money>,
    pub min_purchase: Option<Money>,
    pub bogo_config: Option<BogoConfig>,
    pub includes_product_ids: Vec<Uuid>,
    pub excludes_product_ids: Vec<Uuid>,
    pub includes_category_ids: Vec<Uuid>,
    pub recurring_mode: RecurringMode,
}

/// Result of applying a coupon to a cart.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppliedCoupon {
    pub discount: Money,
    pub applicable_to_recurring: bool,
    pub reason: AppliedReason,
}

/// Why the engine produced this result — useful for display + audit logs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppliedReason {
    /// Flat amount off the cart subtotal.
    FixedCart,
    /// Percentage off the cart subtotal, optionally capped.
    PercentCart,
    /// Percentage off only matching product lines.
    ProductScoped,
    /// BOGO evaluation.
    Bogo,
    /// Below the minimum-purchase threshold.
    BelowMinimum,
    /// No lines matched the include / exclude filter.
    NoApplicableLines,
    /// Empty cart.
    EmptyCart,
}

/// Engine entry point. EC-11 fills in the real `apply` branches; the EC-01
/// drop ships only the types + trait shape so the commerce module compiles
/// and later tickets can land their implementation without churning every
/// module-level export.
pub struct CouponEngine;

impl CouponEngine {
    /// Evaluate `coupon` against `cart`. EC-01 returns a zero-discount
    /// placeholder; EC-11 replaces the body.
    #[must_use]
    pub fn apply(cart: &[CartLine], _coupon: &CouponInput) -> AppliedCoupon {
        if cart.is_empty() {
            return AppliedCoupon {
                discount: Money::zero(),
                applicable_to_recurring: false,
                reason: AppliedReason::EmptyCart,
            };
        }
        // EC-11 will branch on CouponScope + bogo_config + discount_*_cents.
        AppliedCoupon {
            discount: Money::zero(),
            applicable_to_recurring: false,
            reason: AppliedReason::NoApplicableLines,
        }
    }
}
