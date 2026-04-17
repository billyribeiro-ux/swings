//! Resend delivery webhooks.
//!
//! The receiver:
//!   1. HMAC-SHA256-verifies the request body against `RESEND_WEBHOOK_SECRET`
//!      with a ±5-minute timestamp tolerance (mirrors the Stripe flow in
//!      `handlers/webhooks.rs`).
//!   2. De-duplicates via `processed_webhook_events` (composite PK
//!      `(source='resend', event_id)` — see migration 023).
//!   3. Translates the Resend event `type` to the corresponding
//!      `notification_deliveries.status` transition.
//!   4. For `email.bounced`/`email.complained`, inserts a
//!      `notification_suppression` row so future sends short-circuit.
//!
//! # Monotonicity
//!
//! Resend emits events asynchronously and is not guaranteed to deliver them
//! in temporal order (a delayed `email.opened` can race an already-written
//! `email.clicked`). The status ladder is monotonic — see
//! [`precedence`] — so a later webhook never downgrades a row.

use hmac::{Hmac, Mac};
use serde::Deserialize;
use sha2::Sha256;
use sqlx::PgPool;
use tracing::{debug, warn};

use crate::error::AppError;
use crate::notifications::suppression;

/// Outcome of parsing + storing a single Resend webhook event.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WebhookOutcome {
    /// The delivery row was updated (status set by [`status_for_event`]).
    Updated,
    /// The event type is known but has no status transition (e.g.
    /// `email.delivery_delayed` — logged only).
    NoOp,
    /// The event id matched an existing `(source='resend', event_id)` row
    /// and was therefore a duplicate; we ACK with 200.
    Duplicate,
    /// No delivery row matches the supplied provider id. We log + return
    /// `NoOp` so the provider stops retrying (Resend escalates otherwise).
    UnknownDelivery,
}

/// Payload shape for the subset of fields we read. Remaining fields are
/// tolerated by `serde(default)` / the catch-all below.
#[derive(Debug, Deserialize)]
pub struct ResendWebhookEnvelope {
    /// Event type, e.g. `"email.delivered"`.
    #[serde(rename = "type")]
    pub event_type: String,
    /// Provider-assigned webhook event id. Used as the idempotency key.
    #[serde(rename = "id", default)]
    pub event_id: String,
    /// ISO-8601 creation timestamp — not required for processing but often
    /// useful in the logs. Optional for robustness.
    #[serde(default)]
    pub created_at: Option<String>,
    /// Event-specific payload (`email_id`, `to`, bounce reason, …).
    pub data: ResendEventData,
}

#[derive(Debug, Deserialize)]
pub struct ResendEventData {
    /// The Resend-side `id` for the email (aka `notification_deliveries.provider_id`).
    #[serde(rename = "email_id", default)]
    pub email_id: Option<String>,
    /// Recipient address(es). Resend sends a JSON array; take the first
    /// element as the suppression key.
    #[serde(default)]
    pub to: Vec<String>,
    /// Present on `email.bounced`. `"hard"` or `"soft"`; only hard bounces
    /// suppress. Falls back to `None` when Resend hasn't characterised it.
    #[serde(default)]
    pub bounce_type: Option<String>,
}

impl ResendEventData {
    fn primary_recipient(&self) -> Option<&str> {
        self.to.first().map(String::as_str)
    }
}

/// Status ladder for the `notification_deliveries.status` column. Ordered by
/// terminal-ness so [`precedence`] can reject attempts to downgrade a row
/// from `clicked` to `opened` / from `delivered` back to `sent`.
#[must_use]
fn precedence(status: &str) -> u8 {
    match status {
        "queued" => 0,
        "sent" => 1,
        "delivered" => 2,
        "opened" => 3,
        "clicked" => 4,
        // Terminal failure buckets — higher than positive ladder so
        // monotonic rule also protects against "delivered → bounced".
        "bounced" | "complained" | "failed" | "suppressed" => 5,
        _ => 0,
    }
}

/// Translate a Resend event type to the matching
/// `notification_deliveries.status` string. `None` means "known event with
/// no status transition" — currently only `email.delivery_delayed`.
#[must_use]
pub fn status_for_event(event_type: &str) -> Option<&'static str> {
    match event_type {
        "email.sent" => Some("sent"),
        "email.delivered" => Some("delivered"),
        "email.bounced" => Some("bounced"),
        "email.complained" => Some("complained"),
        "email.opened" => Some("opened"),
        "email.clicked" => Some("clicked"),
        "email.delivery_delayed" => None,
        _ => None,
    }
}

