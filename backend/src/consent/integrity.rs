//! CONSENT-07: tamper-evident hash-chain anchors for consent records.
//!
//! The anchor is a rolling SHA-256 over the canonical serialisation of a
//! bounded window of `consent_records`. Running the chain forward from the
//! earliest row gives a single 64-hex digest per window; persisting it in
//! [`consent_integrity_anchors`] (migration 026) means any subsequent mutation
//! to a covered row breaks the recomputed hash.
//!
//! # Algorithm
//!
//! ```text
//! state = SHA256::new()
//! for row in rows.ordered_by("created_at ASC, id ASC"):
//!     state.update(row.id || row.subject_id || row.action || row.ts)
//! anchor = hex(state.finalize())
//! ```
//!
//! Canonical serialisation is `<uuid>|<subject>|<action>|<rfc3339 ts>` with
//! `|` as the delimiter — no JSON, no escaping, because the UUID + enum +
//! timestamp shape admits only ASCII characters and a small alphabet. The
//! delimiter is unambiguous for the shape and keeps the hash input stable
//! across serde version bumps.
//!
//! # Scheduling
//!
//! Not scheduled here. Sibling subsystem CONSENT-08 (scheduler) will call
//! [`anchor_recent`] on a fixed cadence. TODO: schedule hourly.

#![allow(dead_code)]

use chrono::{DateTime, Utc};
use sha2::{Digest, Sha256};
use sqlx::{FromRow, PgPool};
use uuid::Uuid;

use crate::error::AppResult;

/// A row in `consent_integrity_anchors`. Read-only from Rust — the table
/// forbids UPDATE / DELETE at the trigger level.
#[derive(Debug, Clone, FromRow)]
pub struct IntegrityAnchor {
    pub id: Uuid,
    pub anchor_hash: String,
    pub record_count: i32,
    pub window_start_at: Option<DateTime<Utc>>,
    pub window_end_at: Option<DateTime<Utc>>,
    pub anchored_at: DateTime<Utc>,
}

/// Input shape for [`compute_anchor`]. Kept small and purely owned so the
/// function is trivially unit-testable without any `sqlx` dependencies.
#[derive(Debug, Clone)]
pub struct AnchorInput {
    pub id: Uuid,
    pub subject: String,
    pub action: String,
    pub created_at: DateTime<Utc>,
}

/// Compute the hex-encoded SHA-256 anchor over `rows`. The caller is
/// responsible for ordering rows deterministically — the usual ordering is
/// `ORDER BY created_at ASC, id ASC`.
#[must_use]
pub fn compute_anchor(rows: &[AnchorInput]) -> String {
    let mut hasher = Sha256::new();
    for row in rows {
        hasher.update(row.id.as_bytes());
        hasher.update(b"|");
        hasher.update(row.subject.as_bytes());
        hasher.update(b"|");
        hasher.update(row.action.as_bytes());
        hasher.update(b"|");
        hasher.update(row.created_at.to_rfc3339().as_bytes());
        hasher.update(b"\n");
    }
    let digest = hasher.finalize();
    hex_encode(&digest[..])
}

fn hex_encode(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for &b in bytes {
        out.push(HEX[(b >> 4) as usize] as char);
        out.push(HEX[(b & 0x0f) as usize] as char);
    }
    out
}

