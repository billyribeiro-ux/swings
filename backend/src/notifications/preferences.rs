//! Per-user notification preferences.
//!
//! A preference row answers "is the user opted in to receive `<category>` on
//! `<channel>` right now?" — the answer depends on the `enabled` flag, the
//! quiet-hours window (if any), and the stored timezone.
//!
//! # Quiet-hours semantics
//!
//! `quiet_hours_start` + `quiet_hours_end` are local-time ranges in
//! `timezone`. A send attempt during the window is considered *not allowed*.
//! Wrap-around windows (e.g. 22:00-07:00) are supported: when `start > end`
//! the allowed window is the complement of the range.
//!
//! # Timezone support
//!
//! For FDN-05 we accept [`chrono::FixedOffset`]-parseable strings (`"UTC"`,
//! `"+00:00"`, `"-05:00"`, `"+09:30"`) and fall back to UTC otherwise.
//! Full IANA zone support (e.g. `"America/New_York"` with DST) arrives with
//! the `chrono-tz` upgrade in a future subsystem — explicitly out of scope
//! for FDN-05 which is locked to "no new runtime crates".

use chrono::{DateTime, FixedOffset, NaiveTime, TimeZone, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, PgExecutor};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::error::AppError;

/// A (user, category, channel) preference row as stored in
/// `notification_preferences`.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct NotificationPreference {
    pub user_id: Uuid,
    pub category: String,
    pub channel: String,
    pub enabled: bool,
    pub quiet_hours_start: Option<NaiveTime>,
    pub quiet_hours_end: Option<NaiveTime>,
    pub timezone: String,
    pub updated_at: DateTime<Utc>,
}

/// Input shape for the bulk set endpoint. Missing timezone defaults to UTC.
#[derive(Debug, Deserialize, Serialize, ToSchema)]
pub struct PreferenceUpdate {
    pub category: String,
    pub channel: String,
    pub enabled: bool,
    pub quiet_hours_start: Option<NaiveTime>,
    pub quiet_hours_end: Option<NaiveTime>,
    pub timezone: Option<String>,
}

/// Load every preference row for `user_id`.
pub async fn get_preferences<'e, E: PgExecutor<'e>>(
    executor: E,
    user_id: Uuid,
) -> Result<Vec<NotificationPreference>, AppError> {
    let rows = sqlx::query_as::<_, NotificationPreference>(
        r#"
        SELECT user_id, category, channel, enabled, quiet_hours_start,
               quiet_hours_end, timezone, updated_at
        FROM notification_preferences
        WHERE user_id = $1
        ORDER BY category, channel
        "#,
    )
    .bind(user_id)
    .fetch_all(executor)
    .await?;
    Ok(rows)
}

/// Look up a single (user, category, channel) preference. Absence is not an
/// error — treat as "default: enabled" on the caller side.
pub async fn get_preference<'e, E: PgExecutor<'e>>(
    executor: E,
    user_id: Uuid,
    category: &str,
    channel: &str,
) -> Result<Option<NotificationPreference>, AppError> {
    let row = sqlx::query_as::<_, NotificationPreference>(
        r#"
        SELECT user_id, category, channel, enabled, quiet_hours_start,
               quiet_hours_end, timezone, updated_at
        FROM notification_preferences
        WHERE user_id = $1 AND category = $2 AND channel = $3
        "#,
    )
    .bind(user_id)
    .bind(category)
    .bind(channel)
    .fetch_optional(executor)
    .await?;
    Ok(row)
}

/// Upsert a preference. Timezone falls back to `"UTC"` when omitted; the
/// database enforces validity at the application layer (we accept any IANA
/// string and validate on evaluation).
pub async fn set_preference<'e, E: PgExecutor<'e>>(
    executor: E,
    user_id: Uuid,
    update: &PreferenceUpdate,
) -> Result<NotificationPreference, AppError> {
    let tz = update.timezone.as_deref().unwrap_or("UTC");
    let row = sqlx::query_as::<_, NotificationPreference>(
        r#"
        INSERT INTO notification_preferences
            (user_id, category, channel, enabled, quiet_hours_start,
             quiet_hours_end, timezone, updated_at)
        VALUES ($1, $2, $3, $4, $5, $6, $7, NOW())
        ON CONFLICT (user_id, category, channel) DO UPDATE SET
            enabled            = EXCLUDED.enabled,
            quiet_hours_start  = EXCLUDED.quiet_hours_start,
            quiet_hours_end    = EXCLUDED.quiet_hours_end,
            timezone           = EXCLUDED.timezone,
            updated_at         = NOW()
        RETURNING user_id, category, channel, enabled, quiet_hours_start,
                  quiet_hours_end, timezone, updated_at
        "#,
    )
    .bind(user_id)
    .bind(&update.category)
    .bind(&update.channel)
    .bind(update.enabled)
    .bind(update.quiet_hours_start)
    .bind(update.quiet_hours_end)
    .bind(tz)
    .fetch_one(executor)
    .await?;
    Ok(row)
}

