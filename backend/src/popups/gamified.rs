//! POP-04: gamified popup types.
//!
//! This module owns the server-side state machine for `spin_to_win` and
//! `scratch_card` popups: deterministic prize picking, coupon-code template
//! substitution, and the adapter hook we expose to the coupons subsystem.
//!
//! The client never picks a prize on its own — the server always makes the
//! decision so prize distribution is auditable and tamper-evident.

use rand::{rngs::StdRng, Rng, SeedableRng};
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// One prize tile on the wheel / scratch card. `coupon_code_pattern`
/// supports two template tokens: `{{nanoid:N}}` (N random alphanumerics,
/// 4 ≤ N ≤ 32) and `{{ts}}` (unix timestamp). Absence of a pattern means
/// the prize is purely cosmetic (e.g. "Sorry, try again").
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Prize {
    pub label: String,
    pub weight: u32,
    #[serde(default)]
    pub coupon_code_pattern: Option<String>,
}

/// Full spin-to-win configuration stored in `content_json`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpinToWinConfig {
    pub prizes: Vec<Prize>,
}

#[derive(Debug, Error)]
pub enum GamifiedError {
    #[error("spin_to_win config has no prizes")]
    NoPrizes,
    #[error("prize weights sum to zero")]
    ZeroWeight,
    /// Returned by [`generate_coupon_code`] if the pattern references an
    /// unsupported token. This is an authoring error (template typo), not a
    /// runtime failure — handlers should 422 the admin before accepting the
    /// popup.
    #[error("unsupported coupon code token: {0}")]
    UnsupportedToken(String),
    /// Placeholder for the coupons-integration handoff. Emitted only by
    /// [`create_one_time_coupon`] when the coupons subsystem is not yet
    /// wired on the caller's side.
    #[error("coupon integration unavailable")]
    UnsupportedIntegration,
}

/// Weighted-random prize pick. Seed is caller-supplied so submissions can
/// be reproduced for audit (seed = hash of session + prize_draw_id).
pub fn pick_prize(config: &SpinToWinConfig, seed: u64) -> Result<&Prize, GamifiedError> {
    if config.prizes.is_empty() {
        return Err(GamifiedError::NoPrizes);
    }
    let total: u64 = config.prizes.iter().map(|p| p.weight as u64).sum();
    if total == 0 {
        return Err(GamifiedError::ZeroWeight);
    }
    let mut rng = StdRng::seed_from_u64(seed);
    let roll = rng.gen_range(0..total);
    let mut acc: u64 = 0;
    for prize in &config.prizes {
        acc += prize.weight as u64;
        if roll < acc {
            return Ok(prize);
        }
    }
    // Unreachable — the loop terminates because `roll < total == acc_final`.
    Ok(&config.prizes[config.prizes.len() - 1])
}

/// Expand a `coupon_code_pattern` into a concrete code. Supported tokens:
///
/// * `{{nanoid:N}}` — N random ASCII alphanumerics. 4 ≤ N ≤ 32.
/// * `{{ts}}`       — current unix timestamp as a decimal string.
///
/// Returns an [`GamifiedError::UnsupportedToken`] for any other `{{...}}`
/// match so authors cannot silently produce malformed codes.
pub fn generate_coupon_code(pattern: &str, seed: u64) -> Result<String, GamifiedError> {
    let mut out = String::with_capacity(pattern.len());
    let mut rng = StdRng::seed_from_u64(seed);
    let mut i = 0;
    let bytes = pattern.as_bytes();
    while i < bytes.len() {
        if i + 1 < bytes.len() && bytes[i] == b'{' && bytes[i + 1] == b'{' {
            if let Some(end) = find_close(pattern, i + 2) {
                let token = &pattern[i + 2..end];
                let expanded = expand_token(token, &mut rng)?;
                out.push_str(&expanded);
                i = end + 2;
                continue;
            }
        }
        out.push(bytes[i] as char);
        i += 1;
    }
    Ok(out)
}

fn find_close(s: &str, from: usize) -> Option<usize> {
    let b = s.as_bytes();
    let mut i = from;
    while i + 1 < b.len() {
        if b[i] == b'}' && b[i + 1] == b'}' {
            return Some(i);
        }
        i += 1;
    }
    None
}

const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";

fn expand_token(token: &str, rng: &mut StdRng) -> Result<String, GamifiedError> {
    if let Some(n_str) = token.strip_prefix("nanoid:") {
        let n: usize = n_str
            .parse()
            .map_err(|_| GamifiedError::UnsupportedToken(token.to_string()))?;
        if !(4..=32).contains(&n) {
            return Err(GamifiedError::UnsupportedToken(format!(
                "nanoid length {n} outside [4,32]"
            )));
        }
        let mut s = String::with_capacity(n);
        for _ in 0..n {
            let idx = rng.gen_range(0..ALPHABET.len());
            s.push(ALPHABET[idx] as char);
        }
        Ok(s)
    } else if token == "ts" {
        Ok(chrono::Utc::now().timestamp().to_string())
    } else {
        Err(GamifiedError::UnsupportedToken(token.to_string()))
    }
}

