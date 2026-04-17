//! Resend delivery webhooks.
//!
//! FDN-09 TODO: wire `POST /api/webhooks/email/resend` here — HMAC + timestamp
//! tolerance verification plus a status-transition handler that maps
//! `email.delivered` / `email.bounced` / `email.complained` / `email.opened`
//! / `email.clicked` events to `notification_deliveries.status` updates and
//! (for hard bounces + complaints) to `notification_suppression` inserts.
//!
//! For FDN-05 we only ship the signature-verification skeleton so callers in
//! a later subsystem can stack on without re-touching the module tree.

use hmac::{Hmac, Mac};
use sha2::Sha256;

/// Verify a Resend webhook signature using the shared secret.
///
/// Resend signs the request body with HMAC-SHA256 and sends the hex digest in
/// the `svix-signature` header (actually `v1,HEX`). This function takes the
/// raw body and the extracted HEX digest and returns whether they match in
/// constant time.
///
/// FDN-09 TODO: swap to the header format Resend actually ships with and add
/// timestamp tolerance.
#[must_use]
pub fn verify_signature(body: &[u8], secret: &[u8], provided_hex: &str) -> bool {
    let mut mac = match Hmac::<Sha256>::new_from_slice(secret) {
        Ok(m) => m,
        Err(_) => return false,
    };
    mac.update(body);
    let expected = mac.finalize().into_bytes();
    let expected_hex: String = expected.iter().map(|b| format!("{b:02x}")).collect();
    constant_time_eq(provided_hex.as_bytes(), expected_hex.as_bytes())
}

/// Constant-time comparison that does not short-circuit on first mismatch.
/// Prevents timing-based signature oracles.
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

#[cfg(test)]
mod tests {
    use super::*;

    fn sign(body: &[u8], secret: &[u8]) -> String {
        let mut mac = Hmac::<Sha256>::new_from_slice(secret).expect("hmac key");
        mac.update(body);
        mac.finalize()
            .into_bytes()
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect()
    }

    #[test]
    fn accepts_valid_signature() {
        let body = b"{\"type\":\"email.delivered\"}";
        let secret = b"shh";
        let sig = sign(body, secret);
        assert!(verify_signature(body, secret, &sig));
    }

    #[test]
    fn rejects_tampered_body() {
        let body = b"{\"type\":\"email.delivered\"}";
        let secret = b"shh";
        let sig = sign(body, secret);
        assert!(!verify_signature(b"tampered", secret, &sig));
    }

    #[test]
    fn rejects_wrong_secret() {
        let body = b"x";
        let sig = sign(body, b"one");
        assert!(!verify_signature(body, b"two", &sig));
    }

    #[test]
    fn constant_time_eq_length_mismatch() {
        assert!(!constant_time_eq(b"abc", b"abcd"));
    }
}
