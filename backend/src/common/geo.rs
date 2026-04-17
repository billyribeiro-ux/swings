// FDN-06 utility module. Items here are consumed by later Phase 4 subsystems
// (analytics, geo-gated pricing) — they exist ahead of their callers by
// design. Tests exercise every public surface.
#![allow(dead_code)]

//! Country-of-request detection.
//!
//! [`country_from_request`] is the entry point used by handlers. Header
//! precedence (highest first):
//!
//! 1. `cf-ipcountry`            — Cloudflare edge
//! 2. `x-vercel-ip-country`     — Vercel edge
//! 3. `x-country`               — internal override / upstream proxy
//! 4. MaxMind GeoLite2-Country lookup of `remote_ip`
//!
//! Any header whose value is not a valid ISO 3166-1 alpha-2 code is **ignored**
//! (we fall through to the next source rather than rejecting the request).
//!
//! # MaxMind
//!
//! The MaxMind reader is loaded lazily from the path in the `MAXMIND_DB_PATH`
//! environment variable. Absence of the env var — or any I/O or open error —
//! results in a single warning at startup-time first use and all subsequent
//! MaxMind lookups returning `None`. The intention is that the service degrades
//! gracefully to "country unknown" rather than failing or logging per-request.

use std::net::IpAddr;
use std::sync::OnceLock;

use axum::http::HeaderMap;
use serde::{Deserialize, Serialize};

/// ISO 3166-1 alpha-2 country code, stored as two ASCII uppercase bytes.
///
/// Serde form is the two-character string (transparent via `#[serde(into, try_from)]`
/// is not used — we implement [`Serialize`] / [`Deserialize`] directly so the
/// wire form is a plain string, not an array of bytes).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CountryCode([u8; 2]);

impl CountryCode {
    /// Parse a two-letter ASCII alpha code (case-insensitive).
    ///
    /// Returns `None` for anything that isn't exactly two ASCII alphabetic
    /// characters.
    #[must_use]
    pub fn parse(input: &str) -> Option<Self> {
        let bytes = input.as_bytes();
        if bytes.len() != 2 {
            return None;
        }
        if !bytes[0].is_ascii_alphabetic() || !bytes[1].is_ascii_alphabetic() {
            return None;
        }
        Some(Self([
            bytes[0].to_ascii_uppercase(),
            bytes[1].to_ascii_uppercase(),
        ]))
    }

    /// View as a `&str` (always ASCII, always two bytes).
    #[must_use]
    pub fn as_str(&self) -> &str {
        // The bytes came from `is_ascii_alphabetic` / `to_ascii_uppercase`, so
        // they are valid UTF-8. We still go through the checked API to avoid
        // any `unsafe` — `#![forbid(unsafe_code)]` is crate-wide.
        std::str::from_utf8(&self.0).unwrap_or("??")
    }
}

impl std::fmt::Display for CountryCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl Serialize for CountryCode {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for CountryCode {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let s = String::deserialize(d)?;
        Self::parse(&s).ok_or_else(|| serde::de::Error::custom("invalid ISO 3166-1 alpha-2 code"))
    }
}

/// Resolve the caller's country code from request headers, then fall back to
/// MaxMind.
///
/// Returns `None` if every source is missing/invalid. See module docs for
/// precedence and error semantics.
#[must_use]
pub fn country_from_request(headers: &HeaderMap, remote_ip: IpAddr) -> Option<CountryCode> {
    const HEADER_ORDER: [&str; 3] = ["cf-ipcountry", "x-vercel-ip-country", "x-country"];
    for name in HEADER_ORDER {
        if let Some(val) = headers.get(name).and_then(|v| v.to_str().ok()) {
            if let Some(cc) = CountryCode::parse(val.trim()) {
                return Some(cc);
            }
        }
    }
    country_from_ip(remote_ip)
}

/// MaxMind reader cached behind a [`OnceLock`]. `None` means "unavailable" and
/// is a terminal state for the process lifetime — we log a single warning and
/// never retry, to avoid a stampede of errors on misconfigured deployments.
static MAXMIND_READER: OnceLock<Option<maxminddb::Reader<Vec<u8>>>> = OnceLock::new();

/// Look up `ip` in the MaxMind database configured via `MAXMIND_DB_PATH`.
///
/// Returns `None` if the DB is not configured, cannot be opened, or does not
/// contain the address.
#[must_use]
pub fn country_from_ip(ip: IpAddr) -> Option<CountryCode> {
    let reader = MAXMIND_READER.get_or_init(load_reader).as_ref()?;
    let record: maxminddb::geoip2::Country<'_> = reader.lookup(ip).ok()?;
    let iso = record.country.and_then(|c| c.iso_code)?;
    CountryCode::parse(iso)
}

