// POP-01 tests use `Default::default() + field assigns` to keep each rule
// dimension visually isolated; that style is intentional here.
#![allow(clippy::field_reassign_with_default)]
// Doc lists in this module mix prose + bullet variants; clippy's
// indentation lint flips between "without indentation" and "overindented"
// based on which bullet style is used. The lint adds noise without
// materially improving the rendered docs.
#![allow(clippy::doc_lazy_continuation, clippy::doc_overindented_list_items)]

//! POP-01: server-side targeting predicate.
//!
//! The function [`matches_targeting_rules`] is the single authority for
//! "should this popup be shown to this visitor?" It is a pure function — no
//! database, no clock other than the caller-supplied [`VisitorContext::now`]
//! — which keeps it trivially testable.
//!
//! `TargetingRules` is parsed from the `popups.targeting_rules` JSONB column.
//! Keys that are absent or null imply "no constraint on this dimension";
//! keys that are present must match for the popup to be eligible.

use chrono::{DateTime, Datelike, Timelike, Utc};
use regex::Regex;
use serde::{Deserialize, Serialize};

use crate::error::AppError;

/// Parsed shape of `popups.targeting_rules`. Accepting JSON directly (rather
/// than wrangling `serde_json::Value` in every check) lets us validate once
/// and return typed errors to admins who posted malformed regexes.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TargetingRules {
    #[serde(default)]
    pub pages: Option<Vec<String>>,
    #[serde(default, alias = "userStatus")]
    pub user_status: Option<Vec<String>>,
    #[serde(default)]
    pub devices: Option<Vec<String>>,
    /// Singleton alias for `devices` — honored for backwards compatibility.
    #[serde(default)]
    pub device: Option<String>,

    #[serde(default)]
    pub geo: Option<Vec<String>>,
    #[serde(default)]
    pub utm_source: Option<Vec<String>>,
    #[serde(default)]
    pub utm_medium: Option<Vec<String>>,
    #[serde(default)]
    pub utm_campaign: Option<Vec<String>>,
    #[serde(default)]
    pub url_include_regex: Option<String>,
    #[serde(default)]
    pub url_exclude_regex: Option<String>,
    #[serde(default)]
    pub cart_value_cents_min: Option<i64>,
    #[serde(default)]
    pub cart_value_cents_max: Option<i64>,
    #[serde(default)]
    pub cart_contains_sku: Option<Vec<String>>,
    #[serde(default)]
    pub membership_tier: Option<Vec<String>>,
    #[serde(default)]
    pub time_of_day_start: Option<String>,
    #[serde(default)]
    pub time_of_day_end: Option<String>,
    #[serde(default)]
    pub day_of_week: Option<Vec<u8>>,
    #[serde(default)]
    pub returning_visitor: Option<bool>,
    #[serde(default)]
    pub browser_family: Option<Vec<String>>,
    #[serde(default)]
    pub new_visitor_max_pageviews: Option<i64>,
}

impl TargetingRules {
    pub fn from_json(value: &serde_json::Value) -> Result<Self, AppError> {
        serde_json::from_value(value.clone())
            .map_err(|e| AppError::Validation(format!("targeting_rules: {e}")))
    }
}

/// Everything the targeting predicate needs to know about a visitor.
/// Populated by the handler from HTTP headers (geo via FDN-06), cookies
/// (returning_visitor), and query string (UTM).
#[derive(Debug, Clone, Default)]
pub struct VisitorContext {
    pub page_path: String,
    pub device: String,
    pub user_status: String,
    pub geo_country: Option<String>,
    pub utm_source: Option<String>,
    pub utm_medium: Option<String>,
    pub utm_campaign: Option<String>,
    pub cart_value_cents: Option<i64>,
    pub cart_skus: Vec<String>,
    pub membership_tier: Option<String>,
    pub returning_visitor: bool,
    pub browser_family: Option<String>,
    pub pageview_count: i64,
    /// Wall-clock in visitor's local timezone. The handler computes this
    /// from the geo-derived offset; test code can pass UTC directly.
    pub now: DateTime<Utc>,
}

