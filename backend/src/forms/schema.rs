//! FORM-01: Typed field library.
//!
//! The form schema is an ordered list of [`FieldSchema`] variants that the
//! admin builder (FORM-09) serialises into `form_versions.schema_json`.
//! FluentForms Pro parity (April 2026) requires 33 distinct field types,
//! enumerated below. Each variant carries its own typed config so the
//! validation engine (FORM-02) and renderer (FORM-10) can dispatch on
//! `type` without reparsing generic JSON.
//!
//! Wire representation uses `#[serde(tag = "type", rename_all = "snake_case")]`
//! — the JSON shape matches the plan's canonical example:
//!
//! ```json
//! { "type": "email", "key": "email", "label": "Email", "required": true }
//! ```
//!
//! Field `key` is the stable identifier used to lookup values in
//! `data_json`; never rename after the form has collected submissions.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

// ── Shared config blocks ────────────────────────────────────────────────

/// Common fields present on every visible field. Kept as a flattened block
/// on every variant so admin edits don't have to reach into variant-specific
/// config for things like `required` or the human label.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct FieldMeta {
    /// Stable key used to address values in submission data. Required.
    pub key: String,
    /// Human-readable label shown above the input.
    #[serde(default)]
    pub label: String,
    /// Placeholder / ghost text.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub placeholder: Option<String>,
    /// Help text rendered below the input.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub help_text: Option<String>,
    /// Whether the field must be non-empty.
    #[serde(default)]
    pub required: bool,
}

/// String-length constraints for text-ish fields.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct LengthRules {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_length: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_length: Option<usize>,
}

/// Numeric range + step rules.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct NumberRules {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub step: Option<f64>,
}

/// File-upload constraints shared by `file_upload` and `image_upload`.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct FileRules {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_files: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_files: Option<u32>,
    /// RFC 6838 media types that MUST match the first 512 bytes of the upload.
    #[serde(default)]
    pub allowed_mime_types: Vec<String>,
    /// Per-file ceiling, in bytes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_file_size: Option<u64>,
}

/// Single option in a `select`, `radio`, or `multi_select`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ChoiceOption {
    pub value: String,
    pub label: String,
    #[serde(default)]
    pub disabled: bool,
}

/// Async rule name. The handler supplies an [`crate::forms::validation::AsyncRuleRunner`]
/// that resolves the name to a DB query; the validator never hits the DB itself.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum AsyncRule {
    /// Email MUST NOT exist in `users.email` (used on signup forms).
    UniqueEmail,
}

// ── Field variants ──────────────────────────────────────────────────────

