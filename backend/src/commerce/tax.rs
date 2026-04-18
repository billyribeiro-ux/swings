//! EC-08: Tax engine.
//!
//! Looks up a single rate in `tax_rates` by `(region, class)` and computes
//! the tax portion of a sale in integer cents. The engine is deterministic
//! and round-safe (half-away-from-zero); callers feed it a cart's subtotal
//! + the customer's resolved region and tax class.
//!
//! # Compound rates
//! When `compound = true` the stored rate multiplies against the
//! already-taxed subtotal rather than the pre-tax subtotal. Because we
//! look up one rate at a time the `compound` flag only matters when the
//! caller chains multiple rates; a lone compound rate behaves identically
//! to a non-compound one. The checkout handler is responsible for ordering
//! chained rates so non-compound rates apply first. See the unit tests for
//! the two shapes.
//!
//! # Exemptions
//! `tax_exempt_customers` short-circuits the engine to zero. The exemption
//! is honoured regardless of region / class so a reseller permit covers
//! every line on the cart. Expired rows (`expires_at < NOW()`) are filtered
//! by [`is_exempt`].
//!
//! # Stripe Tax parity
//! Stripe Tax is the path of first resort at checkout. This module exists
//! for fallbacks (Stripe outages, unsupported regions) and for offline /
//! admin-mode invoice rebuilds where we cannot re-call Stripe.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, PgPool};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::error::AppResult;

// ── Types ──────────────────────────────────────────────────────────────

/// A single `tax_rates` row. `rate_bps` is basis points of the tax rate —
/// `1234 = 12.34%`.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct TaxRate {
    pub id: Uuid,
    pub region: String,
    pub rate_bps: i32,
    pub compound: bool,
    pub class: String,
    pub label: String,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
}

/// A `tax_exempt_customers` row. Rows whose `expires_at` is in the past are
/// filtered out by [`is_exempt`] so this is only used by admin listings.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct TaxExemptCustomer {
    pub user_id: Uuid,
    pub vat_id: Option<String>,
    pub reason: Option<String>,
    pub expires_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

// ── Pure compute ───────────────────────────────────────────────────────

/// Compute the tax portion of a sale, in integer minor-units.
///
/// * `subtotal_cents` — pre-tax subtotal. Negative values are clamped to 0.
/// * `rate` — `Some(row)` from [`get_rate`]; `None` yields zero.
/// * `exempt` — customer is exempt (result is always 0).
///
/// Rounding follows half-away-from-zero (`(a * b + 5_000) / 10_000`), which
/// matches the convention the coupon engine uses and is what European VAT
/// authorities accept on printed receipts.
#[must_use]
pub fn compute_tax(subtotal_cents: i64, rate: Option<&TaxRate>, exempt: bool) -> i64 {
    if exempt {
        return 0;
    }
    let Some(rate) = rate else {
        return 0;
    };
    if !rate.is_active || rate.rate_bps <= 0 {
        return 0;
    }
    let base = subtotal_cents.max(0);
    // Wide-width math to prevent `i64` overflow on pathological inputs:
    // `base * rate_bps` fits comfortably in `i128` (max ~ 9e18 × 1e5 ≪ 1.7e38).
    let bps = i128::from(rate.rate_bps);
    let product = i128::from(base) * bps;
    // Half-away-from-zero rounding. Since `base` is non-negative and `bps`
    // is non-negative, we only need the positive-side rounding formula.
    let rounded = (product + 5_000) / 10_000;
    // Cast is safe: `base` is `i64`, multiplying by ≤ 100_000 bps keeps the
    // result below `i64::MAX / 10_000 * 10_000 + 5_000`, well within range
    // for any realistic monetary amount. Saturate defensively.
    i64::try_from(rounded).unwrap_or(i64::MAX)
}

