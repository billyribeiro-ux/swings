//! Minimal Stripe REST wrapper used by the billing/admin/forms paths.
//!
//! # Why hand-rolled?
//!
//! The previous implementation used `async-stripe 0.39`, which:
//!   * was flagged by RUSTSEC-2024-0384 (`instant` unmaintained, pulled
//!     transitively via `http-types` → `futures-lite` → `fastrand 1`),
//!   * pulled `hyper-tls` → `native-tls` → `openssl-sys` on Linux even
//!     though the rest of this crate is rustls-only,
//!   * required `pkg-config` + `libssl-dev` in the Docker builder.
//!
//! No maintained successor on crates.io ships rustls-only TLS today
//! (`async-stripe 1.0` is still rc.* and has not stabilised; the
//! `stripe-rust` crate is years stale). Since this codebase only hits
//! **6 REST endpoints** + a webhook signature verifier (already
//! hand-rolled in [`crate::handlers::webhooks`]), the dependency cost
//! of pulling a SDK is not justified.
//!
//! # API surface (kept stable for callers)
//!
//! Every function in this module preserves its previous public signature
//! so handlers under `handlers/admin*.rs`, `handlers/member.rs`, and
//! `handlers/forms.rs` are unaffected. Errors continue to surface as
//! `AppError::BadRequest("Stripe: …")`.
//!
//! # Endpoints
//!
//! | Function                                  | Method  | Path                                      |
//! | ----------------------------------------- | ------- | ----------------------------------------- |
//! | [`create_billing_portal_session`]         | POST    | `/v1/billing_portal/sessions`             |
//! | [`create_form_payment_intent`]            | POST    | `/v1/payment_intents`                     |
//! | [`cancel_subscription_immediately`]       | DELETE  | `/v1/subscriptions/{id}`                  |
//! | [`update_customer_address`]               | POST    | `/v1/customers/{id}`                      |
//! | [`apply_coupon_to_subscription`]          | POST    | `/v1/subscriptions/{id}` (coupon)         |
//! | [`set_subscription_cancel_at_period_end`] | POST    | `/v1/subscriptions/{id}`                  |
//! | [`retrieve_subscription`]                 | GET     | `/v1/subscriptions/{id}?expand[]=…`       |
//! | [`update_subscription_item_price`]        | POST    | `/v1/subscriptions/{id}` (items[…])       |
//!
//! # Idempotency-Key
//!
//! Every POST/DELETE accepts an optional idempotency key. When supplied,
//! the wrapper forwards it as the `Idempotency-Key` HTTP header — Stripe's
//! deduplication primitive. This fixes a long-standing latent bug where
//! `create_form_payment_intent` accepted an `_idempotency_key` parameter
//! that was silently dropped (the underscore prefix was load-bearing).
//!
//! # Stripe-Version
//!
//! We pin `Stripe-Version: 2024-06-20` on every request. Stripe's account
//! default version is whatever was current the day the account was
//! created; pinning here keeps the request shape stable across accounts
//! and prevents silent breakage when Stripe ships a backwards-incompatible
//! API release.

use std::collections::BTreeMap;
use std::time::Duration;

use reqwest::header::{HeaderMap, HeaderName, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use reqwest::{Client as HttpClient, Method, StatusCode};
use serde::Deserialize;

use crate::{
    error::{AppError, AppResult},
    AppState,
};

// ─── Constants ─────────────────────────────────────────────────────────

/// Default Stripe REST API base. Production always uses this; the
/// integration test suite overrides it through [`Config::stripe_api_base_url_override`]
/// so a `wiremock::MockServer` can stand in for the real API.
const STRIPE_API_BASE: &str = "https://api.stripe.com";
/// Stripe API version pin. Bump intentionally; the request shape and
/// response schema depend on this header.
const STRIPE_API_VERSION: &str = "2024-06-20";
/// Conservative per-request timeout. Stripe's own SLO targets sub-second
/// latency for the endpoints we hit; 30s allows for retry-on-network-error
/// behaviour without leaving sockets hanging on a wedged TLS handshake.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);
/// Connect timeout. Forensic Wave-3 (W3-6): the prior client only set
/// the per-request timeout, which means a hung TCP+TLS handshake (slow
/// DNS, BGP hijack, TLS-record drop) holds the call open for the full
/// 30s before reqwest gives up. A 5s connect ceiling fails fast on
/// unreachable network paths so the request worker can recover and the
/// upstream queue doesn't pile up on every checkout / webhook reply.
const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);

// ─── Form-encoding helpers (Stripe bracket notation) ───────────────────

/// Append a single `key=value` pair to a form-encoded body. We percent-
/// encode both halves with `urlencoding::encode` so caller-provided
/// strings (subscription ids, coupon ids, addresses with spaces) are
/// transmitted safely. Stripe expects `application/x-www-form-urlencoded`
/// for all REST writes — JSON is not accepted.
fn append_form_pair(body: &mut String, key: &str, value: &str) {
    if !body.is_empty() {
        body.push('&');
    }
    body.push_str(&urlencoding::encode(key));
    body.push('=');
    body.push_str(&urlencoding::encode(value));
}