/// Typed field tree used by admin builder (FORM-09), validator (FORM-02),
/// and public renderer (FORM-10). The tag discriminator is the `type` field
/// in the wire representation — see module docs for an example.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, ToSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum FieldSchema {
    /// Single-line text input.
    Text {
        #[serde(flatten)]
        meta: FieldMeta,
        #[serde(flatten)]
        length: LengthRules,
        /// Anchored regex the trimmed value must match.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pattern: Option<String>,
    },
    /// Email address; validated via RFC 5321 regex in FORM-02.
    Email {
        #[serde(flatten)]
        meta: FieldMeta,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        async_rule: Option<AsyncRule>,
    },
    /// Phone number; validated as E.164.
    Phone {
        #[serde(flatten)]
        meta: FieldMeta,
    },
    /// URL; must parse via `url::Url::parse`.
    Url {
        #[serde(flatten)]
        meta: FieldMeta,
    },
    /// Multi-line text area.
    Textarea {
        #[serde(flatten)]
        meta: FieldMeta,
        #[serde(flatten)]
        length: LengthRules,
    },
    /// Numeric input.
    Number {
        #[serde(flatten)]
        meta: FieldMeta,
        #[serde(flatten)]
        rules: NumberRules,
    },
    /// Slider — numeric with required min/max so the control has a track.
    Slider {
        #[serde(flatten)]
        meta: FieldMeta,
        min: f64,
        max: f64,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        step: Option<f64>,
    },
    /// Star rating; `max_stars` bounds the response 1..=max_stars.
    Rating {
        #[serde(flatten)]
        meta: FieldMeta,
        #[serde(default = "default_max_stars")]
        max_stars: u32,
    },
    /// ISO-8601 date (YYYY-MM-DD).
    Date {
        #[serde(flatten)]
        meta: FieldMeta,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        min_date: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        max_date: Option<String>,
    },
    /// ISO-8601 time (HH:MM or HH:MM:SS).
    Time {
        #[serde(flatten)]
        meta: FieldMeta,
    },
    /// ISO-8601 datetime (RFC 3339).
    Datetime {
        #[serde(flatten)]
        meta: FieldMeta,
    },
    /// Single-choice dropdown.
    Select {
        #[serde(flatten)]
        meta: FieldMeta,
        options: Vec<ChoiceOption>,
    },
    /// Multiple-choice dropdown; data stored as a `Vec<String>`.
    MultiSelect {
        #[serde(flatten)]
        meta: FieldMeta,
        options: Vec<ChoiceOption>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        min_selections: Option<u32>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        max_selections: Option<u32>,
    },
    /// Single-choice radio group.
    Radio {
        #[serde(flatten)]
        meta: FieldMeta,
        options: Vec<ChoiceOption>,
    },
    /// Boolean checkbox (individual agreement, not a group).
    Checkbox {
        #[serde(flatten)]
        meta: FieldMeta,
    },
    /// Arbitrary file upload.
    FileUpload {
        #[serde(flatten)]
        meta: FieldMeta,
        #[serde(flatten)]
        rules: FileRules,
    },
    /// Image upload — image/* MIME gate enforced by renderer + validator.
    ImageUpload {
        #[serde(flatten)]
        meta: FieldMeta,
        #[serde(flatten)]
        rules: FileRules,
    },
    /// Signature pad; value is an SVG/PNG base64 data URL.
    Signature {
        #[serde(flatten)]
        meta: FieldMeta,
    },
    /// Rich-text editor (TipTap); value is sanitised HTML.
    RichText {
        #[serde(flatten)]
        meta: FieldMeta,
        #[serde(flatten)]
        length: LengthRules,
    },
    /// Non-visible pre-set value (UTM capture, referrer).
    Hidden {
        #[serde(flatten)]
        meta: FieldMeta,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        default_value: Option<serde_json::Value>,
    },
    /// Inline HTML block — no input, just copy.
    HtmlBlock {
        #[serde(flatten)]
        meta: FieldMeta,
        html: String,
    },
    /// Visual separator; renders a horizontal rule + optional title.
    SectionBreak {
        #[serde(flatten)]
        meta: FieldMeta,
    },
    /// Step boundary in multi-step forms (FORM-04).
    PageBreak {
        #[serde(flatten)]
        meta: FieldMeta,
    },
    /// Structured address (street / city / state / zip / country).
    Address {
        #[serde(flatten)]
        meta: FieldMeta,
    },
    /// GDPR consent checkbox — required to be explicit per Art. 7(2).
    GdprConsent {
        #[serde(flatten)]
        meta: FieldMeta,
        consent_text: String,
    },
    /// Terms-of-service acceptance.
    Terms {
        #[serde(flatten)]
        meta: FieldMeta,
        terms_url: String,
    },
    /// Raw HTML passed through sanitiser; admin-only typing.
    CustomHtml {
        #[serde(flatten)]
        meta: FieldMeta,
        html: String,
    },
    /// Stripe Elements card; FORM-08 wires the PaymentIntent.
    /// Stripe Elements card; FORM-08 wires the PaymentIntent.
    ///
    /// Two shapes via `payment_kind`:
    ///   * `one_time` (default) — fixed `amount_cents` charge.
    ///   * `donation` — donor picks from `suggested_amounts` (minor units)
    ///     or supplies a custom amount when `allow_custom` is true.
    Payment {
        #[serde(flatten)]
        meta: FieldMeta,
        /// Amount in minor currency units (cents). For donation kind this
        /// is the default fallback when the donor sends no amount.
        amount_cents: i64,
        #[serde(default = "default_currency")]
        currency: String,
        /// `"one_time"` (default) or `"donation"`.
        #[serde(default = "default_payment_kind")]
        payment_kind: String,
        /// Donation preset amounts (minor units). Empty ⇒ free-form only.
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        suggested_amounts: Vec<i64>,
        /// When true + `payment_kind = donation`, donors may enter a
        /// custom amount that is not in `suggested_amounts`.
        #[serde(default)]
        allow_custom: bool,
    },
    /// Stripe subscription plan; FORM-08.
    Subscription {
        #[serde(flatten)]
        meta: FieldMeta,
        plan_id: uuid::Uuid,
    },
    /// Scored quiz field — FORM-04 surfaces the `score` in submissions.
    Quiz {
        #[serde(flatten)]
        meta: FieldMeta,
        question: String,
        options: Vec<ChoiceOption>,
        correct_value: String,
        #[serde(default = "default_quiz_points")]
        points: u32,
    },
    /// Net-Promoter-Score question (0–10).
    Nps {
        #[serde(flatten)]
        meta: FieldMeta,
    },
    /// Likert scale (1–5 by default).
    Likert {
        #[serde(flatten)]
        meta: FieldMeta,
        #[serde(default = "default_likert_scale")]
        scale: u32,
    },
    /// Matrix of Likert scores keyed by row.
    Matrix {
        #[serde(flatten)]
        meta: FieldMeta,
        rows: Vec<String>,
        columns: Vec<String>,
    },
    /// Drag-to-rank ordering of the given options.
    Ranking {
        #[serde(flatten)]
        meta: FieldMeta,
        options: Vec<ChoiceOption>,
    },
    /// Computed field — FORM-04 evaluates the formula over sibling field values.
    Calculation {
        #[serde(flatten)]
        meta: FieldMeta,
        formula: String,
    },
    /// Dropdown populated via XHR at render time.
    DynamicDropdown {
        #[serde(flatten)]
        meta: FieldMeta,
        endpoint: String,
    },
    /// Chained country → state dropdown pair.
    CountryState {
        #[serde(flatten)]
        meta: FieldMeta,
    },
    /// Picker that resolves to a blog post or product id.
    PostProductSelector {
        #[serde(flatten)]
        meta: FieldMeta,
        /// `post` | `product` — narrows the picker dataset.
        source: String,
    },
}

