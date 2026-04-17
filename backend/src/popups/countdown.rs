//! POP-04: countdown popup helpers.
//!
//! Two modes:
//!
//! * [`CountdownMode::FixedEnd`] — every visitor sees the same absolute end
//!   timestamp. Useful for genuine one-time sales.
//! * [`CountdownMode::RollingPerVisitor`] — each visitor gets a fresh
//!   duration starting at their first impression. Authentic urgency; the
//!   start timestamp is persisted in `popup_visitor_state` (see POP-05).
//!
//! This module is pure: it does not touch the database. Handlers load the
//! visitor's `first_seen_at` separately and pass it to [`time_remaining`].

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};

/// Configured countdown behaviour.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "mode", rename_all = "snake_case")]
pub enum CountdownMode {
    FixedEnd { at: DateTime<Utc> },
    RollingPerVisitor { duration_secs: u32 },
}

/// Result of a countdown evaluation. `None` means the countdown has elapsed
/// and the popup should stop being shown.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TimeRemaining {
    pub seconds: i64,
}

/// Compute the remaining time for a countdown.
///
/// * For `FixedEnd`, remaining = `at - now`.
/// * For `RollingPerVisitor`, remaining = `(first_seen_at + duration) - now`.
///   If `first_seen_at` is `None` (first impression), remaining is the full
///   duration.
///
/// A negative result is returned as `Some(TimeRemaining { seconds: <neg> })`
/// so the caller can distinguish "expired" from "not yet started" — but
/// [`is_expired`] is the canonical test.
#[must_use]
pub fn time_remaining(
    mode: &CountdownMode,
    first_seen_at: Option<DateTime<Utc>>,
    now: DateTime<Utc>,
) -> TimeRemaining {
    let end = match mode {
        CountdownMode::FixedEnd { at } => *at,
        CountdownMode::RollingPerVisitor { duration_secs } => {
            let start = first_seen_at.unwrap_or(now);
            start + Duration::seconds(*duration_secs as i64)
        }
    };
    TimeRemaining {
        seconds: (end - now).num_seconds(),
    }
}

#[must_use]
pub fn is_expired(remaining: TimeRemaining) -> bool {
    remaining.seconds <= 0
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn t(h: u32, m: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 4, 17, h, m, 0).unwrap()
    }

    #[test]
    fn fixed_end_counts_down() {
        let mode = CountdownMode::FixedEnd { at: t(17, 0) };
        let rem = time_remaining(&mode, None, t(16, 30));
        assert_eq!(rem.seconds, 30 * 60);
        assert!(!is_expired(rem));
    }

    #[test]
    fn fixed_end_expired() {
        let mode = CountdownMode::FixedEnd { at: t(15, 0) };
        let rem = time_remaining(&mode, None, t(16, 0));
        assert!(is_expired(rem));
        assert!(rem.seconds < 0);
    }

    #[test]
    fn rolling_first_impression_uses_full_duration() {
        let mode = CountdownMode::RollingPerVisitor { duration_secs: 600 };
        let rem = time_remaining(&mode, None, t(10, 0));
        assert_eq!(rem.seconds, 600);
    }

    #[test]
    fn rolling_partial_elapsed() {
        let mode = CountdownMode::RollingPerVisitor { duration_secs: 600 };
        let first = t(10, 0);
        let rem = time_remaining(&mode, Some(first), t(10, 5));
        assert_eq!(rem.seconds, 300);
    }

    #[test]
    fn rolling_expired() {
        let mode = CountdownMode::RollingPerVisitor { duration_secs: 60 };
        let first = t(10, 0);
        let rem = time_remaining(&mode, Some(first), t(10, 5));
        assert!(is_expired(rem));
    }
}
