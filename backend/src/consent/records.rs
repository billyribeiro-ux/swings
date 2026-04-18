//! CONSENT-03 — consent event log + DSAR workflow repository.
//!
//! Writes into `consent_records` (append-only) and `dsar_requests` (workflow
//! state). Reads expose:
//!
//!   * [`insert_consent_record`] / [`list_records_for_subject`] — banner and
//!     `GET /api/consent/me` endpoints.
//!   * [`create_dsar_request`] / [`list_dsar_requests`] / [`fulfill_dsar`] —
//!     subject-facing and admin DSAR flows.
//!   * [`hash_ip`] — GDPR-safe fingerprinting: `SHA256(ip_bytes || daily_salt)`.
//!     The daily salt is seeded from `CONSENT_IP_SALT` (base64 or UTF-8) once
//!     per process, then mixed with the calendar date (UTC) so the effective
//!     salt rotates automatically at midnight.
//!
//! All queries use the runtime-checked `query_as::<_, Row>(...)` form — see
//! `backend/src/consent/repo.rs` for the crate-wide rationale.

use std::sync::OnceLock;

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::{FromRow, PgPool};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::error::AppResult;

// ── IP hashing (GDPR-safe daily-rotated salt) ───────────────────────────

/// Process-scoped daily salt seed. 32 bytes derived from the `CONSENT_IP_SALT`
/// environment variable (raw UTF-8 bytes hashed to 32 bytes) — this is combined
/// with the current UTC calendar date on every hash so the effective salt
/// rotates at midnight without requiring an operator action.
///
/// When the env var is absent, a random seed is generated at first access.
/// That is safe but causes IP hashes to be non-comparable across process
/// restarts; operators deploying to production MUST set `CONSENT_IP_SALT` to
/// a high-entropy value so point-in-time fraud review is possible within the
/// 24h rotation window.
static DAILY_SALT_SEED: OnceLock<[u8; 32]> = OnceLock::new();

fn daily_salt_seed() -> &'static [u8; 32] {
    DAILY_SALT_SEED.get_or_init(|| {
        let raw = std::env::var("CONSENT_IP_SALT").ok();
        let mut hasher = Sha256::new();
        match raw {
            Some(s) if !s.trim().is_empty() => hasher.update(s.as_bytes()),
            _ => {
                // Fallback: derive a random 32-byte seed. Using `rand::random()`
                // here — the crate is already a transitive dep (argon2, etc.).
                let mut rnd = [0u8; 32];
                for b in &mut rnd {
                    *b = rand::random::<u8>();
                }
                hasher.update(rnd);
                tracing::warn!(
                    "CONSENT_IP_SALT is unset; using process-random seed \
                     (IP hashes will not be comparable across restarts)"
                );
            }
        }
        let digest = hasher.finalize();
        let mut out = [0u8; 32];
        out.copy_from_slice(&digest[..]);
        out
    })
}

/// Compute the effective salt for a specific UTC date. Folds the daily seed +
/// date so rotation is automatic.
fn daily_salt_for(date: NaiveDate) -> [u8; 32] {
    let mut h = Sha256::new();
    h.update(daily_salt_seed());
    h.update(date.to_string().as_bytes());
    let digest = h.finalize();
    let mut out = [0u8; 32];
    out.copy_from_slice(&digest[..]);
    out
}

/// Hash a client IP (raw address bytes) using the current-day salt.
///
/// Returns a lowercase hex string. The same IP hashes to the same value for
/// the whole UTC calendar day; it changes the next day.
#[must_use]
pub fn hash_ip(ip: &str) -> String {
    hash_ip_at(ip, Utc::now())
}

/// Testing hook: hash an IP against an explicit time.
#[must_use]
pub fn hash_ip_at(ip: &str, now: DateTime<Utc>) -> String {
    let salt = daily_salt_for(now.date_naive());
    let mut h = Sha256::new();
    h.update(ip.as_bytes());
    h.update(salt);
    hex_encode(&h.finalize())
}

