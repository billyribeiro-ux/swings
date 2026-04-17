//! POP-01..POP-06 popup subsystem.
//!
//! The handler layer in `crate::handlers::popups` owns HTTP shape and SQL I/O.
//! This module hosts the pure domain logic so it can be unit-tested without a
//! database: targeting predicates (POP-01), variant assignment + significance
//! testing (POP-02), gamified prize picking (POP-04), countdown helpers
//! (POP-04), content locker decisions (POP-04), frequency capping (POP-05),
//! revenue attribution (POP-06), and the content-element schema that drives
//! form embedding (POP-06).
//!
//! Every submodule is `pub` so the handler layer can call into it; external
//! consumers go through `crate::popups::…` paths.

pub mod attribution;
pub mod content;
pub mod content_locker;
pub mod countdown;
pub mod frequency;
pub mod gamified;
pub mod repo;
pub mod significance;
pub mod targeting;
pub mod variants;

pub use targeting::{matches_targeting_rules, TargetingRules, VisitorContext};
