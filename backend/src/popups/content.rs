//! POP-06 content-element schema.
//!
//! `content_json.elements` is an ordered list of typed nodes. This module
//! only models the elements that the server cares about — in particular,
//! `form_ref`, which instructs the renderer to fetch + inline a Domain-2
//! form schema at read time. Other element kinds (headings, buttons,
//! inputs, etc.) pass through as `Opaque` so the frontend keeps full
//! freedom over presentation.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::AppResult;

/// One element in `content_json.elements`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum PopupElement {
    /// Reference to a Domain-2 form. Hydrated server-side into
    /// [`PopupElement::FormEmbed`] before the popup is served.
    FormRef {
        form_id: Uuid,
    },
    /// Inlined form schema, populated by [`hydrate_content`].
    FormEmbed {
        form_id: Uuid,
        schema: serde_json::Value,
    },
    /// Any element the server does not need to reason about. Preserved
    /// verbatim so the frontend renders it as-is.
    #[serde(untagged)]
    Opaque(serde_json::Value),
}

/// Replace every `form_ref` with a `form_embed` whose `schema` holds the
/// published form's JSON. When the form is missing or not published the
/// element is replaced with an `opaque` `{ "type": "form_missing", ... }`
/// stub rather than 500'ing, so a deleted form never blacks out a whole
/// popup.
pub async fn hydrate_content(
    pool: &sqlx::PgPool,
    content: serde_json::Value,
) -> AppResult<serde_json::Value> {
    let mut content = content;
    let Some(elements) = content.get_mut("elements").and_then(|v| v.as_array_mut()) else {
        return Ok(content);
    };

    for el in elements.iter_mut() {
        let Some(form_id) = form_ref_id(el) else {
            continue;
        };
        let schema: Option<serde_json::Value> = load_published_form_schema(pool, form_id).await?;
        *el = match schema {
            Some(s) => serde_json::json!({
                "type": "form_embed",
                "form_id": form_id,
                "schema": s,
            }),
            None => serde_json::json!({
                "type": "form_missing",
                "form_id": form_id,
            }),
        };
    }

    Ok(content)
}

fn form_ref_id(el: &serde_json::Value) -> Option<Uuid> {
    let kind = el.get("type").and_then(|t| t.as_str())?;
    if kind != "form_ref" {
        return None;
    }
    let raw = el.get("form_id").and_then(|f| f.as_str())?;
    Uuid::parse_str(raw).ok()
}

/// Load the published schema for a form. Returns `None` when the form is
/// unknown or unpublished so the caller can fall back gracefully.
///
/// Implementation note: the `forms` subsystem exposes a `form_versions`
/// table keyed by `form_id` + `published_at`. We pick the latest published
/// version. If `forms.latest_published_schema` is not yet in the DB (the
/// forms migrations land independently) the query returns `None` and we
/// surface the same graceful fallback to the renderer.
async fn load_published_form_schema(
    pool: &sqlx::PgPool,
    form_id: Uuid,
) -> AppResult<Option<serde_json::Value>> {
    // `forms` + `form_versions` is owned by FORM-01; we only read. A LEFT
    // JOIN lets us return NULL when the form exists but no version has
    // been published yet.
    let row: Option<(serde_json::Value,)> = sqlx::query_as(
        r#"
        SELECT fv.schema_json
        FROM forms f
        LEFT JOIN LATERAL (
            SELECT schema_json
            FROM form_versions
            WHERE form_id = f.id AND published_at IS NOT NULL
            ORDER BY published_at DESC
            LIMIT 1
        ) fv ON TRUE
        WHERE f.id = $1
        "#,
    )
    .bind(form_id)
    .fetch_optional(pool)
    .await
    .ok()
    .flatten();

    Ok(row.map(|(s,)| s))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn form_ref_id_extracts_uuid() {
        let el = serde_json::json!({
            "type": "form_ref",
            "form_id": "00000000-0000-0000-0000-000000000001",
        });
        assert!(form_ref_id(&el).is_some());
    }

    #[test]
    fn form_ref_id_rejects_non_form_ref() {
        let el = serde_json::json!({ "type": "heading", "text": "x" });
        assert!(form_ref_id(&el).is_none());
    }

    #[test]
    fn form_ref_id_rejects_malformed_uuid() {
        let el = serde_json::json!({ "type": "form_ref", "form_id": "not-a-uuid" });
        assert!(form_ref_id(&el).is_none());
    }
}
