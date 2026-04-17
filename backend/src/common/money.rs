// FDN-06 utility module. Items here are consumed by later Phase 4 subsystems
// (checkout, coupons, subscription-change) — they exist ahead of their
// callers by design. `dead_code` is silenced at the module level rather than
// per-item to keep the file readable; the unit tests exercise every public
// surface.
#![allow(dead_code)]

//! Integer-minor-unit money type.
//!
//! [`Money`] is an `i64` newtype measured in minor currency units (e.g. US cents).
//! All arithmetic is integer — there is deliberately no `f64` anywhere in this
//! file or its public API. Consumers that need decimal display call
//! [`Money::format_usd`]; everything else stays in cents.
//!
//! # Overflow contract
//!
//! The raw `Add`/`Sub`/`Mul<u32>` impls panic on overflow (same semantics as
//! the stdlib operators on primitives in debug, wrapping in release). Production
//! code paths should prefer the `checked_*` variants, which return
//! [`MoneyError::Overflow`] and never panic.
//!
//! # Percentages (basis points)
//!
//! [`Money::apply_percent_bps`] multiplies by a basis-point value (`10000 bps = 100%`).
//! Intermediate product is performed in `i128` to avoid overflow. The result is
//! rounded **half-away-from-zero** (a.k.a. "commercial rounding"); a bps value
//! greater than `10_000` is rejected with [`MoneyError::InvalidPercent`]. The
//! `_saturating` variant clamps the input to `[0, 10_000]` and clamps the
//! product to `i64::{MIN, MAX}` rather than erroring.
//!
//! # Serde
//!
//! Serialization is transparent over the inner `i64` — the wire form is the raw
//! minor-unit integer, not a decimal string. That keeps consumers in lock-step
//! with the internal representation and avoids subtle rounding bugs at API edges.

use std::fmt;
use std::ops::{Add, Mul, Sub};
use std::str::FromStr;

use serde::{Deserialize, Serialize};

/// Errors produced by checked [`Money`] arithmetic.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum MoneyError {
    /// Arithmetic would overflow or underflow the underlying `i64`.
    #[error("money arithmetic overflow")]
    Overflow,
    /// Basis-point percentage outside the allowed `[0, 10_000]` range.
    #[error("invalid percent in basis points: {0} (expected 0..=10_000)")]
    InvalidPercent(u32),
    /// Raw-cents [`FromStr`] input was not a valid signed integer.
    #[error("failed to parse money from string: {0}")]
    Parse(String),
}

/// Maximum allowed basis points (= 100%).
const MAX_BPS: u32 = 10_000;

/// A signed minor-unit currency amount.
///
/// Storage: `i64` minor units (e.g. USD cents). The currency is *implicit* —
/// this type is currency-agnostic at the storage level and relies on callers
/// not mixing currencies in a single calculation. Formatting helpers such as
/// [`Money::format_usd`] apply a specific currency's conventions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Money(i64);

impl Money {
    /// Construct from a raw minor-unit amount (e.g. cents).
    #[must_use]
    pub const fn cents(amount: i64) -> Self {
        Self(amount)
    }

    /// The zero amount.
    #[must_use]
    pub const fn zero() -> Self {
        Self(0)
    }

    /// Return the underlying amount in minor units.
    #[must_use]
    pub const fn as_cents(&self) -> i64 {
        self.0
    }

    /// Checked addition. Returns [`MoneyError::Overflow`] on wrap.
    pub fn checked_add(self, rhs: Self) -> Result<Self, MoneyError> {
        self.0
            .checked_add(rhs.0)
            .map(Self)
            .ok_or(MoneyError::Overflow)
    }

    /// Checked subtraction. Returns [`MoneyError::Overflow`] on wrap.
    pub fn checked_sub(self, rhs: Self) -> Result<Self, MoneyError> {
        self.0
            .checked_sub(rhs.0)
            .map(Self)
            .ok_or(MoneyError::Overflow)
    }

    /// Checked scalar multiplication. Returns [`MoneyError::Overflow`] on wrap.
    pub fn checked_mul(self, rhs: u32) -> Result<Self, MoneyError> {
        self.0
            .checked_mul(i64::from(rhs))
            .map(Self)
            .ok_or(MoneyError::Overflow)
    }

    /// Apply a basis-point percentage using integer arithmetic.
    ///
    /// * `bps` is in basis points; `10_000 bps = 100%`.
    /// * Values `> 10_000` yield [`MoneyError::InvalidPercent`].
    /// * Rounding is half-away-from-zero.
    /// * Intermediate multiplication happens in `i128`, so no intermediate
    ///   overflow is possible for any combination of `i64` and `u32` inputs;
    ///   only the final `i128 -> i64` truncation can overflow, in which case
    ///   [`MoneyError::Overflow`] is returned.
    pub fn apply_percent_bps(self, bps: u32) -> Result<Self, MoneyError> {
        if bps > MAX_BPS {
            return Err(MoneyError::InvalidPercent(bps));
        }
        let product: i128 = i128::from(self.0) * i128::from(bps);
        let rounded = div_round_half_away_from_zero(product, i128::from(MAX_BPS));
        i64::try_from(rounded)
            .map(Self)
            .map_err(|_| MoneyError::Overflow)
    }