impl FieldSchema {
    /// Common meta accessor so validator + logic engine don't have to pattern
    /// match on every variant. Returns `None` for decorative fields that have
    /// no collectable value. (`html_block`, `section_break`, `page_break`
    /// still have a meta — included for discriminator stability — but their
    /// `required` flag is ignored by the validator.)
    pub fn meta(&self) -> &FieldMeta {
        match self {
            FieldSchema::Text { meta, .. }
            | FieldSchema::Email { meta, .. }
            | FieldSchema::Phone { meta, .. }
            | FieldSchema::Url { meta, .. }
            | FieldSchema::Textarea { meta, .. }
            | FieldSchema::Number { meta, .. }
            | FieldSchema::Slider { meta, .. }
            | FieldSchema::Rating { meta, .. }
            | FieldSchema::Date { meta, .. }
            | FieldSchema::Time { meta, .. }
            | FieldSchema::Datetime { meta, .. }
            | FieldSchema::Select { meta, .. }
            | FieldSchema::MultiSelect { meta, .. }
            | FieldSchema::Radio { meta, .. }
            | FieldSchema::Checkbox { meta, .. }
            | FieldSchema::FileUpload { meta, .. }
            | FieldSchema::ImageUpload { meta, .. }
            | FieldSchema::Signature { meta, .. }
            | FieldSchema::RichText { meta, .. }
            | FieldSchema::Hidden { meta, .. }
            | FieldSchema::HtmlBlock { meta, .. }
            | FieldSchema::SectionBreak { meta, .. }
            | FieldSchema::PageBreak { meta, .. }
            | FieldSchema::Address { meta, .. }
            | FieldSchema::GdprConsent { meta, .. }
            | FieldSchema::Terms { meta, .. }
            | FieldSchema::CustomHtml { meta, .. }
            | FieldSchema::Payment { meta, .. }
            | FieldSchema::Subscription { meta, .. }
            | FieldSchema::Quiz { meta, .. }
            | FieldSchema::Nps { meta, .. }
            | FieldSchema::Likert { meta, .. }
            | FieldSchema::Matrix { meta, .. }
            | FieldSchema::Ranking { meta, .. }
            | FieldSchema::Calculation { meta, .. }
            | FieldSchema::DynamicDropdown { meta, .. }
            | FieldSchema::CountryState { meta, .. }
            | FieldSchema::PostProductSelector { meta, .. } => meta,
        }
    }