/// Compute tax across a chain of rates, honouring `compound`. The engine
/// walks the rates left-to-right; each non-compound rate multiplies against
/// the original subtotal, each compound rate multiplies against
/// `subtotal + accumulated_tax`. Matches the Quebec QST-on-GST model.
#[must_use]
pub fn compute_tax_chain(subtotal_cents: i64, rates: &[TaxRate], exempt: bool) -> i64 {
    if exempt {
        return 0;
    }
    let base = subtotal_cents.max(0);
    let mut accumulated: i64 = 0;
    for rate in rates {
        let input = if rate.compound {
            base.saturating_add(accumulated)
        } else {
            base
        };
        let portion = compute_tax(input, Some(rate), false);
        accumulated = accumulated.saturating_add(portion);
    }
    accumulated
}

// ── Repository ─────────────────────────────────────────────────────────

/// Look up the active rate for `(region, class)`. Returns `None` when the
/// row is missing or flagged inactive — callers interpret that as "this
/// region has no tax on this class of good" (which is the correct answer
/// for, e.g., digital goods in most US states).
pub async fn get_rate(pool: &PgPool, region: &str, class: &str) -> AppResult<Option<TaxRate>> {
    let row = sqlx::query_as::<_, TaxRate>(
        r#"
        SELECT id, region, rate_bps, compound, class, label, is_active, created_at
        FROM tax_rates
        WHERE region = $1 AND class = $2 AND is_active = TRUE
        "#,
    )
    .bind(region)
    .bind(class)
    .fetch_optional(pool)
    .await?;
    Ok(row)
}

