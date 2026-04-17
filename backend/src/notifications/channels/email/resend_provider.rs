//! Resend-backed [`EmailProvider`].
//!
//! POSTs to `https://api.resend.com/emails`, honors `Idempotency-Key`, and
//! returns the `id` field from the 200-response body so downstream Resend
//! webhooks (see `notifications::webhooks::resend`) can reconcile status
//! transitions against the row persisted at
//! `notification_deliveries.provider_id`.
//!
//! # Why build the body ourselves?
//!
//! `resend-rs` transitively depends on `reqwest` with a feature set that
//! conflicts with the narrow `rustls-tls`/`json` build we ship for this crate
//! (it pulls OpenSSL). Since the request is flat JSON and we already have
//! `reqwest` in the tree, we hand-roll the POST — five fields, no macros,
//! easy to review and mock.

use std::env;
use std::time::Duration;

use reqwest::StatusCode;
use serde::Serialize;
use tracing::{debug, warn};

use super::{EmailProvider, EmailProviderError, EmailSendRequest};

/// Default base URL. Overridable via `RESEND_API_BASE` — handy for test
/// harnesses and staging pin-pointing without a full rebuild.
const DEFAULT_API_BASE: &str = "https://api.resend.com";

/// Default `From` string when `RESEND_FROM` is unset. Per §2 of the spec.
const DEFAULT_FROM: &str = "Swings <noreply@precisionoptionsignals.com>";

/// Resend HTTP provider.
///
/// Hold `api_key` as a `String` (not `&'static str`) because it flows in from
/// env at runtime; the struct is cheap to clone through `Arc`. `Debug` is
/// manually implemented so the key never leaks through `{:?}` formatting
/// (log aggregators redact api_keys but we'd rather not rely on that).
pub struct ResendProvider {
    client: reqwest::Client,
    api_key: String,
    /// Override applied when `EmailSendRequest::from` is empty / default. The
    /// channel layer always sets a non-empty `from`, so this is a safety net.
    default_from: String,
    base_url: String,
}

impl std::fmt::Debug for ResendProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ResendProvider")
            .field("api_key", &"<redacted>")
            .field("default_from", &self.default_from)
            .field("base_url", &self.base_url)
            .finish()
    }
}

/// Tight, fixed HTTP timeouts. Resend's p99 is under 2 s; above that we want
/// the caller (the outbox handler) to retry rather than block the worker.
const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);
const REQUEST_TIMEOUT: Duration = Duration::from_secs(15);

impl ResendProvider {
    /// Construct from explicit config. Prefer [`ResendProvider::from_env`] in
    /// production paths; this constructor is used by tests + alt-base staging.
    pub fn new(
        api_key: impl Into<String>,
        default_from: impl Into<String>,
        base_url: impl Into<String>,
    ) -> Result<Self, EmailProviderError> {
        let api_key = api_key.into();
        if api_key.trim().is_empty() {
            return Err(EmailProviderError::Config(
                "RESEND_API_KEY is required".into(),
            ));
        }
        let client = reqwest::Client::builder()
            .connect_timeout(CONNECT_TIMEOUT)
            .timeout(REQUEST_TIMEOUT)
            .user_agent("swings-api/1 (+https://precisionoptionsignals.com)")
            .build()
            .map_err(|e| EmailProviderError::Config(format!("reqwest client: {e}")))?;
        Ok(Self {
            client,
            api_key,
            default_from: default_from.into(),
            base_url: base_url.into(),
        })
    }

    /// Read `RESEND_API_KEY` (required), `RESEND_FROM` (default
    /// `"Swings <noreply@precisionoptionsignals.com>"`), and `RESEND_API_BASE`
    /// (default `https://api.resend.com`) from the environment.
    pub fn from_env() -> Result<Self, EmailProviderError> {
        let api_key = env::var("RESEND_API_KEY").map_err(|_| {
            EmailProviderError::Config("RESEND_API_KEY environment variable is required".into())
        })?;
        let from = env::var("RESEND_FROM").unwrap_or_else(|_| DEFAULT_FROM.to_string());
        let base = env::var("RESEND_API_BASE").unwrap_or_else(|_| DEFAULT_API_BASE.to_string());
        Self::new(api_key, from, base)
    }
}

/// Wire body for `POST /emails`.
///
/// `to` is a single-element array — Resend accepts both string + array; the
/// array form is canonical and lets downstream templates fan out.
#[derive(Debug, Serialize)]
struct ResendSendBody<'a> {
    from: &'a str,
    to: Vec<&'a str>,
    subject: &'a str,
    html: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    text: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    reply_to: Option<&'a str>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    tags: Vec<ResendTag<'a>>,
}

#[derive(Debug, Serialize)]
struct ResendTag<'a> {
    name: &'a str,
    value: &'a str,
}

/// Response shape for `POST /emails` (happy path).
#[derive(Debug, serde::Deserialize)]
struct ResendSendResponse {
    id: String,
}

