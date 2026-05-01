//! One-shot unsubscribe tokens.
//!
//! The `send_notification` path mints a token for every outbound marketing
//! e-mail and embeds its base64-url-safe representation into the rendered body
//! (CTA link to `/u/unsubscribe?token=...`). The public route consumes the
//! token, flips the relevant preference (or adds a suppression row when
//! `category` is `NULL` — meaning "all marketing"), and marks the row as used.
//!
//! Tokens are stored as SHA-256 hashes so the DB never holds the plaintext.

use chrono::{DateTime, Duration, Utc};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, PgExecutor, PgPool};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{crypto::hash_token, error::AppError};

use super::preferences::{self, PreferenceUpdate};
use super::suppression::{self, REASON_UNSUBSCRIBE_ALL};

/// Default TTL for newly minted tokens — long enough that recipients can
/// click weeks after the initial send, short enough that harvested tokens
/// eventually stale out.
pub const DEFAULT_TTL_DAYS: i64 = 60;

/// Row shape for `unsubscribe_tokens` (admin ops + tests).
#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct UnsubscribeTokenRow {
    pub token_hash: String,
    pub user_id: Option<Uuid>,
    pub email: String,
    pub category: Option<String>,
    pub expires_at: DateTime<Utc>,
    pub used: bool,
}

/// What the consumer of a token did — returned from [`consume_token`] for
/// observability + the success page.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum UnsubscribeAction {
    /// User was opted-out of a specific category. Preference rows are the
    /// source of truth.
    CategoryDisabled {
        user_id: Option<Uuid>,
        email: String,
        category: String,
    },
    /// User unsubscribed from *all* marketing — a suppression row is added.
    AllMarketing { email: String },
}

