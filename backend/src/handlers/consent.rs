//! CONSENT-01 — public banner config endpoint.
//! CONSENT-03 — consent event log (`POST /record`, `GET /me`) + DSAR workflow
//! (`POST /api/dsar`, admin router under `/api/admin/consent`).
//! CONSENT-05 — geo-resolved banner region variants.
//!
//! `GET /api/consent/banner?locale=<bcp47>&region=<override>` resolves the
//! active banner config for the request's `(region, locale)` pair via
//! [`crate::consent::repo::resolve_banner`] and returns it alongside the
//! category catalogue and current policy version.
//!
//! Region is resolved server-side from the request headers (Cloudflare /
//! Vercel country headers + MaxMind fallback) through
//! [`crate::consent::geo::resolve_region`]. The optional `region` query
//! parameter is a dev/test affordance; production callers should omit it.
//!
//! The response shape is the wire contract consumed by `src/lib/api/consent.ts`
//! on the frontend; the Svelte banner hydrates directly from this payload.
//! Admin CRUD for banners/categories/services/policies belongs to CONSENT-07
//! and lives under `/api/admin/consent/*`.

use std::net::{IpAddr, Ipv4Addr, SocketAddr};

use axum::{
    extract::{ConnectInfo, Path, Query, State},
    http::HeaderMap,
    routing::{get, post},
    Json, Router,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

use crate::{
    common::geo::country_from_request,
    consent::{
        dsar_export,
        geo as region_resolver,
        records::{
            self, ConsentRecordInput, ConsentRecordRow, DsarCreateInput, DsarRow, SubjectSelector,
        },
        repo,
    },
    error::{AppError, AppResult},
    extractors::{AdminUser, AuthUser, OptionalAuthUser},
    notifications::{
        send::{send_notification, Recipient, SendOptions},
        NotifyError,
    },
    AppState,
};

// ── Router ──────────────────────────────────────────────────────────────

pub fn public_router() -> Router<AppState> {
    Router::new()
        .route("/banner", get(get_banner))
        .route("/me", get(get_my_consent))
        .merge(
            Router::new()
                .route("/record", post(post_record))
                .layer(crate::middleware::rate_limit::consent_record_layer()),
        )
}

/// Mounted under `/api/dsar` by `main.rs`.
pub fn public_dsar_router() -> Router<AppState> {
    Router::new()
        .route("/", post(post_dsar))
        .layer(crate::middleware::rate_limit::dsar_submit_layer())
}

/// Mounted under `/api/admin/consent` by `main.rs`.
pub fn admin_router() -> Router<AppState> {
    Router::new()
        .route("/dsar", get(admin_list_dsar))
        .route("/dsar/{id}/fulfill", post(admin_fulfill_dsar))
}

// ── Query + response shapes ─────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct BannerQuery {
    /// BCP-47 locale tag. Falls back to `en` when missing or unrecognised.
    pub locale: Option<String>,
    /// Optional region override. Production clients should omit this — the
    /// server resolves the region from request headers. Dev / admin preview
    /// forces it so an admin can see the EU banner without spoofing headers.
    pub region: Option<String>,
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
        ("locale" = Option<String>, Query, description = "BCP-47 locale tag; defaults to 'en'."),
        ("region" = Option<String>, Query, description = "Regulatory region override (EU, UK, US-CA, …). Usually omitted — server resolves from request headers.")
    ),
    responses(
        (status = 200, description = "Resolved banner config", body = BannerConfig),
        (status = 503, description = "Consent tables not seeded")
    )
)]
pub async fn get_banner(
    State(state): State<AppState>,
    headers: HeaderMap,
    connect: ConnectInfo<SocketAddr>,
    Query(q): Query<BannerQuery>,
) -> AppResult<Json<BannerConfig>> {
    let locale = normalise_locale(q.locale.as_deref());
    let region = resolve_region_from_request(&headers, Some(connect), q.region.as_deref());

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

/// CONSENT-05 region resolver. Honors (in order):
///
///   1. An explicit `?region=` query override (dev / admin preview only —
///      trimmed + truncated to 16 chars so a malicious caller can't force an
///      index scan on a giant string).
///   2. `common::geo::country_from_request(..)` → `consent::geo::resolve_region`.
///   3. The literal `"default"` bucket when no country can be determined.
///
/// The resulting string is always one of the admin-configured bucket names
/// (`default`, `EU`, `UK`, `US-CA`, `US-CO`, `US-STATE`, `CA`, `CA-QC`, `BR`).
fn resolve_region_from_request(
    headers: &HeaderMap,
    connect: Option<ConnectInfo<SocketAddr>>,
    override_param: Option<&str>,
) -> String {
    if let Some(raw) = override_param {
        let trimmed = raw.trim();
        if !trimmed.is_empty() {
            // Bound the string so a pathological query can't OOM a row lookup.
            let bounded: String = trimmed.chars().take(16).collect();
            return bounded;
        }
    }

    let remote_ip: IpAddr = connect
        .as_ref()
        .map(|c| c.0.ip())
        .unwrap_or_else(|| IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)));

    let country = country_from_request(headers, remote_ip);
    region_resolver::resolve_region(country).to_string()
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

