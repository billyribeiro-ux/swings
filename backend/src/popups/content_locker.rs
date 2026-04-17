//! POP-04: content locker decision logic.
//!
//! The locker popup wraps a page region in a blurred preview until the
//! visitor either submits the gating form OR holds a qualifying membership
//! tier. `unlock_decision` returns what the renderer should do; the
//! actual DOM blurring happens client-side.

use serde::{Deserialize, Serialize};

/// Locker configuration carried in `content_json`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LockerConfig {
    /// If set, any visitor whose `membership_tier` is in this list bypasses
    /// the gate entirely (no form submit needed).
    #[serde(default)]
    pub required_membership_tier: Option<Vec<String>>,
    /// If true, an unauthenticated visitor sees a blurred preview instead
    /// of a full block. Matches "tease the content" UX pattern.
    #[serde(default)]
    pub show_blurred_preview: bool,
    /// CSS blur radius in pixels used for the preview.
    #[serde(default = "default_blur")]
    pub blur_radius_px: u8,
}

fn default_blur() -> u8 {
    6
}

/// Visitor fact set needed for a lock decision. Populated by the handler
/// (form submission state from `popup_submissions`, membership tier from
/// the user record).
#[derive(Debug, Clone, Default)]
pub struct VisitorFacts {
    pub form_submitted_for_popup: bool,
    pub membership_tier: Option<String>,
}

/// What the renderer should do with the locked region.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Decision {
    /// Visitor has earned access — render in full.
    Unlocked,
    /// Show the region behind a blur panel with the CTA visible.
    BlurredPreview,
    /// Hide / replace the region entirely until the gate is passed.
    Locked,
}

/// Compute the decision. Resolution order:
///
/// 1. If the visitor already submitted the popup's form → `Unlocked`.
/// 2. Else if their `membership_tier` is in `required_membership_tier` →
///    `Unlocked`.
/// 3. Else if `show_blurred_preview` is true → `BlurredPreview`.
/// 4. Otherwise → `Locked`.
#[must_use]
pub fn unlock_decision(config: &LockerConfig, visitor: &VisitorFacts) -> Decision {
    if visitor.form_submitted_for_popup {
        return Decision::Unlocked;
    }
    if let (Some(required), Some(actual)) =
        (&config.required_membership_tier, &visitor.membership_tier)
    {
        if required.iter().any(|t| t.eq_ignore_ascii_case(actual)) {
            return Decision::Unlocked;
        }
    }
    if config.show_blurred_preview {
        Decision::BlurredPreview
    } else {
        Decision::Locked
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn submitted_unlocks() {
        let cfg = LockerConfig::default();
        let v = VisitorFacts {
            form_submitted_for_popup: true,
            membership_tier: None,
        };
        assert_eq!(unlock_decision(&cfg, &v), Decision::Unlocked);
    }

    #[test]
    fn matching_membership_unlocks() {
        let cfg = LockerConfig {
            required_membership_tier: Some(vec!["pro".into(), "enterprise".into()]),
            ..Default::default()
        };
        let v = VisitorFacts {
            form_submitted_for_popup: false,
            membership_tier: Some("Pro".into()),
        };
        assert_eq!(unlock_decision(&cfg, &v), Decision::Unlocked);
    }

    #[test]
    fn non_matching_membership_stays_locked() {
        let cfg = LockerConfig {
            required_membership_tier: Some(vec!["enterprise".into()]),
            ..Default::default()
        };
        let v = VisitorFacts {
            form_submitted_for_popup: false,
            membership_tier: Some("free".into()),
        };
        assert_eq!(unlock_decision(&cfg, &v), Decision::Locked);
    }

    #[test]
    fn blurred_preview_when_configured() {
        let cfg = LockerConfig {
            show_blurred_preview: true,
            ..Default::default()
        };
        let v = VisitorFacts::default();
        assert_eq!(unlock_decision(&cfg, &v), Decision::BlurredPreview);
    }

    #[test]
    fn defaults_to_locked() {
        let cfg = LockerConfig::default();
        let v = VisitorFacts::default();
        assert_eq!(unlock_decision(&cfg, &v), Decision::Locked);
    }
}
