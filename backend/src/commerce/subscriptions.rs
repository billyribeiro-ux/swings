//! EC-09: Subscriptions 2.0 — pause/resume + prorations + dunning + switching.
//!
//! All business logic is pure where possible:
//!
//!   * [`prorate`] computes the prorated amount when a subscription
//!     switches mid-cycle. The math is `unused_seconds / period_seconds *
//!     old_price - unused_seconds / period_seconds * new_price`. Negative
//!     deltas are credits; positive deltas are charges.
//!   * [`next_dunning_attempt`] returns the schedule (`{+1d, +3d, +7d,
//!     +14d}`) for the n-th retry.
//!   * The repo helpers ([`pause`], [`resume`], [`switch_plan`],
//!     [`record_change`], [`schedule_dunning`]) wrap the SQL — no
//!     side-effects beyond the supplied PgPool.
//!
//! The actual Stripe orchestration (cancel + recreate on switch, immediate
//! invoice, etc.) lives in the handler layer; this module is testable
//! without any external service.

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, PgPool};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::error::AppResult;

#[derive(Debug, thiserror::Error)]
pub enum SubscriptionError {
    #[error("subscription is already paused")]
    AlreadyPaused,
    #[error("subscription is not paused")]
    NotPaused,
    #[error("invalid pause window — pause_resumes_at must be in the future")]
    InvalidPauseWindow,
    #[error("invalid plan switch")]
    InvalidSwitch,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct SubscriptionChange {
    pub id: Uuid,
    pub subscription_id: Uuid,
    pub kind: String,
    pub from_plan_id: Option<Uuid>,
    pub to_plan_id: Option<Uuid>,
    pub proration_cents: i64,
    pub actor_id: Option<Uuid>,
    pub notes: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct DunningAttempt {
    pub subscription_id: Uuid,
    pub attempt: i32,
    pub scheduled_at: DateTime<Utc>,
    pub executed_at: Option<DateTime<Utc>>,
    pub result: Option<String>,
}

// ── Pure math ──────────────────────────────────────────────────────────

/// Compute the prorated charge / credit when switching from `old_price` to
/// `new_price` partway through a billing period.
///
/// Returns positive for amounts the customer owes (upgrades), negative for
/// amounts owed back (downgrades). Caller decides whether to apply as an
/// invoice line or a customer balance.
pub fn prorate(
    period_start: DateTime<Utc>,
    period_end: DateTime<Utc>,
    now: DateTime<Utc>,
    old_price_cents: i64,
    new_price_cents: i64,
) -> i64 {
    if now < period_start || now >= period_end || period_end <= period_start {
        return 0;
    }
    let period = (period_end - period_start).num_seconds().max(1);
    let unused = (period_end - now).num_seconds().max(0);
    let credit = (old_price_cents as i128 * unused as i128) / period as i128;
    let charge = (new_price_cents as i128 * unused as i128) / period as i128;
    (charge - credit) as i64
}

/// Dunning schedule: 1st retry +1d, 2nd +3d, 3rd +7d, 4th +14d. After the
/// 4th the worker MUST stop and emit `subscription.canceled` so the Stripe
/// subscription is canceled and the customer notified.
pub fn next_dunning_attempt(attempt: i32, base: DateTime<Utc>) -> Option<DateTime<Utc>> {
    let days = match attempt {
        1 => 1,
        2 => 3,
        3 => 7,
        4 => 14,
        _ => return None,
    };
    Some(base + Duration::days(days))
}

// ── Repository ─────────────────────────────────────────────────────────

#[allow(clippy::too_many_arguments)]
pub async fn record_change(
    pool: &PgPool,
    subscription_id: Uuid,
    kind: &str,
    from_plan_id: Option<Uuid>,
    to_plan_id: Option<Uuid>,
    proration_cents: i64,
    actor_id: Option<Uuid>,
    notes: Option<&str>,
) -> AppResult<SubscriptionChange> {
    let row = sqlx::query_as::<_, SubscriptionChange>(
        r#"
        INSERT INTO subscription_changes
            (subscription_id, kind, from_plan_id, to_plan_id,
             proration_cents, actor_id, notes)
        VALUES ($1, $2, $3, $4, $5, $6, $7)
        RETURNING id, subscription_id, kind, from_plan_id, to_plan_id,
                  proration_cents, actor_id, notes, created_at
        "#,
    )
    .bind(subscription_id)
    .bind(kind)
    .bind(from_plan_id)
    .bind(to_plan_id)
    .bind(proration_cents)
    .bind(actor_id)
    .bind(notes)
    .fetch_one(pool)
    .await?;
    Ok(row)
}

pub async fn pause(
    pool: &PgPool,
    subscription_id: Uuid,
    resumes_at: Option<DateTime<Utc>>,
) -> AppResult<()> {
    sqlx::query(
        r#"
        UPDATE subscriptions
           SET paused_at = NOW(),
               pause_resumes_at = $2,
               status = 'paused',
               updated_at = NOW()
         WHERE id = $1
        "#,
    )
    .bind(subscription_id)
    .bind(resumes_at)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn resume(pool: &PgPool, subscription_id: Uuid) -> AppResult<()> {
    sqlx::query(
        r#"
        UPDATE subscriptions
           SET paused_at = NULL,
               pause_resumes_at = NULL,
               status = 'active',
               updated_at = NOW()
         WHERE id = $1
        "#,
    )
    .bind(subscription_id)
    .execute(pool)
    .await?;
    Ok(())
}

/// Schedule the n-th dunning attempt. Returns the row inserted, or None
/// when the schedule is exhausted (caller should cancel the subscription).
pub async fn schedule_dunning(
    pool: &PgPool,
    subscription_id: Uuid,
    attempt: i32,
    base: DateTime<Utc>,
) -> AppResult<Option<DunningAttempt>> {
    let Some(scheduled_at) = next_dunning_attempt(attempt, base) else {
        return Ok(None);
    };
    let row = sqlx::query_as::<_, DunningAttempt>(
        r#"
        INSERT INTO dunning_attempts (subscription_id, attempt, scheduled_at)
        VALUES ($1, $2, $3)
        ON CONFLICT (subscription_id, attempt) DO UPDATE
           SET scheduled_at = EXCLUDED.scheduled_at
        RETURNING subscription_id, attempt, scheduled_at, executed_at, result
        "#,
    )
    .bind(subscription_id)
    .bind(attempt)
    .bind(scheduled_at)
    .fetch_one(pool)
    .await?;
    Ok(Some(row))
}

pub async fn record_dunning_result(
    pool: &PgPool,
    subscription_id: Uuid,
    attempt: i32,
    result: &str,
) -> AppResult<()> {
    sqlx::query(
        r#"
        UPDATE dunning_attempts
           SET executed_at = NOW(), result = $3
         WHERE subscription_id = $1 AND attempt = $2
        "#,
    )
    .bind(subscription_id)
    .bind(attempt)
    .bind(result)
    .execute(pool)
    .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn t(y: i32, m: u32, d: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(y, m, d, 0, 0, 0).single().unwrap()
    }

    #[test]
    fn prorate_charges_upgrade_in_middle_of_period() {
        // 30-day period, 15 days remaining, old=$10, new=$30.
        let p = prorate(t(2026, 4, 1), t(2026, 5, 1), t(2026, 4, 16), 1_000, 3_000);
        // Half of (3000 - 1000) = 1000.
        assert!(p > 950 && p < 1_050, "got {p}");
    }

    #[test]
    fn prorate_credits_downgrade() {
        let p = prorate(t(2026, 4, 1), t(2026, 5, 1), t(2026, 4, 16), 3_000, 1_000);
        assert!(p < -950 && p > -1_050, "got {p}");
    }

    #[test]
    fn prorate_returns_zero_when_now_outside_period() {
        let p = prorate(t(2026, 4, 1), t(2026, 5, 1), t(2026, 6, 1), 1_000, 3_000);
        assert_eq!(p, 0);
    }

    #[test]
    fn dunning_schedule_runs_four_attempts() {
        let base = t(2026, 4, 1);
        assert_eq!(
            next_dunning_attempt(1, base),
            Some(t(2026, 4, 1) + Duration::days(1))
        );
        assert_eq!(
            next_dunning_attempt(4, base),
            Some(t(2026, 4, 1) + Duration::days(14))
        );
        assert_eq!(next_dunning_attempt(5, base), None);
        assert_eq!(next_dunning_attempt(0, base), None);
    }
}
