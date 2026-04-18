//! FORM-02: Validation engine.
//!
//! Takes a `FieldSchema` tree + a `serde_json::Value` data map and emits a
//! `Vec<ValidationError>`. The rules implemented here are intentionally
//! byte-for-byte compatible with `src/lib/forms/validate.ts` — shared error
//! codes ensure both sides agree on which constraint failed.
//!
//! # Rule catalogue
//!
//! | Rule                | Error code              | Source              |
//! |---------------------|-------------------------|---------------------|
//! | `required`          | `required`              | `FieldMeta.required`|
//! | `min_length`        | `min_length`            | `LengthRules`       |
//! | `max_length`        | `max_length`            | `LengthRules`       |
//! | `min`, `max`        | `min` / `max`           | `NumberRules`       |
//! | regex `pattern`     | `pattern`               | `Text.pattern`      |
//! | email format        | `email`                 | `Email` variant     |
//! | url format          | `url`                   | `Url` variant       |
//! | phone E.164         | `phone`                 | `Phone` variant     |
//! | date / time / datetime | `date`/`time`/`datetime` | chrono parse    |
//! | file count / size / mime | `min_files`/`max_files`/`max_file_size`/`mime_type` | `FileRules` |
//! | rating range        | `rating_range`          | `Rating.max_stars`  |
//! | cross-field equals  | `equals`                | declared in schema  |
//! | async rule          | `unique_email`          | `AsyncRuleRunner`   |
//!
//! # Async rules
//!
//! The validator never hits the DB directly. Handlers implement
//! [`AsyncRuleRunner`], and the validator calls into it when a field carries
//! an [`crate::forms::schema::AsyncRule`]. This keeps the engine unit-testable
//! without a live pool.

use std::collections::HashMap;

use async_trait::async_trait;
use chrono::{DateTime, NaiveDate, NaiveTime, Utc};
use regex::Regex;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::forms::schema::{
    AsyncRule, FieldMeta, FieldSchema, FileRules, LengthRules, NumberRules,
};

/// Structured validation failure. Stable `code` strings are the shared contract
/// with `src/lib/forms/validate.ts`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, ToSchema)]
pub struct ValidationError {
    pub field_key: String,
    pub code: String,
    pub message: String,
}

impl ValidationError {
    fn new(field_key: impl Into<String>, code: &'static str, message: impl Into<String>) -> Self {
        Self {
            field_key: field_key.into(),
            code: code.to_string(),
            message: message.into(),
        }
    }
}

/// Resolver for async per-field rules (see `FieldSchema::Email::async_rule`).
/// Handlers implement this; the validator never touches the DB itself.
#[async_trait]
pub trait AsyncRuleRunner: Send + Sync {
    async fn run(
        &self,
        field_key: &str,
        rule: &AsyncRule,
        value: &serde_json::Value,
    ) -> Option<ValidationError>;
}

/// A runner that never fails — used in unit tests and by callers that
/// haven't wired up the DB yet.
pub struct NoopRunner;

#[async_trait]
impl AsyncRuleRunner for NoopRunner {
    async fn run(
        &self,
        _field_key: &str,
        _rule: &AsyncRule,
        _value: &serde_json::Value,
    ) -> Option<ValidationError> {
        None
    }
}

/// RFC 5321 local-part / domain pattern. Deliberately minimal — enough to
/// reject the obvious garbage the browser control lets through. Mirrors
/// `src/lib/forms/validate.ts::EMAIL_RE`.
fn email_regex() -> &'static Regex {
    use std::sync::OnceLock;
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"^[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}$")
            .expect("static email regex compiles")
    })
}

/// E.164: `+` followed by 7 to 15 digits.
fn phone_regex() -> &'static Regex {
    use std::sync::OnceLock;
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"^\+[1-9][0-9]{6,14}$").expect("static phone regex compiles"))
}

