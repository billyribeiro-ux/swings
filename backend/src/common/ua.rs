// FDN-06 utility module. Items here are consumed by later Phase 4 subsystems
// (auth, analytics) — they exist ahead of their callers by design. Tests
// exercise every public surface.
#![allow(dead_code)]

//! User-agent parsing with a bounded cache.
//!
//! [`parse_ua`] returns a compact [`UaInfo`] summary derived from `woothee`'s
//! output. Results are memoized in a `moka::sync::Cache` with a 10k-entry cap
//! (approximate LRU/TinyLFU, process-global) — the cache is initialized lazily
//! via [`OnceLock`] so test runners (and first-request latency) don't pay
//! allocation up-front.
//!
//! Empty input yields an all-`"unknown"`, non-bot record.

use std::sync::OnceLock;

use moka::sync::Cache;
use serde::{Deserialize, Serialize};
use woothee::parser::Parser;

/// Maximum cached parse results. Tuned for typical web traffic where a few
/// thousand distinct UAs dominate; 10k gives generous headroom before TinyLFU
/// starts evicting.
const CACHE_CAPACITY: u64 = 10_000;

/// Coarse device classification derived from `woothee` category strings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeviceKind {
    /// Desktop / laptop browser.
    Desktop,
    /// Smartphone or tablet.
    Mobile,
    /// Known crawler / indexer.
    Crawler,
    /// Unknown — empty or unparseable input.
    Unknown,
}

/// Compact summary of a parsed user-agent string.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct UaInfo {
    /// Browser / application family (e.g. `"Chrome"`, `"Safari"`, `"Googlebot"`).
    pub family: String,
    /// Operating system (e.g. `"Mac OSX"`, `"Android"`, `"UNKNOWN"`).
    pub os: String,
    /// Coarse device classification; see [`DeviceKind`].
    pub device_kind: DeviceKind,
    /// Whether `woothee` classified this UA as a crawler.
    pub is_bot: bool,
}

impl UaInfo {
    fn unknown() -> Self {
        Self {
            family: "unknown".to_owned(),
            os: "unknown".to_owned(),
            device_kind: DeviceKind::Unknown,
            is_bot: false,
        }
    }
}

fn cache() -> &'static Cache<String, UaInfo> {
    static CACHE: OnceLock<Cache<String, UaInfo>> = OnceLock::new();
    CACHE.get_or_init(|| Cache::builder().max_capacity(CACHE_CAPACITY).build())
}

fn parser() -> &'static Parser {
    static PARSER: OnceLock<Parser> = OnceLock::new();
    PARSER.get_or_init(Parser::new)
}

/// Parse a user-agent header value into a [`UaInfo`].
///
/// Empty input short-circuits to an all-unknown record without touching the
/// cache (we don't want to waste an entry on the empty string).
#[must_use]
pub fn parse_ua(ua: &str) -> UaInfo {
    if ua.is_empty() {
        return UaInfo::unknown();
    }
    if let Some(hit) = cache().get(ua) {
        return hit;
    }
    let info = match parser().parse(ua) {
        Some(r) => {
            let is_bot = r.category == "crawler";
            let device_kind = match r.category {
                "pc" => DeviceKind::Desktop,
                "smartphone" | "mobilephone" | "appliance" => DeviceKind::Mobile,
                "crawler" => DeviceKind::Crawler,
                _ => DeviceKind::Unknown,
            };
            UaInfo {
                family: r.name.to_owned(),
                os: r.os.to_owned(),
                device_kind,
                is_bot,
            }
        }
        None => UaInfo::unknown(),
    };
    cache().insert(ua.to_owned(), info.clone());
    info
}

#[cfg(test)]
mod tests {
    use super::*;

    const CHROME: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 \
        (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36";
    const SAFARI_IOS: &str = "Mozilla/5.0 (iPhone; CPU iPhone OS 17_0 like Mac OS X) \
        AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.0 Mobile/15E148 Safari/604.1";
    const GOOGLEBOT: &str =
        "Mozilla/5.0 (compatible; Googlebot/2.1; +http://www.google.com/bot.html)";

    #[test]
    fn empty_yields_unknown() {
        let info = parse_ua("");
        assert_eq!(info.device_kind, DeviceKind::Unknown);
        assert!(!info.is_bot);
        assert_eq!(info.family, "unknown");
        assert_eq!(info.os, "unknown");
    }

    #[test]
    fn chrome_is_desktop_not_bot() {
        let info = parse_ua(CHROME);
        assert_eq!(info.device_kind, DeviceKind::Desktop);
        assert!(!info.is_bot);
        assert_eq!(info.family, "Chrome");
    }

    #[test]
    fn safari_ios_is_mobile() {
        let info = parse_ua(SAFARI_IOS);
        assert_eq!(info.device_kind, DeviceKind::Mobile);
        assert!(!info.is_bot);
    }

    #[test]
    fn googlebot_is_crawler_and_bot() {
        let info = parse_ua(GOOGLEBOT);
        assert_eq!(info.device_kind, DeviceKind::Crawler);
        assert!(info.is_bot);
    }

    #[test]
    fn garbage_does_not_panic() {
        let info = parse_ua("!!!not-a-ua!!!");
        // May be Unknown or a low-confidence parse — both are acceptable
        // so long as we don't panic and we return *something*.
        assert!(!info.family.is_empty());
    }

    #[test]
    fn cache_returns_equal_values_on_repeat() {
        let a = parse_ua(CHROME);
        let b = parse_ua(CHROME);
        assert_eq!(a, b);
    }
}