/// Builder for Stripe's bracket-notation form bodies.
///
/// Stripe REST writes look like
/// `metadata[ref]=foo&items[0][id]=si_…&items[0][price]=price_…`. The
/// `serde_urlencoded` crate does not support this nested shape, so we
/// emit it ourselves.
#[derive(Default)]
struct FormBody {
    inner: String,
}

impl FormBody {
    fn new() -> Self {
        Self::default()
    }

    /// Append a flat scalar (`key=value`).
    fn push(&mut self, key: &str, value: impl AsRef<str>) -> &mut Self {
        append_form_pair(&mut self.inner, key, value.as_ref());
        self
    }

    /// Append a nested key (`outer[inner]=value`).
    fn push_nested(&mut self, outer: &str, inner: &str, value: impl AsRef<str>) -> &mut Self {
        let key = format!("{outer}[{inner}]");
        append_form_pair(&mut self.inner, &key, value.as_ref());
        self
    }

    fn into_body(self) -> String {
        self.inner
    }

    fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }
}

// ─── Lightweight Stripe client ─────────────────────────────────────────

/// Stripe HTTP client. Reqwest connection-pool reuse means each call
/// re-uses a kept-alive TLS connection; constructing the client per
/// request is fine for our call volume but we still cache it as part of
/// the workflow helpers below.
///
/// `base_url` is normally [`STRIPE_API_BASE`] (the production endpoint).
/// Tests override it via [`Client::with_base_url`] so a wiremock server
/// can stand in for the real Stripe API. The `https_only` builder flag
/// is therefore disabled — overriding to an `http://` mock is a test
/// concern only and never reaches production code paths because the
/// override is hydrated from a plumbed [`Config`] field rather than a
/// caller-supplied URL.
#[derive(Clone)]
pub struct Client {
    http: HttpClient,
    secret_key: String,
    base_url: String,
}

impl Client {
    /// Construct a new client pointed at production Stripe. `reqwest::Client::new()`
    /// would accept the builder defaults but we prefer to force the timeout
    /// explicitly so future reqwest releases that change the default never
    /// silently extend our blast radius on a wedged TLS handshake.
    pub fn new(secret_key: impl Into<String>) -> AppResult<Self> {
        Self::build(secret_key, STRIPE_API_BASE.to_string(), true)
    }

    /// Construct a new client pointed at a caller-supplied base URL —
    /// **for tests only**. Production callers go through [`Client::new`]
    /// (or the [`client`] helper) which pins the production base URL.
    /// `https_only` is disabled because integration tests run against an
    /// HTTP mock server.
    pub fn with_base_url(
        secret_key: impl Into<String>,
        base_url: impl Into<String>,
    ) -> AppResult<Self> {
        Self::build(secret_key, base_url.into(), false)
    }

    fn build(secret_key: impl Into<String>, base_url: String, https_only: bool) -> AppResult<Self> {
        let http = HttpClient::builder()
            .timeout(REQUEST_TIMEOUT)
            .connect_timeout(CONNECT_TIMEOUT)
            .https_only(https_only)
            .build()
            .map_err(|e| {
                AppError::BadRequest(format!("Stripe: failed to build HTTP client: {e}"))
            })?;
        Ok(Self {
            http,
            secret_key: secret_key.into(),
            base_url: base_url.trim_end_matches('/').to_string(),
        })
    }

    /// Issue a single request to Stripe. Optional `body` is form-encoded;
    /// optional `idempotency_key` is forwarded via the standard header.
    /// `query` is appended to the path verbatim (caller is responsible
    /// for percent-encoding any dynamic segments).
    async fn request<T>(
        &self,
        method: Method,
        path: &str,
        body: Option<String>,
        idempotency_key: Option<&str>,
    ) -> AppResult<T>
    where
        T: for<'de> Deserialize<'de>,
    {
        let url = format!("{}{path}", self.base_url);
        let mut headers = HeaderMap::new();
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&format!("Bearer {}", self.secret_key)).map_err(|_| {
                AppError::BadRequest("Stripe: secret key contains invalid header bytes".into())
            })?,
        );
        let stripe_version_header = HeaderName::from_static("stripe-version");
        headers.insert(
            stripe_version_header,
            HeaderValue::from_static(STRIPE_API_VERSION),
        );
        if body.is_some() {
            headers.insert(
                CONTENT_TYPE,
                HeaderValue::from_static("application/x-www-form-urlencoded"),
            );
        }
        if let Some(key) = idempotency_key {
            let header_name = HeaderName::from_static("idempotency-key");
            let header_value = HeaderValue::from_str(key).map_err(|_| {
                AppError::BadRequest("Stripe: idempotency key contains invalid header bytes".into())
            })?;
            headers.insert(header_name, header_value);
        }

        let mut req = self.http.request(method, &url).headers(headers);
        if let Some(body) = body {
            req = req.body(body);
        }

        let resp = req
            .send()
            .await
            .map_err(|e| AppError::BadRequest(format!("Stripe: request failed: {e}")))?;

        let status = resp.status();
        let bytes = resp
            .bytes()
            .await
            .map_err(|e| AppError::BadRequest(format!("Stripe: read body failed: {e}")))?;

        if !status.is_success() {
            return Err(map_stripe_error(status, &bytes));
        }

        // Some endpoints (DELETE) return a body Stripe doesn't document
        // as a typed shape — for those the caller threads `()` through
        // `serde_json::from_slice::<serde_json::Value>` upstream. For
        // all current paths the typed `T` is either a typed struct with
        // exactly the fields we read or `serde_json::Value`.
        serde_json::from_slice(&bytes).map_err(|e| {
            // Trim Stripe response to keep the error reasonable. We log
            // the full body at trace level for operator debugging.
            tracing::debug!(
                stripe_body = %String::from_utf8_lossy(&bytes),
                "Stripe response decode failed"
            );
            AppError::BadRequest(format!("Stripe: response decode failed: {e}"))
        })
    }
}