/// Entry point: walk the field tree and emit every validation error.
/// Returns an empty `Vec` on success.
pub async fn validate(
    schema: &[FieldSchema],
    data: &serde_json::Value,
    runner: &dyn AsyncRuleRunner,
) -> Vec<ValidationError> {
    let mut errors: Vec<ValidationError> = Vec::new();
    // Extract a flat `{ key -> value }` map for cross-field rules (`equals`).
    let data_map: HashMap<&str, &serde_json::Value> = match data {
        serde_json::Value::Object(o) => o.iter().map(|(k, v)| (k.as_str(), v)).collect(),
        _ => HashMap::new(),
    };

    for field in schema {
        if field.is_decorative() {
            continue;
        }
        validate_field(field, data, &data_map, &mut errors);
    }

    // Async rules run separately so the sync walk stays cheap and the DB
    // round-trip is deferred to the end.
    for field in schema {
        if let FieldSchema::Email {
            meta,
            async_rule: Some(rule),
        } = field
        {
            if let Some(value) = data.get(&meta.key) {
                if !is_empty(value) {
                    if let Some(err) = runner.run(&meta.key, rule, value).await {
                        errors.push(err);
                    }
                }
            }
        }
    }

    errors
}

fn validate_field(
    field: &FieldSchema,
    data: &serde_json::Value,
    data_map: &HashMap<&str, &serde_json::Value>,
    errors: &mut Vec<ValidationError>,
) {
    let meta = field.meta();
    let value = data.get(&meta.key);

    // required
    if meta.required && value.map(is_empty).unwrap_or(true) {
        errors.push(ValidationError::new(
            &meta.key,
            "required",
            format!("{} is required.", label(meta)),
        ));
        return;
    }
    let Some(value) = value else { return };
    if is_empty(value) {
        return;
    }

    match field {
        FieldSchema::Text {
            length, pattern, ..
        } => {
            check_string(meta, value, length, errors);
            if let Some(pat) = pattern {
                check_pattern(meta, value, pat, errors);
            }
        }
        FieldSchema::Email { .. } => check_email(meta, value, errors),
        FieldSchema::Phone { .. } => check_phone(meta, value, errors),
        FieldSchema::Url { .. } => check_url(meta, value, errors),
        FieldSchema::Textarea { length, .. } => check_string(meta, value, length, errors),
        FieldSchema::Number { rules, .. } => check_number(meta, value, rules, errors),
        FieldSchema::Slider { min, max, .. } => {
            check_number(
                meta,
                value,
                &NumberRules {
                    min: Some(*min),
                    max: Some(*max),
                    step: None,
                },
                errors,
            );
        }
        FieldSchema::Rating { max_stars, .. } => check_rating(meta, value, *max_stars, errors),
        FieldSchema::Date {
            min_date, max_date, ..
        } => check_date(
            meta,
            value,
            min_date.as_deref(),
            max_date.as_deref(),
            errors,
        ),
        FieldSchema::Time { .. } => check_time(meta, value, errors),
        FieldSchema::Datetime { .. } => check_datetime(meta, value, errors),
        FieldSchema::FileUpload { rules, .. } | FieldSchema::ImageUpload { rules, .. } => {
            check_files(meta, value, rules, errors)
        }
        FieldSchema::MultiSelect {
            min_selections,
            max_selections,
            ..
        } => check_multiselect(meta, value, *min_selections, *max_selections, errors),
        FieldSchema::Nps { .. } => check_rating_range(meta, value, 0, 10, "nps_range", errors),
        FieldSchema::Likert { scale, .. } => {
            check_rating_range(meta, value, 1, *scale, "likert_range", errors)
        }
        // Equals cross-field rule is declared implicitly in the wire by
        // naming the confirm field `<other>_confirm` — handled below.
        _ => {}
    }

    // Cross-field `equals`: any field with key ending in `_confirm` must
    // match the field whose key is the same stem. Mirrors the TS client.
    if let Some(stem) = meta.key.strip_suffix("_confirm") {
        if let Some(other) = data_map.get(stem) {
            if other != &value {
                errors.push(ValidationError::new(
                    &meta.key,
                    "equals",
                    format!("{} does not match.", label(meta)),
                ));
            }
        }
    }
}