/// Apply every configured rule; short-circuit the first failure. A popup
/// with *no* targeting_rules matches every visitor.
pub fn matches_targeting_rules(rules: &TargetingRules, ctx: &VisitorContext) -> bool {
    if let Some(pages) = rules.pages.as_ref() {
        if !pages
            .iter()
            .any(|p| matches_page_pattern(p, &ctx.page_path))
        {
            return false;
        }
    }
    if let Some(devices) = rules.devices.as_ref() {
        if !devices.iter().any(|d| d.eq_ignore_ascii_case(&ctx.device)) {
            return false;
        }
    }
    if let Some(device) = rules.device.as_ref() {
        if !device.eq_ignore_ascii_case(&ctx.device) {
            return false;
        }
    }
    if let Some(statuses) = rules.user_status.as_ref() {
        let ok = statuses
            .iter()
            .any(|s| s == "all" || s.eq_ignore_ascii_case(&ctx.user_status));
        if !ok {
            return false;
        }
    }
    if let Some(geo) = rules.geo.as_ref() {
        match ctx.geo_country.as_deref() {
            Some(cc) => {
                if !geo.iter().any(|g| g.eq_ignore_ascii_case(cc)) {
                    return false;
                }
            }
            None => return false,
        }
    }
    if !utm_match(&rules.utm_source, ctx.utm_source.as_deref()) {
        return false;
    }
    if !utm_match(&rules.utm_medium, ctx.utm_medium.as_deref()) {
        return false;
    }
    if !utm_match(&rules.utm_campaign, ctx.utm_campaign.as_deref()) {
        return false;
    }
    if let Some(inc) = rules.url_include_regex.as_ref() {
        match Regex::new(inc) {
            Ok(re) if re.is_match(&ctx.page_path) => {}
            _ => return false,
        }
    }
    if let Some(exc) = rules.url_exclude_regex.as_ref() {
        if let Ok(re) = Regex::new(exc) {
            if re.is_match(&ctx.page_path) {
                return false;
            }
        }
    }
    if let Some(min) = rules.cart_value_cents_min {
        if ctx.cart_value_cents.unwrap_or(0) < min {
            return false;
        }
    }
    if let Some(max) = rules.cart_value_cents_max {
        if ctx.cart_value_cents.unwrap_or(i64::MAX) > max {
            return false;
        }
    }
    if let Some(skus) = rules.cart_contains_sku.as_ref() {
        if !skus.iter().any(|s| ctx.cart_skus.iter().any(|v| v == s)) {
            return false;
        }
    }
    if let Some(tiers) = rules.membership_tier.as_ref() {
        let ok = ctx
            .membership_tier
            .as_deref()
            .map(|t| tiers.iter().any(|x| x.eq_ignore_ascii_case(t)))
            .unwrap_or(false);
        if !ok {
            return false;
        }
    }
    if let Some(start) = rules.time_of_day_start.as_ref() {
        if !time_at_or_after(&ctx.now, start) {
            return false;
        }
    }
    if let Some(end) = rules.time_of_day_end.as_ref() {
        if !time_at_or_before(&ctx.now, end) {
            return false;
        }
    }
    if let Some(dow) = rules.day_of_week.as_ref() {
        let day = ctx.now.weekday().num_days_from_sunday() as u8;
        if !dow.contains(&day) {
            return false;
        }
    }
    if let Some(expected) = rules.returning_visitor {
        if expected != ctx.returning_visitor {
            return false;
        }
    }
    if let Some(families) = rules.browser_family.as_ref() {
        let ok = ctx
            .browser_family
            .as_deref()
            .map(|b| families.iter().any(|x| x.eq_ignore_ascii_case(b)))
            .unwrap_or(false);
        if !ok {
            return false;
        }
    }
    if let Some(max_pv) = rules.new_visitor_max_pageviews {
        if ctx.pageview_count > max_pv {
            return false;
        }
    }
    true
}