/// Stripe surfaces typed error envelopes (`{"error": {"message": …, "code": …}}`).
/// We unwrap that so the operator-facing message in `AppError::BadRequest`
/// matches the previous async-stripe formatting.
fn map_stripe_error(status: StatusCode, body: &[u8]) -> AppError {
    #[derive(Deserialize)]
    struct Envelope {
        error: ErrorBody,
    }
    #[derive(Deserialize)]
    struct ErrorBody {
        message: Option<String>,
        code: Option<String>,
        #[serde(rename = "type")]
        kind: Option<String>,
    }
    if let Ok(env) = serde_json::from_slice::<Envelope>(body) {
        let kind = env.error.kind.unwrap_or_else(|| "api_error".into());
        let code = env
            .error
            .code
            .map(|c| format!(" ({c})"))
            .unwrap_or_default();
        let message = env
            .error
            .message
            .unwrap_or_else(|| "no message".to_string());
        return AppError::BadRequest(format!("Stripe: {kind}{code}: {message}"));
    }
    AppError::BadRequest(format!("Stripe: HTTP {status} ({} bytes)", body.len()))
}

// ─── Public client builder ─────────────────────────────────────────────

/// Build a Stripe HTTP client from `AppState`. Mirrors the shape of the
/// previous helper so callers don't have to change.
///
/// When [`Config::stripe_api_base_url_override`] is set (test-only path —
/// hydrated from `STRIPE_API_BASE_URL` only when non-empty) the client is
/// pointed at that base URL. Production never sets the env var, so the
/// hardcoded [`STRIPE_API_BASE`] is used and the request shape on the
/// wire is unchanged.
pub fn client(state: &AppState) -> AppResult<Client> {
    if state.config.stripe_secret_key.is_empty() {
        return Err(AppError::BadRequest(
            "Stripe is not configured (missing STRIPE_SECRET_KEY)".to_string(),
        ));
    }
    match state.config.stripe_api_base_url() {
        Some(base) if !base.is_empty() => {
            Client::with_base_url(&state.config.stripe_secret_key, base)
        }
        _ => Client::new(&state.config.stripe_secret_key),
    }
}

// ─── Typed responses we actually deserialise ───────────────────────────

#[derive(Debug, Deserialize)]
struct BillingPortalSession {
    url: String,
}

#[derive(Debug, Deserialize)]
struct PaymentIntentResponse {
    id: String,
    client_secret: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct SubscriptionItem {
    pub id: String,
}

#[derive(Debug, Deserialize)]
pub struct SubscriptionItemList {
    pub data: Vec<SubscriptionItem>,
}

/// Subscription view used by the pricing-rollout workflow. Only the
/// fields the rollout actually reads are deserialised — Stripe surfaces
/// dozens of others; ignoring them via serde's default `deny_unknown_fields=false`
/// is intentional.
#[derive(Debug, Deserialize)]
pub struct Subscription {
    pub items: SubscriptionItemList,
}

// ─── ID validation helpers ────────────────────────────────────────────

/// Validate a Stripe-issued opaque id. We don't enforce a specific prefix
/// (Stripe ids are versioned: `cus_`, `sub_`, `coupon_`, etc.) but we do
/// reject obviously-bad values that would either break the URL or
/// indicate a misuse from the caller.
fn validate_id(value: &str, label: &str) -> AppResult<()> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(AppError::BadRequest(format!("Invalid Stripe {label} id")));
    }
    // Stripe ids are ASCII (`[A-Za-z0-9_]+`); reject anything that would
    // require escaping in a URL path segment.
    if !trimmed
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_')
    {
        return Err(AppError::BadRequest(format!("Invalid Stripe {label} id")));
    }
    Ok(())
}

/// Validate an ISO-4217 currency code. Stripe expects lowercase three-
/// letter ASCII; our callers pass mixed case.
fn validate_currency(currency: &str) -> AppResult<String> {
    let lc = currency.trim().to_ascii_lowercase();
    if lc.len() != 3 || !lc.chars().all(|c| c.is_ascii_alphabetic()) {
        return Err(AppError::BadRequest(format!(
            "invalid currency `{currency}`"
        )));
    }
    Ok(lc)
}

// ─── Public endpoints (signatures preserved) ───────────────────────────

pub async fn create_billing_portal_session(
    state: &AppState,
    stripe_customer_id: &str,
    return_url: &str,
) -> AppResult<String> {
    validate_id(stripe_customer_id, "customer")?;
    let c = client(state)?;
    let mut form = FormBody::new();
    form.push("customer", stripe_customer_id)
        .push("return_url", return_url);

    let session: BillingPortalSession = c
        .request(
            Method::POST,
            "/v1/billing_portal/sessions",
            Some(form.into_body()),
            None,
        )
        .await?;
    Ok(session.url)
}