// ══════════════════════════════════════════════════════════════════════
// CONSENT-03 — consent event log
// ══════════════════════════════════════════════════════════════════════

#[derive(Debug, Clone, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ConsentRecordRequest {
    /// One of `granted` / `denied` / `updated` / `revoked` / `expired` /
    /// `prefill`. Enforced at the DB CHECK level; validated here for a
    /// friendlier 400.
    pub action: String,
    /// Map of category key → granted bool. Must include every category the
    /// current banner version exposes.
    pub categories: serde_json::Value,
    /// Optional per-service overrides (when the subject used the advanced
    /// picker). Empty object is fine.
    #[serde(default)]
    pub services: Option<serde_json::Value>,
    #[serde(default)]
    pub tcf_string: Option<String>,
    #[serde(default)]
    pub gpc_signal: Option<bool>,
    /// Browser-generated anonymous id (UUID cookie). Used when the subject
    /// is not signed in so the audit log can still be linked across sessions.
    #[serde(default)]
    pub anonymous_id: Option<Uuid>,
    /// Optional banner / policy version overrides — defaults come from the
    /// authoritative banner config when omitted. Kept as options so the
    /// client can send them when it already has a cached banner copy.
    #[serde(default)]
    pub banner_version: Option<i32>,
    #[serde(default)]
    pub policy_version: Option<i32>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ConsentRecordResponse {
    pub id: Uuid,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct MyConsentResponse {
    /// Flattened category → granted map, derived from the most recent record.
    pub categories: serde_json::Value,
    /// ISO-8601 UTC. `None` if the subject has no recorded decisions yet.
    pub decided_at: Option<DateTime<Utc>>,
    pub records: Vec<ConsentRecordRow>,
}

/// Valid DB enum values for `consent_records.action`. Kept in one place so
/// the handler's validation and the migration stay synchronised.
const CONSENT_ACTIONS: [&str; 6] = [
    "granted", "denied", "updated", "revoked", "expired", "prefill",
];

/// `POST /api/consent/record`
#[utoipa::path(
    post,
    path = "/api/consent/record",
    tag = "consent",
    request_body = ConsentRecordRequest,
    responses(
        (status = 200, description = "Consent event recorded", body = ConsentRecordResponse),
        (status = 400, description = "Invalid action or missing subject identifier")
    )
)]
pub(crate) async fn post_record(
    State(state): State<AppState>,
    OptionalAuthUser { user_id }: OptionalAuthUser,
    headers: HeaderMap,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    Json(req): Json<ConsentRecordRequest>,
) -> AppResult<Json<ConsentRecordResponse>> {
    if !CONSENT_ACTIONS.contains(&req.action.as_str()) {
        return Err(AppError::BadRequest(format!(
            "action must be one of {CONSENT_ACTIONS:?}"
        )));
    }
    if user_id.is_none() && req.anonymous_id.is_none() {
        return Err(AppError::BadRequest(
            "either Authorization or anonymousId is required".into(),
        ));
    }
    if !req.categories.is_object() {
        return Err(AppError::BadRequest("categories must be an object".into()));
    }

    // Resolve banner/policy versions. If the client did not supply them, look
    // up the current active banner for the request's region+locale so the
    // audit row carries the version the subject actually saw.
    let region = resolve_region_from_request(&headers, Some(ConnectInfo(peer)), None);
    let (banner_version, policy_version) =
        if let (Some(bv), Some(pv)) = (req.banner_version, req.policy_version) {
            (bv, pv)
        } else {
            let banner = repo::resolve_banner(&state.db, &region, "en").await?;
            let bv = banner.as_ref().map(|b| b.version).unwrap_or(1);
            let policy = repo::latest_policy(&state.db, "en").await?;
            let pv = policy.map(|p| p.version).unwrap_or(1);
            (req.banner_version.unwrap_or(bv), req.policy_version.unwrap_or(pv))
        };

    let ip = client_ip_from(&headers, peer);
    let ip_hash = records::hash_ip(&ip);
    let user_agent = headers
        .get(axum::http::header::USER_AGENT)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.chars().take(512).collect::<String>())
        .unwrap_or_else(|| "unknown".into());
    let country = headers
        .get("cf-ipcountry")
        .or_else(|| headers.get("x-vercel-ip-country"))
        .and_then(|v| v.to_str().ok())
        .filter(|s| s.len() == 2 && s.bytes().all(|b| b.is_ascii_alphabetic()))
        .map(|s| s.to_ascii_uppercase());

    let input = ConsentRecordInput {
        subject_id: user_id,
        anonymous_id: req.anonymous_id,
        ip_hash,
        user_agent,
        country,
        banner_version,
        policy_version,
        categories: req.categories,
        services: req.services.unwrap_or_else(|| serde_json::json!({})),
        action: req.action,
        tcf_string: req.tcf_string,
        gpc_signal: req.gpc_signal,
    };

    let id = records::insert_consent_record(&state.db, &input).await?;
    Ok(Json(ConsentRecordResponse { id }))
}