// ── Per-rule helpers ───────────────────────────────────────────────────

fn label(meta: &FieldMeta) -> String {
    if meta.label.is_empty() {
        meta.key.clone()
    } else {
        meta.label.clone()
    }
}

/// Empty-value predicate shared across required / optional paths.
fn is_empty(value: &serde_json::Value) -> bool {
    match value {
        serde_json::Value::Null => true,
        serde_json::Value::String(s) => s.trim().is_empty(),
        serde_json::Value::Array(a) => a.is_empty(),
        serde_json::Value::Object(o) => o.is_empty(),
        _ => false,
    }
}

fn check_string(
    meta: &FieldMeta,
    value: &serde_json::Value,
    rules: &LengthRules,
    errors: &mut Vec<ValidationError>,
) {
    let s = match value.as_str() {
        Some(s) => s,
        None => {
            errors.push(ValidationError::new(
                &meta.key,
                "type",
                format!("{} must be text.", label(meta)),
            ));
            return;
        }
    };
    let len = s.chars().count();
    if let Some(min) = rules.min_length {
        if len < min {
            errors.push(ValidationError::new(
                &meta.key,
                "min_length",
                format!("{} must be at least {min} characters.", label(meta)),
            ));
        }
    }
    if let Some(max) = rules.max_length {
        if len > max {
            errors.push(ValidationError::new(
                &meta.key,
                "max_length",
                format!("{} must be at most {max} characters.", label(meta)),
            ));
        }
    }
}

fn check_pattern(
    meta: &FieldMeta,
    value: &serde_json::Value,
    pattern: &str,
    errors: &mut Vec<ValidationError>,
) {
    let Some(s) = value.as_str() else { return };
    let re = match Regex::new(pattern) {
        Ok(re) => re,
        Err(_) => {
            // Invalid regex in the schema is an admin bug, not a user bug;
            // skip silently so a bad rule doesn't block the submission.
            return;
        }
    };
    if !re.is_match(s) {
        errors.push(ValidationError::new(
            &meta.key,
            "pattern",
            format!("{} has an invalid format.", label(meta)),
        ));
    }
}

fn check_email(meta: &FieldMeta, value: &serde_json::Value, errors: &mut Vec<ValidationError>) {
    let s = value.as_str().unwrap_or("");
    if !email_regex().is_match(s) {
        errors.push(ValidationError::new(
            &meta.key,
            "email",
            format!("{} must be a valid email address.", label(meta)),
        ));
    }
}

fn check_phone(meta: &FieldMeta, value: &serde_json::Value, errors: &mut Vec<ValidationError>) {
    let s = value.as_str().unwrap_or("");
    if !phone_regex().is_match(s) {
        errors.push(ValidationError::new(
            &meta.key,
            "phone",
            format!(
                "{} must be an E.164 phone number (e.g. +14155551234).",
                label(meta)
            ),
        ));
    }
}

fn check_url(meta: &FieldMeta, value: &serde_json::Value, errors: &mut Vec<ValidationError>) {
    let s = value.as_str().unwrap_or("");
    if url::Url::parse(s).is_err() {
        errors.push(ValidationError::new(
            &meta.key,
            "url",
            format!("{} must be a valid URL.", label(meta)),
        ));
    }
}

fn check_number(
    meta: &FieldMeta,
    value: &serde_json::Value,
    rules: &NumberRules,
    errors: &mut Vec<ValidationError>,
) {
    let n = match value.as_f64() {
        Some(n) => n,
        None => {
            errors.push(ValidationError::new(
                &meta.key,
                "type",
                format!("{} must be a number.", label(meta)),
            ));
            return;
        }
    };
    if let Some(min) = rules.min {
        if n < min {
            errors.push(ValidationError::new(
                &meta.key,
                "min",
                format!("{} must be at least {min}.", label(meta)),
            ));
        }
    }
    if let Some(max) = rules.max {
        if n > max {
            errors.push(ValidationError::new(
                &meta.key,
                "max",
                format!("{} must be at most {max}.", label(meta)),
            ));
        }
    }
}

