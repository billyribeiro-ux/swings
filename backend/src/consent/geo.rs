//! CONSENT-05: country-code → regulatory region resolver.
//!
//! Maps an ISO 3166-1 alpha-2 country code onto one of a small, closed set of
//! regulatory buckets that the admin configures banner copy for:
//!
//!   * `"EU"`      — GDPR + ePrivacy. Every EU-27 member plus EEA (IS, LI, NO).
//!   * `"UK"`      — UK GDPR + PECR. Distinct from EU since post-Brexit divergence.
//!   * `"US-CA"`   — California CCPA / CPRA. Separate because of "Do Not Sell".
//!   * `"US-CO"`   — Colorado CPA. Authoritative denial of analytics by default.
//!   * `"US-STATE"`— Virginia VCDPA / Connecticut CTDPA / Utah UCPA /
//!                   Texas TDPSA / Florida FDBR. All honor opt-out + GPC but
//!                   are less strict than CA/CO.
//!   * `"CA"`      — Canadian federal PIPEDA.
//!   * `"CA-QC"`   — Quebec Law 25. Not yet resolvable from country code alone
//!                   (would require MaxMind region lookup); the resolver
//!                   always returns `"CA"` today.
//!   * `"BR"`      — Brazilian LGPD.
//!   * `"default"` — everywhere else.
//!
//! Subdivisions (CA state, province) require MaxMind-region lookup which the
//! FDN-06 geo module does not yet expose. `resolve_region` therefore returns
//! only country-level buckets; state-level copy variants (US-CA vs US-CO) are
//! stored under the same `"default"` or country-level region today and will
//! refine in a CONSENT-05 v2 pass.
//!
//! TODO (CONSENT-05 v2): thread MaxMind subdivision info through
//! `common::geo::country_from_request` so we can resolve US-CA / US-CO /
//! CA-QC precisely.

#![allow(dead_code)]

use crate::common::geo::CountryCode;

/// Every EU-27 member + the three EEA states we align with GDPR for.
///
/// Kept as a compile-time slice so the lookup is a linear scan over 30
/// entries — cheap enough that a hash-set is overkill. Sorted for ease of
/// maintenance, not for binary search.
const EU_COUNTRIES: &[&str] = &[
    "AT", "BE", "BG", "CY", "CZ", "DE", "DK", "EE", "ES", "FI", "FR", "GR", "HR", "HU", "IE", "IS",
    "IT", "LI", "LT", "LU", "LV", "MT", "NL", "NO", "PL", "PT", "RO", "SE", "SI", "SK",
];

/// US states that have enacted omnibus privacy laws beyond CCPA/CPA but fall
/// short of California's stricter "Do Not Sell" regime. Because
/// [`resolve_region`] cannot (yet) see the state subdivision from the country
/// code, this table is unused at the country-lookup layer — it is documented
/// here as the seed for a v2 implementation that reads MaxMind regions.
#[allow(dead_code)]
const US_STATE_LAWS: &[&str] = &["VA", "CT", "UT", "TX", "FL"];

/// Country codes that get their own distinct bucket without needing a
/// subdivision. Put in table form so adding a new law (e.g. Japan APPI) is a
/// single-line edit.
const COUNTRY_BUCKETS: &[(&str, &str)] = &[("GB", "UK"), ("BR", "BR"), ("CA", "CA")];

/// Resolve a country code onto its regulatory bucket. Returns a
/// `&'static str` so callers can pass it straight into the banner lookup
/// without allocating.
#[must_use]
pub fn resolve_region(country: Option<CountryCode>) -> &'static str {
    let Some(cc) = country else {
        return "default";
    };
    let code = cc.as_str();

    if EU_COUNTRIES.contains(&code) {
        return "EU";
    }

    for (needle, region) in COUNTRY_BUCKETS {
        if *needle == code {
            return region;
        }
    }

    // TODO: US subdivision lookup. Until MaxMind-region plumbing lands we
    // cannot distinguish US-CA, US-CO, US-STATE from each other — so we
    // fall through to the "default" bucket for every US request. The
    // seeded US-CA banner (migration 026) therefore requires a subsequent
    // CONSENT-05 v2 commit to actually route traffic onto it.
    if code == "US" {
        return "default";
    }

    "default"
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::geo::CountryCode;

    fn cc(s: &str) -> Option<CountryCode> {
        CountryCode::parse(s)
    }

    #[test]
    fn no_country_is_default() {
        assert_eq!(resolve_region(None), "default");
    }

    #[test]
    fn eu_members_resolve_to_eu() {
        for code in ["DE", "FR", "IT", "ES", "NL", "SE"] {
            assert_eq!(resolve_region(cc(code)), "EU", "{code} should be EU");
        }
    }

    #[test]
    fn eea_states_resolve_to_eu() {
        for code in ["IS", "LI", "NO"] {
            assert_eq!(resolve_region(cc(code)), "EU", "{code} should be EU");
        }
    }

    #[test]
    fn uk_has_own_bucket() {
        assert_eq!(resolve_region(cc("GB")), "UK");
    }

    #[test]
    fn canada_has_own_bucket() {
        assert_eq!(resolve_region(cc("CA")), "CA");
    }

    #[test]
    fn brazil_has_own_bucket() {
        assert_eq!(resolve_region(cc("BR")), "BR");
    }

    #[test]
    fn unknown_country_is_default() {
        assert_eq!(resolve_region(cc("JP")), "default");
        assert_eq!(resolve_region(cc("AU")), "default");
    }

    #[test]
    fn us_falls_to_default_until_subdivisions_wire() {
        // CONSENT-05 v2 will upgrade this to US-CA / US-CO / US-STATE
        // once MaxMind region lookup is threaded.
        assert_eq!(resolve_region(cc("US")), "default");
    }
}