/// FORM-08: Mint a one-shot Stripe PaymentIntent for a form-driven
/// payment. Caller supplies the amount in minor units + currency code +
/// the canonical metadata id (`form:{form_id}` or `submission:{id}`)
/// the webhook reconciler keys off.
///
/// `idempotency_key` is now correctly forwarded as the
/// `Idempotency-Key` HTTP header (the previous implementation accepted
/// the parameter but silently dropped it — see commit history).
pub async fn create_form_payment_intent(
    state: &AppState,
    amount_cents: i64,
    currency: &str,
    receipt_email: &str,
    metadata_id: &str,
    idempotency_key: Option<&str>,
) -> AppResult<(String, String)> {
    let cur = validate_currency(currency)?;
    if amount_cents <= 0 {
        return Err(AppError::BadRequest(
            "amount_cents must be positive".to_string(),
        ));
    }
    let c = client(state)?;
    let mut form = FormBody::new();
    form.push("amount", amount_cents.to_string())
        .push("currency", &cur)
        .push("receipt_email", receipt_email)
        .push_nested("automatic_payment_methods", "enabled", "true")
        .push_nested("metadata", "source", "form")
        .push_nested("metadata", "ref", metadata_id);

    let pi: PaymentIntentResponse = c
        .request(
            Method::POST,
            "/v1/payment_intents",
            Some(form.into_body()),
            idempotency_key,
        )
        .await?;
    let secret = pi.client_secret.ok_or_else(|| {
        AppError::BadRequest("Stripe returned PI without client_secret".to_string())
    })?;
    Ok((pi.id, secret))
}

/// ADM-15: cancel a subscription *immediately* (not at period end).
///
/// Used by the admin members surface when banning or hard-deleting a
/// member — leaving the subscription billable would let a banned account
/// continue to accrue charges. Stripe's `DELETE /subscriptions/{id}`
/// returns the subscription with `status='canceled'`; we don't surface
/// that body to the caller because the local mirror already gets
/// updated by the matching `customer.subscription.deleted` webhook.
pub async fn cancel_subscription_immediately(
    state: &AppState,
    stripe_subscription_id: &str,
) -> AppResult<()> {
    validate_id(stripe_subscription_id, "subscription")?;
    let c = client(state)?;
    let path = format!("/v1/subscriptions/{stripe_subscription_id}");
    let _ignored: serde_json::Value = c.request(Method::DELETE, &path, None, None).await?;
    Ok(())
}

/// ADM-15: write the customer's billing address back to the Stripe
/// Customer object. Mirrors what Stripe's hosted billing portal would do
/// when the customer edits their address themselves; called after the
/// admin members PATCH so the two stores stay in sync.
///
/// Every field is optional — Stripe treats missing keys as "no change".
/// If the caller passes only `phone` we still issue the POST so the
/// phone number lands; if every field is `None` we short-circuit and
/// avoid a no-op round trip.
#[allow(clippy::too_many_arguments)]
pub async fn update_customer_address(
    state: &AppState,
    stripe_customer_id: &str,
    line1: Option<&str>,
    line2: Option<&str>,
    city: Option<&str>,
    state_or_region: Option<&str>,
    postal_code: Option<&str>,
    country: Option<&str>,
    phone: Option<&str>,
) -> AppResult<()> {
    validate_id(stripe_customer_id, "customer")?;
    let c = client(state)?;
    let mut form = FormBody::new();

    let mut address_pairs: BTreeMap<&'static str, &str> = BTreeMap::new();
    if let Some(v) = line1 {
        address_pairs.insert("line1", v);
    }
    if let Some(v) = line2 {
        address_pairs.insert("line2", v);
    }
    if let Some(v) = city {
        address_pairs.insert("city", v);
    }
    if let Some(v) = state_or_region {
        address_pairs.insert("state", v);
    }
    if let Some(v) = postal_code {
        address_pairs.insert("postal_code", v);
    }
    if let Some(v) = country {
        address_pairs.insert("country", v);
    }
    for (k, v) in &address_pairs {
        form.push_nested("address", k, *v);
    }
    if let Some(p) = phone {
        form.push("phone", p);
    }

    if form.is_empty() {
        // Nothing to send — preserve the previous behaviour where an
        // empty mutation is a silent no-op rather than an API call.
        return Ok(());
    }

    let path = format!("/v1/customers/{stripe_customer_id}");
    let _ignored: serde_json::Value = c
        .request(Method::POST, &path, Some(form.into_body()), None)
        .await?;
    Ok(())
}