fn utm_match(rule: &Option<Vec<String>>, actual: Option<&str>) -> bool {
    match rule {
        None => true,
        Some(values) => match actual {
            Some(a) => values.iter().any(|v| v.eq_ignore_ascii_case(a)),
            None => false,
        },
    }
}

fn matches_page_pattern(pattern: &str, path: &str) -> bool {
    if pattern == "*" {
        return true;
    }
    if let Some(prefix) = pattern.strip_suffix('*') {
        return path.starts_with(prefix);
    }
    pattern == path
}

fn parse_hhmm(s: &str) -> Option<(u32, u32)> {
    let mut parts = s.splitn(2, ':');
    let h = parts.next()?.parse::<u32>().ok()?;
    let m = parts.next()?.parse::<u32>().ok()?;
    if h < 24 && m < 60 {
        Some((h, m))
    } else {
        None
    }
}

fn time_at_or_after(now: &DateTime<Utc>, hhmm: &str) -> bool {
    match parse_hhmm(hhmm) {
        Some((h, m)) => (now.hour(), now.minute()) >= (h, m),
        None => true,
    }
}

fn time_at_or_before(now: &DateTime<Utc>, hhmm: &str) -> bool {
    match parse_hhmm(hhmm) {
        Some((h, m)) => (now.hour(), now.minute()) <= (h, m),
        None => true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn ctx() -> VisitorContext {
        VisitorContext {
            page_path: "/pricing".into(),
            device: "desktop".into(),
            user_status: "anonymous".into(),
            geo_country: Some("US".into()),
            utm_source: Some("google".into()),
            utm_medium: Some("cpc".into()),
            utm_campaign: Some("spring".into()),
            cart_value_cents: Some(5_000),
            cart_skus: vec!["sku-a".into(), "sku-b".into()],
            membership_tier: Some("pro".into()),
            returning_visitor: true,
            browser_family: Some("Chrome".into()),
            pageview_count: 3,
            now: Utc.with_ymd_and_hms(2026, 4, 17, 14, 30, 0).unwrap(),
        }
    }

    #[test]
    fn empty_rules_match_everything() {
        assert!(matches_targeting_rules(&TargetingRules::default(), &ctx()));
    }

    #[test]
    fn page_rule() {
        let mut r = TargetingRules::default();
        r.pages = Some(vec!["/pricing".into()]);
        assert!(matches_targeting_rules(&r, &ctx()));
        r.pages = Some(vec!["/other".into()]);
        assert!(!matches_targeting_rules(&r, &ctx()));
    }

    #[test]
    fn device_rule() {
        let mut r = TargetingRules::default();
        r.devices = Some(vec!["desktop".into(), "tablet".into()]);
        assert!(matches_targeting_rules(&r, &ctx()));
        r.devices = Some(vec!["mobile".into()]);
        assert!(!matches_targeting_rules(&r, &ctx()));
    }

    #[test]
    fn user_status_rule() {
        let mut r = TargetingRules::default();
        r.user_status = Some(vec!["anonymous".into()]);
        assert!(matches_targeting_rules(&r, &ctx()));
        r.user_status = Some(vec!["authenticated".into()]);
        assert!(!matches_targeting_rules(&r, &ctx()));
        r.user_status = Some(vec!["all".into()]);
        assert!(matches_targeting_rules(&r, &ctx()));
    }

    #[test]
    fn geo_rule() {
        let mut r = TargetingRules::default();
        r.geo = Some(vec!["US".into(), "CA".into()]);
        assert!(matches_targeting_rules(&r, &ctx()));
        r.geo = Some(vec!["FR".into()]);
        assert!(!matches_targeting_rules(&r, &ctx()));
        // missing geo_country fails closed when a rule is configured
        let mut c = ctx();
        c.geo_country = None;
        assert!(!matches_targeting_rules(&r, &c));
    }

    #[test]
    fn utm_rules() {
        let mut r = TargetingRules::default();
        r.utm_source = Some(vec!["Google".into()]);
        assert!(matches_targeting_rules(&r, &ctx()));
        r.utm_source = Some(vec!["bing".into()]);
        assert!(!matches_targeting_rules(&r, &ctx()));
        r.utm_source = None;
        r.utm_campaign = Some(vec!["summer".into()]);
        assert!(!matches_targeting_rules(&r, &ctx()));
    }

    #[test]
    fn url_regex_rules() {
        let mut r = TargetingRules::default();
        r.url_include_regex = Some(r"^/pric".into());
        assert!(matches_targeting_rules(&r, &ctx()));
        r.url_include_regex = Some(r"^/blog".into());
        assert!(!matches_targeting_rules(&r, &ctx()));
        r.url_include_regex = None;
        r.url_exclude_regex = Some(r"/pricing".into());
        assert!(!matches_targeting_rules(&r, &ctx()));
    }

    #[test]
    fn cart_value_and_sku() {
        let mut r = TargetingRules::default();
        r.cart_value_cents_min = Some(1_000);
        r.cart_value_cents_max = Some(10_000);
        assert!(matches_targeting_rules(&r, &ctx()));
        r.cart_value_cents_min = Some(10_000);
        assert!(!matches_targeting_rules(&r, &ctx()));
        r.cart_value_cents_min = None;
        r.cart_value_cents_max = None;
        r.cart_contains_sku = Some(vec!["sku-z".into()]);
        assert!(!matches_targeting_rules(&r, &ctx()));
        r.cart_contains_sku = Some(vec!["sku-b".into()]);
        assert!(matches_targeting_rules(&r, &ctx()));
    }

    #[test]
    fn membership_and_returning() {
        let mut r = TargetingRules::default();
        r.membership_tier = Some(vec!["pro".into(), "enterprise".into()]);
        assert!(matches_targeting_rules(&r, &ctx()));
        r.membership_tier = Some(vec!["free".into()]);
        assert!(!matches_targeting_rules(&r, &ctx()));
        r.membership_tier = None;
        r.returning_visitor = Some(false);
        assert!(!matches_targeting_rules(&r, &ctx()));
        r.returning_visitor = Some(true);
        assert!(matches_targeting_rules(&r, &ctx()));
    }

    #[test]
    fn time_and_day_rules() {
        let mut r = TargetingRules::default();
        r.time_of_day_start = Some("09:00".into());
        r.time_of_day_end = Some("17:00".into());
        assert!(matches_targeting_rules(&r, &ctx()));
        r.time_of_day_start = Some("15:00".into());
        assert!(!matches_targeting_rules(&r, &ctx()));
        r.time_of_day_start = None;
        r.time_of_day_end = None;
        // 2026-04-17 is a Friday = day 5
        r.day_of_week = Some(vec![5]);
        assert!(matches_targeting_rules(&r, &ctx()));
        r.day_of_week = Some(vec![0, 6]);
        assert!(!matches_targeting_rules(&r, &ctx()));
    }

    #[test]
    fn browser_and_pageviews() {
        let mut r = TargetingRules::default();
        r.browser_family = Some(vec!["chrome".into()]);
        assert!(matches_targeting_rules(&r, &ctx()));
        r.browser_family = Some(vec!["safari".into()]);
        assert!(!matches_targeting_rules(&r, &ctx()));
        r.browser_family = None;
        r.new_visitor_max_pageviews = Some(5);
        assert!(matches_targeting_rules(&r, &ctx()));
        r.new_visitor_max_pageviews = Some(1);
        assert!(!matches_targeting_rules(&r, &ctx()));
    }

    #[test]
    fn composite_rule_all_pass() {
        let mut r = TargetingRules::default();
        r.pages = Some(vec!["/pric*".into()]);
        r.geo = Some(vec!["US".into()]);
        r.utm_source = Some(vec!["google".into()]);
        r.cart_value_cents_min = Some(100);
        r.membership_tier = Some(vec!["pro".into()]);
        r.day_of_week = Some(vec![5]);
        assert!(matches_targeting_rules(&r, &ctx()));

        // Flip one dimension and fail.
        r.geo = Some(vec!["FR".into()]);
        assert!(!matches_targeting_rules(&r, &ctx()));
    }
}