fn check_rating(
    meta: &FieldMeta,
    value: &serde_json::Value,
    max_stars: u32,
    errors: &mut Vec<ValidationError>,
) {
    check_rating_range(meta, value, 1, max_stars, "rating_range", errors);
}

fn check_rating_range(
    meta: &FieldMeta,
    value: &serde_json::Value,
    min: u32,
    max: u32,
    code: &'static str,
    errors: &mut Vec<ValidationError>,
) {
    let Some(n) = value.as_i64() else {
        errors.push(ValidationError::new(
            &meta.key,
            "type",
            format!("{} must be a number.", label(meta)),
        ));
        return;
    };
    if n < (min as i64) || n > (max as i64) {
        errors.push(ValidationError::new(
            &meta.key,
            code,
            format!("{} must be between {min} and {max}.", label(meta)),
        ));
    }
}

fn check_date(
    meta: &FieldMeta,
    value: &serde_json::Value,
    min: Option<&str>,
    max: Option<&str>,
    errors: &mut Vec<ValidationError>,
) {
    let s = value.as_str().unwrap_or("");
    let parsed = NaiveDate::parse_from_str(s, "%Y-%m-%d");
    let date = match parsed {
        Ok(d) => d,
        Err(_) => {
            errors.push(ValidationError::new(
                &meta.key,
                "date",
                format!("{} must be an ISO-8601 date (YYYY-MM-DD).", label(meta)),
            ));
            return;
        }
    };
    if let Some(min_s) = min {
        if let Ok(md) = NaiveDate::parse_from_str(min_s, "%Y-%m-%d") {
            if date < md {
                errors.push(ValidationError::new(
                    &meta.key,
                    "min_date",
                    format!("{} must be on or after {min_s}.", label(meta)),
                ));
            }
        }
    }
    if let Some(max_s) = max {
        if let Ok(md) = NaiveDate::parse_from_str(max_s, "%Y-%m-%d") {
            if date > md {
                errors.push(ValidationError::new(
                    &meta.key,
                    "max_date",
                    format!("{} must be on or before {max_s}.", label(meta)),
                ));
            }
        }
    }
}

fn check_time(meta: &FieldMeta, value: &serde_json::Value, errors: &mut Vec<ValidationError>) {
    let s = value.as_str().unwrap_or("");
    let ok = NaiveTime::parse_from_str(s, "%H:%M").is_ok()
        || NaiveTime::parse_from_str(s, "%H:%M:%S").is_ok();
    if !ok {
        errors.push(ValidationError::new(
            &meta.key,
            "time",
            format!(
                "{} must be a 24-hour time (HH:MM or HH:MM:SS).",
                label(meta)
            ),
        ));
    }
}

fn check_datetime(meta: &FieldMeta, value: &serde_json::Value, errors: &mut Vec<ValidationError>) {
    let s = value.as_str().unwrap_or("");
    if DateTime::parse_from_rfc3339(s).is_err() && s.parse::<DateTime<Utc>>().is_err() {
        errors.push(ValidationError::new(
            &meta.key,
            "datetime",
            format!("{} must be an RFC 3339 datetime.", label(meta)),
        ));
    }
}