    /// Same as [`apply_percent_bps`](Self::apply_percent_bps) but clamps
    /// `bps` to `[0, 10_000]` and saturates the product into `i64` rather than
    /// erroring. Intended for display-only paths.
    #[must_use]
    pub fn apply_percent_bps_saturating(self, bps: u32) -> Self {
        let clamped = bps.min(MAX_BPS);
        let product: i128 = i128::from(self.0) * i128::from(clamped);
        let rounded = div_round_half_away_from_zero(product, i128::from(MAX_BPS));
        let saturated = if rounded > i128::from(i64::MAX) {
            i64::MAX
        } else if rounded < i128::from(i64::MIN) {
            i64::MIN
        } else {
            // Guarded above, safe to narrow.
            rounded as i64
        };
        Self(saturated)
    }

    /// Format as a US dollar amount with `$`, two decimals, thousands
    /// separators, and a leading `-` for negatives.
    ///
    /// Examples: `Money::cents(0).format_usd() == "$0.00"`,
    /// `Money::cents(-12345).format_usd() == "-$123.45"`,
    /// `Money::cents(1234567).format_usd() == "$12,345.67"`.
    #[must_use]
    pub fn format_usd(&self) -> String {
        let negative = self.0 < 0;
        // `i64::MIN.unsigned_abs()` is `i64::MAX as u64 + 1`, which fits in
        // `u64`, so we avoid using `abs()` (which can overflow).
        let magnitude: u64 = self.0.unsigned_abs();
        let dollars = magnitude / 100;
        let cents = magnitude % 100;
        let dollars_str = group_thousands(dollars);
        if negative {
            format!("-${dollars_str}.{cents:02}")
        } else {
            format!("${dollars_str}.{cents:02}")
        }
    }
}

/// `Display` prints raw minor units, e.g. `Money::cents(123)` -> `"123"`.
///
/// This is intentional: [`Display`](fmt::Display) is the wire-friendly view
/// (stable, locale-independent). Use [`Money::format_usd`] for human output.
impl fmt::Display for Money {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// `FromStr` parses raw minor units only (no `$`, no decimal point).
impl FromStr for Money {
    type Err = MoneyError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.parse::<i64>()
            .map(Self)
            .map_err(|e| MoneyError::Parse(e.to_string()))
    }
}

impl Add for Money {
    type Output = Self;
    fn add(self, rhs: Self) -> Self {
        Self(self.0 + rhs.0)
    }
}

impl Sub for Money {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self {
        Self(self.0 - rhs.0)
    }
}

impl Mul<u32> for Money {
    type Output = Self;
    fn mul(self, rhs: u32) -> Self {
        Self(self.0 * i64::from(rhs))
    }
}

/// Insert thousands separators into a non-negative integer's decimal form.
fn group_thousands(mut n: u64) -> String {
    if n == 0 {
        return "0".to_owned();
    }
    let mut groups: Vec<String> = Vec::new();
    while n > 0 {
        let chunk = n % 1000;
        n /= 1000;
        if n == 0 {
            groups.push(format!("{chunk}"));
        } else {
            groups.push(format!("{chunk:03}"));
        }
    }
    groups.reverse();
    groups.join(",")
}