/// Phase 4.6: attach a Stripe coupon to an existing subscription so the
/// next invoice picks the discount up automatically.
///
/// `stripe_coupon_id` is the value stored on `coupons.stripe_coupon_id`
/// when an admin authored the coupon — locally-only coupons (no Stripe
/// twin) cannot be applied to a Stripe subscription, and the caller is
/// expected to short-circuit before reaching this helper.
pub async fn apply_coupon_to_subscription(
    state: &AppState,
    stripe_subscription_id: &str,
    stripe_coupon_id: &str,
) -> AppResult<()> {
    validate_id(stripe_subscription_id, "subscription")?;
    validate_id(stripe_coupon_id, "coupon")?;
    let c = client(state)?;
    let mut form = FormBody::new();
    form.push("coupon", stripe_coupon_id);
    let path = format!("/v1/subscriptions/{stripe_subscription_id}");
    let _ignored: serde_json::Value = c
        .request(Method::POST, &path, Some(form.into_body()), None)
        .await?;
    Ok(())
}

pub async fn set_subscription_cancel_at_period_end(
    state: &AppState,
    stripe_subscription_id: &str,
    cancel: bool,
) -> AppResult<()> {
    validate_id(stripe_subscription_id, "subscription")?;
    let c = client(state)?;
    let mut form = FormBody::new();
    form.push(
        "cancel_at_period_end",
        if cancel { "true" } else { "false" },
    );
    let path = format!("/v1/subscriptions/{stripe_subscription_id}");
    let _ignored: serde_json::Value = c
        .request(Method::POST, &path, Some(form.into_body()), None)
        .await?;
    Ok(())
}

// ─── Workflow helpers consumed by `services::pricing_rollout` ──────────

/// Retrieve a subscription with the `items.data` field expanded so the
/// rollout workflow can read the existing subscription_item id.
pub async fn retrieve_subscription(
    state: &AppState,
    stripe_subscription_id: &str,
) -> AppResult<Subscription> {
    validate_id(stripe_subscription_id, "subscription")?;
    let c = client(state)?;
    // `expand[]=items.data` is Stripe's standard query-string form.
    let path = format!(
        "/v1/subscriptions/{}?{}",
        stripe_subscription_id,
        urlencoding::encode("expand[]") + "=items.data"
    );
    let sub: Subscription = c.request(Method::GET, &path, None, None).await?;
    Ok(sub)
}

/// Description of the new pricing to push to a subscription's single
/// line item. Supports either a static `price_id` or an inline
/// `price_data` (currency / recurring interval / unit_amount).
pub struct PriceUpdate<'a> {
    pub item_id: &'a str,
    pub kind: PriceUpdateKind<'a>,
}

pub enum PriceUpdateKind<'a> {
    /// Reuse an existing `price_*` resource.
    StaticPrice { price_id: &'a str },
    /// Inline price_data — Stripe creates the price under the hood.
    Inline {
        currency: &'a str,
        product_id: &'a str,
        interval: SubscriptionInterval,
        interval_count: u64,
        unit_amount_cents: i64,
    },
}

/// Recurring interval enum mirrored from Stripe's vocabulary.
#[derive(Clone, Copy, Debug)]
pub enum SubscriptionInterval {
    Month,
    Year,
}

impl SubscriptionInterval {
    fn as_str(self) -> &'static str {
        match self {
            SubscriptionInterval::Month => "month",
            SubscriptionInterval::Year => "year",
        }
    }
}

/// Update a single-item subscription's price. Used by the catalog
/// pricing rollout workflow. `idempotency_key` is honoured — the
/// rollout job derives a unique key per `(plan, target)` pair so
/// retries are exact-once at the Stripe boundary.
pub async fn update_subscription_item_price(
    state: &AppState,
    stripe_subscription_id: &str,
    update: &PriceUpdate<'_>,
    idempotency_key: &str,
) -> AppResult<()> {
    validate_id(stripe_subscription_id, "subscription")?;
    validate_id(update.item_id, "subscription_item")?;
    let c = client(state)?;
    let mut form = FormBody::new();

    // `items[0][id]=…` selects which line item to mutate.
    form.push("items[0][id]", update.item_id);
    match &update.kind {
        PriceUpdateKind::StaticPrice { price_id } => {
            if price_id.trim().is_empty() {
                return Err(AppError::BadRequest(
                    "stripe_price_id is set but empty — clear it or paste a valid price_ id".into(),
                ));
            }
            form.push("items[0][price]", *price_id);
        }
        PriceUpdateKind::Inline {
            currency,
            product_id,
            interval,
            interval_count,
            unit_amount_cents,
        } => {
            if product_id.trim().is_empty() {
                return Err(AppError::BadRequest(
                    "stripe_product_id is required for dynamic-price rollout".into(),
                ));
            }
            let cur = validate_currency(currency)?;
            form.push("items[0][price_data][currency]", &cur);
            form.push("items[0][price_data][product]", *product_id);
            form.push(
                "items[0][price_data][unit_amount]",
                unit_amount_cents.to_string(),
            );
            form.push(
                "items[0][price_data][recurring][interval]",
                interval.as_str(),
            );
            form.push(
                "items[0][price_data][recurring][interval_count]",
                (*interval_count).max(1).to_string(),
            );
        }
    }

    let path = format!("/v1/subscriptions/{stripe_subscription_id}");
    let _ignored: serde_json::Value = c
        .request(
            Method::POST,
            &path,
            Some(form.into_body()),
            Some(idempotency_key),
        )
        .await?;
    Ok(())
}

// ─── Member-facing plan-switch helpers ─────────────────────────────────