fn load_reader() -> Option<maxminddb::Reader<Vec<u8>>> {
    let path = match std::env::var("MAXMIND_DB_PATH") {
        Ok(p) if !p.is_empty() => p,
        _ => {
            tracing::warn!("MAXMIND_DB_PATH not set; geo::country_from_ip will always return None");
            return None;
        }
    };
    match maxminddb::Reader::open_readfile(&path) {
        Ok(r) => Some(r),
        Err(err) => {
            tracing::warn!(error = %err, path = %path, "failed to open MaxMind DB; geo fallback disabled");
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::HeaderValue;
    use std::net::Ipv4Addr;

    fn hdr(kv: &[(&str, &str)]) -> HeaderMap {
        let mut h = HeaderMap::new();
        for (k, v) in kv {
            h.insert(
                axum::http::HeaderName::from_bytes(k.as_bytes()).expect("header name"),
                HeaderValue::from_str(v).expect("header value"),
            );
        }
        h
    }

    #[test]
    fn parse_accepts_lower_and_upper() {
        assert_eq!(CountryCode::parse("us").unwrap().as_str(), "US");
        assert_eq!(CountryCode::parse("Gb").unwrap().as_str(), "GB");
    }

    #[test]
    fn parse_rejects_bad_inputs() {
        assert!(CountryCode::parse("").is_none());
        assert!(CountryCode::parse("u").is_none());
        assert!(CountryCode::parse("usa").is_none());
        assert!(CountryCode::parse("u1").is_none());
        assert!(CountryCode::parse("  ").is_none());
    }

    #[test]
    fn display_and_as_str_agree() {
        let c = CountryCode::parse("de").unwrap();
        assert_eq!(format!("{c}"), "DE");
        assert_eq!(c.as_str(), "DE");
    }

    #[test]
    fn serde_round_trip() {
        let c = CountryCode::parse("fr").unwrap();
        let j = serde_json::to_string(&c).unwrap();
        assert_eq!(j, "\"FR\"");
        let back: CountryCode = serde_json::from_str(&j).unwrap();
        assert_eq!(back, c);
    }

    #[test]
    fn serde_rejects_invalid_payload() {
        assert!(serde_json::from_str::<CountryCode>("\"USA\"").is_err());
        assert!(serde_json::from_str::<CountryCode>("\"1!\"").is_err());
    }

    #[test]
    fn cf_header_wins_over_vercel() {
        let h = hdr(&[("cf-ipcountry", "US"), ("x-vercel-ip-country", "GB")]);
        let got = country_from_request(&h, IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1)));
        assert_eq!(got.map(|c| c.as_str().to_owned()), Some("US".into()));
    }

    #[test]
    fn vercel_header_wins_over_x_country() {
        let h = hdr(&[("x-vercel-ip-country", "CA"), ("x-country", "GB")]);
        let got = country_from_request(&h, IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1)));
        assert_eq!(got.map(|c| c.as_str().to_owned()), Some("CA".into()));
    }

    #[test]
    fn x_country_used_when_others_absent() {
        let h = hdr(&[("x-country", "jp")]);
        let got = country_from_request(&h, IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1)));
        assert_eq!(got.map(|c| c.as_str().to_owned()), Some("JP".into()));
    }

    #[test]
    fn invalid_header_value_is_ignored_and_falls_through() {
        // `cf-ipcountry` invalid → skip to `x-vercel-ip-country`.
        let h = hdr(&[("cf-ipcountry", "ZZZ"), ("x-vercel-ip-country", "US")]);
        let got = country_from_request(&h, IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1)));
        assert_eq!(got.map(|c| c.as_str().to_owned()), Some("US".into()));
    }

    #[test]
    fn no_headers_no_maxmind_returns_none() {
        // Ensure MAXMIND_DB_PATH is unset for this test's view; env is
        // process-global, so we only assert that absence of any header source
        // plus the default (unset) env yields None. Setting env here would
        // race with other tests, so we explicitly accept None when unset.
        let h = HeaderMap::new();
        // We do NOT mutate env vars; we only assert the header path returns
        // None (MaxMind fallback may or may not be configured on the test host).
        // The sensible default when neither is configured is None.
        let got = country_from_request(&h, IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)));
        // If MaxMind is not configured we expect None. If it *is* configured
        // on the test host, a 127.0.0.1 lookup should still yield None.
        assert!(got.is_none());
    }

    #[test]
    fn country_from_ip_without_env_is_none() {
        // Loopback + (almost certainly) no MAXMIND_DB_PATH in CI ⇒ None.
        let got = country_from_ip(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)));
        assert!(got.is_none());
    }
}