/// Verify a Resend (Svix-format) signature header.
///
/// Resend uses the Svix signing scheme: the header is `svix-signature:
/// v1,<base64>` with `v1,<base64>` repeated for each valid signature, and
/// the signed string is `"{svix_id}.{svix_timestamp}.{body}"`. We accept
/// both the `svix-signature` header and the simpler `v1,HEX` format so
/// fixture-based tests can stand up a verifier without carrying the full
/// Svix id.
///
/// * `timestamp` — Unix seconds pulled from `svix-timestamp` (or
///   `resend-timestamp` in minimal-header mode). Rejected when more than
///   `MAX_TIMESTAMP_SKEW_SECS` off the current wall clock.
/// * `provided_header` — raw header value.
/// * `svix_id` — the `svix-id` header contents (empty string when the
///   legacy fixture format is used).
#[must_use]
pub fn verify_signature(
    body: &[u8],
    secret: &[u8],
    svix_id: &str,
    timestamp: i64,
    provided_header: &str,
) -> bool {
    let now = chrono::Utc::now().timestamp();
    if (now - timestamp).abs() > MAX_TIMESTAMP_SKEW_SECS {
        return false;
    }

    // Svix-format payload: "{id}.{timestamp}.{body}". When the caller
    // supplies an empty svix_id we fall back to "{timestamp}.{body}" so
    // simpler fixture flows (and the Stripe parallel) still verify.
    let signed_string: Vec<u8> = if svix_id.is_empty() {
        format!("{timestamp}.")
            .into_bytes()
            .into_iter()
            .chain(body.iter().copied())
            .collect()
    } else {
        format!("{svix_id}.{timestamp}.")
            .into_bytes()
            .into_iter()
            .chain(body.iter().copied())
            .collect()
    };

    let mut mac = match Hmac::<Sha256>::new_from_slice(secret) {
        Ok(m) => m,
        Err(_) => return false,
    };
    mac.update(&signed_string);
    let expected = mac.finalize().into_bytes();

    // Resend/Svix emit base64 in the "svix-signature" header. We also accept
    // hex (for our own fixture-style tests). Both paths use constant-time
    // comparison on the raw header component.
    let expected_hex: String = expected.iter().map(|b| format!("{b:02x}")).collect();
    let expected_b64 = base64_encode(&expected);

    // Header format: one or more `v1,<sig>` items separated by whitespace.
    provided_header
        .split_whitespace()
        .filter_map(|item| {
            let mut parts = item.splitn(2, ',');
            let scheme = parts.next()?.trim();
            let sig = parts.next()?.trim();
            if scheme.eq_ignore_ascii_case("v1") {
                Some(sig)
            } else {
                None
            }
        })
        .any(|candidate| {
            constant_time_eq(candidate.as_bytes(), expected_hex.as_bytes())
                || constant_time_eq(candidate.as_bytes(), expected_b64.as_bytes())
        })
}

/// ±5 minutes mirrors the Stripe verifier.
pub const MAX_TIMESTAMP_SKEW_SECS: i64 = 300;

fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}

/// Minimal base64 encoder so we don't pull a whole crate just for the
/// webhook comparison path. Standard alphabet (RFC 4648 §4) with padding.
fn base64_encode(input: &[u8]) -> String {
    const ALPHA: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(input.len().div_ceil(3) * 4);
    let mut iter = input.chunks_exact(3);
    for chunk in iter.by_ref() {
        let n = ((chunk[0] as u32) << 16) | ((chunk[1] as u32) << 8) | (chunk[2] as u32);
        out.push(ALPHA[((n >> 18) & 0x3f) as usize] as char);
        out.push(ALPHA[((n >> 12) & 0x3f) as usize] as char);
        out.push(ALPHA[((n >> 6) & 0x3f) as usize] as char);
        out.push(ALPHA[(n & 0x3f) as usize] as char);
    }
    let rem = iter.remainder();
    match rem.len() {
        0 => {}
        1 => {
            let n = (rem[0] as u32) << 16;
            out.push(ALPHA[((n >> 18) & 0x3f) as usize] as char);
            out.push(ALPHA[((n >> 12) & 0x3f) as usize] as char);
            out.push('=');
            out.push('=');
        }
        2 => {
            let n = ((rem[0] as u32) << 16) | ((rem[1] as u32) << 8);
            out.push(ALPHA[((n >> 18) & 0x3f) as usize] as char);
            out.push(ALPHA[((n >> 12) & 0x3f) as usize] as char);
            out.push(ALPHA[((n >> 6) & 0x3f) as usize] as char);
            out.push('=');
        }
        _ => unreachable!(),
    }
    out
}