/// Member-facing subscription line-item swap with caller-controlled proration.
///
/// Mirrors [`update_subscription_item_price`] (which the catalog rollout
/// workflow uses) but exposes Stripe's `proration_behavior` flag — the
/// member endpoints accept a `prorate` boolean from the SPA so a downgrade
/// "next period only" path doesn't surface a surprise immediate charge.
pub async fn swap_subscription_price_with_proration(
    state: &AppState,
    stripe_subscription_id: &str,
    item_id: &str,
    new_price_id: &str,
    prorate: bool,
    idempotency_key: Option<&str>,
) -> AppResult<()> {
    validate_id(stripe_subscription_id, "subscription")?;
    validate_id(item_id, "subscription_item")?;
    if new_price_id.trim().is_empty() {
        return Err(AppError::BadRequest(
            "stripe_price_id is required for plan switch".into(),
        ));
    }
    let c = client(state)?;
    let mut form = FormBody::new();
    form.push("items[0][id]", item_id);
    form.push("items[0][price]", new_price_id);
    form.push(
        "proration_behavior",
        if prorate { "create_prorations" } else { "none" },
    );

    let path = format!("/v1/subscriptions/{stripe_subscription_id}");
    let _ignored: serde_json::Value = c
        .request(Method::POST, &path, Some(form.into_body()), idempotency_key)
        .await?;
    Ok(())
}

/// Aggregated proration preview returned by [`preview_subscription_change`].
///
/// All values are in the smallest currency unit (cents) and follow Stripe's
/// invoice convention: positive `proration_charge_cents` means the member
/// owes that amount on the next invoice; positive `proration_credit_cents`
/// means the member is being credited.
#[derive(Debug, Clone)]
pub struct SubscriptionChangePreview {
    pub proration_credit_cents: i64,
    pub proration_charge_cents: i64,
    pub immediate_total_cents: i64,
    pub next_invoice_total_cents: i64,
    pub currency: String,
}

/// Stripe `/v1/invoices/upcoming` proration preview for a single-item
/// subscription line-item swap. Returns the cents that would be charged
/// or credited if the caller went on to call
/// [`swap_subscription_price_with_proration`] with `prorate = true`.
///
/// Lives behind [`crate::handlers::member`]'s switch-plan preview endpoint.
pub async fn preview_subscription_change(
    state: &AppState,
    stripe_subscription_id: &str,
    item_id: &str,
    new_price_id: &str,
) -> AppResult<SubscriptionChangePreview> {
    validate_id(stripe_subscription_id, "subscription")?;
    validate_id(item_id, "subscription_item")?;
    if new_price_id.trim().is_empty() {
        return Err(AppError::BadRequest(
            "stripe_price_id is required for plan switch preview".into(),
        ));
    }
    let c = client(state)?;
    // Stripe's `/v1/invoices/upcoming` is a GET that nevertheless takes
    // bracket-encoded subscription_items. We percent-encode each pair.
    let sub = urlencoding::encode(stripe_subscription_id);
    let item_key = urlencoding::encode("subscription_items[0][id]");
    let item_val = urlencoding::encode(item_id);
    let price_key = urlencoding::encode("subscription_items[0][price]");
    let price_val = urlencoding::encode(new_price_id);
    let behavior_key = urlencoding::encode("subscription_proration_behavior");
    let behavior_val = urlencoding::encode("create_prorations");
    let path = format!(
        "/v1/invoices/upcoming?subscription={sub}\
         &{item_key}={item_val}\
         &{price_key}={price_val}\
         &{behavior_key}={behavior_val}",
    );

    #[derive(Deserialize)]
    struct InvoiceLineItem {
        amount: Option<i64>,
        proration: Option<bool>,
    }
    #[derive(Deserialize)]
    struct InvoiceLines {
        #[serde(default)]
        data: Vec<InvoiceLineItem>,
    }
    #[derive(Deserialize)]
    struct UpcomingInvoice {
        amount_due: Option<i64>,
        total: Option<i64>,
        currency: Option<String>,
        #[serde(default)]
        lines: Option<InvoiceLines>,
    }

    let invoice: UpcomingInvoice = c.request(Method::GET, &path, None, None).await?;
    let mut credit_cents: i64 = 0;
    let mut charge_cents: i64 = 0;
    if let Some(lines) = &invoice.lines {
        for line in &lines.data {
            if line.proration.unwrap_or(false) {
                let amount = line.amount.unwrap_or(0);
                if amount < 0 {
                    credit_cents = credit_cents.saturating_sub(amount); // amount is negative → adds positive
                } else {
                    charge_cents = charge_cents.saturating_add(amount);
                }
            }
        }
    }
    let immediate_total_cents = invoice.amount_due.unwrap_or(0);
    let next_invoice_total_cents = invoice.total.unwrap_or(immediate_total_cents);
    let currency = invoice.currency.unwrap_or_else(|| "usd".to_string());
    Ok(SubscriptionChangePreview {
        proration_credit_cents: credit_cents,
        proration_charge_cents: charge_cents,
        immediate_total_cents,
        next_invoice_total_cents,
        currency,
    })
}

// ─── Member-facing payment-method wrappers ─────────────────────────────

