//! EC-07: Digital-delivery grants + download-token consumption.
//!
//! The lifecycle:
//!
//! 1. `order.completed` fires an outbox event that the `digital_delivery`
//!    handler picks up; for each downloadable order_item it calls
//!    [`grant_download`], which mints a 32-byte random token and persists
//!    a `user_downloads` row with a quota + expiry.
//! 2. The customer receives a URL containing the token (via email or the
//!    `my-account/downloads` page).
//! 3. `GET /api/downloads/{token}` calls [`consume_download`] which
//!    atomically decrements `downloads_remaining` and returns the row's
//!    `storage_key` + `mime_type`. The handler then calls
//!    [`make_signed_url`] to mint a short-lived R2 URL and 302-redirects
//!    the user.
//!
//! Real R2 presigning is a later concern; [`make_signed_url`] currently
//! returns an internal path with an expiry so integration tests can drive
//! the full flow without an R2 dependency.

use std::time::{SystemTime, UNIX_EPOCH};

use chrono::{DateTime, Duration, Utc};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, PgPool};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::error::AppResult;

/// `user_downloads` row. The `download_token` field carries the raw 32-byte
/// random issued at grant time.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct UserDownload {
    pub id: Uuid,
    pub user_id: Uuid,
    pub order_id: Option<Uuid>,
    pub product_id: Uuid,
    pub asset_id: Uuid,
    pub storage_key: String,
    pub mime_type: String,
    /// Raw 32-byte token. Serialized as hex when the row is rendered on the
    /// wire; internal code compares / binds as bytes directly.
    #[serde(with = "hex_bytes")]
    pub download_token: Vec<u8>,
    pub expires_at: DateTime<Utc>,
    pub downloads_remaining: i32,
    pub created_at: DateTime<Utc>,
}

/// Payload returned from [`grant_download`]. Bundles the persisted row with
/// the plaintext token — the only place in the codebase that exposes the
/// token as a hex string. Callers ship this to the end user and then drop
/// the value; the DB row keeps the bytes for future lookups.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct GrantedDownload {
    pub download: UserDownload,
    /// Hex-encoded token. Safe to embed in a URL.
    pub token: String,
}

/// Return payload of [`consume_download`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct ResolvedDownload {
    pub storage_key: String,
    pub mime_type: String,
    pub downloads_remaining: i32,
}

// ── Token helpers ──────────────────────────────────────────────────────

/// Generate a 32-byte random token using the OS CSPRNG.
#[must_use]
pub fn new_token() -> Vec<u8> {
    let mut buf = vec![0_u8; 32];
    rand::thread_rng().fill_bytes(&mut buf);
    buf
}

/// Encode raw bytes as lowercase hex.
#[must_use]
pub fn encode_token(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        out.push_str(&format!("{b:02x}"));
    }
    out
}

/// Decode a hex token string back to raw bytes. Returns `None` when the
/// input is malformed — the handler surfaces a 404 in that case so we
/// don't leak whether the token existed at all.
#[must_use]
pub fn decode_token(s: &str) -> Option<Vec<u8>> {
    if !s.len().is_multiple_of(2) {
        return None;
    }
    let mut out = Vec::with_capacity(s.len() / 2);
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        let hi = hex_nibble(bytes[i])?;
        let lo = hex_nibble(bytes[i + 1])?;
        out.push((hi << 4) | lo);
        i += 2;
    }
    Some(out)
}

fn hex_nibble(b: u8) -> Option<u8> {
    Some(match b {
        b'0'..=b'9' => b - b'0',
        b'a'..=b'f' => b - b'a' + 10,
        b'A'..=b'F' => b - b'A' + 10,
        _ => return None,
    })
}

mod hex_bytes {
    //! serde helper — renders `Vec<u8>` as lowercase hex, parses hex back to
    //! bytes. Kept private so the rest of the crate keeps dealing in
    //! `Vec<u8>`.
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S: Serializer>(bytes: &[u8], ser: S) -> Result<S::Ok, S::Error> {
        ser.serialize_str(&super::encode_token(bytes))
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(de: D) -> Result<Vec<u8>, D::Error> {
        let s = String::deserialize(de)?;
        super::decode_token(&s).ok_or_else(|| serde::de::Error::custom("invalid hex"))
    }
}

// ── Grant / consume ────────────────────────────────────────────────────

/// Input bundle for [`grant_download`]. Keeping the arg list in a struct
/// keeps call sites readable at the handler layer.
#[derive(Debug, Clone)]
pub struct GrantInput<'a> {
    pub user_id: Uuid,
    pub order_id: Option<Uuid>,
    pub product_id: Uuid,
    pub asset_id: Uuid,
    pub storage_key: &'a str,
    pub mime_type: &'a str,
    pub downloads_allowed: i32,
    pub ttl: Duration,
}

