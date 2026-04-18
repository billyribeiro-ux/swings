//! EC-11 coupon engine.
//!
//! Deterministic, pure pricing logic. The engine never touches the database;
//! callers hydrate a [`CouponInput`] from the `coupons` row and a
//! `Vec<CartLine>` from wherever they built the cart (checkout, preview, the
//! `public_validate_coupon` handler, etc.) and receive an [`AppliedCoupon`]
//! back.
//!
//! # Arithmetic guarantees
//! * All money math flows through [`Money`] — integer cents, no `f64`.
//! * Percentage discounts use basis points (`10_000 bps = 100%`) and the
//!   half-away-from-zero rounding [`Money::apply_percent_bps`] enforces.
//! * The engine never returns a negative discount; it clamps to the cart
//!   subtotal so `subtotal - discount >= 0`.
//!
//! # Subscription vs one-time
//! The engine flags applicability via `AppliedCoupon::applicable_to_recurring`.
//! The checkout + subscription-change subsystems hook that flag into the
//! Stripe coupon / promotion-code passthrough in later tickets; at engine
//! time we only compute it deterministically.

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

impl CouponScope {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            CouponScope::Cart => "cart",
            CouponScope::Product => "product",
            CouponScope::Category => "category",
            CouponScope::Subscription => "subscription",
        }
    }

    pub fn from_str_lower(s: &str) -> Option<Self> {
        Some(match s {
            "cart" => CouponScope::Cart,
            "product" => CouponScope::Product,
            "category" => CouponScope::Category,
            "subscription" => CouponScope::Subscription,
            _ => return None,
        })
    }
}

/// Subscription billing applicability for a coupon.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum RecurringMode {
    /// Applies to the first invoice only.
    OneTime,
    /// Applies to every billing cycle, forever.
    Forever,
    /// Applies for a fixed number of cycles (managed by the billing layer).
    Repeating,
}

impl RecurringMode {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            RecurringMode::OneTime => "one_time",
            RecurringMode::Forever => "forever",
            RecurringMode::Repeating => "repeating",
        }
    }

    pub fn from_str_lower(s: &str) -> Option<Self> {
        Some(match s {
            "one_time" => RecurringMode::OneTime,
            "forever" => RecurringMode::Forever,
            "repeating" => RecurringMode::Repeating,
            _ => return None,
        })
    }
}

/// Configuration for a "buy N, get M free" promotion.
///
/// The engine groups cart lines matching `applies_to_product_ids` (empty = any
/// product), totals their quantity, and emits `floor(total / (buy + get)) * get`
/// free units, evaluated at the cheapest matching unit price so the customer's
/// "free" units come off the least-expensive line first. That rule mirrors
/// WooCommerce's default BOGO behaviour and avoids the abuse of stacking a
/// high-priced free unit onto a low-priced purchase.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct BogoConfig {
    pub buy_qty: i32,
    pub get_qty: i32,
    #[serde(default)]
    pub applies_to_product_ids: Vec<Uuid>,
}

/// A single line in the pricing cart as seen by the engine.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CartLine {
    pub product_id: Uuid,
    pub variant_id: Option<Uuid>,
    pub category_id: Option<Uuid>,
    pub unit_price: Money,
    pub quantity: i32,
    pub is_subscription: bool,
}

impl CartLine {
    /// Line subtotal — `unit_price × quantity`, saturating at `i64::MAX` on
    /// the arithmetic impossible-in-practice overflow path.
    #[must_use]
    pub fn subtotal(&self) -> Money {
        let q: u32 = u32::try_from(self.quantity.max(0)).unwrap_or(0);
        self.unit_price
            .checked_mul(q)
            .unwrap_or_else(|_| Money::cents(i64::MAX))
    }
}