/// `GET /api/consent/me`
#[utoipa::path(
    get,
    path = "/api/consent/me",
    tag = "consent",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Subject consent state", body = MyConsentResponse),
        (status = 401, description = "Authentication required")
    )
)]
pub(crate) async fn get_my_consent(
    State(state): State<AppState>,
    user: AuthUser,
) -> AppResult<Json<MyConsentResponse>> {
    let records =
        records::list_records_for_subject(&state.db, SubjectSelector::Subject(user.user_id), 50)
            .await?;

    let (categories, decided_at) = records
        .first()
        .map(|r| (r.categories.clone(), Some(r.created_at)))
        .unwrap_or_else(|| (serde_json::json!({}), None));

    Ok(Json(MyConsentResponse {
        categories,
        decided_at,
        records,
    }))
}

// ══════════════════════════════════════════════════════════════════════
// CONSENT-03 — DSAR workflow
// ══════════════════════════════════════════════════════════════════════

const DSAR_KINDS: [&str; 5] = [
    "access",
    "delete",
    "portability",
    "rectification",
    "opt_out_sale",
];

#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct DsarSubmitRequest {
    pub email: String,
    pub kind: String,
    #[serde(default)]
    pub payload: Option<serde_json::Value>,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct DsarSubmitResponse {
    pub id: Uuid,
}

