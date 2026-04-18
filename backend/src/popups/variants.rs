//! POP-02: variant assignment.
//!
//! Assignment is a pure function of `(popup_id, anonymous_id)`. We hash the
//! pair with SipHash and pick a variant whose cumulative weight window
//! contains the hash's modulo. Because the hash is deterministic, a returning
//! visitor lands on the same variant across sessions even if the cookie is
//! cleared — the frontend still sets a `swings_popup_variant:{popup_id}`
//! cookie so the edge (CDN) cache can key off it without re-hashing.
//!
//! The assignment is weighted, NOT uniform: a variant with `traffic_weight=20`
//! gets roughly 20% of visitors, independent of the number of sibling
//! variants. Weights are summed; if the sum is zero we fall back to the
//! first variant so misconfigured popups do not 500.

use std::hash::{BuildHasher, Hasher};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Row shape for `popup_variants`.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct PopupVariant {
    pub id: Uuid,
    pub popup_id: Uuid,
    pub name: String,
    pub content_json: serde_json::Value,
    pub style_json: serde_json::Value,
    pub traffic_weight: i32,
    pub is_winner: bool,
    pub created_at: DateTime<Utc>,
}

/// Deterministically pick a variant for the given anonymous id.
///
/// Returns `None` only when `variants` is empty; any non-empty slice yields a
/// winner even if every weight is zero (we clamp to the first entry).
#[must_use]
pub fn assign_variant(
    popup_id: Uuid,
    anonymous_id: Uuid,
    variants: &[PopupVariant],
) -> Option<&PopupVariant> {
    if variants.is_empty() {
        return None;
    }
    let total: u64 = variants
        .iter()
        .map(|v| v.traffic_weight.max(0) as u64)
        .sum();
    if total == 0 {
        return variants.first();
    }
    let h = stable_hash(popup_id, anonymous_id);
    let bucket = h % total;
    let mut acc: u64 = 0;
    for v in variants {
        acc = acc.saturating_add(v.traffic_weight.max(0) as u64);
        if bucket < acc {
            return Some(v);
        }
    }
    // Floating-point-free math above; the loop must terminate because
    // `bucket < total == acc_final`. Guard anyway.
    variants.last()
}

/// SipHash-1-3 over the 32 bytes of the two UUIDs. `RandomState` would
/// introduce per-process randomness which breaks stickiness; we use
/// `std::hash::BuildHasherDefault<SipHasher>`... except that's removed
/// from std. Instead we implement the hash manually with a fixed key.
pub(crate) fn stable_hash(popup_id: Uuid, anonymous_id: Uuid) -> u64 {
    let mut hasher = FixedSipHasher::new();
    hasher.write(popup_id.as_bytes());
    hasher.write(anonymous_id.as_bytes());
    hasher.finish()
}

/// Cookie name for the sticky assignment. The popup_id is appended so
/// visitors can be in different variants across different popups.
#[must_use]
pub fn cookie_name(popup_id: Uuid) -> String {
    format!("swings_popup_variant:{popup_id}")
}

// ── FixedSipHasher ───────────────────────────────────────────────────────
// A 64-bit SipHash-2-4 hasher with a hard-coded key. We need determinism
// across processes, so we cannot use `std::hash::RandomState`. The
// implementation below is a straightforward port suitable for the
// assignment use-case — it is NOT a cryptographic primitive.

pub struct FixedSipHasher {
    v0: u64,
    v1: u64,
    v2: u64,
    v3: u64,
    tail: u64,
    ntail: usize,
    length: usize,
}

impl FixedSipHasher {
    /// Arbitrary but fixed key. Do not rotate without a migration plan —
    /// changing the key reshuffles every visitor into a new variant.
    const K0: u64 = 0x0123_4567_89ab_cdef;
    const K1: u64 = 0xfedc_ba98_7654_3210;

    fn new() -> Self {
        Self {
            v0: Self::K0 ^ 0x736f_6d65_7073_6575,
            v1: Self::K1 ^ 0x646f_7261_6e64_6f6d,
            v2: Self::K0 ^ 0x6c79_6765_6e65_7261,
            v3: Self::K1 ^ 0x7465_6462_7974_6573,
            tail: 0,
            ntail: 0,
            length: 0,
        }
    }

    fn c_rounds(&mut self) {
        for _ in 0..2 {
            self.sipround();
        }
    }

    fn d_rounds(&mut self) {
        for _ in 0..4 {
            self.sipround();
        }
    }

    fn sipround(&mut self) {
        self.v0 = self.v0.wrapping_add(self.v1);
        self.v1 = self.v1.rotate_left(13);
        self.v1 ^= self.v0;
        self.v0 = self.v0.rotate_left(32);
        self.v2 = self.v2.wrapping_add(self.v3);
        self.v3 = self.v3.rotate_left(16);
        self.v3 ^= self.v2;
        self.v0 = self.v0.wrapping_add(self.v3);
        self.v3 = self.v3.rotate_left(21);
        self.v3 ^= self.v0;
        self.v2 = self.v2.wrapping_add(self.v1);
        self.v1 = self.v1.rotate_left(17);
        self.v1 ^= self.v2;
        self.v2 = self.v2.rotate_left(32);
    }
}