/// Coupons-subsystem handoff. The real implementation will land when the
/// coupons engine grows a "create one-time code" entry point; for now we
/// return [`GamifiedError::UnsupportedIntegration`] so the calling handler
/// can log a warning and fall back to sending the raw code by email.
///
/// Accepts the prize and a pre-rendered coupon code so the caller does not
/// need to re-run `generate_coupon_code` just to recover the string.
pub fn create_one_time_coupon(_prize: &Prize, _code: &str) -> Result<(), GamifiedError> {
    Err(GamifiedError::UnsupportedIntegration)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg() -> SpinToWinConfig {
        SpinToWinConfig {
            prizes: vec![
                Prize {
                    label: "A".into(),
                    weight: 50,
                    coupon_code_pattern: Some("A-{{nanoid:6}}".into()),
                },
                Prize {
                    label: "B".into(),
                    weight: 30,
                    coupon_code_pattern: None,
                },
                Prize {
                    label: "C".into(),
                    weight: 20,
                    coupon_code_pattern: Some("C-{{nanoid:4}}".into()),
                },
            ],
        }
    }

    #[test]
    fn pick_prize_is_deterministic_for_same_seed() {
        let a = pick_prize(&cfg(), 42).unwrap().label.clone();
        let b = pick_prize(&cfg(), 42).unwrap().label.clone();
        assert_eq!(a, b);
    }

    #[test]
    fn pick_prize_weight_distribution() {
        let mut counts = [0u32; 3];
        for seed in 0..10_000 {
            let label = pick_prize(&cfg(), seed).unwrap().label.clone();
            let idx = match label.as_str() {
                "A" => 0,
                "B" => 1,
                "C" => 2,
                _ => unreachable!(),
            };
            counts[idx] += 1;
        }
        let pct = |c: u32| c as f64 / 10_000.0 * 100.0;
        assert!(
            (pct(counts[0]) - 50.0).abs() < 2.0,
            "A drift {}",
            pct(counts[0])
        );
        assert!(
            (pct(counts[1]) - 30.0).abs() < 2.0,
            "B drift {}",
            pct(counts[1])
        );
        assert!(
            (pct(counts[2]) - 20.0).abs() < 2.0,
            "C drift {}",
            pct(counts[2])
        );
    }

    #[test]
    fn pick_prize_empty_errors() {
        let empty = SpinToWinConfig { prizes: vec![] };
        assert!(matches!(
            pick_prize(&empty, 0),
            Err(GamifiedError::NoPrizes)
        ));
    }

    #[test]
    fn pick_prize_zero_weight_errors() {
        let c = SpinToWinConfig {
            prizes: vec![Prize {
                label: "x".into(),
                weight: 0,
                coupon_code_pattern: None,
            }],
        };
        assert!(matches!(pick_prize(&c, 0), Err(GamifiedError::ZeroWeight)));
    }

    #[test]
    fn nanoid_token_expands_to_correct_length() {
        let code = generate_coupon_code("ABC-{{nanoid:8}}", 1).unwrap();
        assert!(code.starts_with("ABC-"));
        assert_eq!(code.len(), 4 + 8);
        assert!(code[4..].bytes().all(|b| b.is_ascii_alphanumeric()));
    }

    #[test]
    fn nanoid_token_is_deterministic() {
        let a = generate_coupon_code("{{nanoid:16}}", 7).unwrap();
        let b = generate_coupon_code("{{nanoid:16}}", 7).unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn ts_token_expands() {
        let code = generate_coupon_code("T-{{ts}}", 0).unwrap();
        assert!(code.starts_with("T-"));
        assert!(code[2..].chars().all(|c| c.is_ascii_digit()));
    }

    #[test]
    fn unsupported_token_errors() {
        let err = generate_coupon_code("X-{{lol}}", 0).unwrap_err();
        assert!(matches!(err, GamifiedError::UnsupportedToken(_)));
    }

    #[test]
    fn create_one_time_coupon_returns_unsupported_until_wired() {
        let p = Prize {
            label: "x".into(),
            weight: 1,
            coupon_code_pattern: None,
        };
        assert!(matches!(
            create_one_time_coupon(&p, "X-ABC"),
            Err(GamifiedError::UnsupportedIntegration)
        ));
    }
}
