//! CONSENT — cookie / tracking consent management.
//!
//! Scope by subsystem:
//!   * **CONSENT-01** (this module bootstrap): category + banner config model,
//!     public banner lookup endpoint. Admin CRUD lives in CONSENT-07.
//!   * **CONSENT-03** adds the event log (`consent_records`) + DSAR workflow;
//!     that work layers on top of the `repo` module here.
//!   * **CONSENT-05** adds region-aware banner resolution; the lookup shape in
//!     [`repo::BannerLookup`] is already region+locale-aware so the layer
//!     swap is mechanical.

pub mod repo;

pub use repo::{BannerConfigRow, CategoryRow, PolicyRow, ServiceRow};