/// Coupon inputs the engine needs to decide applicability + compute a
/// discount amount. Mirrors the enriched `041_coupons_refactor.sql` columns.
#[derive(Debug, Clone)]
pub struct CouponInput {
    pub code: String,
    pub scope: CouponScope,
    /// Flat amount off. Set at most one of this vs `discount_percent_bps`.
    pub discount_value_cents: Option<Money>,
    /// Percentage off in basis points (`10_000 bps = 100%`).
    pub discount_percent_bps: Option<u32>,
    /// Optional cap on the percentage discount (display-only; the engine
    /// honours it by clamping).
    pub max_discount: Option<Money>,
    /// Minimum cart subtotal required for the coupon to apply.
    pub min_purchase: Option<Money>,
    /// BOGO config, set only when the coupon is BOGO-shaped.
    pub bogo_config: Option<BogoConfig>,
    /// Product whitelist. Empty means "every product".
    pub includes_product_ids: Vec<Uuid>,
    /// Product exclusion list — wins over the whitelist when a product
    /// appears in both.
    pub excludes_product_ids: Vec<Uuid>,
    /// Category whitelist for Category-scoped coupons. Empty means "every
    /// category".
    pub includes_category_ids: Vec<Uuid>,
    pub recurring_mode: RecurringMode,
}

/// Result of applying a coupon to a cart.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppliedCoupon {
    pub discount: Money,
    /// True when at least one of the matched lines is a subscription and the
    /// coupon's `recurring_mode` permits recurring charges. Billing layer
    /// reads this to decide whether to attach the coupon to the subscription
    /// vs. to the first invoice only.
    pub applicable_to_recurring: bool,
    pub reason: AppliedReason,
}

/// Why the engine produced the result — useful for display + audit logs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppliedReason {
    /// Flat amount off the cart subtotal (`CouponScope::Cart`).
    FixedCart,
    /// Percentage off the cart subtotal, optionally capped.
    PercentCart,
    /// Percentage / flat off only matching product lines.
    ProductScoped,
    /// Percentage / flat off only matching category lines.
    CategoryScoped,
    /// BOGO evaluation.
    Bogo,
    /// Flat amount off subscription lines only.
    SubscriptionScoped,
    /// Below the minimum-purchase threshold.
    BelowMinimum,
    /// No lines matched the include / exclude filter.
    NoApplicableLines,
    /// Empty cart.
    EmptyCart,
    /// Coupon is missing both fixed + percent fields.
    Misconfigured,
}

/// Engine entry point.
pub struct CouponEngine;

impl CouponEngine {
    /// Evaluate `coupon` against `cart`. The function is pure; call sites
    /// can unit-test it against any shape without a DB.
    #[must_use]
    pub fn apply(cart: &[CartLine], coupon: &CouponInput) -> AppliedCoupon {
        if cart.is_empty() {
            return zero(AppliedReason::EmptyCart);
        }

        let subtotal = cart_subtotal(cart);

        if let Some(min) = coupon.min_purchase {
            if subtotal < min {
                return zero(AppliedReason::BelowMinimum);
            }
        }

        match coupon.scope {
            CouponScope::Cart => apply_cart(cart, coupon, subtotal),
            CouponScope::Product => apply_product_scoped(cart, coupon),
            CouponScope::Category => apply_category_scoped(cart, coupon),
            CouponScope::Subscription => apply_subscription(cart, coupon),
        }
    }
}

// ── Helpers ────────────────────────────────────────────────────────────

fn zero(reason: AppliedReason) -> AppliedCoupon {
    AppliedCoupon {
        discount: Money::zero(),
        applicable_to_recurring: false,
        reason,
    }
}

/// Sum a cart's line subtotals with saturating arithmetic.
fn cart_subtotal(cart: &[CartLine]) -> Money {
    cart.iter()
        .fold(Money::zero(), |acc, line| acc + line.subtotal())
}

/// Evaluate whether the coupon permits recurring application for a matching
/// line. `OneTime` means the engine still surfaces the discount but flags it
/// as not applicable to the recurring invoice.
fn permits_recurring(mode: RecurringMode) -> bool {
    matches!(mode, RecurringMode::Forever | RecurringMode::Repeating)
}