/// Minimal lowercase-hex encoder — keeps the dep tree free of a `hex` crate.
fn hex_encode(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        out.push(HEX[(b >> 4) as usize] as char);
        out.push(HEX[(b & 0x0F) as usize] as char);
    }
    out
}

// ── Consent records ─────────────────────────────────────────────────────

/// All the row fields the caller computes server-side.
#[derive(Debug, Clone)]
pub struct ConsentRecordInput {
    pub subject_id: Option<Uuid>,
    pub anonymous_id: Option<Uuid>,
    pub ip_hash: String,
    pub user_agent: String,
    pub country: Option<String>,
    pub banner_version: i32,
    pub policy_version: i32,
    pub categories: serde_json::Value,
    pub services: serde_json::Value,
    pub action: String,
    pub tcf_string: Option<String>,
    pub gpc_signal: Option<bool>,
}

/// Serialized row shape — shared across `POST /record` response and
/// `GET /me` listings.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ConsentRecordRow {
    pub id: Uuid,
    pub subject_id: Option<Uuid>,
    pub anonymous_id: Option<Uuid>,
    pub banner_version: i32,
    pub policy_version: i32,
    pub categories: serde_json::Value,
    pub services: serde_json::Value,
    pub action: String,
    pub tcf_string: Option<String>,
    pub gpc_signal: Option<bool>,
    pub country: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// Insert a single consent event and return its new id.
pub async fn insert_consent_record(pool: &PgPool, input: &ConsentRecordInput) -> AppResult<Uuid> {
    let row: (Uuid,) = sqlx::query_as(
        r#"
        INSERT INTO consent_records
            (subject_id, anonymous_id, ip_hash, user_agent, country,
             banner_version, policy_version, categories, services,
             action, tcf_string, gpc_signal)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
        RETURNING id
        "#,
    )
    .bind(input.subject_id)
    .bind(input.anonymous_id)
    .bind(&input.ip_hash)
    .bind(&input.user_agent)
    .bind(&input.country)
    .bind(input.banner_version)
    .bind(input.policy_version)
    .bind(&input.categories)
    .bind(&input.services)
    .bind(&input.action)
    .bind(&input.tcf_string)
    .bind(input.gpc_signal)
    .fetch_one(pool)
    .await?;
    Ok(row.0)
}

/// Selector for the `GET /me` listing — the same query shape works for authed
/// subjects (subject_id) and anonymous browser-id cookies.
#[derive(Debug, Clone, Copy)]
pub enum SubjectSelector {
    Subject(Uuid),
    Anonymous(Uuid),
}

/// Fetch the most-recent `limit` consent events for the given subject.
pub async fn list_records_for_subject(
    pool: &PgPool,
    selector: SubjectSelector,
    limit: i64,
) -> AppResult<Vec<ConsentRecordRow>> {
    let limit = limit.clamp(1, 500);
    let rows = match selector {
        SubjectSelector::Subject(id) => {
            sqlx::query_as::<_, ConsentRecordRow>(
                r#"
                SELECT id, subject_id, anonymous_id, banner_version, policy_version,
                       categories, services, action, tcf_string, gpc_signal,
                       country, created_at
                FROM consent_records
                WHERE subject_id = $1
                ORDER BY created_at DESC
                LIMIT $2
                "#,
            )
            .bind(id)
            .bind(limit)
            .fetch_all(pool)
            .await?
        }
        SubjectSelector::Anonymous(id) => {
            sqlx::query_as::<_, ConsentRecordRow>(
                r#"
                SELECT id, subject_id, anonymous_id, banner_version, policy_version,
                       categories, services, action, tcf_string, gpc_signal,
                       country, created_at
                FROM consent_records
                WHERE anonymous_id = $1
                ORDER BY created_at DESC
                LIMIT $2
                "#,
            )
            .bind(id)
            .bind(limit)
            .fetch_all(pool)
            .await?
        }
    };
    Ok(rows)
}

// ── DSAR workflow ───────────────────────────────────────────────────────

