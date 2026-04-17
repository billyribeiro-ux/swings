//! FORM-02: Validation engine. Placeholder in FORM-01 — the real implementation
//! lands in the next commit.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Structured validation failure. Stable `code` strings are the shared contract
/// with `src/lib/forms/validate.ts`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, ToSchema)]
pub struct ValidationError {
    pub field_key: String,
    pub code: String,
    pub message: String,
}

/// Resolver for async per-field rules (see `FieldSchema::Email::async_rule`).
/// The handler implements this; the validator never touches the DB.
#[async_trait::async_trait]
pub trait AsyncRuleRunner: Send + Sync {
    /// Return `Some(error)` if the rule fails; `None` on success.
    /// `rule` is the wire token; `value` is the submitted field value.
    async fn run(
        &self,
        field_key: &str,
        rule: &crate::forms::schema::AsyncRule,
        value: &serde_json::Value,
    ) -> Option<ValidationError>;
}

/// Placeholder for FORM-02 — returns an empty error list.
pub async fn validate(
    _schema: &[crate::forms::schema::FieldSchema],
    _data: &serde_json::Value,
    _runner: &dyn AsyncRuleRunner,
) -> Vec<ValidationError> {
    Vec::new()
}