/// Does the line participate in `coupon`'s product filter?
fn product_matches(coupon: &CouponInput, line: &CartLine) -> bool {
    if coupon.excludes_product_ids.contains(&line.product_id) {
        return false;
    }
    coupon.includes_product_ids.is_empty() || coupon.includes_product_ids.contains(&line.product_id)
}

/// Does the line participate in `coupon`'s category filter?
fn category_matches(coupon: &CouponInput, line: &CartLine) -> bool {
    if coupon.excludes_product_ids.contains(&line.product_id) {
        return false;
    }
    let Some(cat) = line.category_id else {
        return false;
    };
    coupon.includes_category_ids.is_empty() || coupon.includes_category_ids.contains(&cat)
}

/// BOGO-specific line match: use bogo_config.applies_to_product_ids when set,
/// otherwise fall back to the coupon-wide include/exclude filter.
fn bogo_matches(bogo: &BogoConfig, coupon: &CouponInput, line: &CartLine) -> bool {
    if !bogo.applies_to_product_ids.is_empty() {
        return bogo.applies_to_product_ids.contains(&line.product_id);
    }
    product_matches(coupon, line)
}

/// Clamp `discount` to the total matching subtotal so the discount never
/// exceeds what the customer is paying.
fn clamp_to_subtotal(discount: Money, subtotal: Money) -> Money {
    if discount > subtotal {
        subtotal
    } else {
        discount
    }
}

/// Apply a cart-wide discount (fixed or percentage) to the full subtotal.
fn apply_cart(cart: &[CartLine], coupon: &CouponInput, subtotal: Money) -> AppliedCoupon {
    // BOGO takes precedence when configured — even when `scope=cart` we can't
    // ignore a buy-N/get-M rule.
    if let Some(bogo) = &coupon.bogo_config {
        return apply_bogo(cart, coupon, bogo);
    }
    if let Some(flat) = coupon.discount_value_cents {
        let discount = clamp_to_subtotal(flat, subtotal);
        let recurring =
            cart.iter().any(|l| l.is_subscription) && permits_recurring(coupon.recurring_mode);
        return AppliedCoupon {
            discount,
            applicable_to_recurring: recurring,
            reason: AppliedReason::FixedCart,
        };
    }
    if let Some(bps) = coupon.discount_percent_bps {
        let raw = subtotal
            .apply_percent_bps(bps)
            .unwrap_or_else(|_| Money::zero());
        let capped = match coupon.max_discount {
            Some(max) if raw > max => max,
            _ => raw,
        };
        let discount = clamp_to_subtotal(capped, subtotal);
        let recurring =
            cart.iter().any(|l| l.is_subscription) && permits_recurring(coupon.recurring_mode);
        return AppliedCoupon {
            discount,
            applicable_to_recurring: recurring,
            reason: AppliedReason::PercentCart,
        };
    }
    zero(AppliedReason::Misconfigured)
}

/// Product-scoped: discount applies only to the matching lines' subtotal.
fn apply_product_scoped(cart: &[CartLine], coupon: &CouponInput) -> AppliedCoupon {
    if let Some(bogo) = &coupon.bogo_config {
        return apply_bogo(cart, coupon, bogo);
    }
    let matched_subtotal = cart
        .iter()
        .filter(|l| product_matches(coupon, l))
        .fold(Money::zero(), |acc, l| acc + l.subtotal());
    if matched_subtotal == Money::zero() {
        return zero(AppliedReason::NoApplicableLines);
    }
    let raw = match (coupon.discount_percent_bps, coupon.discount_value_cents) {
        (Some(bps), _) => matched_subtotal
            .apply_percent_bps(bps)
            .unwrap_or_else(|_| Money::zero()),
        (None, Some(flat)) => flat,
        _ => return zero(AppliedReason::Misconfigured),
    };
    let capped = match coupon.max_discount {
        Some(max) if raw > max => max,
        _ => raw,
    };
    let discount = clamp_to_subtotal(capped, matched_subtotal);
    let recurring = cart
        .iter()
        .any(|l| l.is_subscription && product_matches(coupon, l))
        && permits_recurring(coupon.recurring_mode);
    AppliedCoupon {
        discount,
        applicable_to_recurring: recurring,
        reason: AppliedReason::ProductScoped,
    }
}