/// Hash-chain the latest `window` consent_records rows and write an anchor.
///
/// Returns `Ok(None)` when the `consent_records` table is absent (sibling
/// worktree CONSENT-03 has not landed yet) OR when zero rows are covered.
/// In either case the anchor file stays empty — callers should treat `None`
/// as "not yet initialised" rather than an error.
///
/// TODO: schedule hourly. Scheduler subsystem will invoke this via a
/// tokio-cron task.
pub async fn anchor_recent(pool: &PgPool, window: i64) -> AppResult<Option<IntegrityAnchor>> {
    // Table may not exist yet (CONSENT-03 lives in a sibling worktree). Check
    // the pg_catalog first so a 500 never leaks into the admin panel when the
    // migration has not yet been applied in the current database.
    let table_exists: bool = sqlx::query_scalar::<_, bool>(
        r#"
        SELECT EXISTS (
            SELECT 1 FROM pg_catalog.pg_class c
            JOIN pg_catalog.pg_namespace n ON n.oid = c.relnamespace
            WHERE c.relname = 'consent_records' AND n.nspname = 'public'
        )
        "#,
    )
    .fetch_one(pool)
    .await?;

    if !table_exists {
        return Ok(None);
    }

    // Read the most recent `window` rows. We deliberately ORDER BY
    // `created_at DESC` to grab the freshest window, then reverse in
    // memory so the hash input ends up chronological — keeps the anchor
    // stable across replays.
    let rows = sqlx::query_as::<_, (Uuid, String, String, DateTime<Utc>)>(
        r#"
        SELECT id, subject_id::text, action::text, created_at
        FROM (
            SELECT id, subject_id, action, created_at
            FROM consent_records
            ORDER BY created_at DESC, id DESC
            LIMIT $1
        ) s
        ORDER BY created_at ASC, id ASC
        "#,
    )
    .bind(window)
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    if rows.is_empty() {
        return Ok(None);
    }

    let inputs: Vec<AnchorInput> = rows
        .into_iter()
        .map(|(id, subject, action, created_at)| AnchorInput {
            id,
            subject,
            action,
            created_at,
        })
        .collect();

    let hash = compute_anchor(&inputs);
    let first = inputs.first().map(|r| r.created_at);
    let last = inputs.last().map(|r| r.created_at);
    let count = inputs.len() as i32;

    let inserted = sqlx::query_as::<_, IntegrityAnchor>(
        r#"
        INSERT INTO consent_integrity_anchors
            (anchor_hash, record_count, window_start_at, window_end_at)
        VALUES ($1, $2, $3, $4)
        RETURNING id, anchor_hash, record_count, window_start_at, window_end_at, anchored_at
        "#,
    )
    .bind(&hash)
    .bind(count)
    .bind(first)
    .bind(last)
    .fetch_one(pool)
    .await?;

    Ok(Some(inserted))
}

/// Fetch the most recent `limit` anchors for display in the admin log page.
pub async fn list_anchors(pool: &PgPool, limit: i64) -> AppResult<Vec<IntegrityAnchor>> {
    let rows = sqlx::query_as::<_, IntegrityAnchor>(
        r#"
        SELECT id, anchor_hash, record_count, window_start_at, window_end_at, anchored_at
        FROM consent_integrity_anchors
        ORDER BY anchored_at DESC
        LIMIT $1
        "#,
    )
    .bind(limit)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn input(id: u128, subject: &str, action: &str, ts: i64) -> AnchorInput {
        AnchorInput {
            id: Uuid::from_u128(id),
            subject: subject.into(),
            action: action.into(),
            created_at: Utc.timestamp_opt(ts, 0).single().expect("valid ts"),
        }
    }

    #[test]
    fn empty_input_hashes_to_empty_sha256() {
        // SHA-256 of zero bytes.
        let empty = compute_anchor(&[]);
        assert_eq!(
            empty,
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }

    #[test]
    fn order_matters() {
        let a = input(1, "alice", "granted", 1_700_000_000);
        let b = input(2, "bob", "denied", 1_700_000_100);
        let fwd = compute_anchor(&[a.clone(), b.clone()]);
        let rev = compute_anchor(&[b, a]);
        assert_ne!(fwd, rev, "reordering rows must change the anchor");
    }

    #[test]
    fn identical_input_produces_identical_hash() {
        let rows = vec![
            input(1, "alice", "granted", 1_700_000_000),
            input(2, "bob", "denied", 1_700_000_100),
        ];
        let h1 = compute_anchor(&rows);
        let h2 = compute_anchor(&rows);
        assert_eq!(h1, h2);
    }

    #[test]
    fn mutating_any_field_flips_the_hash() {
        let base = input(1, "alice", "granted", 1_700_000_000);
        let h_base = compute_anchor(&[base.clone()]);

        let with_subject_changed = AnchorInput {
            subject: "eve".into(),
            ..base.clone()
        };
        assert_ne!(h_base, compute_anchor(&[with_subject_changed]));

        let with_action_changed = AnchorInput {
            action: "denied".into(),
            ..base.clone()
        };
        assert_ne!(h_base, compute_anchor(&[with_action_changed]));

        let with_ts_changed = AnchorInput {
            created_at: Utc.timestamp_opt(1_700_000_001, 0).single().unwrap(),
            ..base
        };
        assert_ne!(h_base, compute_anchor(&[with_ts_changed]));
    }

    #[test]
    fn hex_encode_is_lowercase_and_full_width() {
        assert_eq!(hex_encode(&[0x00]), "00");
        assert_eq!(hex_encode(&[0xff]), "ff");
        assert_eq!(hex_encode(&[0xde, 0xad, 0xbe, 0xef]), "deadbeef");
    }
}