impl Hasher for FixedSipHasher {
    fn write(&mut self, mut bytes: &[u8]) {
        self.length += bytes.len();
        // Fill the tail first if we have a partial word buffered.
        if self.ntail != 0 {
            let needed = 8 - self.ntail;
            let take = needed.min(bytes.len());
            let mut word: u64 = 0;
            for (i, b) in bytes.iter().take(take).enumerate() {
                word |= u64::from(*b) << (8 * i);
            }
            self.tail |= word << (8 * self.ntail);
            self.ntail += take;
            bytes = &bytes[take..];
            if self.ntail == 8 {
                self.v3 ^= self.tail;
                self.c_rounds();
                self.v0 ^= self.tail;
                self.tail = 0;
                self.ntail = 0;
            }
        }
        while bytes.len() >= 8 {
            let mut w: u64 = 0;
            for (i, b) in bytes.iter().take(8).enumerate() {
                w |= u64::from(*b) << (8 * i);
            }
            self.v3 ^= w;
            self.c_rounds();
            self.v0 ^= w;
            bytes = &bytes[8..];
        }
        for (i, b) in bytes.iter().enumerate() {
            self.tail |= u64::from(*b) << (8 * i);
        }
        self.ntail += bytes.len();
    }

    fn finish(&self) -> u64 {
        // We need an owned, mutable copy because SipHash finalisation mutates.
        let mut c = FixedSipHasher {
            v0: self.v0,
            v1: self.v1,
            v2: self.v2,
            v3: self.v3,
            tail: self.tail,
            ntail: self.ntail,
            length: self.length,
        };
        let b = ((c.length as u64 & 0xff) << 56) | c.tail;
        c.v3 ^= b;
        c.c_rounds();
        c.v0 ^= b;
        c.v2 ^= 0xff;
        c.d_rounds();
        c.v0 ^ c.v1 ^ c.v2 ^ c.v3
    }
}

/// `BuildHasher` glue so callers can hash via `RandomState`-style ergonomics
/// if they ever need to. Not currently wired but keeps the surface symmetric.
pub struct FixedSipBuildHasher;
impl BuildHasher for FixedSipBuildHasher {
    type Hasher = FixedSipHasher;
    fn build_hasher(&self) -> Self::Hasher {
        FixedSipHasher::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn variant(id: Uuid, weight: i32) -> PopupVariant {
        PopupVariant {
            id,
            popup_id: Uuid::nil(),
            name: format!("w{weight}"),
            content_json: serde_json::json!({}),
            style_json: serde_json::json!({}),
            traffic_weight: weight,
            is_winner: false,
            created_at: Utc::now(),
        }
    }

    #[test]
    fn empty_variants_returns_none() {
        assert!(assign_variant(Uuid::new_v4(), Uuid::new_v4(), &[]).is_none());
    }

    #[test]
    fn zero_total_weight_returns_first() {
        let v = [variant(Uuid::new_v4(), 0), variant(Uuid::new_v4(), 0)];
        let got = assign_variant(Uuid::new_v4(), Uuid::new_v4(), &v).unwrap();
        assert_eq!(got.id, v[0].id);
    }

    #[test]
    fn assignment_is_stable() {
        let popup = Uuid::new_v4();
        let anon = Uuid::new_v4();
        let v = [variant(Uuid::new_v4(), 50), variant(Uuid::new_v4(), 50)];
        let a = assign_variant(popup, anon, &v).unwrap().id;
        let b = assign_variant(popup, anon, &v).unwrap().id;
        assert_eq!(a, b);
    }

    #[test]
    fn weights_distribute_within_tolerance() {
        // 70/30 split over 10k anon IDs must land within ±2 percentage
        // points of the configured weight on both sides. Uses a fixed RNG
        // seed so the test is deterministic across runs.
        let popup = Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap();
        let a = variant(
            Uuid::parse_str("00000000-0000-0000-0000-0000000000aa").unwrap(),
            70,
        );
        let b = variant(
            Uuid::parse_str("00000000-0000-0000-0000-0000000000bb").unwrap(),
            30,
        );
        let variants = [a.clone(), b.clone()];

        let mut count_a = 0usize;
        let mut count_b = 0usize;
        for i in 0u128..10_000 {
            let mut bytes = [0u8; 16];
            bytes[..16].copy_from_slice(&i.to_be_bytes());
            let anon = Uuid::from_bytes(bytes);
            let pick = assign_variant(popup, anon, &variants).unwrap();
            if pick.id == a.id {
                count_a += 1;
            } else {
                count_b += 1;
            }
        }
        let pct_a = count_a as f64 / 10_000.0 * 100.0;
        let pct_b = count_b as f64 / 10_000.0 * 100.0;
        assert!(
            (pct_a - 70.0).abs() < 2.0,
            "A drift {pct_a}% (count {count_a}) vs expected 70%"
        );
        assert!(
            (pct_b - 30.0).abs() < 2.0,
            "B drift {pct_b}% (count {count_b}) vs expected 30%"
        );
    }

    #[test]
    fn different_popups_decorrelate() {
        let anon = Uuid::new_v4();
        let v = [variant(Uuid::new_v4(), 50), variant(Uuid::new_v4(), 50)];
        let p1 = Uuid::new_v4();
        let p2 = Uuid::new_v4();
        let h1 = stable_hash(p1, anon);
        let h2 = stable_hash(p2, anon);
        assert_ne!(
            h1, h2,
            "same anon + different popup should hash differently"
        );
        let _ = assign_variant(p1, anon, &v);
        let _ = assign_variant(p2, anon, &v);
    }
}