/// Category-scoped: discount applies only to lines whose `category_id` matches.
fn apply_category_scoped(cart: &[CartLine], coupon: &CouponInput) -> AppliedCoupon {
    let matched_subtotal = cart
        .iter()
        .filter(|l| category_matches(coupon, l))
        .fold(Money::zero(), |acc, l| acc + l.subtotal());
    if matched_subtotal == Money::zero() {
        return zero(AppliedReason::NoApplicableLines);
    }
    let raw = match (coupon.discount_percent_bps, coupon.discount_value_cents) {
        (Some(bps), _) => matched_subtotal
            .apply_percent_bps(bps)
            .unwrap_or_else(|_| Money::zero()),
        (None, Some(flat)) => flat,
        _ => return zero(AppliedReason::Misconfigured),
    };
    let capped = match coupon.max_discount {
        Some(max) if raw > max => max,
        _ => raw,
    };
    let discount = clamp_to_subtotal(capped, matched_subtotal);
    let recurring = cart
        .iter()
        .any(|l| l.is_subscription && category_matches(coupon, l))
        && permits_recurring(coupon.recurring_mode);
    AppliedCoupon {
        discount,
        applicable_to_recurring: recurring,
        reason: AppliedReason::CategoryScoped,
    }
}

/// Subscription-scoped: discount only on subscription lines.
fn apply_subscription(cart: &[CartLine], coupon: &CouponInput) -> AppliedCoupon {
    let matched_subtotal = cart
        .iter()
        .filter(|l| l.is_subscription && product_matches(coupon, l))
        .fold(Money::zero(), |acc, l| acc + l.subtotal());
    if matched_subtotal == Money::zero() {
        return zero(AppliedReason::NoApplicableLines);
    }
    let raw = match (coupon.discount_percent_bps, coupon.discount_value_cents) {
        (Some(bps), _) => matched_subtotal
            .apply_percent_bps(bps)
            .unwrap_or_else(|_| Money::zero()),
        (None, Some(flat)) => flat,
        _ => return zero(AppliedReason::Misconfigured),
    };
    let capped = match coupon.max_discount {
        Some(max) if raw > max => max,
        _ => raw,
    };
    let discount = clamp_to_subtotal(capped, matched_subtotal);
    AppliedCoupon {
        discount,
        applicable_to_recurring: permits_recurring(coupon.recurring_mode),
        reason: AppliedReason::SubscriptionScoped,
    }
}

/// BOGO evaluation — buy N of X, get M free.
///
/// Strategy:
///   1. Filter cart to BOGO-matching lines.
///   2. Expand each line into its individual units (`unit_price` repeated
///      `quantity` times). The expansion is bounded by the sum of quantities
///      in the cart, which the caller already validates.
///   3. Sort units ascending by price.
///   4. For every complete `(buy + get)` group, apply the `get` cheapest
///      units as free (discount = sum of those unit prices).
fn apply_bogo(cart: &[CartLine], coupon: &CouponInput, bogo: &BogoConfig) -> AppliedCoupon {
    if bogo.buy_qty <= 0 || bogo.get_qty <= 0 {
        return zero(AppliedReason::Misconfigured);
    }
    let mut units: Vec<Money> = Vec::new();
    let mut touched_subscription = false;
    for line in cart.iter().filter(|l| bogo_matches(bogo, coupon, l)) {
        if line.is_subscription {
            touched_subscription = true;
        }
        // Clamp quantity to a safe upper bound — the caller validates too,
        // but the engine should not allocate unboundedly on a hostile input.
        let qty = line.quantity.clamp(0, 10_000);
        for _ in 0..qty {
            units.push(line.unit_price);
        }
    }
    if units.is_empty() {
        return zero(AppliedReason::NoApplicableLines);
    }
    units.sort();

    let group_size = bogo.buy_qty + bogo.get_qty;
    // i32 arithmetic is fine here — group_size is small, units.len() is bounded above.
    let unit_count = i32::try_from(units.len()).unwrap_or(i32::MAX);
    let complete_groups = unit_count / group_size;
    let free_units = complete_groups * bogo.get_qty;
    if free_units <= 0 {
        return zero(AppliedReason::NoApplicableLines);
    }

    let take = usize::try_from(free_units).unwrap_or(0).min(units.len());
    let discount = units
        .iter()
        .take(take)
        .fold(Money::zero(), |acc, m| acc + *m);

    AppliedCoupon {
        discount,
        applicable_to_recurring: touched_subscription && permits_recurring(coupon.recurring_mode),
        reason: AppliedReason::Bogo,
    }
}

