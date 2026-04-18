//! FORM-06: anti-spam pipeline.
//!
//! The pipeline is a vector of [`SpamCheck`] impls invoked in order on
//! every `public_submit` call. The first verdict that returns
//! [`SpamVerdict::Rejected`] short-circuits the rest; the submission
//! handler maps a rejection to HTTP 422 with a stable error code.
//!
//! Shipped checks:
//!
//!   * [`Honeypot`] — rejects if the renderer's hidden `form_hp` field
//!     contains anything; bots that auto-fill every input trip on this.
//!   * [`Turnstile`] — verifies the Cloudflare Turnstile token via
//!     `siteverify`; gracefully no-ops when `TURNSTILE_SECRET` is unset.
//!   * [`Akismet`] — submits free-text fields to Akismet; gracefully
//!     no-ops when `AKISMET_API_KEY` / `AKISMET_SITE_URL` are unset.
//!   * [`Dedup`] — in-process TTL cache keyed by
//!     `hash(form_id + normalised_email + payload_sha256)`; rejects
//!     replays inside a 60-second window so a double-clicked submit
//!     button doesn't double-fire downstream integrations.

use std::{
    collections::HashMap,
    sync::Mutex,
    time::{Duration, Instant},
};

use async_trait::async_trait;
use sha2::{Digest, Sha256};
use uuid::Uuid;

/// Outcome of a single check. `Rejected` carries a stable machine code
/// that the handler logs + returns; never expose the raw bot-trip detail
/// to the public response (false positives leak heuristics).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SpamVerdict {
    Clean,
    Rejected(&'static str),
}

#[async_trait]
pub trait SpamCheck: Send + Sync {
    async fn evaluate(&self, ctx: &SubmissionContext<'_>) -> SpamVerdict;
}

/// Slim view of the submission used by every check. Borrowed so the
/// pipeline doesn't pay for re-serialising the data JSON per check.
pub struct SubmissionContext<'a> {
    pub form_id: Uuid,
    pub data: &'a serde_json::Value,
    pub ip_hash: &'a str,
    pub turnstile_token: Option<&'a str>,
}

impl<'a> SubmissionContext<'a> {
    /// Concatenate all string-typed values across the data payload —
    /// used by Akismet to score the submission as a whole.
    pub fn free_text(&self) -> String {
        let mut out = String::new();
        collect_strings(self.data, &mut out);
        out
    }

    /// Best-effort email extraction for dedupe. Looks at common keys:
    /// `email`, `Email`, `EMAIL`, `e_mail`, then any string value
    /// containing `@`.
    pub fn email(&self) -> Option<String> {
        if let serde_json::Value::Object(m) = self.data {
            for k in ["email", "Email", "EMAIL", "e_mail"] {
                if let Some(serde_json::Value::String(v)) = m.get(k) {
                    return Some(normalise_email(v));
                }
            }
            for v in m.values() {
                if let serde_json::Value::String(s) = v {
                    if s.contains('@') {
                        return Some(normalise_email(s));
                    }
                }
            }
        }
        None
    }
}

fn collect_strings(v: &serde_json::Value, out: &mut String) {
    match v {
        serde_json::Value::String(s) => {
            out.push_str(s);
            out.push('\n');
        }
        serde_json::Value::Array(arr) => arr.iter().for_each(|x| collect_strings(x, out)),
        serde_json::Value::Object(m) => m.values().for_each(|x| collect_strings(x, out)),
        _ => {}
    }
}

fn normalise_email(s: &str) -> String {
    s.trim().to_ascii_lowercase()
}

/// Drop the email field from the payload object before hashing, so the
/// case-normalised email contributes to the dedupe key only via the
/// extracted [`SubmissionContext::email`].
fn scrub_email(v: &serde_json::Value) -> serde_json::Value {
    if let serde_json::Value::Object(m) = v {
        let mut clone = m.clone();
        for k in ["email", "Email", "EMAIL", "e_mail"] {
            clone.remove(k);
        }
        serde_json::Value::Object(clone)
    } else {
        v.clone()
    }
}

// ── Honeypot ───────────────────────────────────────────────────────────

/// FORM-10 always renders a hidden `form_hp` input. Real users don't
/// fill it; bots do.
pub struct Honeypot;

#[async_trait]
impl SpamCheck for Honeypot {
    async fn evaluate(&self, ctx: &SubmissionContext<'_>) -> SpamVerdict {
        if let serde_json::Value::Object(m) = ctx.data {
            if let Some(v) = m.get("form_hp") {
                let filled = match v {
                    serde_json::Value::String(s) => !s.trim().is_empty(),
                    serde_json::Value::Null => false,
                    other => !other.is_null(),
                };
                if filled {
                    return SpamVerdict::Rejected("honeypot_tripped");
                }
            }
        }
        SpamVerdict::Clean
    }
}

// ── Turnstile ──────────────────────────────────────────────────────────

