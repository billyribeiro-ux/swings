//! CONSENT-01 — public banner config endpoint.
//!
//! `GET /api/consent/banner?locale=<bcp47>` resolves the active banner config
//! for the request's `(region, locale)` pair via
//! [`crate::consent::repo::resolve_banner`] and returns it alongside the
//! category catalogue and current policy version.
//!
//! Region resolution is a CONSENT-05 concern; this handler currently passes
//! `"default"` so the single seeded row is always returned. When CONSENT-05
//! lands the geo resolver, only [`resolve_region`] below changes — the rest
//! of the response shape is already region-aware.
//!
//! The response shape is the wire contract consumed by `src/lib/api/consent.ts`
//! on the frontend; the Svelte banner hydrates directly from this payload.
//! Admin CRUD for banners/categories/services/policies belongs to CONSENT-07
//! and lives under `/api/admin/consent/*`.

use axum::{extract::Query, extract::State, routing::get, Json, Router};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::{
    consent::repo,
    error::{AppError, AppResult},
    AppState,
};

// ── Router ──────────────────────────────────────────────────────────────

pub fn public_router() -> Router<AppState> {
    Router::new().route("/banner", get(get_banner))
}

// ── Query + response shapes ─────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct BannerQuery {
    /// BCP-47 locale tag. Falls back to `en` when missing or unrecognised.
    pub locale: Option<String>,
}

/// Single-category entry in the banner response.
///
/// Wire fields are camelCased to match the frontend stub at
/// `src/lib/api/consent.ts`; when schema codegen replaces the stub the
/// Svelte components consume this shape unchanged.
#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ConsentCategoryDef {
    /// Stable key — MUST NOT be renamed after a row has been written to the
    /// consent log (CONSENT-03). Migration-level change only.
    pub key: String,
    pub label: String,
    pub description: String,
    /// When true, toggle is disabled in the preferences modal.
    pub required: bool,
    /// Whether the category is pre-checked before the user interacts.
    /// GDPR Art. 4(11) + EDPB 05/2020 §86: non-required categories MUST default
    /// to `false`. Derived here rather than stored so a data fix is cheap.
    pub default_enabled: bool,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum BannerLayout {
    Bar,
    Box,
    Popup,
    Fullscreen,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "kebab-case")]
pub enum BannerPosition {
    Top,
    Bottom,
    Center,
    BottomStart,
    BottomEnd,
}

impl BannerLayout {
    fn parse(s: &str) -> Self {
        match s {
            "box" => Self::Box,
            "popup" => Self::Popup,
            "fullscreen" => Self::Fullscreen,
            _ => Self::Bar,
        }
    }
}

