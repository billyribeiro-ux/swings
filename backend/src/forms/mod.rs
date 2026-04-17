//! FORM — FluentForms Pro-parity form builder primitives.
//!
//! Scope by subsystem:
//!   * **FORM-01** (this module bootstrap): schema model, logic rules,
//!     repo queries. Admin builder lives in FORM-09; public renderer
//!     in FORM-10.
//!   * **FORM-02** adds the validation engine ([`validation`]) shared
//!     byte-for-byte with the TS client (`src/lib/forms/validate.ts`).
//!   * **FORM-03** adds the submission handler and outbox event emission;
//!     see `crate::handlers::forms`.
//!
//! The tables these modules read/write live in migration `025_forms.sql`.

pub mod antispam;
pub mod logic;
pub mod repo;
pub mod schema;
pub mod uploads;
pub mod validation;

pub use antispam::{
    Akismet, AntispamPipeline, Dedup, Honeypot, SpamCheck, SpamVerdict, SubmissionContext,
    Turnstile,
};

pub use logic::{Action, Condition, LogicRule};
pub use schema::{
    AsyncRule, ChoiceOption, FieldMeta, FieldSchema, FileRules, LengthRules, NumberRules,
};
pub use uploads::{
    finalize_upload, make_storage_key, sniff_and_enforce, ChunkedUploadStore, ContentRange,
    InMemoryStorage, StorageProvider, StoredUpload, UploadError,
};
pub use validation::{validate, AsyncRuleRunner, ValidationError};