fn check_files(
    meta: &FieldMeta,
    value: &serde_json::Value,
    rules: &FileRules,
    errors: &mut Vec<ValidationError>,
) {
    // The submitted value is expected to be an array of file descriptors
    // (shape matches `form_submissions.files_json` — `[{ field_key, file_id,
    // filename, mime_type, size }]`). Absent arrays are treated as empty.
    let arr = match value {
        serde_json::Value::Array(a) => a.as_slice(),
        _ => {
            errors.push(ValidationError::new(
                &meta.key,
                "type",
                format!("{} must be a list of files.", label(meta)),
            ));
            return;
        }
    };
    if let Some(min) = rules.min_files {
        if (arr.len() as u32) < min {
            errors.push(ValidationError::new(
                &meta.key,
                "min_files",
                format!("{} requires at least {min} file(s).", label(meta)),
            ));
        }
    }
    if let Some(max) = rules.max_files {
        if (arr.len() as u32) > max {
            errors.push(ValidationError::new(
                &meta.key,
                "max_files",
                format!("{} accepts at most {max} file(s).", label(meta)),
            ));
        }
    }
    for (idx, f) in arr.iter().enumerate() {
        if let Some(size) = f.get("size").and_then(|v| v.as_u64()) {
            if let Some(max_size) = rules.max_file_size {
                if size > max_size {
                    errors.push(ValidationError::new(
                        &meta.key,
                        "max_file_size",
                        format!(
                            "File #{} on {} exceeds {} byte limit.",
                            idx + 1,
                            label(meta),
                            max_size
                        ),
                    ));
                }
            }
        }
        if !rules.allowed_mime_types.is_empty() {
            let mime = f.get("mime_type").and_then(|v| v.as_str()).unwrap_or("");
            if !rules
                .allowed_mime_types
                .iter()
                .any(|m| mime.eq_ignore_ascii_case(m))
            {
                errors.push(ValidationError::new(
                    &meta.key,
                    "mime_type",
                    format!(
                        "File #{} on {} has an unsupported type.",
                        idx + 1,
                        label(meta)
                    ),
                ));
            }
        }
    }
}

fn check_multiselect(
    meta: &FieldMeta,
    value: &serde_json::Value,
    min: Option<u32>,
    max: Option<u32>,
    errors: &mut Vec<ValidationError>,
) {
    let arr = match value {
        serde_json::Value::Array(a) => a,
        _ => {
            errors.push(ValidationError::new(
                &meta.key,
                "type",
                format!("{} must be a list.", label(meta)),
            ));
            return;
        }
    };
    if let Some(m) = min {
        if (arr.len() as u32) < m {
            errors.push(ValidationError::new(
                &meta.key,
                "min_selections",
                format!("{} requires at least {m} selection(s).", label(meta)),
            ));
        }
    }
    if let Some(m) = max {
        if (arr.len() as u32) > m {
            errors.push(ValidationError::new(
                &meta.key,
                "max_selections",
                format!("{} accepts at most {m} selection(s).", label(meta)),
            ));
        }
    }
}