/// Parse a stored timezone string into a [`FixedOffset`]. Accepts `"UTC"`
/// (case-insensitive) plus `"±HH:MM"` / `"±HHMM"` / `"±HH"` forms. Unknown
/// labels fall back to UTC — full IANA support lands with a later subsystem.
fn parse_offset(tz: &str) -> FixedOffset {
    let trimmed = tz.trim();
    if trimmed.is_empty() || trimmed.eq_ignore_ascii_case("UTC") || trimmed == "Z" {
        return FixedOffset::east_opt(0).expect("utc offset");
    }
    // Try ISO-8601 "+HH:MM" / "-HH:MM" / "+HHMM" / "±HH".
    if let Some(offset) = parse_iso_offset(trimmed) {
        return offset;
    }
    FixedOffset::east_opt(0).expect("utc offset fallback")
}

fn parse_iso_offset(s: &str) -> Option<FixedOffset> {
    let bytes = s.as_bytes();
    let (sign, rest) = match bytes.first()? {
        b'+' => (1i32, &s[1..]),
        b'-' => (-1i32, &s[1..]),
        _ => return None,
    };
    let (hh, mm) = match rest.len() {
        2 => (rest.parse::<i32>().ok()?, 0),
        4 => (
            rest[..2].parse::<i32>().ok()?,
            rest[2..].parse::<i32>().ok()?,
        ),
        5 if rest.as_bytes().get(2) == Some(&b':') => (
            rest[..2].parse::<i32>().ok()?,
            rest[3..].parse::<i32>().ok()?,
        ),
        _ => return None,
    };
    if !(0..=23).contains(&hh) || !(0..=59).contains(&mm) {
        return None;
    }
    FixedOffset::east_opt(sign * (hh * 3600 + mm * 60))
}

/// Evaluate whether a send is currently allowed for a loaded preference.
///
/// If the preference has `enabled=false`, returns `false` unconditionally. If
/// quiet-hours are unset, returns `enabled`. Otherwise compares the current
/// time (in the preference's timezone) to the window.
#[must_use]
pub fn is_allowed_for_preference(pref: &NotificationPreference, now_utc: DateTime<Utc>) -> bool {
    if !pref.enabled {
        return false;
    }
    let (Some(start), Some(end)) = (pref.quiet_hours_start, pref.quiet_hours_end) else {
        return true;
    };
    let offset = parse_offset(&pref.timezone);
    let local = offset.from_utc_datetime(&now_utc.naive_utc()).time();
    let in_window = if start <= end {
        local >= start && local < end
    } else {
        // Wrap-around: e.g. start=22:00, end=07:00 — window spans midnight.
        local >= start || local < end
    };
    !in_window
}