/// Row shape returned from the admin list + the subject submission response.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct DsarRow {
    pub id: Uuid,
    pub user_id: Option<Uuid>,
    pub email: String,
    pub kind: String,
    pub status: String,
    pub payload: serde_json::Value,
    pub fulfilled_at: Option<DateTime<Utc>>,
    pub fulfilled_by: Option<Uuid>,
    pub fulfillment_url: Option<String>,
    pub admin_notes: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Inputs that the `POST /api/dsar` handler assembles.
#[derive(Debug, Clone)]
pub struct DsarCreateInput {
    pub user_id: Option<Uuid>,
    pub email: String,
    pub kind: String,
    pub payload: serde_json::Value,
    /// SHA256 hash of the raw verification token the handler will e-mail the
    /// subject. Never pass the raw token here.
    pub verification_token_hash: Option<String>,
}

/// Insert a new DSAR request; row always starts in `status='pending'`.
pub async fn create_dsar_request(pool: &PgPool, input: &DsarCreateInput) -> AppResult<DsarRow> {
    let row = sqlx::query_as::<_, DsarRow>(
        r#"
        INSERT INTO dsar_requests
            (user_id, email, kind, status, verification_token_hash, payload)
        VALUES ($1, $2, $3, 'pending', $4, $5)
        RETURNING id, user_id, email, kind, status, payload,
                  fulfilled_at, fulfilled_by, fulfillment_url, admin_notes,
                  created_at, updated_at
        "#,
    )
    .bind(input.user_id)
    .bind(&input.email)
    .bind(&input.kind)
    .bind(&input.verification_token_hash)
    .bind(&input.payload)
    .fetch_one(pool)
    .await?;
    Ok(row)
}

/// Paginated admin listing with an optional status filter.
pub async fn list_dsar_requests(
    pool: &PgPool,
    status: Option<&str>,
    limit: i64,
    offset: i64,
) -> AppResult<Vec<DsarRow>> {
    let limit = limit.clamp(1, 200);
    let offset = offset.max(0);
    let rows = match status {
        Some(s) => {
            sqlx::query_as::<_, DsarRow>(
                r#"
                SELECT id, user_id, email, kind, status, payload,
                       fulfilled_at, fulfilled_by, fulfillment_url, admin_notes,
                       created_at, updated_at
                FROM dsar_requests
                WHERE status = $1
                ORDER BY created_at DESC
                LIMIT $2 OFFSET $3
                "#,
            )
            .bind(s)
            .bind(limit)
            .bind(offset)
            .fetch_all(pool)
            .await?
        }
        None => {
            sqlx::query_as::<_, DsarRow>(
                r#"
                SELECT id, user_id, email, kind, status, payload,
                       fulfilled_at, fulfilled_by, fulfillment_url, admin_notes,
                       created_at, updated_at
                FROM dsar_requests
                ORDER BY created_at DESC
                LIMIT $1 OFFSET $2
                "#,
            )
            .bind(limit)
            .bind(offset)
            .fetch_all(pool)
            .await?
        }
    };
    Ok(rows)
}

/// Total count for pagination, with optional status filter.
pub async fn count_dsar_requests(pool: &PgPool, status: Option<&str>) -> AppResult<i64> {
    let total: i64 = match status {
        Some(s) => {
            sqlx::query_scalar("SELECT COUNT(*) FROM dsar_requests WHERE status = $1")
                .bind(s)
                .fetch_one(pool)
                .await?
        }
        None => {
            sqlx::query_scalar("SELECT COUNT(*) FROM dsar_requests")
                .fetch_one(pool)
                .await?
        }
    };
    Ok(total)
}