// ── Unit tests ──────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::forms::schema::{FieldMeta, FieldSchema, FileRules, LengthRules, NumberRules};
    use serde_json::json;

    fn required_meta(key: &str) -> FieldMeta {
        FieldMeta {
            key: key.to_string(),
            label: key.to_string(),
            placeholder: None,
            help_text: None,
            required: true,
        }
    }

    fn optional_meta(key: &str) -> FieldMeta {
        FieldMeta {
            key: key.to_string(),
            label: key.to_string(),
            placeholder: None,
            help_text: None,
            required: false,
        }
    }

    #[tokio::test]
    async fn required_flags_missing_values() {
        let schema = vec![FieldSchema::Text {
            meta: required_meta("name"),
            length: LengthRules::default(),
            pattern: None,
        }];
        let data = json!({});
        let errs = validate(&schema, &data, &NoopRunner).await;
        assert_eq!(errs.len(), 1);
        assert_eq!(errs[0].code, "required");
        assert_eq!(errs[0].field_key, "name");
    }

    #[tokio::test]
    async fn required_treats_whitespace_as_empty() {
        let schema = vec![FieldSchema::Text {
            meta: required_meta("name"),
            length: LengthRules::default(),
            pattern: None,
        }];
        let data = json!({ "name": "   " });
        let errs = validate(&schema, &data, &NoopRunner).await;
        assert_eq!(errs.len(), 1);
        assert_eq!(errs[0].code, "required");
    }

    #[tokio::test]
    async fn min_max_length_rules_emit_codes() {
        let schema = vec![FieldSchema::Text {
            meta: optional_meta("name"),
            length: LengthRules {
                min_length: Some(3),
                max_length: Some(5),
            },
            pattern: None,
        }];
        let short = validate(&schema, &json!({ "name": "ab" }), &NoopRunner).await;
        assert!(short.iter().any(|e| e.code == "min_length"));
        let long = validate(&schema, &json!({ "name": "abcdefg" }), &NoopRunner).await;
        assert!(long.iter().any(|e| e.code == "max_length"));
        let ok = validate(&schema, &json!({ "name": "abcd" }), &NoopRunner).await;
        assert!(ok.is_empty());
    }

    #[tokio::test]
    async fn numeric_min_max_rules() {
        let schema = vec![FieldSchema::Number {
            meta: optional_meta("age"),
            rules: NumberRules {
                min: Some(18.0),
                max: Some(65.0),
                step: None,
            },
        }];
        let lo = validate(&schema, &json!({ "age": 10 }), &NoopRunner).await;
        assert!(lo.iter().any(|e| e.code == "min"));
        let hi = validate(&schema, &json!({ "age": 80 }), &NoopRunner).await;
        assert!(hi.iter().any(|e| e.code == "max"));
    }

    #[tokio::test]
    async fn regex_pattern_rejects_unmatched_strings() {
        let schema = vec![FieldSchema::Text {
            meta: optional_meta("code"),
            length: LengthRules::default(),
            pattern: Some(r"^[A-Z]{3}$".into()),
        }];
        let bad = validate(&schema, &json!({ "code": "abc" }), &NoopRunner).await;
        assert!(bad.iter().any(|e| e.code == "pattern"));
        let ok = validate(&schema, &json!({ "code": "ABC" }), &NoopRunner).await;
        assert!(ok.is_empty());
    }

    #[tokio::test]
    async fn email_format_rule() {
        let schema = vec![FieldSchema::Email {
            meta: optional_meta("email"),
            async_rule: None,
        }];
        let ok = validate(
            &schema,
            &json!({ "email": "jane@example.com" }),
            &NoopRunner,
        )
        .await;
        assert!(ok.is_empty(), "valid email should pass");
        let bad = validate(&schema, &json!({ "email": "not-an-email" }), &NoopRunner).await;
        assert!(bad.iter().any(|e| e.code == "email"));
    }

    #[tokio::test]
    async fn url_and_phone_rules() {
        let schema = vec![
            FieldSchema::Url {
                meta: optional_meta("site"),
            },
            FieldSchema::Phone {
                meta: optional_meta("tel"),
            },
        ];
        let errs = validate(
            &schema,
            &json!({ "site": "not a url", "tel": "555-1234" }),
            &NoopRunner,
        )
        .await;
        assert!(errs.iter().any(|e| e.code == "url"));
        assert!(errs.iter().any(|e| e.code == "phone"));

        let ok = validate(
            &schema,
            &json!({ "site": "https://example.com/", "tel": "+14155551234" }),
            &NoopRunner,
        )
        .await;
        assert!(ok.is_empty());
    }

    #[tokio::test]
    async fn date_time_datetime_rules() {
        let schema = vec![
            FieldSchema::Date {
                meta: optional_meta("d"),
                min_date: None,
                max_date: None,
            },
            FieldSchema::Time {
                meta: optional_meta("t"),
            },
            FieldSchema::Datetime {
                meta: optional_meta("dt"),
            },
        ];
        let bad = validate(
            &schema,
            &json!({ "d": "foo", "t": "bar", "dt": "baz" }),
            &NoopRunner,
        )
        .await;
        assert!(bad.iter().any(|e| e.code == "date"));
        assert!(bad.iter().any(|e| e.code == "time"));
        assert!(bad.iter().any(|e| e.code == "datetime"));

        let ok = validate(
            &schema,
            &json!({
                "d": "2026-04-17",
                "t": "14:30",
                "dt": "2026-04-17T14:30:00Z",
            }),
            &NoopRunner,
        )
        .await;
        assert!(ok.is_empty());
    }

    #[tokio::test]
    async fn file_rules_min_max_mime_size() {
        let schema = vec![FieldSchema::FileUpload {
            meta: optional_meta("cv"),
            rules: FileRules {
                min_files: Some(1),
                max_files: Some(2),
                allowed_mime_types: vec!["application/pdf".into()],
                max_file_size: Some(1024),
            },
        }];
        let too_big = validate(
            &schema,
            &json!({ "cv": [{ "mime_type": "application/pdf", "size": 2048 }] }),
            &NoopRunner,
        )
        .await;
        assert!(too_big.iter().any(|e| e.code == "max_file_size"));

        let bad_mime = validate(
            &schema,
            &json!({ "cv": [{ "mime_type": "image/png", "size": 100 }] }),
            &NoopRunner,
        )
        .await;
        assert!(bad_mime.iter().any(|e| e.code == "mime_type"));

        let ok = validate(
            &schema,
            &json!({ "cv": [{ "mime_type": "application/pdf", "size": 100 }] }),
            &NoopRunner,
        )
        .await;
        assert!(ok.is_empty());
    }

    #[tokio::test]
    async fn rating_and_nps_range() {
        let schema = vec![
            FieldSchema::Rating {
                meta: optional_meta("score"),
                max_stars: 5,
            },
            FieldSchema::Nps {
                meta: optional_meta("nps"),
            },
        ];
        let errs = validate(&schema, &json!({ "score": 7, "nps": 15 }), &NoopRunner).await;
        assert!(errs.iter().any(|e| e.code == "rating_range"));
        assert!(errs.iter().any(|e| e.code == "nps_range"));

        let ok = validate(&schema, &json!({ "score": 5, "nps": 10 }), &NoopRunner).await;
        assert!(ok.is_empty());
    }

    #[tokio::test]
    async fn cross_field_equals_rule() {
        let schema = vec![
            FieldSchema::Text {
                meta: required_meta("password"),
                length: LengthRules::default(),
                pattern: None,
            },
            FieldSchema::Text {
                meta: required_meta("password_confirm"),
                length: LengthRules::default(),
                pattern: None,
            },
        ];
        let mismatch = validate(
            &schema,
            &json!({ "password": "secret", "password_confirm": "typo" }),
            &NoopRunner,
        )
        .await;
        assert!(mismatch.iter().any(|e| e.code == "equals"));

        let ok = validate(
            &schema,
            &json!({ "password": "secret", "password_confirm": "secret" }),
            &NoopRunner,
        )
        .await;
        assert!(ok.is_empty());
    }

    /// AsyncRuleRunner that always fails with a canned error. Used to show
    /// the validator plumbing works end-to-end without a live DB.
    struct FailingRunner;

    #[async_trait]
    impl AsyncRuleRunner for FailingRunner {
        async fn run(
            &self,
            field_key: &str,
            _rule: &AsyncRule,
            _value: &serde_json::Value,
        ) -> Option<ValidationError> {
            Some(ValidationError::new(
                field_key,
                "unique_email",
                "This email is already registered.".to_string(),
            ))
        }
    }

    #[tokio::test]
    async fn async_rule_runner_is_invoked() {
        let schema = vec![FieldSchema::Email {
            meta: optional_meta("email"),
            async_rule: Some(AsyncRule::UniqueEmail),
        }];
        let errs = validate(
            &schema,
            &json!({ "email": "jane@example.com" }),
            &FailingRunner,
        )
        .await;
        assert!(errs.iter().any(|e| e.code == "unique_email"));
    }
}