/// Cloudflare Turnstile verifier. Construct with
/// [`Turnstile::from_env`]; if the env var is missing the check returns
/// `Clean` (configuration error, not a spam signal — log a warn at
/// startup so ops notices the gap).
pub struct Turnstile {
    secret: String,
    http: reqwest::Client,
    /// Override for tests — production passes `None` and the verifier
    /// hits the canonical Cloudflare URL.
    endpoint: Option<String>,
}

impl Turnstile {
    pub fn from_env(http: reqwest::Client) -> Option<Self> {
        let secret = std::env::var("TURNSTILE_SECRET").ok()?;
        Some(Self {
            secret,
            http,
            endpoint: None,
        })
    }

    /// Test seam — point at a `wiremock` mock instead of Cloudflare.
    #[cfg(test)]
    pub fn with_endpoint(secret: String, http: reqwest::Client, endpoint: String) -> Self {
        Self {
            secret,
            http,
            endpoint: Some(endpoint),
        }
    }

    fn url(&self) -> &str {
        self.endpoint
            .as_deref()
            .unwrap_or("https://challenges.cloudflare.com/turnstile/v0/siteverify")
    }
}

#[async_trait]
impl SpamCheck for Turnstile {
    async fn evaluate(&self, ctx: &SubmissionContext<'_>) -> SpamVerdict {
        let Some(token) = ctx.turnstile_token else {
            return SpamVerdict::Rejected("turnstile_missing");
        };
        let body = [("secret", self.secret.as_str()), ("response", token)];
        let res = self.http.post(self.url()).form(&body).send().await;
        match res {
            Ok(r) => match r.json::<TurnstileResponse>().await {
                Ok(j) if j.success => SpamVerdict::Clean,
                _ => SpamVerdict::Rejected("turnstile_failed"),
            },
            Err(_) => SpamVerdict::Rejected("turnstile_unreachable"),
        }
    }
}

#[derive(serde::Deserialize)]
struct TurnstileResponse {
    success: bool,
}

// ── Akismet ────────────────────────────────────────────────────────────

pub struct Akismet {
    api_key: String,
    site_url: String,
    http: reqwest::Client,
    endpoint: Option<String>,
}

impl Akismet {
    pub fn from_env(http: reqwest::Client) -> Option<Self> {
        let api_key = std::env::var("AKISMET_API_KEY").ok()?;
        let site_url = std::env::var("AKISMET_SITE_URL").ok()?;
        Some(Self {
            api_key,
            site_url,
            http,
            endpoint: None,
        })
    }

    #[cfg(test)]
    pub fn with_endpoint(
        api_key: String,
        site_url: String,
        http: reqwest::Client,
        endpoint: String,
    ) -> Self {
        Self {
            api_key,
            site_url,
            http,
            endpoint: Some(endpoint),
        }
    }

    fn url(&self) -> String {
        self.endpoint.clone().unwrap_or_else(|| {
            format!(
                "https://{}.rest.akismet.com/1.1/comment-check",
                self.api_key
            )
        })
    }
}

#[async_trait]
impl SpamCheck for Akismet {
    async fn evaluate(&self, ctx: &SubmissionContext<'_>) -> SpamVerdict {
        let body = [
            ("blog", self.site_url.as_str()),
            ("user_ip", ctx.ip_hash),
            ("comment_type", "contact-form"),
            ("comment_content", &ctx.free_text()),
        ];
        let res = self.http.post(self.url()).form(&body).send().await;
        let Ok(r) = res else {
            return SpamVerdict::Rejected("akismet_unreachable");
        };
        match r.text().await.as_deref() {
            Ok("true") => SpamVerdict::Rejected("akismet_spam"),
            Ok("false") => SpamVerdict::Clean,
            _ => SpamVerdict::Rejected("akismet_invalid"),
        }
    }
}

// ── Dedup ──────────────────────────────────────────────────────────────

/// Exponential-decay-free TTL map keyed by the dedupe hash. A real
/// concurrent cache (`moka`) is overkill for a 60-second horizon — the
/// `Mutex<HashMap>` evicts lazily on every insert.
pub struct Dedup {
    inner: Mutex<HashMap<[u8; 32], Instant>>,
    window: Duration,
}

impl Default for Dedup {
    fn default() -> Self {
        Self {
            inner: Mutex::new(HashMap::new()),
            window: Duration::from_secs(60),
        }
    }
}

impl Dedup {
    pub fn new(window: Duration) -> Self {
        Self {
            inner: Mutex::new(HashMap::new()),
            window,
        }
    }
}

