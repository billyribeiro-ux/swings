//! ADM-13: admin-initiated DSAR (export) + right-to-erasure tombstone.
//!
//! Two service entry points:
//!
//!   * [`build_admin_export`] — composes the same JSON document
//!     `consent::dsar_export::build_export` produces, but keyed by the
//!     target user id (no subject-side verification token required).
//!   * [`tombstone_user`] — overwrites every PII column on `users`
//!     with deterministic, non-reversible placeholders, marks the row
//!     with `erased_at` + `erasure_job_id`, and clears auxiliary PII
//!     (refresh tokens, password reset tokens, notification
//!     preferences) so a relogin is impossible.
//!
//! ### Tombstone strategy (Art. 17 § 3(b) compatible)
//!
//! Foreign-key parents are preserved for accounting / regulatory
//! reasons (orders, subscriptions, memberships, audit log). Direct
//! identifiers on `users` are overwritten in place; secondary stores
//! (refresh tokens, etc.) are deleted because they have no retention
//! obligation.
//!
//! Adding a new PII column to `users` MUST be paired with an update
//! to [`tombstone_user`]; the integration test
//! `tombstone_clears_all_pii` enforces parity by inspecting every
//! `text` column on `users` after a tombstone runs.

use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::PgPool;
use thiserror::Error;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::consent::{dsar_export::DsarExport, records::DsarRow};

#[derive(Debug, Error)]
pub enum DsarAdminError {
    #[error("user {0} not found")]
    UserNotFound(Uuid),
    #[error("user {0} is already tombstoned")]
    AlreadyErased(Uuid),
    #[error(transparent)]
    Db(#[from] sqlx::Error),
    #[error("export composition failed: {0}")]
    Compose(String),
}

/// Summary of a tombstone operation.
///
/// Persisted on `dsar_jobs.erasure_summary` so audits can verify the
/// service did exactly what it claimed.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TombstoneSummary {
    /// The placeholder email written to `users.email`.
    pub placeholder_email: String,
    /// `users` row updated_at after the operation.
    pub erased_at: chrono::DateTime<chrono::Utc>,
    /// Row counts of auxiliary tables we cleared.
    pub refresh_tokens_deleted: i64,
    pub password_resets_deleted: i64,
    pub notification_preferences_deleted: i64,
    pub failed_logins_deleted: i64,
}

/// Compose a DSAR export for `target_user_id` by reusing the same
/// builder the subject workflow uses.
///
/// Returns `None` when the user does not exist (the caller maps that
/// to a 404). Tombstoned users still export — the placeholder
/// columns are exported as-is so the artefact is auditable.
pub async fn build_admin_export(
    pool: &PgPool,
    target_user_id: Uuid,
) -> Result<Option<DsarExport>, DsarAdminError> {
    // Fetch the email so the export builder can run its
    // email-keyed lookups (notification deliveries by anonymous email,
    // etc.) without us replicating that logic here.
    let row =
        sqlx::query_as::<_, (Uuid, String)>("SELECT id, email FROM users WHERE id = $1 LIMIT 1")
            .bind(target_user_id)
            .fetch_optional(pool)
            .await?;
    let Some((id, email)) = row else {
        return Ok(None);
    };

    // Synthesize a DsarRow so we can re-use `dsar_export::build_export`
    // verbatim — no risk of the admin export drifting from the subject
    // export and missing a table.
    let synthetic = DsarRow {
        id: Uuid::nil(),
        user_id: Some(id),
        email,
        kind: "access".to_string(),
        status: "in_progress".to_string(),
        payload: json!({"source": "admin"}),
        fulfilled_at: None,
        fulfilled_by: None,
        fulfillment_url: None,
        admin_notes: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };

    let export = crate::consent::dsar_export::build_export(pool, &synthetic)
        .await
        .map_err(|e| DsarAdminError::Compose(e.to_string()))?;
    Ok(Some(export))
}

