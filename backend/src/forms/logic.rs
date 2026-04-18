//! FORM-01: Conditional-logic rule model and evaluator.
//!
//! A form's `logic_json` column is a `Vec<LogicRule>`. At render time
//! (FORM-10) each rule's [`Condition`] is evaluated against the current
//! data map; matching rules apply their [`Action`]. On the server,
//! FORM-03's submit handler uses the same evaluator to enforce
//! `RequireField` / `Show` / `Hide` decisions so a client that bypasses
//! the renderer cannot supply values for hidden fields.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Rule pair: condition → action. Multiple rules may target the same field
/// key; the renderer applies them in document order.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, ToSchema)]
pub struct LogicRule {
    pub condition: Condition,
    pub action: Action,
}

/// Boolean predicate over the submitted data map. The `And` / `Or` variants
/// are recursive so arbitrarily deep trees are expressible.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, ToSchema)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum Condition {
    /// True when `data[field] == value` under JSON equality.
    FieldEquals {
        field: String,
        value: serde_json::Value,
    },
    /// Inverse of [`Condition::FieldEquals`].
    FieldNotEquals {
        field: String,
        value: serde_json::Value,
    },
    /// True when the value is a number greater than `value`. Non-numeric
    /// values always compare false (no coercion).
    FieldGreaterThan { field: String, value: f64 },
    /// Inverse of [`Condition::FieldGreaterThan`].
    FieldLessThan { field: String, value: f64 },
    /// True when the value is a string that contains `value` (case-sensitive),
    /// or an array that contains `value` as a string element.
    FieldContains { field: String, value: String },
    /// Boolean AND over sub-conditions; empty vector evaluates `true`.
    And(Vec<Condition>),
    /// Boolean OR over sub-conditions; empty vector evaluates `false`.
    Or(Vec<Condition>),
}

/// Side-effect applied when the associated [`Condition`] matches. Both the
/// renderer and the server's submit validator interpret these identically.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, ToSchema)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Action {
    /// Make the referenced field visible (default state is visible so this
    /// is only meaningful after a `Hide` rule).
    Show { field: String },
    /// Hide the referenced field; server MUST ignore values for hidden fields.
    Hide { field: String },
    /// Toggle the field's `required` flag to `true` regardless of its schema.
    RequireField { field: String },
    /// Skip a step in a multi-step form; `step` is the 0-based step index.
    SkipStep { step: u32 },
    /// Set the field's value to a JSON literal (used for computed defaults).
    SetValue {
        field: String,
        value: serde_json::Value,
    },
}

impl Condition {
    /// Evaluate against a flat `{ field_key -> value }` map. Unknown fields
    /// compare as missing; the only branches that return true without a
    /// corresponding entry are the negated / logical-combinator variants.
    pub fn evaluate(&self, data: &serde_json::Value) -> bool {
        match self {
            Condition::FieldEquals { field, value } => {
                get(data, field).map(|v| v == value).unwrap_or(false)
            }
            Condition::FieldNotEquals { field, value } => {
                // "not equals" is true when either the field is absent or the
                // stored value differs. This matches FluentForms' semantics.
                get(data, field).map(|v| v != value).unwrap_or(true)
            }
            Condition::FieldGreaterThan { field, value } => get(data, field)
                .and_then(|v| v.as_f64())
                .map(|n| n > *value)
                .unwrap_or(false),
            Condition::FieldLessThan { field, value } => get(data, field)
                .and_then(|v| v.as_f64())
                .map(|n| n < *value)
                .unwrap_or(false),
            Condition::FieldContains { field, value } => match get(data, field) {
                Some(v) => match v {
                    serde_json::Value::String(s) => s.contains(value),
                    serde_json::Value::Array(arr) => arr
                        .iter()
                        .any(|e| e.as_str().map(|s| s == value).unwrap_or(false)),
                    _ => false,
                },
                None => false,
            },
            Condition::And(conds) => conds.iter().all(|c| c.evaluate(data)),
            Condition::Or(conds) => !conds.is_empty() && conds.iter().any(|c| c.evaluate(data)),
        }
    }
}

fn get<'a>(data: &'a serde_json::Value, field: &str) -> Option<&'a serde_json::Value> {
    data.get(field)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn data() -> serde_json::Value {
        json!({
            "age": 42,
            "country": "US",
            "tags": ["a", "b"],
            "bio": "hello world"
        })
    }

    #[test]
    fn field_equals_matches_on_json_equality() {
        let c = Condition::FieldEquals {
            field: "country".into(),
            value: json!("US"),
        };
        assert!(c.evaluate(&data()));
        let miss = Condition::FieldEquals {
            field: "country".into(),
            value: json!("EU"),
        };
        assert!(!miss.evaluate(&data()));
    }

    #[test]
    fn field_not_equals_is_true_for_missing_field() {
        let c = Condition::FieldNotEquals {
            field: "missing".into(),
            value: json!("x"),
        };
        assert!(c.evaluate(&data()));
    }

    #[test]
    fn numeric_comparisons() {
        let gt = Condition::FieldGreaterThan {
            field: "age".into(),
            value: 18.0,
        };
        assert!(gt.evaluate(&data()));
        let lt = Condition::FieldLessThan {
            field: "age".into(),
            value: 50.0,
        };
        assert!(lt.evaluate(&data()));
        let not_numeric = Condition::FieldGreaterThan {
            field: "country".into(),
            value: 0.0,
        };
        assert!(!not_numeric.evaluate(&data()));
    }

    #[test]
    fn field_contains_works_for_strings_and_arrays() {
        let s = Condition::FieldContains {
            field: "bio".into(),
            value: "hello".into(),
        };
        assert!(s.evaluate(&data()));
        let a = Condition::FieldContains {
            field: "tags".into(),
            value: "a".into(),
        };
        assert!(a.evaluate(&data()));
        let miss = Condition::FieldContains {
            field: "tags".into(),
            value: "z".into(),
        };
        assert!(!miss.evaluate(&data()));
    }

    #[test]
    fn and_or_combinators() {
        let and = Condition::And(vec![
            Condition::FieldEquals {
                field: "country".into(),
                value: json!("US"),
            },
            Condition::FieldGreaterThan {
                field: "age".into(),
                value: 18.0,
            },
        ]);
        assert!(and.evaluate(&data()));

        let or = Condition::Or(vec![
            Condition::FieldEquals {
                field: "country".into(),
                value: json!("EU"),
            },
            Condition::FieldEquals {
                field: "country".into(),
                value: json!("US"),
            },
        ]);
        assert!(or.evaluate(&data()));

        let empty_or = Condition::Or(vec![]);
        assert!(!empty_or.evaluate(&data()));
    }

    #[test]
    fn logic_rule_round_trips_through_serde() {
        let rule = LogicRule {
            condition: Condition::FieldEquals {
                field: "country".into(),
                value: json!("US"),
            },
            action: Action::Show {
                field: "state".into(),
            },
        };
        let v = serde_json::to_value(&rule).expect("serialize");
        let back: LogicRule = serde_json::from_value(v).expect("deserialize");
        assert_eq!(rule, back);
    }

    #[test]
    fn action_variants_round_trip() {
        let actions = vec![
            Action::Show { field: "a".into() },
            Action::Hide { field: "b".into() },
            Action::RequireField { field: "c".into() },
            Action::SkipStep { step: 2 },
            Action::SetValue {
                field: "total".into(),
                value: json!(42),
            },
        ];
        for a in actions {
            let v = serde_json::to_value(&a).expect("serialize");
            let back: Action = serde_json::from_value(v).expect("deserialize");
            assert_eq!(a, back);
        }
    }
}
