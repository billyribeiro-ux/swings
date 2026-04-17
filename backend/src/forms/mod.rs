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

pub mod logic;
pub mod repo;
pub mod schema;
pub mod validation;

pub use logic::{Action, Condition, LogicRule};
pub use schema::{
    AsyncRule, ChoiceOption, FieldMeta, FieldSchema, FileRules, LengthRules, NumberRules,
};
pub use validation::{validate, AsyncRuleRunner, ValidationError};
