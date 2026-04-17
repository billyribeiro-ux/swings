//! POP-06: revenue attribution.
//!
//! `attribute_order` looks for the most recent popup submission on the
//! session within the configured window, then writes a
//! `popup_attributions` row linking that popup (and variant, if known) to
//! the order or subscription. Fails silently (no row written, `Ok(None)`)
//! when no qualifying submission exists — the caller ignores the return if
//! they do not care.

use chrono::{DateTime, Duration, Utc};
use uuid::Uuid;

use crate::error::AppResult;

/// Default attribution window. Overridable via the
/// `ATTRIBUTION_WINDOW_HOURS` env var at callsite construction time.
pub const DEFAULT_WINDOW_HOURS: i64 = 24;

/// What the caller knows about the revenue event.
#[derive(Debug, Clone, Copy)]
pub struct RevenueEvent {
    pub session_id: Uuid,
    pub order_id: Option<Uuid>,
    pub subscription_id: Option<Uuid>,
    pub amount_cents: i64,
}

/// Result of the attribution probe. `None` means "no popup submission in
/// window" — write nothing and return quietly.
#[derive(Debug, Clone, Copy)]
pub struct AttributionRow {
    pub id: Uuid,
    pub popup_id: Uuid,
    pub variant_id: Option<Uuid>,
}

/// Walk the submission history for `event.session_id` and, if a submission
/// exists within `window` prior to `now`, insert a row into
/// `popup_attributions`.
pub async fn attribute_order(
    pool: &sqlx::PgPool,
    event: RevenueEvent,
    currency: &str,
    window: Duration,
    now: DateTime<Utc>,
) -> AppResult<Option<AttributionRow>> {
    let cutoff = now - window;
    // Most recent submission on this session in-window wins. variant_id
    // may be NULL on legacy rows that predate POP-02; SET NULL on delete
    // covers the reverse — variant removed after attribution.
    let row: Option<(Uuid, Option<Uuid>)> = sqlx::query_as(
        r#"
        SELECT popup_id, variant_id
        FROM popup_submissions
        WHERE session_id = $1
          AND submitted_at >= $2
          AND submitted_at <= $3
        ORDER BY submitted_at DESC
        LIMIT 1
        "#,
    )
    .bind(event.session_id)
    .bind(cutoff)
    .bind(now)
    .fetch_optional(pool)
    .await?;

    let (popup_id, variant_id) = match row {
        Some(r) => r,
        None => return Ok(None),
    };

    let id: Uuid = sqlx::query_scalar(
        r#"
        INSERT INTO popup_attributions
            (popup_id, variant_id, session_id, order_id, subscription_id,
             amount_cents, currency, attributed_at)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        RETURNING id
        "#,
    )
    .bind(popup_id)
    .bind(variant_id)
    .bind(event.session_id)
    .bind(event.order_id)
    .bind(event.subscription_id)
    .bind(event.amount_cents)
    .bind(currency)
    .bind(now)
    .fetch_one(pool)
    .await?;

    Ok(Some(AttributionRow {
        id,
        popup_id,
        variant_id,
    }))
}

/// Pure predicate: is `submitted_at` within `[now - window, now]`? Exposed
/// as its own function so unit tests can exercise the window math without
/// a database.
#[must_use]
pub fn is_in_window(
    submitted_at: DateTime<Utc>,
    now: DateTime<Utc>,
    window: Duration,
) -> bool {
    let cutoff = now - window;
    submitted_at >= cutoff && submitted_at <= now
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn now() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 4, 17, 12, 0, 0).unwrap()
    }

    #[test]
    fn submission_inside_window_matches() {
        let w = Duration::hours(24);
        let at = now() - Duration::hours(3);
        assert!(is_in_window(at, now(), w));
    }

    #[test]
    fn submission_outside_window_rejected() {
        let w = Duration::hours(24);
        let at = now() - Duration::hours(25);
        assert!(!is_in_window(at, now(), w));
    }

    #[test]
    fn submission_in_the_future_rejected() {
        let w = Duration::hours(24);
        let at = now() + Duration::hours(1);
        assert!(!is_in_window(at, now(), w));
    }

    #[test]
    fn exactly_at_cutoff_matches() {
        let w = Duration::hours(1);
        let at = now() - w;
        assert!(is_in_window(at, now(), w));
    }
}