#[derive(Debug, thiserror::Error)]
pub enum UnsubscribeError {
    #[error("token not found")]
    NotFound,
    #[error("token already used")]
    AlreadyUsed,
    #[error("token expired")]
    Expired,
    #[error("database error: {0}")]
    Database(#[from] sqlx::Error),
    #[error("authz error: {0}")]
    App(#[from] AppError),
}

impl From<UnsubscribeError> for AppError {
    fn from(err: UnsubscribeError) -> Self {
        match err {
            UnsubscribeError::NotFound
            | UnsubscribeError::AlreadyUsed
            | UnsubscribeError::Expired => AppError::BadRequest(err.to_string()),
            UnsubscribeError::Database(e) => AppError::Database(e),
            UnsubscribeError::App(e) => e,
        }
    }
}

/// SHA-256 hex of a raw token string.
/// Mint a fresh token. Returns the raw 32-byte URL-safe base64 representation
/// — the caller embeds this in the rendered e-mail; only the SHA-256 hash
/// lives in the DB.
pub async fn mint_token<'e, E: PgExecutor<'e>>(
    executor: E,
    user_id: Option<Uuid>,
    email: &str,
    category: Option<&str>,
    ttl: Option<Duration>,
) -> Result<String, AppError> {
    let mut bytes = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut bytes);
    let raw = url_safe_base64(&bytes);
    let hash = hash_token(&raw);
    let expires_at = Utc::now() + ttl.unwrap_or(Duration::days(DEFAULT_TTL_DAYS));

    sqlx::query(
        r#"
        INSERT INTO unsubscribe_tokens
            (token_hash, user_id, email, category, expires_at, used)
        VALUES ($1, $2, $3, $4, $5, FALSE)
        "#,
    )
    .bind(&hash)
    .bind(user_id)
    .bind(email)
    .bind(category)
    .bind(expires_at)
    .execute(executor)
    .await?;
    Ok(raw)
}

/// Consume a raw token end-to-end: validate, mark used, flip preference /
/// suppression. Runs inside a single transaction so a crash between the
/// "mark used" and "flip preference" writes cannot leave an inconsistent
/// state.
pub async fn consume_token(
    pool: &PgPool,
    raw: &str,
) -> Result<UnsubscribeAction, UnsubscribeError> {
    let hash = hash_token(raw);

    let mut tx = pool.begin().await?;

    let row = sqlx::query_as::<_, UnsubscribeTokenRow>(
        r#"
        SELECT token_hash, user_id, email, category, expires_at, used
        FROM unsubscribe_tokens
        WHERE token_hash = $1
        FOR UPDATE
        "#,
    )
    .bind(&hash)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or(UnsubscribeError::NotFound)?;

    if row.used {
        tx.rollback().await?;
        return Err(UnsubscribeError::AlreadyUsed);
    }
    if row.expires_at <= Utc::now() {
        tx.rollback().await?;
        return Err(UnsubscribeError::Expired);
    }

    sqlx::query("UPDATE unsubscribe_tokens SET used = TRUE WHERE token_hash = $1")
        .bind(&hash)
        .execute(&mut *tx)
        .await?;

    let action = match (row.user_id, row.category.as_deref()) {
        (Some(user_id), Some(cat)) => {
            // Flip all channels for (user, category) off.
            let update = PreferenceUpdate {
                category: cat.to_string(),
                channel: "email".to_string(),
                enabled: false,
                quiet_hours_start: None,
                quiet_hours_end: None,
                timezone: None,
            };
            preferences::set_preference(&mut *tx, user_id, &update).await?;
            UnsubscribeAction::CategoryDisabled {
                user_id: Some(user_id),
                email: row.email.clone(),
                category: cat.to_string(),
            }
        }
        _ => {
            // No user_id or no category → suppress the e-mail wholesale.
            suppression::suppress(&mut *tx, &row.email, REASON_UNSUBSCRIBE_ALL).await?;
            UnsubscribeAction::AllMarketing {
                email: row.email.clone(),
            }
        }
    };

    tx.commit().await?;
    Ok(action)
}

/// URL-safe base64 without padding. Keeps tokens CLI-paste-friendly.
fn url_safe_base64(bytes: &[u8]) -> String {
    // Hand-rolled to avoid pulling in `base64` crate — the alphabet is short
    // and the function is tiny.
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
    let mut out = String::with_capacity((bytes.len() * 4).div_ceil(3));
    let mut i = 0;
    while i + 3 <= bytes.len() {
        let n =
            (u32::from(bytes[i]) << 16) | (u32::from(bytes[i + 1]) << 8) | u32::from(bytes[i + 2]);
        out.push(ALPHABET[((n >> 18) & 0x3f) as usize] as char);
        out.push(ALPHABET[((n >> 12) & 0x3f) as usize] as char);
        out.push(ALPHABET[((n >> 6) & 0x3f) as usize] as char);
        out.push(ALPHABET[(n & 0x3f) as usize] as char);
        i += 3;
    }
    let rem = bytes.len() - i;
    if rem == 1 {
        let n = u32::from(bytes[i]) << 16;
        out.push(ALPHABET[((n >> 18) & 0x3f) as usize] as char);
        out.push(ALPHABET[((n >> 12) & 0x3f) as usize] as char);
    } else if rem == 2 {
        let n = (u32::from(bytes[i]) << 16) | (u32::from(bytes[i + 1]) << 8);
        out.push(ALPHABET[((n >> 18) & 0x3f) as usize] as char);
        out.push(ALPHABET[((n >> 12) & 0x3f) as usize] as char);
        out.push(ALPHABET[((n >> 6) & 0x3f) as usize] as char);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_is_hex_64_chars() {
        let h = hash_token("some-raw-token");
        assert_eq!(h.len(), 64);
        assert!(h.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn hash_is_deterministic() {
        assert_eq!(hash_token("same"), hash_token("same"));
        assert_ne!(hash_token("a"), hash_token("b"));
    }

    #[test]
    fn base64_round_length_matches_input() {
        assert_eq!(url_safe_base64(&[]).len(), 0);
        assert_eq!(url_safe_base64(&[0x00]).len(), 2); // 1 byte → 2 chars
        assert_eq!(url_safe_base64(&[0x00, 0x01]).len(), 3); // 2 bytes → 3 chars
        assert_eq!(url_safe_base64(&[0x00, 0x01, 0x02]).len(), 4); // 3 bytes → 4 chars
                                                                   // 32 bytes = 10 full groups (30 bytes → 40 chars) + 2 trailing (→ 3 chars) = 43.
        assert_eq!(url_safe_base64(&[0u8; 32]).len(), 43);
    }

    #[test]
    fn base64_only_uses_url_safe_alphabet() {
        let raw = url_safe_base64(&[0xff, 0xfb, 0xfc, 0xab, 0x00]);
        assert!(!raw.contains('+'));
        assert!(!raw.contains('/'));
        assert!(!raw.contains('='));
    }

    #[test]
    fn base64_zero_length_returns_empty() {
        assert_eq!(url_safe_base64(&[]), "");
    }
}