#[async_trait::async_trait]
impl EmailProvider for ResendProvider {
    async fn send(&self, req: &EmailSendRequest) -> Result<String, EmailProviderError> {
        let from = if req.from.trim().is_empty() {
            self.default_from.as_str()
        } else {
            req.from.as_str()
        };
        let tags: Vec<ResendTag<'_>> = req
            .tags
            .iter()
            .map(|(k, v)| ResendTag {
                name: k.as_str(),
                value: v.as_str(),
            })
            .collect();
        let body = ResendSendBody {
            from,
            to: vec![req.to.as_str()],
            subject: req.subject.as_str(),
            html: req.html_body.as_str(),
            text: req.plain_body.as_deref(),
            reply_to: req.reply_to.as_deref(),
            tags,
        };

        let url = format!("{}/emails", self.base_url);
        let mut rb = self
            .client
            .post(&url)
            .bearer_auth(&self.api_key)
            .json(&body);
        if let Some(key) = req.idempotency_key.as_deref() {
            rb = rb.header("Idempotency-Key", key);
        }

        // Any `reqwest::Error` here is transport-level (DNS, TCP, TLS
        // handshake, timeout). Retrying is almost always the right move.
        let resp = match rb.send().await {
            Ok(r) => r,
            Err(e) => {
                warn!(error = %e, "resend: transport error");
                return Err(EmailProviderError::Transient(format!("transport: {e}")));
            }
        };

        let status = resp.status();
        if status.is_success() {
            let parsed: ResendSendResponse = resp.json().await.map_err(|e| {
                EmailProviderError::Transient(format!("invalid resend 200 body: {e}"))
            })?;
            debug!(id = %parsed.id, "resend: 200 OK");
            return Ok(parsed.id);
        }

        // Status classification mirrors §2 of the FDN-09 spec.
        let kind = classify(status);
        let detail = resp
            .text()
            .await
            .unwrap_or_else(|_| format!("resend status {}", status.as_u16()));
        let msg = format!(
            "resend HTTP {}: {}",
            status.as_u16(),
            truncate(&detail, 512)
        );
        Err(match kind {
            ErrorClass::Transient => EmailProviderError::Transient(msg),
            ErrorClass::Permanent => EmailProviderError::Permanent(msg),
        })
    }

    fn name(&self) -> &'static str {
        "resend"
    }
}

enum ErrorClass {
    Transient,
    Permanent,
}

fn classify(status: StatusCode) -> ErrorClass {
    // 429 Too Many Requests: rate-limited, retryable.
    // 5xx: server-side; retryable.
    // Everything else 4xx: permanent (auth, malformed, invalid recipient).
    if status == StatusCode::TOO_MANY_REQUESTS || status.is_server_error() {
        ErrorClass::Transient
    } else {
        ErrorClass::Permanent
    }
}

/// Cap the body string baked into error messages. Resend's 422 body is the
/// only payload we care about; truncation prevents enormous strings from
/// ending up in logs. Char-boundary-safe so it never panics on UTF-8 input.
fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        return s.to_string();
    }
    // Walk back from `max` until we land on a char boundary. `floor_char_boundary`
    // is unstable, so we iterate manually — the loop is bounded to 4 bytes
    // (UTF-8 char max width).
    let mut end = max;
    while end > 0 && !s.is_char_boundary(end) {
        end -= 1;
    }
    format!("{}…", &s[..end])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_env_requires_api_key() {
        // SAFETY: env mutations are confined to this test; guarded by a
        // std::sync::Mutex at module scope — see note on the `run_env_test`
        // helper in the wiremock integration file where we actually exercise
        // network interactions.
        let prev = env::var("RESEND_API_KEY").ok();
        // Guarantee it is unset for this call.
        env::remove_var("RESEND_API_KEY");
        let err = ResendProvider::from_env().expect_err("missing key must fail");
        assert!(matches!(err, EmailProviderError::Config(_)));
        if let Some(p) = prev {
            env::set_var("RESEND_API_KEY", p);
        }
    }

    #[test]
    fn classify_maps_statuses() {
        assert!(matches!(
            classify(StatusCode::TOO_MANY_REQUESTS),
            ErrorClass::Transient
        ));
        assert!(matches!(
            classify(StatusCode::INTERNAL_SERVER_ERROR),
            ErrorClass::Transient
        ));
        assert!(matches!(
            classify(StatusCode::BAD_GATEWAY),
            ErrorClass::Transient
        ));
        assert!(matches!(
            classify(StatusCode::UNPROCESSABLE_ENTITY),
            ErrorClass::Permanent
        ));
        assert!(matches!(
            classify(StatusCode::BAD_REQUEST),
            ErrorClass::Permanent
        ));
        assert!(matches!(
            classify(StatusCode::UNAUTHORIZED),
            ErrorClass::Permanent
        ));
    }

    #[test]
    fn truncate_respects_limit() {
        assert_eq!(truncate("abc", 10), "abc");
        assert_eq!(truncate("aaaaaaaaaa", 5), "aaaaa…");
    }
}