/// List every rate for a region. Useful for admin UI + for compound-chain
/// resolution when a region charges multiple simultaneous taxes (GST + QST
/// in Quebec, CGST + SGST in India).
pub async fn list_rates(pool: &PgPool, region: &str) -> AppResult<Vec<TaxRate>> {
    let rows = sqlx::query_as::<_, TaxRate>(
        r#"
        SELECT id, region, rate_bps, compound, class, label, is_active, created_at
        FROM tax_rates
        WHERE region = $1 AND is_active = TRUE
        ORDER BY compound ASC, class ASC
        "#,
    )
    .bind(region)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

/// List every active rate across every region. Admin-only.
pub async fn list_all_rates(pool: &PgPool) -> AppResult<Vec<TaxRate>> {
    let rows = sqlx::query_as::<_, TaxRate>(
        r#"
        SELECT id, region, rate_bps, compound, class, label, is_active, created_at
        FROM tax_rates
        WHERE is_active = TRUE
        ORDER BY region ASC, class ASC
        "#,
    )
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

/// True iff the user carries an unexpired exemption row.
pub async fn is_exempt(pool: &PgPool, user_id: Uuid) -> AppResult<bool> {
    let exists: Option<bool> = sqlx::query_scalar(
        r#"
        SELECT TRUE FROM tax_exempt_customers
        WHERE user_id = $1 AND (expires_at IS NULL OR expires_at > NOW())
        LIMIT 1
        "#,
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await?;
    Ok(exists.unwrap_or(false))
}

// ── Cart integration ───────────────────────────────────────────────────

/// Compute the tax portion for a cart. The cart module owns the items list;
/// this helper exists here so `cart.rs` stays ignorant of regional VAT
/// rules. The checkout handler calls this with the resolved billing region +
/// the customer's id and passes the result into
/// [`crate::commerce::cart::compute_totals`] as the `tax_cents` adjustment.
///
/// `region` and `class` are already resolved by the caller (checkout maps
/// `billing_address.country / state` → region, and the product's
/// `tax_class` field is read at the line-item layer). This helper does not
/// inspect line items at all — the cart subtotal is the engine input.
pub async fn tax_for_cart(
    pool: &PgPool,
    subtotal_cents: i64,
    user_id: Option<Uuid>,
    region: &str,
    class: &str,
) -> AppResult<i64> {
    let exempt = if let Some(uid) = user_id {
        is_exempt(pool, uid).await?
    } else {
        false
    };
    if exempt {
        return Ok(0);
    }
    let rate = get_rate(pool, region, class).await?;
    Ok(compute_tax(subtotal_cents, rate.as_ref(), false))
}

// ── Unit tests ─────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn rate(region: &str, class: &str, bps: i32, compound: bool) -> TaxRate {
        TaxRate {
            id: Uuid::new_v4(),
            region: region.to_string(),
            rate_bps: bps,
            compound,
            class: class.to_string(),
            label: format!("{region} {class}"),
            is_active: true,
            created_at: Utc::now(),
        }
    }

    #[test]
    fn standard_rate_applies() {
        // 10000 cents × 8.25% = 825.
        let r = rate("US-CA", "standard", 825, false);
        assert_eq!(compute_tax(10_000, Some(&r), false), 825);
    }

    #[test]
    fn exempt_short_circuits_to_zero() {
        let r = rate("US-CA", "standard", 825, false);
        assert_eq!(compute_tax(10_000, Some(&r), true), 0);
    }

    #[test]
    fn missing_rate_yields_zero() {
        assert_eq!(compute_tax(10_000, None, false), 0);
    }

    #[test]
    fn zero_subtotal_yields_zero() {
        let r = rate("DE", "standard", 1900, false);
        assert_eq!(compute_tax(0, Some(&r), false), 0);
    }

    #[test]
    fn negative_subtotal_clamps_to_zero() {
        let r = rate("DE", "standard", 1900, false);
        assert_eq!(compute_tax(-100, Some(&r), false), 0);
    }

    #[test]
    fn inactive_rate_yields_zero() {
        let mut r = rate("GB", "standard", 2000, false);
        r.is_active = false;
        assert_eq!(compute_tax(10_000, Some(&r), false), 0);
    }

    #[test]
    fn rounding_is_half_away_from_zero() {
        // 333 × 1% = 3.33 cents → rounds to 3 (half-away-from-zero over ties;
        // non-tie rounds follow normal banker-free rounding).
        let r = rate("US-NY", "standard", 100, false);
        assert_eq!(compute_tax(333, Some(&r), false), 3);
        // 350 × 1% = 3.5 cents → rounds to 4.
        assert_eq!(compute_tax(350, Some(&r), false), 4);
    }

    #[test]
    fn multiple_classes_are_independent() {
        let standard = rate("GB", "standard", 2000, false);
        let reduced = rate("GB", "reduced", 500, false);
        assert_eq!(compute_tax(10_000, Some(&standard), false), 2_000);
        assert_eq!(compute_tax(10_000, Some(&reduced), false), 500);
    }

    #[test]
    fn compound_chain_applies_on_top() {
        // Quebec: 5% GST (federal, non-compound) + 9.975% QST (provincial,
        // compound in historical models). On $100:
        //   GST  = 500
        //   QST  = (10000 + 500) * 9.975% ≈ 1047.38 → 1047
        //   total tax = 1547
        let gst = rate("QC-CA", "federal", 500, false);
        let qst = rate("QC-CA", "provincial", 998, true);
        let total = compute_tax_chain(10_000, &[gst, qst], false);
        assert_eq!(total, 500 + 1_048);
    }

    #[test]
    fn chain_exempt_short_circuits() {
        let gst = rate("QC-CA", "federal", 500, false);
        let qst = rate("QC-CA", "provincial", 998, true);
        assert_eq!(compute_tax_chain(10_000, &[gst, qst], true), 0);
    }

    #[test]
    fn chain_is_order_independent_for_non_compound_pair() {
        // Two non-compound rates: order does not matter.
        let a = rate("IN", "cgst", 900, false);
        let b = rate("IN", "sgst", 900, false);
        let t1 = compute_tax_chain(10_000, &[a.clone(), b.clone()], false);
        let t2 = compute_tax_chain(10_000, &[b, a], false);
        assert_eq!(t1, t2);
    }
}