/// Compact view of a Stripe `PaymentMethod` (card type only) returned to
/// the SPA. The wire shape is intentionally minimal — we never expose
/// raw Stripe identifiers beyond the `pm_*` id, and we never round-trip
/// the underlying card number, expiry CVC, or fingerprint. The
/// publishable key + Stripe Elements iframe own the PCI scope.
#[derive(Debug, Clone, serde::Serialize, Deserialize, utoipa::ToSchema)]
pub struct PaymentMethodSummary {
    pub id: String,
    /// Card brand (`visa`, `mastercard`, `amex`, `discover`, etc.). Always
    /// lowercased per Stripe's contract.
    pub brand: String,
    pub last4: String,
    pub exp_month: i64,
    pub exp_year: i64,
    /// Set true when this `id` matches the customer's
    /// `invoice_settings.default_payment_method`. Computed at the API edge
    /// because it requires a separate `GET /v1/customers/{id}` round trip.
    pub is_default: bool,
}

/// Internal: a Stripe payment-method row deserialised from
/// `GET /v1/customers/{id}/payment_methods` or
/// `GET /v1/payment_methods/{id}`. Carries the customer link so the
/// member handler can enforce ownership before mutating.
#[derive(Debug, Clone, Deserialize)]
pub struct PaymentMethodFull {
    pub id: String,
    pub card: PaymentMethodCard,
    /// `customer` is `null` on detached payment methods. We surface the
    /// `Option` so the handler can 404 a probe rather than a 500.
    pub customer: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PaymentMethodCard {
    pub brand: String,
    pub last4: String,
    pub exp_month: i64,
    pub exp_year: i64,
}

impl From<&PaymentMethodFull> for PaymentMethodSummary {
    fn from(pm: &PaymentMethodFull) -> Self {
        Self {
            id: pm.id.clone(),
            brand: pm.card.brand.clone(),
            last4: pm.card.last4.clone(),
            exp_month: pm.card.exp_month,
            exp_year: pm.card.exp_year,
            is_default: false,
        }
    }
}

#[derive(Debug, Deserialize)]
struct PaymentMethodListResponse {
    data: Vec<PaymentMethodFull>,
}

#[derive(Debug, Deserialize)]
struct CustomerInvoiceSettingsView {
    #[serde(default)]
    invoice_settings: Option<CustomerInvoiceSettings>,
}

#[derive(Debug, Deserialize)]
struct CustomerInvoiceSettings {
    /// Stripe returns the id as a string when expanded=false (our default
    /// — we don't ask for expansion). May also be null when the customer
    /// has never had a default set.
    #[serde(default)]
    default_payment_method: Option<String>,
}

#[derive(Debug, Deserialize)]
struct SetupIntentResponse {
    client_secret: Option<String>,
}

/// List every card-type payment method attached to the given Stripe
/// customer. The `is_default` field on each row is left `false` —
/// callers should pair this with [`get_customer_default_payment_method`]
/// and stamp the matching id themselves.
pub async fn list_customer_payment_methods(
    state: &AppState,
    customer_id: &str,
) -> AppResult<Vec<PaymentMethodSummary>> {
    validate_id(customer_id, "customer")?;
    let c = client(state)?;
    let path = format!("/v1/customers/{customer_id}/payment_methods?type=card");
    let resp: PaymentMethodListResponse = c.request(Method::GET, &path, None, None).await?;
    Ok(resp.data.iter().map(PaymentMethodSummary::from).collect())
}

/// Read the customer's `invoice_settings.default_payment_method`. Returns
/// `None` when the customer exists but has no default on file — the
/// caller distinguishes this from the 404 path because Stripe returns a
/// 200 with the field nulled out.
pub async fn get_customer_default_payment_method(
    state: &AppState,
    customer_id: &str,
) -> AppResult<Option<String>> {
    validate_id(customer_id, "customer")?;
    let c = client(state)?;
    let path = format!("/v1/customers/{customer_id}");
    let cust: CustomerInvoiceSettingsView = c.request(Method::GET, &path, None, None).await?;
    Ok(cust.invoice_settings.and_then(|s| s.default_payment_method))
}

/// Mint a Stripe SetupIntent so the SPA can collect a new card via
/// Stripe Elements. `usage=off_session` is required for Stripe to attach
/// the resulting payment method to the customer for future charges
/// (subscription renewals, off-session top-ups, etc.).
///
/// `idempotency_key` is forwarded as the `Idempotency-Key` header so a
/// retried POST reuses the same SetupIntent rather than minting a second
/// one — important because every SetupIntent counts against Stripe's
/// rate-limit budget.
pub async fn create_setup_intent(
    state: &AppState,
    customer_id: &str,
    idempotency_key: Option<&str>,
) -> AppResult<String> {
    validate_id(customer_id, "customer")?;
    let c = client(state)?;
    let mut form = FormBody::new();
    form.push("customer", customer_id)
        .push("usage", "off_session")
        .push("payment_method_types[0]", "card");
    let resp: SetupIntentResponse = c
        .request(
            Method::POST,
            "/v1/setup_intents",
            Some(form.into_body()),
            idempotency_key,
        )
        .await?;
    resp.client_secret.ok_or_else(|| {
        AppError::BadRequest("Stripe returned SetupIntent without client_secret".to_string())
    })
}

/// Pin `pm_id` as the customer's default payment method. Stripe stores
/// this on `invoice_settings.default_payment_method`; future invoices
/// will charge that method automatically.
pub async fn set_default_payment_method(
    state: &AppState,
    customer_id: &str,
    pm_id: &str,
) -> AppResult<()> {
    validate_id(customer_id, "customer")?;
    validate_id(pm_id, "payment_method")?;
    let c = client(state)?;
    let mut form = FormBody::new();
    form.push("invoice_settings[default_payment_method]", pm_id);
    let path = format!("/v1/customers/{customer_id}");
    let _ignored: serde_json::Value = c
        .request(Method::POST, &path, Some(form.into_body()), None)
        .await?;
    Ok(())
}

/// Detach a payment method from its customer. After this call the `pm_*`
/// id is no longer chargeable — Stripe will reject any future
/// PaymentIntent that references it without re-attachment.
pub async fn detach_payment_method(state: &AppState, pm_id: &str) -> AppResult<()> {
    validate_id(pm_id, "payment_method")?;
    let c = client(state)?;
    let path = format!("/v1/payment_methods/{pm_id}/detach");
    // Detach is a POST with no body — Stripe accepts an empty
    // form-encoded body, but reqwest needs `Some("")` to set the
    // content-type header so Stripe doesn't reject the missing CT.
    let _ignored: serde_json::Value = c
        .request(Method::POST, &path, Some(String::new()), None)
        .await?;
    Ok(())
}

/// Fetch a single payment method by id. Used by the member handlers to
/// validate ownership (the returned `customer` field must match the
/// caller's stripe_customer_id) before either setting it as default or
/// detaching it.
pub async fn get_payment_method(state: &AppState, pm_id: &str) -> AppResult<PaymentMethodFull> {
    validate_id(pm_id, "payment_method")?;
    let c = client(state)?;
    let path = format!("/v1/payment_methods/{pm_id}");
    let pm: PaymentMethodFull = c.request(Method::GET, &path, None, None).await?;
    Ok(pm)
}

// ─── Tests ─────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    //! Pure-helper coverage. The HTTP edge is covered by integration
    //! tests under `backend/tests/stripe_api_http.rs` (wiremock-driven).