    /// Returns `true` for fields that have no collectable value
    /// (decorative / structural only). The validator short-circuits on these.
    pub fn is_decorative(&self) -> bool {
        matches!(
            self,
            FieldSchema::HtmlBlock { .. }
                | FieldSchema::SectionBreak { .. }
                | FieldSchema::PageBreak { .. }
                | FieldSchema::CustomHtml { .. }
        )
    }
}

fn default_max_stars() -> u32 {
    5
}

fn default_currency() -> String {
    "usd".to_string()
}

fn default_payment_kind() -> String {
    "one_time".to_string()
}

fn default_quiz_points() -> u32 {
    1
}

fn default_likert_scale() -> u32 {
    5
}

// ── Unit tests ──────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn meta(k: &str) -> FieldMeta {
        FieldMeta {
            key: k.to_string(),
            label: format!("{k}-label"),
            placeholder: None,
            help_text: None,
            required: false,
        }
    }

    /// Every variant round-trips through serde. A single helper asserts the
    /// tag lands where we expect, and that `meta().key` survives the trip.
    fn round_trip(field: FieldSchema, expected_tag: &str, expected_key: &str) {
        let v = serde_json::to_value(&field).expect("serialize");
        assert_eq!(
            v.get("type").and_then(|s| s.as_str()),
            Some(expected_tag),
            "tag mismatch for {expected_tag}"
        );
        let back: FieldSchema = serde_json::from_value(v).expect("deserialize");
        assert_eq!(back.meta().key, expected_key);
        assert_eq!(field, back);
    }

    #[test]
    fn text_variant_round_trips() {
        round_trip(
            FieldSchema::Text {
                meta: meta("first_name"),
                length: LengthRules {
                    min_length: Some(1),
                    max_length: Some(80),
                },
                pattern: None,
            },
            "text",
            "first_name",
        );
    }

    #[test]
    fn email_variant_carries_async_rule() {
        // Note: the enum uses `rename_all = "snake_case"` so field names inside
        // variants are snake_case. `FieldMeta` is flattened with its own
        // `rename_all = "camelCase"` — that's why `helpText` (if present)
        // would appear camelCased, while `async_rule` is snake_case.
        let json_v = json!({
            "type": "email",
            "key": "email",
            "label": "Email",
            "required": true,
            "async_rule": "unique_email"
        });
        let parsed: FieldSchema = serde_json::from_value(json_v).expect("deserialize");
        match &parsed {
            FieldSchema::Email { async_rule, .. } => {
                assert_eq!(async_rule.as_ref(), Some(&AsyncRule::UniqueEmail));
            }
            _ => panic!("expected Email"),
        }
        // Round-trip through Value to confirm serialisation stability.
        let v = serde_json::to_value(&parsed).expect("serialize");
        assert_eq!(v.get("type").and_then(|s| s.as_str()), Some("email"));
    }

    #[test]
    fn phone_and_url_round_trip() {
        round_trip(FieldSchema::Phone { meta: meta("tel") }, "phone", "tel");
        round_trip(FieldSchema::Url { meta: meta("site") }, "url", "site");
    }

    #[test]
    fn numeric_variants_round_trip() {
        round_trip(
            FieldSchema::Number {
                meta: meta("age"),
                rules: NumberRules {
                    min: Some(0.0),
                    max: Some(120.0),
                    step: Some(1.0),
                },
            },
            "number",
            "age",
        );
        round_trip(
            FieldSchema::Slider {
                meta: meta("vol"),
                min: 0.0,
                max: 100.0,
                step: Some(1.0),
            },
            "slider",
            "vol",
        );
        round_trip(
            FieldSchema::Rating {
                meta: meta("score"),
                max_stars: 10,
            },
            "rating",
            "score",
        );
    }

    #[test]
    fn date_time_variants_round_trip() {
        round_trip(
            FieldSchema::Date {
                meta: meta("dob"),
                min_date: Some("1900-01-01".into()),
                max_date: None,
            },
            "date",
            "dob",
        );
        round_trip(FieldSchema::Time { meta: meta("t") }, "time", "t");
        round_trip(
            FieldSchema::Datetime { meta: meta("ts") },
            "datetime",
            "ts",
        );
    }

    #[test]
    fn choice_variants_round_trip() {
        let opts = vec![
            ChoiceOption {
                value: "a".into(),
                label: "A".into(),
                disabled: false,
            },
            ChoiceOption {
                value: "b".into(),
                label: "B".into(),
                disabled: false,
            },
        ];
        round_trip(
            FieldSchema::Select {
                meta: meta("choice"),
                options: opts.clone(),
            },
            "select",
            "choice",
        );
        round_trip(
            FieldSchema::MultiSelect {
                meta: meta("tags"),
                options: opts.clone(),
                min_selections: Some(1),
                max_selections: Some(3),
            },
            "multi_select",
            "tags",
        );
        round_trip(
            FieldSchema::Radio {
                meta: meta("color"),
                options: opts.clone(),
            },
            "radio",
            "color",
        );
    }

    #[test]
    fn file_variants_round_trip() {
        let rules = FileRules {
            min_files: Some(1),
            max_files: Some(3),
            allowed_mime_types: vec!["application/pdf".into()],
            max_file_size: Some(1024 * 1024),
        };
        round_trip(
            FieldSchema::FileUpload {
                meta: meta("cv"),
                rules: rules.clone(),
            },
            "file_upload",
            "cv",
        );
        round_trip(
            FieldSchema::ImageUpload {
                meta: meta("photo"),
                rules,
            },
            "image_upload",
            "photo",
        );
    }

    #[test]
    fn structural_and_decorative_variants_round_trip() {
        round_trip(
            FieldSchema::Textarea {
                meta: meta("msg"),
                length: LengthRules {
                    min_length: None,
                    max_length: Some(2000),
                },
            },
            "textarea",
            "msg",
        );
        round_trip(
            FieldSchema::Checkbox { meta: meta("opt") },
            "checkbox",
            "opt",
        );
        round_trip(
            FieldSchema::Signature { meta: meta("sig") },
            "signature",
            "sig",
        );
        round_trip(
            FieldSchema::RichText {
                meta: meta("body"),
                length: LengthRules::default(),
            },
            "rich_text",
            "body",
        );
        round_trip(
            FieldSchema::Hidden {
                meta: meta("utm"),
                default_value: Some(json!("organic")),
            },
            "hidden",
            "utm",
        );
        round_trip(
            FieldSchema::HtmlBlock {
                meta: meta("banner"),
                html: "<p>hello</p>".into(),
            },
            "html_block",
            "banner",
        );
        round_trip(
            FieldSchema::SectionBreak {
                meta: meta("sep1"),
            },
            "section_break",
            "sep1",
        );
        round_trip(
            FieldSchema::PageBreak { meta: meta("p1") },
            "page_break",
            "p1",
        );
        round_trip(
            FieldSchema::Address { meta: meta("addr") },
            "address",
            "addr",
        );
        round_trip(
            FieldSchema::GdprConsent {
                meta: meta("gdpr"),
                consent_text: "I agree".into(),
            },
            "gdpr_consent",
            "gdpr",
        );
        round_trip(
            FieldSchema::Terms {
                meta: meta("tos"),
                terms_url: "/tos".into(),
            },
            "terms",
            "tos",
        );
        round_trip(
            FieldSchema::CustomHtml {
                meta: meta("raw"),
                html: "<div/>".into(),
            },
            "custom_html",
            "raw",
        );
    }

    #[test]
    fn payment_quiz_nps_likert_round_trip() {
        round_trip(
            FieldSchema::Payment {
                meta: meta("pay"),
                amount_cents: 1999,
                currency: "usd".into(),
                payment_kind: "one_time".into(),
                suggested_amounts: vec![],
                allow_custom: false,
            },
            "payment",
            "pay",
        );
        round_trip(
            FieldSchema::Subscription {
                meta: meta("sub"),
                plan_id: uuid::Uuid::nil(),
            },
            "subscription",
            "sub",
        );
        round_trip(
            FieldSchema::Quiz {
                meta: meta("q1"),
                question: "2+2?".into(),
                options: vec![
                    ChoiceOption {
                        value: "4".into(),
                        label: "4".into(),
                        disabled: false,
                    },
                    ChoiceOption {
                        value: "5".into(),
                        label: "5".into(),
                        disabled: false,
                    },
                ],
                correct_value: "4".into(),
                points: 1,
            },
            "quiz",
            "q1",
        );
        round_trip(FieldSchema::Nps { meta: meta("nps") }, "nps", "nps");
        round_trip(
            FieldSchema::Likert {
                meta: meta("l1"),
                scale: 7,
            },
            "likert",
            "l1",
        );
    }

    #[test]
    fn matrix_ranking_calc_dynamic_round_trip() {
        round_trip(
            FieldSchema::Matrix {
                meta: meta("m"),
                rows: vec!["r1".into(), "r2".into()],
                columns: vec!["c1".into(), "c2".into()],
            },
            "matrix",
            "m",
        );
        round_trip(
            FieldSchema::Ranking {
                meta: meta("rank"),
                options: vec![ChoiceOption {
                    value: "a".into(),
                    label: "A".into(),
                    disabled: false,
                }],
            },
            "ranking",
            "rank",
        );
        round_trip(
            FieldSchema::Calculation {
                meta: meta("total"),
                formula: "qty * price".into(),
            },
            "calculation",
            "total",
        );
        round_trip(
            FieldSchema::DynamicDropdown {
                meta: meta("dd"),
                endpoint: "/api/tags".into(),
            },
            "dynamic_dropdown",
            "dd",
        );
        round_trip(
            FieldSchema::CountryState { meta: meta("cs") },
            "country_state",
            "cs",
        );
        round_trip(
            FieldSchema::PostProductSelector {
                meta: meta("ps"),
                source: "product".into(),
            },
            "post_product_selector",
            "ps",
        );
    }

    #[test]
    fn is_decorative_flags_only_structural_variants() {
        assert!(FieldSchema::HtmlBlock {
            meta: meta("x"),
            html: "".into()
        }
        .is_decorative());
        assert!(FieldSchema::SectionBreak { meta: meta("x") }.is_decorative());
        assert!(FieldSchema::PageBreak { meta: meta("x") }.is_decorative());
        assert!(FieldSchema::CustomHtml {
            meta: meta("x"),
            html: "".into()
        }
        .is_decorative());
        assert!(!FieldSchema::Text {
            meta: meta("x"),
            length: LengthRules::default(),
            pattern: None
        }
        .is_decorative());
    }
}