/// Parse + persist a single Resend event. Assumes the signature has already
/// been verified (see [`verify_signature`]) and the request body was
/// JSON-decoded into [`ResendWebhookEnvelope`].
pub async fn process_event(
    pool: &PgPool,
    envelope: &ResendWebhookEnvelope,
) -> Result<WebhookOutcome, AppError> {
    // Idempotency: claim the (source, event_id) pair. An empty event id
    // disables dedup for this specific event (rare; mostly dev fixtures).
    if !envelope.event_id.is_empty() {
        let claimed = crate::db::try_claim_webhook_event(
            pool,
            "resend",
            &envelope.event_id,
            &envelope.event_type,
        )
        .await?;
        if !claimed {
            debug!(event_id = %envelope.event_id, "resend webhook duplicate; acknowledging");
            return Ok(WebhookOutcome::Duplicate);
        }
    }

    let Some(new_status) = status_for_event(&envelope.event_type) else {
        debug!(
            event_type = %envelope.event_type,
            "resend webhook: informational event (no status transition)"
        );
        return Ok(WebhookOutcome::NoOp);
    };

    let Some(provider_id) = envelope.data.email_id.as_deref() else {
        warn!(
            event_type = %envelope.event_type,
            "resend webhook missing data.email_id"
        );
        return Ok(WebhookOutcome::NoOp);
    };

    // Load the current status so we can enforce monotonic transitions.
    // Matching on provider_id (not delivery id) because that's all Resend
    // knows about.
    let current: Option<(sqlx::types::Uuid, String)> = sqlx::query_as(
        "SELECT id, status FROM notification_deliveries WHERE provider_id = $1 LIMIT 1",
    )
    .bind(provider_id)
    .fetch_optional(pool)
    .await?;

    let Some((delivery_id, current_status)) = current else {
        warn!(
            provider_id,
            event_type = %envelope.event_type,
            "resend webhook references unknown provider_id"
        );
        return Ok(WebhookOutcome::UnknownDelivery);
    };

    if precedence(new_status) < precedence(&current_status) {
        debug!(
            delivery_id = %delivery_id,
            from = %current_status,
            to = %new_status,
            "resend webhook: skipping non-monotonic status transition"
        );
        return Ok(WebhookOutcome::NoOp);
    }

    sqlx::query(
        r#"
        UPDATE notification_deliveries
        SET status = $2,
            metadata = metadata || jsonb_build_object('resend_last_event', $3::text),
            updated_at = NOW()
        WHERE id = $1
        "#,
    )
    .bind(delivery_id)
    .bind(new_status)
    .bind(&envelope.event_type)
    .execute(pool)
    .await?;

    // Bounces + complaints also feed the per-email suppression list so
    // future sends bypass the provider entirely.
    match envelope.event_type.as_str() {
        "email.bounced" => {
            // Only hard bounces suppress. Soft bounces are transient
            // (full mailbox, temporary DNS failure) — future sends may
            // well succeed, so we leave them off the deny list.
            let is_hard = envelope
                .data
                .bounce_type
                .as_deref()
                .map(|t| t.eq_ignore_ascii_case("hard"))
                .unwrap_or(false);
            if is_hard {
                if let Some(email) = envelope.data.primary_recipient() {
                    let _ =
                        suppression::suppress(pool, email, suppression::REASON_BOUNCE_HARD).await;
                }
            }
        }
        "email.complained" => {
            if let Some(email) = envelope.data.primary_recipient() {
                let _ = suppression::suppress(pool, email, suppression::REASON_COMPLAINT).await;
            }
        }
        _ => {}
    }

    Ok(WebhookOutcome::Updated)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sign(body: &[u8], secret: &[u8], timestamp: i64, svix_id: &str) -> String {
        let prefix = if svix_id.is_empty() {
            format!("{timestamp}.")
        } else {
            format!("{svix_id}.{timestamp}.")
        };
        let mut mac = Hmac::<Sha256>::new_from_slice(secret).expect("hmac key");
        mac.update(prefix.as_bytes());
        mac.update(body);
        let digest = mac.finalize().into_bytes();
        let hex: String = digest.iter().map(|b| format!("{b:02x}")).collect();
        format!("v1,{hex}")
    }

    #[test]
    fn accepts_valid_signature_no_svix_id() {
        let body = br#"{"type":"email.delivered"}"#;
        let secret = b"whsec_shh";
        let ts = chrono::Utc::now().timestamp();
        let sig = sign(body, secret, ts, "");
        assert!(verify_signature(body, secret, "", ts, &sig));
    }

    #[test]
    fn accepts_valid_signature_with_svix_id() {
        let body = br#"{"type":"email.opened"}"#;
        let secret = b"whsec_shh";
        let ts = chrono::Utc::now().timestamp();
        let svix_id = "msg_01HQZ0J";
        let sig = sign(body, secret, ts, svix_id);
        assert!(verify_signature(body, secret, svix_id, ts, &sig));
    }

    #[test]
    fn rejects_tampered_body() {
        let secret = b"whsec_shh";
        let ts = chrono::Utc::now().timestamp();
        let sig = sign(br#"{"type":"email.delivered"}"#, secret, ts, "");
        assert!(!verify_signature(b"tampered", secret, "", ts, &sig));
    }

    #[test]
    fn rejects_wrong_secret() {
        let body = br#"{}"#;
        let ts = chrono::Utc::now().timestamp();
        let sig = sign(body, b"one", ts, "");
        assert!(!verify_signature(body, b"two", "", ts, &sig));
    }

    #[test]
    fn rejects_stale_timestamp() {
        let body = br#"{}"#;
        let secret = b"shh";
        let stale = chrono::Utc::now().timestamp() - 1_000;
        let sig = sign(body, secret, stale, "");
        assert!(!verify_signature(body, secret, "", stale, &sig));
    }

    #[test]
    fn rejects_missing_v1_prefix() {
        let body = br#"{}"#;
        let secret = b"shh";
        let ts = chrono::Utc::now().timestamp();
        let mut mac = Hmac::<Sha256>::new_from_slice(secret).unwrap();
        mac.update(format!("{ts}.").as_bytes());
        mac.update(body);
        let digest = mac.finalize().into_bytes();
        let hex: String = digest.iter().map(|b| format!("{b:02x}")).collect();
        let header_without_scheme = hex; // no "v1," prefix
        assert!(!verify_signature(
            body,
            secret,
            "",
            ts,
            &header_without_scheme
        ));
    }

    #[test]
    fn constant_time_eq_length_mismatch() {
        assert!(!constant_time_eq(b"abc", b"abcd"));
    }

    #[test]
    fn status_mapping_table() {
        assert_eq!(status_for_event("email.sent"), Some("sent"));
        assert_eq!(status_for_event("email.delivered"), Some("delivered"));
        assert_eq!(status_for_event("email.bounced"), Some("bounced"));
        assert_eq!(status_for_event("email.complained"), Some("complained"));
        assert_eq!(status_for_event("email.opened"), Some("opened"));
        assert_eq!(status_for_event("email.clicked"), Some("clicked"));
        assert_eq!(status_for_event("email.delivery_delayed"), None);
        assert_eq!(status_for_event("email.unknown"), None);
    }

    #[test]
    fn precedence_monotonicity() {
        assert!(precedence("delivered") > precedence("sent"));
        assert!(precedence("clicked") > precedence("opened"));
        assert!(precedence("opened") > precedence("delivered"));
        // Terminal failure classes sit at or above the engagement tier so a
        // late bounce can still overwrite a prior `delivered` row.
        assert!(precedence("bounced") >= precedence("delivered"));
    }

    #[test]
    fn base64_encode_matches_known_vectors() {
        assert_eq!(base64_encode(b""), "");
        assert_eq!(base64_encode(b"f"), "Zg==");
        assert_eq!(base64_encode(b"fo"), "Zm8=");
        assert_eq!(base64_encode(b"foo"), "Zm9v");
        assert_eq!(base64_encode(b"hello"), "aGVsbG8=");
    }
}