/// Short-circuit helper — loads the preference row (if any) and evaluates.
/// When no row exists the default is "allowed" (transactional defaults open,
/// per the audit plan §2 FDN-05).
pub async fn is_allowed<'e, E: PgExecutor<'e>>(
    executor: E,
    user_id: Uuid,
    category: &str,
    channel: &str,
    now_utc: DateTime<Utc>,
) -> Result<bool, AppError> {
    let Some(pref) = get_preference(executor, user_id, category, channel).await? else {
        return Ok(true);
    };
    Ok(is_allowed_for_preference(&pref, now_utc))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};

    fn pref(
        enabled: bool,
        qh_start: Option<&str>,
        qh_end: Option<&str>,
        tz: &str,
    ) -> NotificationPreference {
        NotificationPreference {
            user_id: Uuid::nil(),
            category: "transactional".into(),
            channel: "email".into(),
            enabled,
            quiet_hours_start: qh_start.map(|s| s.parse::<NaiveTime>().expect("valid time")),
            quiet_hours_end: qh_end.map(|s| s.parse::<NaiveTime>().expect("valid time")),
            timezone: tz.into(),
            updated_at: Utc::now(),
        }
    }

    #[test]
    fn disabled_blocks_always() {
        let p = pref(false, None, None, "UTC");
        let now = Utc.with_ymd_and_hms(2025, 1, 1, 12, 0, 0).unwrap();
        assert!(!is_allowed_for_preference(&p, now));
    }

    #[test]
    fn enabled_without_quiet_hours_always_allows() {
        let p = pref(true, None, None, "UTC");
        let now = Utc.with_ymd_and_hms(2025, 1, 1, 3, 0, 0).unwrap();
        assert!(is_allowed_for_preference(&p, now));
    }

    #[test]
    fn simple_quiet_hours_block_inside_window() {
        let p = pref(true, Some("09:00:00"), Some("17:00:00"), "UTC");
        let now = Utc.with_ymd_and_hms(2025, 1, 1, 12, 0, 0).unwrap();
        assert!(!is_allowed_for_preference(&p, now));
    }

    #[test]
    fn simple_quiet_hours_allow_outside_window() {
        let p = pref(true, Some("09:00:00"), Some("17:00:00"), "UTC");
        let now = Utc.with_ymd_and_hms(2025, 1, 1, 20, 0, 0).unwrap();
        assert!(is_allowed_for_preference(&p, now));
    }

    #[test]
    fn quiet_hours_end_exclusive() {
        let p = pref(true, Some("09:00:00"), Some("17:00:00"), "UTC");
        let now = Utc.with_ymd_and_hms(2025, 1, 1, 17, 0, 0).unwrap();
        assert!(is_allowed_for_preference(&p, now));
    }

    #[test]
    fn wrap_around_quiet_hours_block_past_midnight() {
        let p = pref(true, Some("22:00:00"), Some("07:00:00"), "UTC");
        let now = Utc.with_ymd_and_hms(2025, 1, 1, 3, 0, 0).unwrap();
        assert!(!is_allowed_for_preference(&p, now));
    }

    #[test]
    fn wrap_around_quiet_hours_block_before_midnight() {
        let p = pref(true, Some("22:00:00"), Some("07:00:00"), "UTC");
        let now = Utc.with_ymd_and_hms(2025, 1, 1, 23, 0, 0).unwrap();
        assert!(!is_allowed_for_preference(&p, now));
    }

    #[test]
    fn wrap_around_quiet_hours_allow_daytime() {
        let p = pref(true, Some("22:00:00"), Some("07:00:00"), "UTC");
        let now = Utc.with_ymd_and_hms(2025, 1, 1, 10, 0, 0).unwrap();
        assert!(is_allowed_for_preference(&p, now));
    }

    #[test]
    fn fixed_offset_timezone_shifts_window() {
        // 09:00-17:00 in -05:00 (e.g. EST, no DST). 16:00 UTC = 11:00 local →
        // in window → blocked.
        let p = pref(true, Some("09:00:00"), Some("17:00:00"), "-05:00");
        let now = Utc.with_ymd_and_hms(2025, 1, 1, 16, 0, 0).unwrap();
        assert!(!is_allowed_for_preference(&p, now));
        // 03:00 UTC = 22:00 local (previous calendar day) → outside window → allowed.
        let late = Utc.with_ymd_and_hms(2025, 1, 1, 3, 0, 0).unwrap();
        assert!(is_allowed_for_preference(&p, late));
    }

    #[test]
    fn malformed_timezone_falls_back_to_utc() {
        let p = pref(true, Some("09:00:00"), Some("17:00:00"), "Not/A/Zone");
        let now = Utc.with_ymd_and_hms(2025, 1, 1, 12, 0, 0).unwrap();
        assert!(!is_allowed_for_preference(&p, now));
    }

    #[test]
    fn parse_offset_accepts_utc_label() {
        assert_eq!(parse_offset("UTC"), FixedOffset::east_opt(0).unwrap());
        assert_eq!(parse_offset("utc"), FixedOffset::east_opt(0).unwrap());
        assert_eq!(parse_offset("Z"), FixedOffset::east_opt(0).unwrap());
        assert_eq!(parse_offset(""), FixedOffset::east_opt(0).unwrap());
    }

    #[test]
    fn parse_offset_parses_signed_iso_forms() {
        assert_eq!(
            parse_offset("+05:00"),
            FixedOffset::east_opt(5 * 3600).unwrap()
        );
        assert_eq!(
            parse_offset("-05:00"),
            FixedOffset::west_opt(5 * 3600).unwrap()
        );
        assert_eq!(
            parse_offset("+0930"),
            FixedOffset::east_opt(9 * 3600 + 30 * 60).unwrap()
        );
        assert_eq!(
            parse_offset("+05"),
            FixedOffset::east_opt(5 * 3600).unwrap()
        );
    }

    #[test]
    fn parse_offset_rejects_garbage() {
        assert_eq!(parse_offset("gibberish"), FixedOffset::east_opt(0).unwrap());
        assert_eq!(parse_offset("+99:99"), FixedOffset::east_opt(0).unwrap());
    }
}