/// Divide `num / den` rounding half-away-from-zero.
///
/// `den` must be positive; callers pass a constant divisor (`MAX_BPS = 10_000`).
fn div_round_half_away_from_zero(num: i128, den: i128) -> i128 {
    debug_assert!(den > 0, "denominator must be positive");
    let half = den / 2;
    if num >= 0 {
        (num + half) / den
    } else {
        (num - half) / den
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_and_cents_constructors() {
        assert_eq!(Money::zero().as_cents(), 0);
        assert_eq!(Money::cents(42).as_cents(), 42);
        assert_eq!(Money::cents(-7).as_cents(), -7);
    }

    #[test]
    fn checked_add_overflow() {
        let a = Money::cents(i64::MAX);
        let b = Money::cents(1);
        assert_eq!(a.checked_add(b), Err(MoneyError::Overflow));
    }

    #[test]
    fn checked_add_ok() {
        assert_eq!(
            Money::cents(100).checked_add(Money::cents(250)),
            Ok(Money::cents(350))
        );
    }

    #[test]
    fn checked_sub_underflow() {
        let a = Money::cents(i64::MIN);
        let b = Money::cents(1);
        assert_eq!(a.checked_sub(b), Err(MoneyError::Overflow));
    }

    #[test]
    fn checked_sub_ok() {
        assert_eq!(
            Money::cents(500).checked_sub(Money::cents(125)),
            Ok(Money::cents(375))
        );
    }

    #[test]
    fn checked_mul_overflow() {
        assert_eq!(
            Money::cents(i64::MAX).checked_mul(2),
            Err(MoneyError::Overflow)
        );
    }

    #[test]
    fn checked_mul_ok() {
        assert_eq!(Money::cents(25).checked_mul(4), Ok(Money::cents(100)));
        assert_eq!(Money::cents(-25).checked_mul(3), Ok(Money::cents(-75)));
    }

    #[test]
    fn percent_bps_rejects_over_100() {
        assert_eq!(
            Money::cents(100).apply_percent_bps(10_001),
            Err(MoneyError::InvalidPercent(10_001))
        );
    }

    #[test]
    fn percent_bps_full_and_zero() {
        assert_eq!(
            Money::cents(1234).apply_percent_bps(10_000),
            Ok(Money::cents(1234))
        );
        assert_eq!(Money::cents(1234).apply_percent_bps(0), Ok(Money::zero()));
    }

    #[test]
    fn percent_bps_half_away_from_zero_positive() {
        // 100 * 1234 / 10000 = 12.34 -> 12
        assert_eq!(
            Money::cents(100).apply_percent_bps(1234),
            Ok(Money::cents(12))
        );
        // 100 * 1250 / 10000 = 12.5 -> 13 (half-away)
        assert_eq!(
            Money::cents(100).apply_percent_bps(1250),
            Ok(Money::cents(13))
        );
        // 100 * 1249 / 10000 = 12.49 -> 12
        assert_eq!(
            Money::cents(100).apply_percent_bps(1249),
            Ok(Money::cents(12))
        );
    }

    #[test]
    fn percent_bps_half_away_from_zero_negative() {
        // -100 * 1250 / 10000 = -12.5 -> -13 (away from zero)
        assert_eq!(
            Money::cents(-100).apply_percent_bps(1250),
            Ok(Money::cents(-13))
        );
        // -100 * 1249 / 10000 = -12.49 -> -12
        assert_eq!(
            Money::cents(-100).apply_percent_bps(1249),
            Ok(Money::cents(-12))
        );
    }

    #[test]
    fn percent_bps_saturating_clamps_bps() {
        // Clamped to 10_000, so result equals input.
        assert_eq!(
            Money::cents(1234).apply_percent_bps_saturating(99_999),
            Money::cents(1234)
        );
    }

    #[test]
    fn percent_bps_saturating_result_cannot_overflow() {
        // i64::MAX * 10_000 overflows i64 but is clamped to i64::MAX.
        assert_eq!(
            Money::cents(i64::MAX).apply_percent_bps_saturating(10_000),
            Money::cents(i64::MAX)
        );
    }

    #[test]
    fn format_usd_positive_with_separators() {
        assert_eq!(Money::cents(0).format_usd(), "$0.00");
        assert_eq!(Money::cents(9).format_usd(), "$0.09");
        assert_eq!(Money::cents(99).format_usd(), "$0.99");
        assert_eq!(Money::cents(100).format_usd(), "$1.00");
        assert_eq!(Money::cents(12_345).format_usd(), "$123.45");
        assert_eq!(Money::cents(1_234_567).format_usd(), "$12,345.67");
        assert_eq!(Money::cents(1_000_000_000).format_usd(), "$10,000,000.00");
    }

    #[test]
    fn format_usd_negative() {
        assert_eq!(Money::cents(-1).format_usd(), "-$0.01");
        assert_eq!(Money::cents(-12_345).format_usd(), "-$123.45");
        assert_eq!(Money::cents(-1_234_567).format_usd(), "-$12,345.67");
    }

    #[test]
    fn format_usd_extremes() {
        // These must not panic — we avoid i64::abs() which can overflow.
        let _ = Money::cents(i64::MIN).format_usd();
        let _ = Money::cents(i64::MAX).format_usd();
    }

    #[test]
    fn display_prints_raw_cents() {
        assert_eq!(Money::cents(12_345).to_string(), "12345");
        assert_eq!(Money::cents(-42).to_string(), "-42");
    }

    #[test]
    fn from_str_happy() {
        let m: Money = "12345".parse().unwrap();
        assert_eq!(m, Money::cents(12345));
        let m: Money = "-7".parse().unwrap();
        assert_eq!(m, Money::cents(-7));
    }

    #[test]
    fn from_str_rejects_decimal_and_currency() {
        assert!("12.34".parse::<Money>().is_err());
        assert!("$12".parse::<Money>().is_err());
        assert!("abc".parse::<Money>().is_err());
        assert!("".parse::<Money>().is_err());
    }

    #[test]
    fn serde_transparent_round_trip() {
        let m = Money::cents(12_345);
        let json = serde_json::to_string(&m).unwrap();
        assert_eq!(json, "12345");
        let back: Money = serde_json::from_str(&json).unwrap();
        assert_eq!(back, m);
    }

    #[test]
    fn serde_negative_round_trip() {
        let m = Money::cents(-98_765);
        let json = serde_json::to_string(&m).unwrap();
        assert_eq!(json, "-98765");
        let back: Money = serde_json::from_str(&json).unwrap();
        assert_eq!(back, m);
    }

    #[test]
    fn operator_overloads_work_on_in_range_values() {
        assert_eq!(Money::cents(10) + Money::cents(5), Money::cents(15));
        assert_eq!(Money::cents(10) - Money::cents(5), Money::cents(5));
        assert_eq!(Money::cents(10) * 3u32, Money::cents(30));
    }
}
