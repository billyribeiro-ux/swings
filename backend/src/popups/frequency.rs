// Doc lists in this module use a layout clippy classifies as
// "overindented"; rewriting to satisfy the lint hurts readability.
#![allow(clippy::doc_overindented_list_items)]

//! POP-05: frequency capping.
//!
//! Given a popup's `frequency_config` and a visitor's current state, decide
//! whether to show the popup on this request. The config is a JSONB object
//! with the following (all optional) keys:
//!
//! * `every_n_days`       — u32. Do not re-show within this window.
//! * `max_dismissals`     — u32. Give up after this many ignores.
//! * `until_converted`    — bool. Once true, stop forever after a submit.
//! * `once_per_session`   — bool. Honored client-side via a session cookie
//!                          (server-side we treat it as "at least one shown"
//!                          when `session_impressions > 0`).
//! * `once_per_visitor`   — bool. Equivalent to `max_dismissals=1` but also
//!                          caps successful shows.
//!
//! Missing keys default to "no cap on this dimension".

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};

use crate::error::AppError;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FrequencyConfig {
    #[serde(default)]
    pub every_n_days: Option<u32>,
    #[serde(default)]
    pub max_dismissals: Option<u32>,
    #[serde(default)]
    pub until_converted: Option<bool>,
    #[serde(default)]
    pub once_per_session: Option<bool>,
    #[serde(default)]
    pub once_per_visitor: Option<bool>,
}

impl FrequencyConfig {
    pub fn from_json(value: &serde_json::Value) -> Result<Self, AppError> {
        serde_json::from_value(value.clone())
            .map_err(|e| AppError::Validation(format!("frequency_config: {e}")))
    }
}

/// Row shape for `popup_visitor_state`.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct VisitorState {
    pub anonymous_id: uuid::Uuid,
    pub popup_id: uuid::Uuid,
    pub first_shown_at: Option<DateTime<Utc>>,
    pub last_shown_at: Option<DateTime<Utc>>,
    pub times_shown: i32,
    pub times_dismissed: i32,
    pub converted: bool,
    pub updated_at: DateTime<Utc>,
}

/// Caller-supplied "did this visitor already see the popup in this session?"
/// flag. The server has no concept of session without extra plumbing; the
/// frontend passes a bool in the request (derived from its own sessionStorage).
#[derive(Debug, Clone, Copy, Default)]
pub struct SessionFlags {
    pub shown_this_session: bool,
}

/// Decide whether to show this popup for this visitor, given the config,
/// their persisted state, and session-local flags. Pure function; no I/O.
#[must_use]
pub fn should_show(
    config: &FrequencyConfig,
    state: Option<&VisitorState>,
    session: SessionFlags,
    now: DateTime<Utc>,
) -> bool {
    // Missing state = first-ever impression; every rule that depends on
    // history passes trivially.
    let state = match state {
        Some(s) => s,
        None => {
            // once_per_session with a true session flag still applies —
            // the visitor may not have an anon_id cookie yet but the
            // session storage says the popup already fired.
            if config.once_per_session.unwrap_or(false) && session.shown_this_session {
                return false;
            }
            return true;
        }
    };

    if config.until_converted.unwrap_or(false) && state.converted {
        return false;
    }
    if config.once_per_visitor.unwrap_or(false) && state.times_shown >= 1 {
        return false;
    }
    if config.once_per_session.unwrap_or(false) && session.shown_this_session {
        return false;
    }
    if let Some(max) = config.max_dismissals {
        if state.times_dismissed >= max as i32 {
            return false;
        }
    }
    if let Some(n_days) = config.every_n_days {
        if let Some(last) = state.last_shown_at {
            let window = Duration::days(n_days as i64);
            if now - last < window {
                return false;
            }
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    use uuid::Uuid;

    fn state(ts_days_ago: i64, shown: i32, dismissed: i32, converted: bool) -> VisitorState {
        let now = Utc.with_ymd_and_hms(2026, 4, 17, 12, 0, 0).unwrap();
        let last = now - Duration::days(ts_days_ago);
        VisitorState {
            anonymous_id: Uuid::new_v4(),
            popup_id: Uuid::new_v4(),
            first_shown_at: Some(last),
            last_shown_at: Some(last),
            times_shown: shown,
            times_dismissed: dismissed,
            converted,
            updated_at: last,
        }
    }

    fn now() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 4, 17, 12, 0, 0).unwrap()
    }

    #[test]
    fn no_state_shows_by_default() {
        let cfg = FrequencyConfig::default();
        assert!(should_show(&cfg, None, SessionFlags::default(), now()));
    }

    #[test]
    fn every_n_days_blocks_within_window() {
        let cfg = FrequencyConfig {
            every_n_days: Some(7),
            ..Default::default()
        };
        let s = state(3, 1, 0, false);
        assert!(!should_show(&cfg, Some(&s), SessionFlags::default(), now()));
        let old = state(10, 1, 0, false);
        assert!(should_show(
            &cfg,
            Some(&old),
            SessionFlags::default(),
            now()
        ));
    }

    #[test]
    fn until_converted_blocks_converts() {
        let cfg = FrequencyConfig {
            until_converted: Some(true),
            ..Default::default()
        };
        let s = state(1, 1, 0, true);
        assert!(!should_show(&cfg, Some(&s), SessionFlags::default(), now()));
        let s2 = state(1, 1, 0, false);
        assert!(should_show(&cfg, Some(&s2), SessionFlags::default(), now()));
    }

    #[test]
    fn max_dismissals_respected() {
        let cfg = FrequencyConfig {
            max_dismissals: Some(3),
            ..Default::default()
        };
        let at_cap = state(1, 3, 3, false);
        assert!(!should_show(
            &cfg,
            Some(&at_cap),
            SessionFlags::default(),
            now()
        ));
        let below = state(1, 1, 2, false);
        assert!(should_show(
            &cfg,
            Some(&below),
            SessionFlags::default(),
            now()
        ));
    }

    #[test]
    fn once_per_session_honored() {
        let cfg = FrequencyConfig {
            once_per_session: Some(true),
            ..Default::default()
        };
        let s = state(1, 1, 0, false);
        assert!(should_show(
            &cfg,
            Some(&s),
            SessionFlags {
                shown_this_session: false
            },
            now()
        ));
        assert!(!should_show(
            &cfg,
            Some(&s),
            SessionFlags {
                shown_this_session: true
            },
            now()
        ));
        // Even with no persisted state, session flag wins.
        assert!(!should_show(
            &cfg,
            None,
            SessionFlags {
                shown_this_session: true
            },
            now()
        ));
    }

    #[test]
    fn once_per_visitor_blocks_after_one() {
        let cfg = FrequencyConfig {
            once_per_visitor: Some(true),
            ..Default::default()
        };
        let s = state(90, 1, 0, false);
        assert!(!should_show(&cfg, Some(&s), SessionFlags::default(), now()));
        let fresh = state(0, 0, 0, false);
        assert!(should_show(
            &cfg,
            Some(&fresh),
            SessionFlags::default(),
            now()
        ));
    }
}
