//! POP-02: two-proportion z-test for conversion-rate significance.
//!
//! Given two variants' (impressions, conversions), the test computes the
//! z-score of their difference in conversion rate under the pooled-variance
//! null hypothesis. A two-tailed p-value is returned alongside; the caller
//! decides the significance threshold (typically α = 0.05).
//!
//! The winner, if any, is whichever variant has the higher conversion rate
//! AND a p-value below the supplied alpha. If both are tied, or neither has
//! a statistically-distinguishable advantage, `winner` is `None`.

use serde::Serialize;
use uuid::Uuid;

/// Input sample for a single variant.
#[derive(Debug, Clone, Copy)]
pub struct SamplePoint {
    pub variant_id: Uuid,
    pub impressions: i64,
    pub conversions: i64,
}

/// Result shape returned to admin clients.
#[derive(Debug, Clone, Serialize)]
pub struct TestResult {
    pub variant_a: Uuid,
    pub variant_b: Uuid,
    pub rate_a: f64,
    pub rate_b: f64,
    pub z_score: f64,
    pub p_value: f64,
    pub winner: Option<Uuid>,
}

/// Two-proportion pooled z-test. Returns a [`TestResult`] with:
///
/// * `z_score` — standard normal deviate of `(p_b - p_a)`
/// * `p_value` — two-tailed probability of observing a difference this
///   extreme under the null
/// * `winner`  — variant id whose rate is higher AND `p_value < alpha`
///
/// If either sample has zero impressions, or the pooled variance is zero
/// (both proportions equal), the z-score is `0.0`, the p-value is `1.0`,
/// and `winner` is `None`.
#[must_use]
pub fn two_proportion_z_test(a: SamplePoint, b: SamplePoint, alpha: f64) -> TestResult {
    let na = a.impressions.max(0) as f64;
    let nb = b.impressions.max(0) as f64;
    let xa = a.conversions.max(0) as f64;
    let xb = b.conversions.max(0) as f64;

    let (rate_a, rate_b) = (safe_rate(xa, na), safe_rate(xb, nb));

    if na <= 0.0 || nb <= 0.0 {
        return TestResult {
            variant_a: a.variant_id,
            variant_b: b.variant_id,
            rate_a,
            rate_b,
            z_score: 0.0,
            p_value: 1.0,
            winner: None,
        };
    }

    let p_pool = (xa + xb) / (na + nb);
    let variance = p_pool * (1.0 - p_pool) * (1.0 / na + 1.0 / nb);
    let (z, p) = if variance <= 0.0 {
        (0.0, 1.0)
    } else {
        let z = (rate_b - rate_a) / variance.sqrt();
        // Two-tailed p-value under the standard normal.
        let p = 2.0 * (1.0 - standard_normal_cdf(z.abs()));
        (z, p)
    };

    let winner = if p < alpha && (rate_a - rate_b).abs() > f64::EPSILON {
        if rate_a > rate_b {
            Some(a.variant_id)
        } else {
            Some(b.variant_id)
        }
    } else {
        None
    };

    TestResult {
        variant_a: a.variant_id,
        variant_b: b.variant_id,
        rate_a,
        rate_b,
        z_score: z,
        p_value: p,
        winner,
    }
}

fn safe_rate(numer: f64, denom: f64) -> f64 {
    if denom <= 0.0 {
        0.0
    } else {
        numer / denom
    }
}

/// Standard normal CDF via Abramowitz & Stegun 7.1.26 — accurate to ~1.5e-7,
/// which is comfortably below the noise floor we care about for A/B tests.
fn standard_normal_cdf(x: f64) -> f64 {
    // Φ(x) = ½ · [1 + erf(x / √2)]
    0.5 * (1.0 + erf(x / std::f64::consts::SQRT_2))
}

fn erf(x: f64) -> f64 {
    // A&S 7.1.26
    let sign = if x < 0.0 { -1.0 } else { 1.0 };
    let x = x.abs();
    let a1 = 0.254_829_592;
    let a2 = -0.284_496_736;
    let a3 = 1.421_413_741;
    let a4 = -1.453_152_027;
    let a5 = 1.061_405_429;
    let p = 0.327_591_1;
    let t = 1.0 / (1.0 + p * x);
    let y = 1.0 - (((((a5 * t + a4) * t) + a3) * t + a2) * t + a1) * t * (-x * x).exp();
    sign * y
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pt(imp: i64, conv: i64) -> SamplePoint {
        SamplePoint {
            variant_id: Uuid::new_v4(),
            impressions: imp,
            conversions: conv,
        }
    }

    #[test]
    fn identical_samples_have_z_zero() {
        let a = pt(1000, 100);
        let b = pt(1000, 100);
        let r = two_proportion_z_test(a, b, 0.05);
        assert!(r.z_score.abs() < 1e-9);
        // erf is an approximation (A&S 7.1.26); erf(0) may carry a tiny
        // constant-term residual. Accept anything within 1e-6 of 1.0.
        assert!((r.p_value - 1.0).abs() < 1e-6);
        assert!(r.winner.is_none());
    }

    #[test]
    fn zero_impressions_is_no_op() {
        let a = pt(0, 0);
        let b = pt(100, 10);
        let r = two_proportion_z_test(a, b, 0.05);
        assert_eq!(r.z_score, 0.0);
        assert_eq!(r.p_value, 1.0);
        assert!(r.winner.is_none());
    }

    #[test]
    fn known_inputs_match_reference() {
        // Reference computed via scipy:
        //   stats.proportions_ztest([50, 30], [500, 500])
        //   -> z ≈ 2.365, two-sided p ≈ 0.018
        let a = pt(500, 50);
        let b = pt(500, 30);
        let r = two_proportion_z_test(a, b, 0.05);
        // Our sign convention: z = (rate_b - rate_a)/se → negative when A is higher.
        assert!(r.z_score < 0.0);
        assert!(
            (r.z_score.abs() - 2.365).abs() < 0.05,
            "z={} expected ~2.365",
            r.z_score
        );
        assert!(
            (r.p_value - 0.018).abs() < 0.01,
            "p={} expected ~0.018",
            r.p_value
        );
        assert_eq!(r.winner, Some(a.variant_id));
    }

    #[test]
    fn equal_rates_no_winner() {
        let a = pt(1000, 200);
        let b = pt(1000, 201);
        let r = two_proportion_z_test(a, b, 0.05);
        assert!(r.winner.is_none(), "tiny differences should not trip α=0.05");
    }

    #[test]
    fn clearly_different_rates_declare_winner() {
        let a = pt(1000, 100); // 10%
        let b = pt(1000, 150); // 15%
        let r = two_proportion_z_test(a, b, 0.05);
        assert_eq!(r.winner, Some(b.variant_id));
        assert!(r.p_value < 0.01);
    }

    #[test]
    fn erf_against_known_values() {
        assert!((erf(0.0) - 0.0).abs() < 1e-7);
        assert!((erf(1.0) - 0.842_700_79).abs() < 1e-3);
        assert!((erf(-1.0) + 0.842_700_79).abs() < 1e-3);
        assert!((erf(2.0) - 0.995_322_27).abs() < 1e-3);
    }
}