#[async_trait]
impl SpamCheck for Dedup {
    async fn evaluate(&self, ctx: &SubmissionContext<'_>) -> SpamVerdict {
        let mut hasher = Sha256::new();
        hasher.update(ctx.form_id.as_bytes());
        if let Some(email) = ctx.email() {
            hasher.update(b"\0email=");
            hasher.update(email.as_bytes());
        }
        // Hash the payload with the email field stripped + normalised
        // separately above. This is what makes case- and whitespace-only
        // diffs in the email collapse onto the same dedupe key.
        let scrubbed = scrub_email(ctx.data);
        let payload_bytes = serde_json::to_vec(&scrubbed).unwrap_or_default();
        let mut payload_hasher = Sha256::new();
        payload_hasher.update(&payload_bytes);
        let payload_digest: [u8; 32] = payload_hasher.finalize().into();
        hasher.update(b"\0payload=");
        hasher.update(payload_digest);
        let key: [u8; 32] = hasher.finalize().into();

        let mut g = self.inner.lock().expect("dedup mutex poisoned");
        let now = Instant::now();
        g.retain(|_, t| now.duration_since(*t) < self.window);
        if g.contains_key(&key) {
            return SpamVerdict::Rejected("duplicate_submission");
        }
        g.insert(key, now);
        SpamVerdict::Clean
    }
}

// ── Pipeline runner ────────────────────────────────────────────────────

pub struct AntispamPipeline {
    checks: Vec<Box<dyn SpamCheck>>,
}

impl AntispamPipeline {
    pub fn new(checks: Vec<Box<dyn SpamCheck>>) -> Self {
        Self { checks }
    }

    /// Run every check until one rejects. Returns the rejection verdict
    /// or `Clean` if every check passed.
    pub async fn evaluate(&self, ctx: &SubmissionContext<'_>) -> SpamVerdict {
        for check in &self.checks {
            let v = check.evaluate(ctx).await;
            if let SpamVerdict::Rejected(_) = v {
                return v;
            }
        }
        SpamVerdict::Clean
    }
}

// ── Unit tests ─────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn ctx<'a>(data: &'a serde_json::Value) -> SubmissionContext<'a> {
        SubmissionContext {
            form_id: Uuid::nil(),
            data,
            ip_hash: "ip0",
            turnstile_token: None,
        }
    }

    #[tokio::test]
    async fn honeypot_rejects_filled_field() {
        let data = json!({ "form_hp": "buy-cheap-meds" });
        assert_eq!(
            Honeypot.evaluate(&ctx(&data)).await,
            SpamVerdict::Rejected("honeypot_tripped")
        );
    }

    #[tokio::test]
    async fn honeypot_passes_empty_field() {
        let data = json!({ "form_hp": "" });
        assert_eq!(Honeypot.evaluate(&ctx(&data)).await, SpamVerdict::Clean);
    }

    #[tokio::test]
    async fn honeypot_passes_when_field_absent() {
        let data = json!({ "name": "Ada" });
        assert_eq!(Honeypot.evaluate(&ctx(&data)).await, SpamVerdict::Clean);
    }

    #[tokio::test]
    async fn dedup_rejects_replay_within_window() {
        let dd = Dedup::new(Duration::from_secs(60));
        let data = json!({ "email": "a@example.com", "msg": "hi" });
        assert_eq!(dd.evaluate(&ctx(&data)).await, SpamVerdict::Clean);
        assert_eq!(
            dd.evaluate(&ctx(&data)).await,
            SpamVerdict::Rejected("duplicate_submission")
        );
    }

    #[tokio::test]
    async fn dedup_distinguishes_payload() {
        let dd = Dedup::new(Duration::from_secs(60));
        let a = json!({ "email": "a@example.com", "msg": "hi" });
        let b = json!({ "email": "a@example.com", "msg": "different" });
        assert_eq!(dd.evaluate(&ctx(&a)).await, SpamVerdict::Clean);
        assert_eq!(dd.evaluate(&ctx(&b)).await, SpamVerdict::Clean);
    }

    #[tokio::test]
    async fn dedup_distinguishes_email() {
        let dd = Dedup::new(Duration::from_secs(60));
        let a = json!({ "email": "a@example.com", "msg": "hi" });
        let b = json!({ "email": "b@example.com", "msg": "hi" });
        assert_eq!(dd.evaluate(&ctx(&a)).await, SpamVerdict::Clean);
        assert_eq!(dd.evaluate(&ctx(&b)).await, SpamVerdict::Clean);
    }

    #[tokio::test]
    async fn dedup_normalises_email_case_and_whitespace() {
        let dd = Dedup::new(Duration::from_secs(60));
        let a = json!({ "email": "  Ada@Example.COM ", "msg": "hi" });
        let b = json!({ "email": "ada@example.com", "msg": "hi" });
        assert_eq!(dd.evaluate(&ctx(&a)).await, SpamVerdict::Clean);
        assert_eq!(
            dd.evaluate(&ctx(&b)).await,
            SpamVerdict::Rejected("duplicate_submission")
        );
    }

    #[tokio::test]
    async fn pipeline_short_circuits_on_first_rejection() {
        let pipe = AntispamPipeline::new(vec![Box::new(Honeypot), Box::new(Dedup::default())]);
        let data = json!({ "form_hp": "trip", "email": "a@example.com" });
        let v = pipe.evaluate(&ctx(&data)).await;
        assert_eq!(v, SpamVerdict::Rejected("honeypot_tripped"));
    }
}
