//! Template registry + renderer.
//!
//! Every template row in `notification_templates` is resolvable by
//! `(key, channel, locale)` with English as the fallback locale. Rendering runs
//! through [`tera::Tera`] with a JSON context — we deliberately reuse the
//! already-vendored Tera crate instead of pulling in handlebars or minijinja.
//!
//! # Versioning
//!
//! Templates are versioned (`notification_templates.version`). A create/update
//! through the admin API bumps the version and leaves prior versions in place,
//! so audit logs remain reproducible. Lookup always picks the highest active
//! version for `(key, channel, locale)`.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use sqlx::PgPool;
use tera::{Context, Tera};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::error::AppError;

/// A concrete template row loaded from `notification_templates`.
///
/// `body_compiled` is maintained by the admin API at save time (currently a
/// straight copy of `body_source` until MJML compilation lands in FDN-09);
/// rendering reads `body_compiled` so the hot path never re-parses MJML.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct Template {
    pub id: Uuid,
    pub key: String,
    pub channel: String,
    pub locale: String,
    pub subject: Option<String>,
    pub body_source: String,
    pub body_compiled: String,
    pub variables: serde_json::Value,
    pub version: i32,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Output of [`Template::render`]. Kept narrow so channels do not have to
/// re-parse Tera output.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct RenderedTemplate {
    pub subject: Option<String>,
    pub body: String,
}

/// Errors raised while resolving or rendering a template.
#[derive(Debug, thiserror::Error)]
pub enum TemplateError {
    /// No row matched `(key, channel, locale)` even after the English
    /// fallback — the caller should surface an operator-visible error.
    #[error("template not found: key={key} channel={channel} locale={locale}")]
    NotFound {
        key: String,
        channel: String,
        locale: String,
    },

    /// The template's Tera source failed to compile. Caller-supplied source
    /// should be validated at save time, not at render time, so reaching this
    /// variant typically indicates a DB edit that bypassed the admin API.
    #[error("template compile error: {0}")]
    Compile(String),

    /// Rendering completed but the JSON context failed to interpolate — a
    /// missing variable, type mismatch, etc.
    #[error("template render error: {0}")]
    Render(String),