/// Mint a fresh download grant. Called by the `digital_delivery` outbox
/// handler after `order.completed`.
pub async fn grant_download(pool: &PgPool, input: GrantInput<'_>) -> AppResult<GrantedDownload> {
    let token = new_token();
    let expires = Utc::now() + input.ttl;
    let row = sqlx::query_as::<_, UserDownload>(
        r#"
        INSERT INTO user_downloads (
            user_id, order_id, product_id, asset_id,
            storage_key, mime_type, download_token, expires_at, downloads_remaining
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
        RETURNING id, user_id, order_id, product_id, asset_id, storage_key,
                  mime_type, download_token, expires_at, downloads_remaining, created_at
        "#,
    )
    .bind(input.user_id)
    .bind(input.order_id)
    .bind(input.product_id)
    .bind(input.asset_id)
    .bind(input.storage_key)
    .bind(input.mime_type)
    .bind(&token)
    .bind(expires)
    .bind(input.downloads_allowed)
    .fetch_one(pool)
    .await?;
    let token_str = encode_token(&token);
    Ok(GrantedDownload {
        download: row,
        token: token_str,
    })
}

/// Decrement the quota on the grant keyed by `raw_token`. Returns
/// `Ok(Some(_))` when a fresh download was approved, `Ok(None)` when the
/// token is unknown / expired / exhausted.
///
/// The UPDATE uses a `WHERE downloads_remaining > 0 AND expires_at > NOW()`
/// guard so concurrent calls cannot race past the quota; the atomic
/// decrement is the only contended path.
pub async fn consume_download(
    pool: &PgPool,
    raw_token: &[u8],
) -> AppResult<Option<ResolvedDownload>> {
    let row = sqlx::query_as::<_, (String, String, i32)>(
        r#"
        UPDATE user_downloads
           SET downloads_remaining = downloads_remaining - 1
         WHERE download_token = $1
           AND downloads_remaining > 0
           AND expires_at > NOW()
        RETURNING storage_key, mime_type, downloads_remaining
        "#,
    )
    .bind(raw_token)
    .fetch_optional(pool)
    .await?;
    Ok(
        row.map(|(storage_key, mime_type, remaining)| ResolvedDownload {
            storage_key,
            mime_type,
            downloads_remaining: remaining,
        }),
    )
}

/// List every active grant for a user. Used by the `my-account/downloads`
/// page — tokens are intentionally included so the page can render a
/// clickable link without an extra round-trip.
pub async fn list_for_user(pool: &PgPool, user_id: Uuid) -> AppResult<Vec<UserDownload>> {
    let rows = sqlx::query_as::<_, UserDownload>(
        r#"
        SELECT id, user_id, order_id, product_id, asset_id, storage_key,
               mime_type, download_token, expires_at, downloads_remaining, created_at
        FROM user_downloads
        WHERE user_id = $1 AND expires_at > NOW() AND downloads_remaining > 0
        ORDER BY created_at DESC
        "#,
    )
    .bind(user_id)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

// ── Signed URL ─────────────────────────────────────────────────────────

/// Produce a short-lived internal redirect URL pointing at the R2 object.
///
/// Used when no R2 client is in scope (tests, CLI, dev without R2 env
/// vars). The handler 302s to this URL and an internal `/internal/r2/`
/// route streams the bytes from local disk — same UX as the production
/// presigner without requiring a real bucket.
#[must_use]
pub fn make_signed_url(storage_key: &str, ttl: Duration) -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    let expires = now.saturating_add(ttl.num_seconds());
    format!("/internal/r2/{storage_key}?expires={expires}")
}

/// EC-07: produce a real R2 presigned GET URL for the supplied storage
/// key, valid for `ttl`. Falls back to [`make_signed_url`] when the
/// supplied storage backend is not R2-backed (dev / tests).
pub async fn make_presigned_or_local_url(
    backend: &crate::services::storage::MediaBackend,
    storage_key: &str,
    ttl: Duration,
) -> String {
    if let crate::services::storage::MediaBackend::R2(r2) = backend {
        let std_ttl = std::time::Duration::from_secs(ttl.num_seconds().max(1) as u64);
        match r2.presign_get(storage_key, std_ttl).await {
            Ok(url) => return url,
            Err(e) => {
                tracing::warn!(
                    "R2 presign failed for {storage_key}: {e}; falling back to local URL"
                );
            }
        }
    }
    make_signed_url(storage_key, ttl)
}

// ── Unit tests ─────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokens_are_32_bytes_and_unique() {
        let a = new_token();
        let b = new_token();
        assert_eq!(a.len(), 32);
        assert_eq!(b.len(), 32);
        assert_ne!(a, b);
    }

    #[test]
    fn hex_round_trips() {
        let raw = new_token();
        let s = encode_token(&raw);
        assert_eq!(s.len(), 64);
        let back = decode_token(&s).expect("valid hex");
        assert_eq!(back, raw);
    }

    #[test]
    fn decode_rejects_malformed() {
        assert!(decode_token("ZZ").is_none());
        assert!(decode_token("abc").is_none()); // odd length
        assert!(decode_token(" ").is_none());
    }

    #[test]
    fn signed_url_has_expiry() {
        let url = make_signed_url("assets/foo.zip", Duration::minutes(5));
        assert!(url.starts_with("/internal/r2/assets/foo.zip"));
        assert!(url.contains("expires="));
    }

    #[test]
    fn signed_url_expiry_moves_forward() {
        // Two URLs with different TTLs — longer TTL should have a strictly
        // greater `expires` stamp. Guards against a future refactor that
        // drops the TTL addition.
        let short = make_signed_url("k", Duration::seconds(1));
        let long = make_signed_url("k", Duration::hours(24));
        let extract = |u: &str| -> i64 {
            u.split("expires=")
                .nth(1)
                .and_then(|s| s.parse::<i64>().ok())
                .unwrap_or(0)
        };
        assert!(extract(&long) > extract(&short));
    }

    #[test]
    fn resolved_download_is_serializable() {
        let r = ResolvedDownload {
            storage_key: "k".into(),
            mime_type: "application/pdf".into(),
            downloads_remaining: 3,
        };
        let s = serde_json::to_string(&r).expect("serialize");
        let back: ResolvedDownload = serde_json::from_str(&s).expect("deserialize");
        assert_eq!(r, back);
    }
}