#[derive(Debug, Deserialize, IntoParams)]
#[serde(rename_all = "camelCase")]
pub struct DsarListQuery {
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub page: Option<i64>,
    #[serde(default)]
    pub per_page: Option<i64>,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct DsarListResponse {
    pub data: Vec<DsarRow>,
    pub total: i64,
    pub page: i64,
    pub per_page: i64,
    pub total_pages: i64,
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct DsarFulfillRequest {
    #[serde(default)]
    pub fulfillment_url: Option<String>,
    #[serde(default)]
    pub admin_notes: Option<String>,
}

/// Admin fulfill response. When the DSAR is `access` / `portability` and the
/// admin did not provide a URL, the exported JSON is inlined in `export` so
/// the caller can pipe it to a download without a second round-trip.
#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct DsarFulfillResponse {
    pub request: DsarRow,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub export: Option<serde_json::Value>,
}

/// `POST /api/dsar` (public — no auth required).
#[utoipa::path(
    post,
    path = "/api/dsar",
    tag = "consent",
    request_body = DsarSubmitRequest,
    responses(
        (status = 200, description = "DSAR request accepted", body = DsarSubmitResponse),
        (status = 400, description = "Missing or invalid fields")
    )
)]
pub(crate) async fn post_dsar(
    State(state): State<AppState>,
    Json(req): Json<DsarSubmitRequest>,
) -> AppResult<Json<DsarSubmitResponse>> {
    let email = req.email.trim().to_ascii_lowercase();
    if email.is_empty() || !email.contains('@') {
        return Err(AppError::BadRequest("valid email is required".into()));
    }
    if !DSAR_KINDS.contains(&req.kind.as_str()) {
        return Err(AppError::BadRequest(format!(
            "kind must be one of {DSAR_KINDS:?}"
        )));
    }

    // Mint a one-shot verification token, store only its hash, and e-mail the
    // raw token to the subject. The verification page is a CONSENT-07 concern;
    // `verify_url` points at it unconditionally so the template is stable.
    let raw_token = Uuid::new_v4().simple().to_string();
    let digest = Sha256::digest(raw_token.as_bytes());
    let token_hash = hex_lower(&digest[..]);

    let input = DsarCreateInput {
        user_id: None, // subject_id resolution is part of fulfilment, not submission
        email: email.clone(),
        kind: req.kind.clone(),
        payload: req.payload.unwrap_or_else(|| serde_json::json!({})),
        verification_token_hash: Some(token_hash),
    };
    let row = records::create_dsar_request(&state.db, &input).await?;

    // Best-effort notification — send failures log but do not fail the
    // submission. The admin can still fulfil the request through the queue
    // even if e-mail provider is momentarily down.
    let verify_url = format!(
        "{}/privacy/dsar/verify?id={}&token={}",
        state.config.app_url.trim_end_matches('/'),
        row.id,
        raw_token
    );
    let ctx = serde_json::json!({
        "kind": req.kind,
        "verify_url": verify_url,
        "app_url": state.config.app_url,
        "year": Utc::now().format("%Y").to_string(),
    });
    match send_notification(
        &state.db,
        "dsar.verify",
        &Recipient::Anonymous { email },
        ctx,
        SendOptions::default(),
    )
    .await
    {
        Ok(_) => {}
        Err(NotifyError::Template(_)) => {
            tracing::warn!(
                dsar_id = %row.id,
                "dsar.verify template not seeded — migration 025 likely missing"
            );
        }
        Err(e) => {
            tracing::warn!(dsar_id = %row.id, error = %e, "failed to enqueue dsar verify email");
        }
    }

    Ok(Json(DsarSubmitResponse { id: row.id }))
}

/// `GET /api/admin/consent/dsar` (AdminUser).
#[utoipa::path(
    get,
    path = "/api/admin/consent/dsar",
    tag = "consent",
    security(("bearer_auth" = [])),
    params(DsarListQuery),
    responses(
        (status = 200, description = "Paginated DSAR list", body = DsarListResponse),
        (status = 403, description = "Forbidden")
    )
)]
pub(crate) async fn admin_list_dsar(
    State(state): State<AppState>,
    _admin: AdminUser,
    Query(q): Query<DsarListQuery>,
) -> AppResult<Json<DsarListResponse>> {
    let per_page = q.per_page.unwrap_or(25).clamp(1, 200);
    let page = q.page.unwrap_or(1).max(1);
    let offset = (page - 1) * per_page;

    let status_filter = q.status.as_deref().filter(|s| !s.is_empty());
    let data = records::list_dsar_requests(&state.db, status_filter, per_page, offset).await?;
    let total = records::count_dsar_requests(&state.db, status_filter).await?;
    let total_pages = if per_page > 0 {
        ((total as f64) / (per_page as f64)).ceil() as i64
    } else {
        0
    };

    Ok(Json(DsarListResponse {
        data,
        total,
        page,
        per_page,
        total_pages,
    }))
}

