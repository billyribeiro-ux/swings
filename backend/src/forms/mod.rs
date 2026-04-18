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
pub mod integration_config;
pub mod integrations;
pub mod logic;
pub mod payment;
pub mod repo;
pub mod schema;
pub mod uploads;
pub mod validation;

pub use payment::{
    find_by_idempotency_key as find_payment_intent_by_key,
    insert_intent as insert_payment_intent, mark_succeeded as mark_payment_succeeded,
    validate_donation_amount, FormPaymentIntent, PaymentError, PaymentIntentResponse,
    PaymentKind,
};

pub use integration_config::{
    decrypt as decrypt_credential, encrypt as encrypt_credential, integration_id, CryptoError,
    IntegrationConfig, SealedCredential,
};
pub use integrations::{
    adapter_for, IntegrationAdapter, IntegrationError, SubmissionPayload,
};

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