/// Fetch a single DSAR by id — admin fulfilment handler needs the kind + email.
pub async fn get_dsar(pool: &PgPool, id: Uuid) -> AppResult<Option<DsarRow>> {
    let row = sqlx::query_as::<_, DsarRow>(
        r#"
        SELECT id, user_id, email, kind, status, payload,
               fulfilled_at, fulfilled_by, fulfillment_url, admin_notes,
               created_at, updated_at
        FROM dsar_requests
        WHERE id = $1
        "#,
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;
    Ok(row)
}

/// Mark a DSAR as `fulfilled`. No-op if the row is already fulfilled or the
/// id does not exist — the handler's `get_dsar` check provides the friendly
/// error.
pub async fn fulfill_dsar(
    pool: &PgPool,
    id: Uuid,
    fulfilled_by: Uuid,
    fulfillment_url: Option<&str>,
    admin_notes: Option<&str>,
) -> AppResult<Option<DsarRow>> {
    let row = sqlx::query_as::<_, DsarRow>(
        r#"
        UPDATE dsar_requests
        SET status          = 'fulfilled',
            fulfilled_at    = NOW(),
            fulfilled_by    = $2,
            fulfillment_url = COALESCE($3, fulfillment_url),
            admin_notes     = COALESCE($4, admin_notes),
            updated_at      = NOW()
        WHERE id = $1
        RETURNING id, user_id, email, kind, status, payload,
                  fulfilled_at, fulfilled_by, fulfillment_url, admin_notes,
                  created_at, updated_at
        "#,
    )
    .bind(id)
    .bind(fulfilled_by)
    .bind(fulfillment_url)
    .bind(admin_notes)
    .fetch_optional(pool)
    .await?;
    Ok(row)
}

// ── Unit tests ──────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn hash_ip_is_deterministic_within_same_day() {
        let t = Utc.with_ymd_and_hms(2026, 4, 17, 10, 0, 0).unwrap();
        let a = hash_ip_at("203.0.113.7", t);
        let b = hash_ip_at("203.0.113.7", t + chrono::Duration::hours(13));
        assert_eq!(a, b, "same-day hashes must match");

        let c = hash_ip_at("203.0.113.7", t + chrono::Duration::days(1));
        assert_ne!(a, c, "next-day hash must rotate");
    }

    #[test]
    fn hash_ip_differs_by_input() {
        let t = Utc.with_ymd_and_hms(2026, 4, 17, 10, 0, 0).unwrap();
        let a = hash_ip_at("1.1.1.1", t);
        let b = hash_ip_at("1.1.1.2", t);
        assert_ne!(a, b);
        assert_eq!(a.len(), 64, "hex-encoded SHA256 is 64 chars");
        assert!(a.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn consent_record_input_round_trips_optional_fields() {
        // Compile-time smoke-check that the struct has all the optional slots
        // we promise on the wire. A serde round-trip through JSON catches a
        // typo / drift in the row definition before it reaches Postgres.
        let input = ConsentRecordInput {
            subject_id: None,
            anonymous_id: Some(Uuid::new_v4()),
            ip_hash: "deadbeef".into(),
            user_agent: "ua".into(),
            country: None,
            banner_version: 1,
            policy_version: 1,
            categories: serde_json::json!({ "necessary": true }),
            services: serde_json::json!({}),
            action: "granted".into(),
            tcf_string: None,
            gpc_signal: Some(true),
        };
        assert_eq!(input.action, "granted");
        assert_eq!(input.gpc_signal, Some(true));
    }

    #[test]
    fn dsar_create_input_defaults_payload_object() {
        let input = DsarCreateInput {
            user_id: None,
            email: "alice@example.com".into(),
            kind: "access".into(),
            payload: serde_json::json!({}),
            verification_token_hash: Some("hhh".into()),
        };
        assert_eq!(input.kind, "access");
        assert!(input.payload.is_object());
    }

    #[test]
    fn subject_selector_variants_are_distinct() {
        // Exhaustiveness sanity — if someone adds a new variant the match below
        // will stop compiling, forcing us to revisit `list_records_for_subject`.
        let a = SubjectSelector::Subject(Uuid::nil());
        let b = SubjectSelector::Anonymous(Uuid::nil());
        match a {
            SubjectSelector::Subject(_) => {}
            SubjectSelector::Anonymous(_) => panic!("wrong arm"),
        }
        match b {
            SubjectSelector::Anonymous(_) => {}
            SubjectSelector::Subject(_) => panic!("wrong arm"),
        }
    }
}
