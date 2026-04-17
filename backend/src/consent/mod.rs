//! CONSENT — cookie / tracking consent management.
//!
//! Scope by subsystem:
//!   * **CONSENT-01** (module bootstrap): category + banner config model,
//!     public banner lookup endpoint. Admin CRUD lives in CONSENT-07.
//!   * **CONSENT-03** adds the event log (`consent_records`) + DSAR workflow;
//!     see [`records`] for the repository and [`dsar_export`] for the export
//!     builder used by the admin fulfilment endpoint.
//!   * **CONSENT-05** adds region-aware banner resolution; the lookup shape in
//!     [`repo::BannerLookup`] is already region+locale-aware so the layer
//!     swap is mechanical.

pub mod dsar_export;
pub mod geo;
pub mod integrity;
pub mod records;
pub mod repo;

pub use geo::resolve_region;
pub use records::{
    count_dsar_requests, create_dsar_request, fulfill_dsar, get_dsar, hash_ip, hash_ip_at,
    insert_consent_record, list_dsar_requests, list_records_for_subject, ConsentRecordInput,
    ConsentRecordRow, DsarCreateInput, DsarRow, SubjectSelector,
};
pub use repo::{BannerConfigRow, CategoryRow, PolicyRow, ServiceRow};