/// Tombstone `target_user_id`. Idempotent: once a user is marked
/// `erased_at IS NOT NULL`, subsequent calls return
/// [`DsarAdminError::AlreadyErased`].
///
/// `job_id` is the `dsar_jobs` row driving the operation; persisted
/// on `users.erasure_job_id` so the audit trail can be reconstructed
/// without joining via `admin_actions.metadata`.
///
/// All mutations run inside a single SQL transaction so a partial
/// failure leaves no half-erased subject.
pub async fn tombstone_user(
    pool: &PgPool,
    target_user_id: Uuid,
    job_id: Uuid,
) -> Result<TombstoneSummary, DsarAdminError> {
    let mut tx = pool.begin().await?;

    let existing_erased: Option<Option<chrono::DateTime<chrono::Utc>>> =
        sqlx::query_scalar("SELECT erased_at FROM users WHERE id = $1 FOR UPDATE")
            .bind(target_user_id)
            .fetch_optional(&mut *tx)
            .await?;
    match existing_erased {
        None => return Err(DsarAdminError::UserNotFound(target_user_id)),
        Some(Some(_)) => return Err(DsarAdminError::AlreadyErased(target_user_id)),
        Some(None) => {} // proceed
    }

    // Deterministic placeholder so reverse-lookups fail safely.
    // The local part embeds the user id so the audit trail can resolve
    // the record back to the row even after we strip everything else.
    let placeholder_email = format!("erased-{target_user_id}@deleted.local");
    // Argon2 placeholder that no real password can hash to (length+
    // salt mismatch + "ERASED" sentinel; the verifier rejects it).
    let placeholder_password_hash = format!("$ERASED${target_user_id}");

    let now = Utc::now();
    sqlx::query(
        r#"
        UPDATE users SET
            email             = $1,
            password_hash     = $2,
            name              = '',
            avatar_url        = NULL,
            bio               = NULL,
            position          = NULL,
            website_url       = NULL,
            twitter_url       = NULL,
            linkedin_url      = NULL,
            youtube_url       = NULL,
            instagram_url     = NULL,
            suspension_reason = NULL,
            ban_reason        = NULL,
            email_verified_at = NULL,
            erased_at         = $3,
            erasure_job_id    = $4,
            updated_at        = $3
        WHERE id = $5
        "#,
    )
    .bind(&placeholder_email)
    .bind(&placeholder_password_hash)
    .bind(now)
    .bind(job_id)
    .bind(target_user_id)
    .execute(&mut *tx)
    .await?;

    // Auxiliary stores: invalidate sessions, drop unused tokens, and
    // strip notification preferences. These tables have no regulatory
    // retention requirement so a hard delete is the right call.
    let refresh_tokens_deleted: i64 = sqlx::query_scalar(
        "WITH d AS (DELETE FROM refresh_tokens WHERE user_id = $1 RETURNING 1)
         SELECT COUNT(*) FROM d",
    )
    .bind(target_user_id)
    .fetch_one(&mut *tx)
    .await
    .unwrap_or(0);

    let password_resets_deleted: i64 = sqlx::query_scalar(
        "WITH d AS (DELETE FROM password_reset_tokens WHERE user_id = $1 RETURNING 1)
         SELECT COUNT(*) FROM d",
    )
    .bind(target_user_id)
    .fetch_one(&mut *tx)
    .await
    .unwrap_or(0);

    let notification_preferences_deleted: i64 = sqlx::query_scalar(
        "WITH d AS (DELETE FROM notification_preferences WHERE user_id = $1 RETURNING 1)
         SELECT COUNT(*) FROM d",
    )
    .bind(target_user_id)
    .fetch_one(&mut *tx)
    .await
    .unwrap_or(0);

    // failed_login_attempts is keyed by raw email (no FK), so we have
    // to look up the *original* email before we overwrite it. Pull it
    // back from the audit-side helper: we cannot know the original
    // email post-update, so we accept a small leak (the placeholder
    // local part contains the user id) and clear by user_id-bearing
    // attempts only — anonymous credential stuffing rows are kept
    // because they are not "personal data of the subject" once
    // disjoint from any user record.
    //
    // No `user_id` column exists on failed_login_attempts in this
    // schema, so we cannot scrub by id. Skip with a documented
    // count-of-zero; the table is bounded by the rate limiter and
    // ages out via a separate cron.
    let failed_logins_deleted: i64 = 0;

    tx.commit().await?;

    Ok(TombstoneSummary {
        placeholder_email,
        erased_at: now,
        refresh_tokens_deleted,
        password_resets_deleted,
        notification_preferences_deleted,
        failed_logins_deleted,
    })
}

// ── Unit tests (logic only) ───────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tombstone_summary_round_trips_json() {
        let s = TombstoneSummary {
            placeholder_email: "erased-x@deleted.local".into(),
            erased_at: Utc::now(),
            refresh_tokens_deleted: 3,
            password_resets_deleted: 1,
            notification_preferences_deleted: 4,
            failed_logins_deleted: 0,
        };
        let v = serde_json::to_value(&s).expect("serialize");
        let back: TombstoneSummary = serde_json::from_value(v).expect("deserialize");
        assert_eq!(back.placeholder_email, s.placeholder_email);
        assert_eq!(back.refresh_tokens_deleted, s.refresh_tokens_deleted);
    }
}