impl BannerPosition {
    fn parse(s: &str) -> Self {
        match s {
            "top" => Self::Top,
            "center" => Self::Center,
            "bottom-start" => Self::BottomStart,
            "bottom-end" => Self::BottomEnd,
            _ => Self::Bottom,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct BannerCopy {
    pub title: String,
    pub body: String,
    pub accept_all: String,
    pub reject_all: String,
    pub customize: String,
    pub save_preferences: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub privacy_policy_href: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub privacy_policy_label: Option<String>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct BannerConfig {
    /// Banner config row version. Bumped when copy/layout changes; used by
    /// CONSENT-03 to re-prompt subjects whose recorded consent predates this.
    pub version: i32,
    /// Current privacy-policy version, sourced from `consent_policies`.
    pub policy_version: i32,
    pub layout: BannerLayout,
    pub position: BannerPosition,
    pub locale: String,
    pub region: String,
    pub categories: Vec<ConsentCategoryDef>,
    pub copy: BannerCopy,
    /// Opaque theme overrides; the frontend maps known keys to PE7 CSS vars.
    pub theme: serde_json::Value,
}

// ── Handlers ────────────────────────────────────────────────────────────

/// Resolve the active banner config + category list + current policy version.
///
/// Always returns 200 with the `default`/`en` seed if no better match exists.
/// The only error shape is a `503` if the tables are empty (which can only
/// happen if migration `024_consent.sql` has not run).
#[utoipa::path(
    get,
    path = "/api/consent/banner",
    tag = "consent",
    params(
        ("locale" = Option<String>, Query, description = "BCP-47 locale tag; defaults to 'en'.")
    ),
    responses(
        (status = 200, description = "Resolved banner config", body = BannerConfig),
        (status = 503, description = "Consent tables not seeded")
    )
)]
pub async fn get_banner(
    State(state): State<AppState>,
    Query(q): Query<BannerQuery>,
) -> AppResult<Json<BannerConfig>> {
    let locale = normalise_locale(q.locale.as_deref());
    let region = resolve_region();

    let banner = repo::resolve_banner(&state.db, &region, &locale)
        .await?
        .ok_or_else(|| {
            AppError::ServiceUnavailable(
                "consent banner configs not seeded; run migrations".to_string(),
            )
        })?;

    let categories_rows = repo::list_categories(&state.db).await?;
    let categories = categories_rows
        .into_iter()
        .map(|c| ConsentCategoryDef {
            key: c.key,
            label: c.label,
            description: c.description,
            // EDPB 05/2020 §86 — non-required categories default to off.
            default_enabled: c.is_required,
            required: c.is_required,
        })
        .collect();

    let copy = parse_copy(&banner.copy_json).unwrap_or_else(default_copy);

    let policy = repo::latest_policy(&state.db, &banner.locale).await?;
    let policy_version = policy.map(|p| p.version).unwrap_or(1);

    Ok(Json(BannerConfig {
        version: banner.version,
        policy_version,
        layout: BannerLayout::parse(&banner.layout),
        position: BannerPosition::parse(&banner.position),
        locale: banner.locale,
        region: banner.region,
        categories,
        copy,
        theme: banner.theme_json,
    }))
}

// ── Helpers ─────────────────────────────────────────────────────────────

/// Reduce an arbitrary BCP-47 tag to the canonical lowercase primary language
/// subtag (the only form the seed rows key on in CONSENT-01). CONSENT-06 will
/// widen this to full region-aware matching once the translation catalogues
/// ship. An empty / unparseable input falls back to `en`.
fn normalise_locale(raw: Option<&str>) -> String {
    let s = raw.unwrap_or("").trim();
    if s.is_empty() {
        return "en".to_string();
    }
    let lower = s.to_ascii_lowercase();
    let primary = lower.split(['-', '_']).next().unwrap_or("en");
    if primary.is_empty() {
        "en".to_string()
    } else {
        primary.to_string()
    }
}

/// Region resolver. CONSENT-01 returns the literal `"default"`; CONSENT-05
/// will replace this with a `common::geo`-backed lookup that maps
/// country-codes onto the `EU` / `US-CA` / etc. buckets the admin configures.
fn resolve_region() -> String {
    "default".to_string()
}

fn parse_copy(value: &serde_json::Value) -> Option<BannerCopy> {
    serde_json::from_value::<BannerCopy>(value.clone()).ok()
}

/// Defaults mirror the seed in `024_consent.sql` — used as a safety net if
/// an admin ever writes an incomplete `copy_json` blob via CONSENT-07.
fn default_copy() -> BannerCopy {
    BannerCopy {
        title: "We value your privacy".to_string(),
        body: "We use cookies and similar technologies to power the site, understand usage, and — with your permission — personalize content.".to_string(),
        accept_all: "Accept all".to_string(),
        reject_all: "Reject all".to_string(),
        customize: "Customize".to_string(),
        save_preferences: "Save preferences".to_string(),
        privacy_policy_href: Some("/privacy".to_string()),
        privacy_policy_label: Some("Privacy policy".to_string()),
    }
}

// ── Unit tests ──────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalise_locale_defaults_to_en() {
        assert_eq!(normalise_locale(None), "en");
        assert_eq!(normalise_locale(Some("")), "en");
        assert_eq!(normalise_locale(Some("   ")), "en");
    }

    #[test]
    fn normalise_locale_extracts_primary_subtag() {
        assert_eq!(normalise_locale(Some("EN")), "en");
        assert_eq!(normalise_locale(Some("en-GB")), "en");
        assert_eq!(normalise_locale(Some("pt_BR")), "pt");
        assert_eq!(normalise_locale(Some("zh-Hant-TW")), "zh");
    }

    #[test]
    fn default_copy_parses_round_trip() {
        let dc = default_copy();
        let v = serde_json::to_value(&dc).expect("serialize");
        let back = parse_copy(&v).expect("parse");
        assert_eq!(back.title, dc.title);
        assert_eq!(back.accept_all, dc.accept_all);
        assert_eq!(back.privacy_policy_href, dc.privacy_policy_href);
    }

    #[test]
    fn parse_copy_returns_none_for_bad_shape() {
        let v = serde_json::json!({ "title": "ok" });
        assert!(parse_copy(&v).is_none(), "missing fields should fail parse");
    }

    #[test]
    fn seed_copy_json_parses() {
        // Mirrors the jsonb_build_object in `024_consent.sql` seed. If this
        // test breaks after a migration edit, the seed and the handler have
        // diverged.
        let v = serde_json::json!({
            "title": "We value your privacy",
            "body": "copy",
            "acceptAll": "Accept all",
            "rejectAll": "Reject all",
            "customize": "Customize",
            "savePreferences": "Save preferences",
            "privacyPolicyHref": "/privacy",
            "privacyPolicyLabel": "Privacy policy"
        });
        let parsed = parse_copy(&v).expect("seed shape");
        assert_eq!(parsed.accept_all, "Accept all");
        assert_eq!(parsed.save_preferences, "Save preferences");
    }
}
