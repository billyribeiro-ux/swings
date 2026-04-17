//! Shared utilities used across the crate.
//!
//! Each submodule here is deliberately self-contained and free of cross-cutting
//! dependencies on handlers, models, or persistence layers. They exist so that
//! domain code can reach for a vetted primitive (integer money arithmetic,
//! geo/UA detection, HTML sanitization) rather than re-rolling one per feature.
//!
//! # Contents
//!
//! * [`geo`]  — country-of-request detection via CDN headers + MaxMind fallback.
//! * [`html`] — `ammonia`-based HTML sanitization with a project allowlist.
//! * [`money`] — integer-minor-unit [`Money`](money::Money) newtype with checked
//!   arithmetic and basis-point percentage application.
//! * [`ua`]   — cached user-agent parsing via `woothee`.

pub mod geo;
pub mod html;
pub mod money;
pub mod ua;