/// `POST /api/admin/consent/dsar/{id}/fulfill` (AdminUser).
#[utoipa::path(
    post,
    path = "/api/admin/consent/dsar/{id}/fulfill",
    tag = "consent",
    security(("bearer_auth" = [])),
    params(("id" = Uuid, Path, description = "DSAR request id")),
    request_body = DsarFulfillRequest,
    responses(
        (status = 200, description = "DSAR fulfilled", body = DsarFulfillResponse),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "DSAR not found")
    )
)]
pub(crate) async fn admin_fulfill_dsar(
    State(state): State<AppState>,
    admin: AdminUser,
    Path(id): Path<Uuid>,
    Json(req): Json<DsarFulfillRequest>,
) -> AppResult<Json<DsarFulfillResponse>> {
    let existing = records::get_dsar(&state.db, id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("DSAR {id} not found")))?;

    // For access / portability requests with no admin-supplied URL, build a
    // JSON export of the subject's data and inline it as a data: URI. Full
    // R2 upload is tracked for a later subsystem.
    let (effective_url, export_json): (Option<String>, Option<serde_json::Value>) = if req
        .fulfillment_url
        .is_none()
        && matches!(existing.kind.as_str(), "access" | "portability")
    {
        let export = dsar_export::build_export(&state.db, &existing).await?;
        let value = serde_json::to_value(&export).map_err(|e| {
            AppError::Internal(anyhow::anyhow!("dsar export serialization failed: {e}"))
        })?;
        let uri = dsar_export::export_to_data_uri(&export);
        (Some(uri), Some(value))
    } else {
        (req.fulfillment_url.clone(), None)
    };

    let updated = records::fulfill_dsar(
        &state.db,
        id,
        admin.user_id,
        effective_url.as_deref(),
        req.admin_notes.as_deref(),
    )
    .await?
    .ok_or_else(|| AppError::NotFound(format!("DSAR {id} disappeared during update")))?;

    Ok(Json(DsarFulfillResponse {
        request: updated,
        export: export_json,
    }))
}

// ── Shared helpers ──────────────────────────────────────────────────────

fn hex_lower(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        out.push(HEX[(b >> 4) as usize] as char);
        out.push(HEX[(b & 0x0F) as usize] as char);
    }
    out
}

/// Mirror `middleware::rate_limit::client_ip` precedence so hashes stay in
/// sync with the rate-limit keying. Trimmed, lowercased for the salt mix.
fn client_ip_from(headers: &HeaderMap, peer: SocketAddr) -> String {
    if let Some(v) = headers.get("x-forwarded-for").and_then(|v| v.to_str().ok()) {
        if let Some(first) = v.split(',').next() {
            let t = first.trim();
            if !t.is_empty() {
                return t.to_ascii_lowercase();
            }
        }
    }
    if let Some(v) = headers.get("x-real-ip").and_then(|v| v.to_str().ok()) {
        let t = v.trim();
        if !t.is_empty() {
            return t.to_ascii_lowercase();
        }
    }
    peer.ip().to_string()
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

    #[test]
    fn hex_lower_encodes_all_bytes() {
        assert_eq!(hex_lower(&[0x00, 0xff, 0x10, 0xab]), "00ff10ab");
    }

    #[test]
    fn consent_actions_match_migration() {
        // The DB CHECK enforces the same set; the handler refuses unknown
        // strings early so the caller sees a friendly 400 not a sqlx error.
        for a in ["granted", "denied", "updated", "revoked", "expired", "prefill"] {
            assert!(CONSENT_ACTIONS.contains(&a), "missing action {a}");
        }
    }

    #[test]
    fn dsar_kinds_match_migration() {
        for k in ["access", "delete", "portability", "rectification", "opt_out_sale"] {
            assert!(DSAR_KINDS.contains(&k), "missing kind {k}");
        }
    }

    #[test]
    fn client_ip_prefers_forwarded_header() {
        let mut h = HeaderMap::new();
        h.insert("x-forwarded-for", "1.2.3.4, 10.0.0.1".parse().unwrap());
        let peer: SocketAddr = "9.9.9.9:443".parse().unwrap();
        assert_eq!(client_ip_from(&h, peer), "1.2.3.4");

        let empty = HeaderMap::new();
        assert_eq!(client_ip_from(&empty, peer), "9.9.9.9");
    }
}