    use super::*;

    #[test]
    fn form_body_flat_pair() {
        let mut f = FormBody::new();
        f.push("a", "1").push("b", "two");
        assert_eq!(f.into_body(), "a=1&b=two");
    }

    #[test]
    fn form_body_nested_pair() {
        let mut f = FormBody::new();
        f.push_nested("metadata", "ref", "form:abc");
        // brackets must be percent-encoded in keys — `[` => `%5B`, `]` => `%5D`
        assert_eq!(f.into_body(), "metadata%5Bref%5D=form%3Aabc");
    }

    #[test]
    fn form_body_percent_encodes_values() {
        let mut f = FormBody::new();
        f.push("name", "John Smith & Co");
        let body = f.into_body();
        assert!(body.contains("John%20Smith%20%26%20Co"), "body: {body}");
    }

    #[test]
    fn validate_id_rejects_empty() {
        assert!(validate_id("", "customer").is_err());
        assert!(validate_id("   ", "customer").is_err());
    }

    #[test]
    fn validate_id_rejects_url_metacharacters() {
        assert!(validate_id("cus_123/extra", "customer").is_err());
        assert!(validate_id("cus 123", "customer").is_err());
        assert!(validate_id("cus_123?q=1", "customer").is_err());
    }

    #[test]
    fn validate_id_accepts_typical_stripe_id() {
        assert!(validate_id("cus_NffrFeUfNV2Hib", "customer").is_ok());
        assert!(validate_id("sub_1Mz1AaLkdIwHu7ix2cIvDelB", "subscription").is_ok());
    }

    #[test]
    fn validate_currency_normalises_case() {
        assert_eq!(validate_currency("USD").unwrap(), "usd");
        assert_eq!(validate_currency(" eur ").unwrap(), "eur");
    }

    #[test]
    fn validate_currency_rejects_garbage() {
        assert!(validate_currency("ZZZZ").is_err());
        assert!(validate_currency("us").is_err());
        assert!(validate_currency("12$").is_err());
    }

    #[test]
    fn map_stripe_error_unwraps_envelope() {
        let body = br#"{"error":{"type":"invalid_request_error","code":"resource_missing","message":"No such customer: cus_x"}}"#;
        let e = map_stripe_error(StatusCode::NOT_FOUND, body);
        let msg = e.to_string();
        assert!(msg.contains("invalid_request_error"), "msg: {msg}");
        assert!(msg.contains("resource_missing"), "msg: {msg}");
        assert!(msg.contains("No such customer"), "msg: {msg}");
    }

    #[test]
    fn map_stripe_error_falls_back_on_garbage_body() {
        let e = map_stripe_error(StatusCode::INTERNAL_SERVER_ERROR, b"<html>");
        let msg = e.to_string();
        assert!(msg.contains("HTTP 500"), "msg: {msg}");
    }

    #[test]
    fn subscription_interval_serialises() {
        assert_eq!(SubscriptionInterval::Month.as_str(), "month");
        assert_eq!(SubscriptionInterval::Year.as_str(), "year");
    }
}