// ══════════════════════════════════════════════════════════════════════
// TESTS
// ══════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    fn pid(s: &str) -> Uuid {
        Uuid::parse_str(s).unwrap()
    }

    fn base_coupon() -> CouponInput {
        CouponInput {
            code: "TEST".into(),
            scope: CouponScope::Cart,
            discount_value_cents: None,
            discount_percent_bps: None,
            max_discount: None,
            min_purchase: None,
            bogo_config: None,
            includes_product_ids: vec![],
            excludes_product_ids: vec![],
            includes_category_ids: vec![],
            recurring_mode: RecurringMode::OneTime,
        }
    }

    fn line(product: Uuid, unit_cents: i64, qty: i32) -> CartLine {
        CartLine {
            product_id: product,
            variant_id: None,
            category_id: None,
            unit_price: Money::cents(unit_cents),
            quantity: qty,
            is_subscription: false,
        }
    }

    fn sub_line(product: Uuid, unit_cents: i64, qty: i32) -> CartLine {
        CartLine {
            product_id: product,
            variant_id: None,
            category_id: None,
            unit_price: Money::cents(unit_cents),
            quantity: qty,
            is_subscription: true,
        }
    }

    const P1: &str = "11111111-1111-1111-1111-111111111111";
    const P2: &str = "22222222-2222-2222-2222-222222222222";
    const P3: &str = "33333333-3333-3333-3333-333333333333";
    const C1: &str = "aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa";
    const C2: &str = "bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb";

    #[test]
    fn empty_cart_yields_empty_cart_reason() {
        let c = base_coupon();
        let result = CouponEngine::apply(&[], &c);
        assert_eq!(result.discount, Money::zero());
        assert_eq!(result.reason, AppliedReason::EmptyCart);
        assert!(!result.applicable_to_recurring);
    }

    #[test]
    fn fixed_cart_discount_with_clamp() {
        // Cart subtotal $50, flat $75 coupon → clamp to $50.
        let mut c = base_coupon();
        c.discount_value_cents = Some(Money::cents(7_500));
        let cart = vec![line(pid(P1), 5_000, 1)];
        let result = CouponEngine::apply(&cart, &c);
        assert_eq!(result.discount, Money::cents(5_000));
        assert_eq!(result.reason, AppliedReason::FixedCart);
    }

    #[test]
    fn percent_cart_discount_with_cap() {
        // $200 @ 25% = $50, cap is $30 → discount should be $30.
        let mut c = base_coupon();
        c.discount_percent_bps = Some(2_500);
        c.max_discount = Some(Money::cents(3_000));
        let cart = vec![line(pid(P1), 20_000, 1)];
        let result = CouponEngine::apply(&cart, &c);
        assert_eq!(result.discount, Money::cents(3_000));
        assert_eq!(result.reason, AppliedReason::PercentCart);
    }

    #[test]
    fn below_minimum_blocks_discount() {
        let mut c = base_coupon();
        c.discount_value_cents = Some(Money::cents(1_000));
        c.min_purchase = Some(Money::cents(10_000));
        let cart = vec![line(pid(P1), 5_000, 1)];
        let result = CouponEngine::apply(&cart, &c);
        assert_eq!(result.discount, Money::zero());
        assert_eq!(result.reason, AppliedReason::BelowMinimum);
    }

    #[test]
    fn product_scope_filters_by_includes() {
        // Only P1 qualifies; 25% off just P1's line = $25 off $100.
        let mut c = base_coupon();
        c.scope = CouponScope::Product;
        c.discount_percent_bps = Some(2_500);
        c.includes_product_ids = vec![pid(P1)];
        let cart = vec![line(pid(P1), 10_000, 1), line(pid(P2), 5_000, 1)];
        let result = CouponEngine::apply(&cart, &c);
        assert_eq!(result.discount, Money::cents(2_500));
        assert_eq!(result.reason, AppliedReason::ProductScoped);
    }

    #[test]
    fn excludes_win_over_includes() {
        // P2 is both whitelisted AND blacklisted → exclusion wins.
        let mut c = base_coupon();
        c.scope = CouponScope::Product;
        c.discount_percent_bps = Some(5_000); // 50%
        c.includes_product_ids = vec![pid(P1), pid(P2)];
        c.excludes_product_ids = vec![pid(P2)];
        let cart = vec![line(pid(P1), 4_000, 1), line(pid(P2), 10_000, 1)];
        let result = CouponEngine::apply(&cart, &c);
        // Only P1 counts → 50% of $40 = $20.
        assert_eq!(result.discount, Money::cents(2_000));
        assert_eq!(result.reason, AppliedReason::ProductScoped);
    }

    #[test]
    fn product_scope_with_no_matching_lines() {
        let mut c = base_coupon();
        c.scope = CouponScope::Product;
        c.discount_percent_bps = Some(2_500);
        c.includes_product_ids = vec![pid(P3)]; // nothing in cart matches
        let cart = vec![line(pid(P1), 10_000, 1)];
        let result = CouponEngine::apply(&cart, &c);
        assert_eq!(result.discount, Money::zero());
        assert_eq!(result.reason, AppliedReason::NoApplicableLines);
    }

    #[test]
    fn category_scope_filters_by_category() {
        let mut c = base_coupon();
        c.scope = CouponScope::Category;
        c.discount_percent_bps = Some(1_000); // 10%
        c.includes_category_ids = vec![pid(C1)];
        let cart = vec![
            CartLine {
                product_id: pid(P1),
                variant_id: None,
                category_id: Some(pid(C1)),
                unit_price: Money::cents(10_000),
                quantity: 1,
                is_subscription: false,
            },
            CartLine {
                product_id: pid(P2),
                variant_id: None,
                category_id: Some(pid(C2)),
                unit_price: Money::cents(20_000),
                quantity: 1,
                is_subscription: false,
            },
        ];
        let result = CouponEngine::apply(&cart, &c);
        // 10% of $100 only (the C2 line is not included).
        assert_eq!(result.discount, Money::cents(1_000));
        assert_eq!(result.reason, AppliedReason::CategoryScoped);
    }

    #[test]
    fn bogo_buy_one_get_one_evaluates_cheapest_free() {
        // Buy 1, get 1 free on P1. Cart: 3×$50. Groups of 2 → 1 complete group → 1 free unit.
        let mut c = base_coupon();
        c.scope = CouponScope::Product;
        c.bogo_config = Some(BogoConfig {
            buy_qty: 1,
            get_qty: 1,
            applies_to_product_ids: vec![pid(P1)],
        });
        c.includes_product_ids = vec![pid(P1)];
        let cart = vec![line(pid(P1), 5_000, 3)];
        let result = CouponEngine::apply(&cart, &c);
        assert_eq!(result.discount, Money::cents(5_000));
        assert_eq!(result.reason, AppliedReason::Bogo);
    }

    #[test]
    fn bogo_threshold_requires_complete_group() {
        // Buy 2, get 1: need at least 3 units to trigger. Cart has only 2.
        let mut c = base_coupon();
        c.scope = CouponScope::Product;
        c.bogo_config = Some(BogoConfig {
            buy_qty: 2,
            get_qty: 1,
            applies_to_product_ids: vec![],
        });
        let cart = vec![line(pid(P1), 3_000, 2)];
        let result = CouponEngine::apply(&cart, &c);
        assert_eq!(result.discount, Money::zero());
        assert_eq!(result.reason, AppliedReason::NoApplicableLines);
    }

    #[test]
    fn bogo_cheapest_free_across_mixed_prices() {
        // BOGO 1+1; cart has 1×$100 and 3×$10. Groups of 2 over 4 units = 2 groups → 2 free.
        // The engine sorts ascending, so the two cheapest ($10 + $10) are free.
        let mut c = base_coupon();
        c.scope = CouponScope::Product;
        c.bogo_config = Some(BogoConfig {
            buy_qty: 1,
            get_qty: 1,
            applies_to_product_ids: vec![],
        });
        let cart = vec![line(pid(P1), 10_000, 1), line(pid(P2), 1_000, 3)];
        let result = CouponEngine::apply(&cart, &c);
        assert_eq!(result.discount, Money::cents(2_000));
        assert_eq!(result.reason, AppliedReason::Bogo);
    }

    #[test]
    fn recurring_mode_forever_sets_flag_on_subscription_line() {
        let mut c = base_coupon();
        c.scope = CouponScope::Subscription;
        c.discount_percent_bps = Some(1_000);
        c.recurring_mode = RecurringMode::Forever;
        let cart = vec![sub_line(pid(P1), 10_000, 1)];
        let result = CouponEngine::apply(&cart, &c);
        assert_eq!(result.discount, Money::cents(1_000));
        assert!(result.applicable_to_recurring);
    }

    #[test]
    fn recurring_mode_one_time_unsets_recurring_flag() {
        let mut c = base_coupon();
        c.scope = CouponScope::Cart;
        c.discount_percent_bps = Some(1_000);
        c.recurring_mode = RecurringMode::OneTime;
        let cart = vec![sub_line(pid(P1), 10_000, 1)];
        let result = CouponEngine::apply(&cart, &c);
        assert_eq!(result.discount, Money::cents(1_000));
        assert!(!result.applicable_to_recurring);
    }

    #[test]
    fn percent_uses_money_half_away_rounding() {
        // 12.34% on 12.34 cents isn't quite meaningful — use 100 cents × 1250 bps.
        // 100 × 1250 / 10_000 = 12.5 → 13 (half-away-from-zero per Money::apply_percent_bps).
        let mut c = base_coupon();
        c.discount_percent_bps = Some(1_250);
        let cart = vec![line(pid(P1), 100, 1)];
        let result = CouponEngine::apply(&cart, &c);
        assert_eq!(result.discount, Money::cents(13));
    }

    #[test]
    fn scope_enum_round_trip() {
        for s in [
            CouponScope::Cart,
            CouponScope::Product,
            CouponScope::Category,
            CouponScope::Subscription,
        ] {
            assert_eq!(CouponScope::from_str_lower(s.as_str()), Some(s));
        }
    }

    #[test]
    fn recurring_mode_round_trip() {
        for r in [
            RecurringMode::OneTime,
            RecurringMode::Forever,
            RecurringMode::Repeating,
        ] {
            assert_eq!(RecurringMode::from_str_lower(r.as_str()), Some(r));
        }
    }

    #[test]
    fn misconfigured_coupon_returns_zero() {
        // Cart-scope coupon missing both fixed + percent fields.
        let c = base_coupon();
        let cart = vec![line(pid(P1), 1_000, 1)];
        let result = CouponEngine::apply(&cart, &c);
        assert_eq!(result.discount, Money::zero());
        assert_eq!(result.reason, AppliedReason::Misconfigured);
    }
}