    /// Database error while looking up the template row.
    #[error("database error: {0}")]
    Database(#[from] sqlx::Error),

    /// Serialization error while building the Tera context.
    #[error("context serialization error: {0}")]
    Context(#[from] serde_json::Error),
}

impl From<TemplateError> for AppError {
    fn from(err: TemplateError) -> Self {
        match err {
            TemplateError::NotFound { .. } => AppError::NotFound(err.to_string()),
            TemplateError::Compile(msg) | TemplateError::Render(msg) => {
                AppError::BadRequest(format!("template error: {msg}"))
            }
            TemplateError::Database(e) => AppError::Database(e),
            TemplateError::Context(e) => AppError::BadRequest(format!("context error: {e}")),
        }
    }
}

impl Template {
    /// Resolve by `(key, channel, locale)`. Falls back to locale `'en'` when
    /// the requested locale yields no row. Returns [`TemplateError::NotFound`]
    /// if both locales miss.
    pub async fn resolve(
        pool: &PgPool,
        key: &str,
        channel: &str,
        locale: &str,
    ) -> Result<Self, TemplateError> {
        let found = sqlx::query_as::<_, Template>(
            r#"
            SELECT id, key, channel, locale, subject, body_source, body_compiled,
                   variables, version, is_active, created_at, updated_at
            FROM notification_templates
            WHERE key = $1 AND channel = $2 AND locale = $3 AND is_active = TRUE
            ORDER BY version DESC
            LIMIT 1
            "#,
        )
        .bind(key)
        .bind(channel)
        .bind(locale)
        .fetch_optional(pool)
        .await?;

        if let Some(t) = found {
            return Ok(t);
        }

        if locale != "en" {
            let fallback = sqlx::query_as::<_, Template>(
                r#"
                SELECT id, key, channel, locale, subject, body_source, body_compiled,
                       variables, version, is_active, created_at, updated_at
                FROM notification_templates
                WHERE key = $1 AND channel = $2 AND locale = 'en' AND is_active = TRUE
                ORDER BY version DESC
                LIMIT 1
                "#,
            )
            .bind(key)
            .bind(channel)
            .fetch_optional(pool)
            .await?;
            if let Some(t) = fallback {
                return Ok(t);
            }
        }

        Err(TemplateError::NotFound {
            key: key.into(),
            channel: channel.into(),
            locale: locale.into(),
        })
    }

    /// Render `body_compiled` (with `subject` as a separate pass) against the
    /// supplied JSON context.
    ///
    /// A private, single-use [`Tera`] instance is built per call rather than
    /// cached. The motivation: templates can be edited via the admin API and
    /// we need writes to be reflected on the next send without shipping a
    /// cache-invalidation channel. The cost is negligible — template sources
    /// are well under 1 KB each.
    pub fn render(&self, ctx: &serde_json::Value) -> Result<RenderedTemplate, TemplateError> {
        let mut tera = Tera::default();
        tera.add_raw_template("body", &self.body_compiled)
            .map_err(|e| TemplateError::Compile(e.to_string()))?;

        let tera_ctx =
            Context::from_value(ctx.clone()).map_err(|e| TemplateError::Render(e.to_string()))?;

        let body = tera
            .render("body", &tera_ctx)
            .map_err(|e| TemplateError::Render(e.to_string()))?;

        let subject = if let Some(subj) = &self.subject {
            tera.add_raw_template("subject", subj)
                .map_err(|e| TemplateError::Compile(e.to_string()))?;
            let rendered = tera
                .render("subject", &tera_ctx)
                .map_err(|e| TemplateError::Render(e.to_string()))?;
            Some(rendered)
        } else {
            None
        };

        Ok(RenderedTemplate { subject, body })
    }

    /// Next `version` number for a (key, channel, locale) triple — used by
    /// the admin API when saving a new template revision.
    pub async fn next_version(
        pool: &PgPool,
        key: &str,
        channel: &str,
        locale: &str,
    ) -> Result<i32, TemplateError> {
        let max: Option<i32> = sqlx::query_scalar(
            r#"
            SELECT MAX(version) FROM notification_templates
            WHERE key = $1 AND channel = $2 AND locale = $3
            "#,
        )
        .bind(key)
        .bind(channel)
        .bind(locale)
        .fetch_one(pool)
        .await?;
        Ok(max.unwrap_or(0) + 1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn fixture(body: &str, subject: Option<&str>) -> Template {
        Template {
            id: Uuid::nil(),
            key: "test.t".into(),
            channel: "email".into(),
            locale: "en".into(),
            subject: subject.map(|s| s.into()),
            body_source: body.into(),
            body_compiled: body.into(),
            variables: serde_json::json!([]),
            version: 1,
            is_active: true,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    #[test]
    fn renders_variable_substitution() {
        let t = fixture("Hello {{ name }}!", Some("Hi {{ name }}"));
        let out = t
            .render(&serde_json::json!({"name": "Ada"}))
            .expect("render");
        assert_eq!(out.body, "Hello Ada!");
        assert_eq!(out.subject.as_deref(), Some("Hi Ada"));
    }

    #[test]
    fn rendering_missing_variable_errors() {
        let t = fixture("Hello {{ missing }}", None);
        let err = t
            .render(&serde_json::json!({"name": "Ada"}))
            .expect_err("render fails on missing");
        assert!(matches!(err, TemplateError::Render(_)));
    }

    #[test]
    fn rendering_no_subject_returns_none() {
        let t = fixture("body only", None);
        let out = t.render(&serde_json::json!({})).expect("render ok");
        assert!(out.subject.is_none());
        assert_eq!(out.body, "body only");
    }

    #[test]
    fn template_error_to_app_error_not_found() {
        let err: AppError = TemplateError::NotFound {
            key: "x".into(),
            channel: "email".into(),
            locale: "en".into(),
        }
        .into();
        assert!(matches!(err, AppError::NotFound(_)));
    }

    #[test]
    fn template_error_to_app_error_render_is_bad_request() {
        let err: AppError = TemplateError::Render("boom".into()).into();
        assert!(matches!(err, AppError::BadRequest(_)));
    }
}
