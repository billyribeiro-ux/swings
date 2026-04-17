//! Top-level send API.
//!
//! Single entry point for "please emit notification X to user Y". Performs
//! every domain check (preference, suppression, template resolve) and then
//! hands delivery off to the outbox — the actual channel invocation happens
//! later in [`crate::events::handlers::notify::NotifyHandler`].
//!
//! # Flow
//!
//! ```text
//! caller
//!   │ BEGIN TX
//!   ├─ domain writes (optional)
//!   ├─ send_notification(pool, "user.welcome", recipient, ctx, opts)
//!   │     ├─ resolve template (locale fallback)
//!   │     ├─ preference check (if user_id present)
//!   │     ├─ suppression check
//!   │     ├─ render subject + body
//!   │     ├─ INSERT notification_deliveries (status='queued')
//!   │     └─ outbox::publish_in_tx('notification.queued', {delivery_id})
//!   └─ COMMIT
//! ```

use chrono::Utc;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::events::outbox::{self, Event, EventHeaders};

use super::preferences::{self, is_allowed_for_preference};
use super::suppression;
use super::templates::{Template, TemplateError};

/// Who's receiving the notification. Supporting anonymous senders keeps the
/// API useful for pre-signup flows (double-opt-in form confirmations, etc.).
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Recipient {
    /// Authenticated user — preference + suppression checks both run.
    User { user_id: Uuid, email: String },
    /// Anonymous visitor — only the suppression check runs.
    Anonymous { email: String },
}

impl Recipient {
    #[must_use]
    pub fn email(&self) -> &str {
        match self {
            Recipient::User { email, .. } | Recipient::Anonymous { email } => email.as_str(),
        }
    }

    #[must_use]
    pub fn user_id(&self) -> Option<Uuid> {
        match self {
            Recipient::User { user_id, .. } => Some(*user_id),
            Recipient::Anonymous { .. } => None,
        }
    }
}

/// Caller-tunable knobs for a single send.
#[derive(Debug, Clone, Default)]
pub struct SendOptions {
    /// Template locale. Defaults to `"en"`.
    pub locale: Option<String>,
    /// Category used for the preference check. Defaults to `"transactional"`.
    pub category: Option<String>,
    /// Recipient display name used by channels that render "Name <email>".
    pub to_name: Option<String>,
    /// Producer-supplied idempotency key. Propagates into the outbox event
    /// headers + the channel-level request for provider de-duplication.
    pub idempotency_key: Option<String>,
}

/// Terminal outcome flags. The worker rewrites the delivery row to `sent` /
/// `failed`; `Queued` means the row is visible with `status='queued'` and the
/// outbox event has been enqueued.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SendOutcome {
    /// Enqueued on the outbox; the worker will deliver shortly.
    Queued,
    /// Preference check rejected — the delivery row is `status='suppressed'`.
    Suppressed,
}

#[derive(Debug, thiserror::Error)]
pub enum NotifyError {
    #[error(transparent)]
    Template(#[from] TemplateError),
    #[error("database error: {0}")]
    Database(#[from] sqlx::Error),
    #[error("outbox error: {0}")]
    Outbox(#[from] outbox::OutboxError),
    /// Propagated from preference / suppression helpers which already return
    /// [`AppError`]. Keeping the wrapper lets us stay typed on the call site
    /// boundary without losing the redacted-problem contract.
    #[error(transparent)]
    App(#[from] crate::error::AppError),
    #[error("recipient e-mail is empty")]
    EmptyRecipient,
}

impl From<NotifyError> for crate::error::AppError {
    fn from(err: NotifyError) -> Self {
        match err {
            NotifyError::Template(e) => e.into(),
            NotifyError::Database(e) => crate::error::AppError::Database(e),
            NotifyError::Outbox(outbox::OutboxError::Database(e)) => {
                crate::error::AppError::Database(e)
            }
            NotifyError::Outbox(outbox::OutboxError::Serialize(e)) => {
                crate::error::AppError::Internal(anyhow::anyhow!("outbox serialization: {e}"))
            }
            NotifyError::App(e) => e,
            NotifyError::EmptyRecipient => {
                crate::error::AppError::BadRequest("recipient e-mail is required".into())
            }
        }
    }
}

/// The combined result of a single send attempt.
#[derive(Debug, Clone)]
pub struct SendResult {
    pub delivery_id: Uuid,
    pub outcome: SendOutcome,
}

/// Send a notification end-to-end.
///
/// Accepts a [`PgPool`] and starts its own transaction — callers that need to
/// enqueue the notification inside an existing transaction can compose the
/// pieces directly via [`insert_delivery`] + [`crate::events::publish_in_tx`]
/// (see `handlers/auth.rs::register` for an example).
pub async fn send_notification(
    pool: &PgPool,
    template_key: &str,
    recipient: &Recipient,
    context: serde_json::Value,
    opts: SendOptions,
) -> Result<SendResult, NotifyError> {
    if recipient.email().trim().is_empty() {
        return Err(NotifyError::EmptyRecipient);
    }
    let locale = opts.locale.as_deref().unwrap_or("en");
    let category = opts.category.as_deref().unwrap_or("transactional");

    // Template is resolved outside the transaction — look-up is idempotent
    // and holding a row lock on a template would serialize every send.
    let template = Template::resolve(pool, template_key, "email", locale).await?;
    let rendered = template.render(&context)?;

    // Preference check only applies when we know the user. The "default open"
    // policy matches the audit plan §2 FDN-05 contract (transactional defaults
    // open; marketing requires explicit opt-in which is taken care of by the
    // category check at the call site).
    let now = Utc::now();
    let mut status = "queued";
    if let Some(user_id) = recipient.user_id() {
        if let Some(pref) = preferences::get_preference(pool, user_id, category, "email").await? {
            if !is_allowed_for_preference(&pref, now) {
                status = "suppressed";
            }
        }
    }

    // Suppression list — same rule for anon + authed recipients.
    if status == "queued" && suppression::is_suppressed(pool, recipient.email()).await? {
        status = "suppressed";
    }

    let mut tx = pool.begin().await?;

    let delivery_id = insert_delivery_inner(
        &mut tx,
        recipient,
        template_key,
        "email",
        rendered.subject.as_deref(),
        &rendered.body,
        status,
    )
    .await?;

    let outcome = if status == "queued" {
        let headers = EventHeaders {
            idempotency_key: opts.idempotency_key.clone(),
            correlation_id: None,
            tenant: None,
        };
        let event = Event {
            aggregate_type: "notification".into(),
            aggregate_id: delivery_id.to_string(),
            event_type: "notification.queued".into(),
            payload: serde_json::json!({
                "delivery_id": delivery_id,
                "template_key": template_key,
                "channel": "email",
                "locale": locale,
            }),
            headers,
        };
        outbox::publish_in_tx(&mut tx, &event).await?;
        SendOutcome::Queued
    } else {
        SendOutcome::Suppressed
    };

    tx.commit().await?;

    Ok(SendResult {
        delivery_id,
        outcome,
    })
}

/// Low-level insert used by [`send_notification`] — exposed for tests and the
/// future `FORM-05` "double-opt-in" adapter that wants to insert the delivery
/// row in its own transaction.
pub async fn insert_delivery_inner(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    recipient: &Recipient,
    template_key: &str,
    channel: &str,
    subject: Option<&str>,
    rendered_body: &str,
    status: &str,
) -> Result<Uuid, sqlx::Error> {
    let (user_id, anon_email) = match recipient {
        Recipient::User { user_id, email } => (Some(*user_id), Some(email.as_str())),
        Recipient::Anonymous { email } => (None, Some(email.as_str())),
    };
    let row: (Uuid,) = sqlx::query_as(
        r#"
        INSERT INTO notification_deliveries
            (user_id, anonymous_email, template_key, channel, status, subject,
             rendered_body, metadata)
        VALUES ($1, $2, $3, $4, $5, $6, $7, '{}'::jsonb)
        RETURNING id
        "#,
    )
    .bind(user_id)
    .bind(anon_email)
    .bind(template_key)
    .bind(channel)
    .bind(status)
    .bind(subject)
    .bind(rendered_body)
    .fetch_one(&mut **tx)
    .await?;
    Ok(row.0)
}

/// Update the delivery row's status. Called by the outbox notify handler
/// after a channel send completes (or fails permanently).
pub async fn mark_delivery_status(
    pool: &PgPool,
    delivery_id: Uuid,
    status: &str,
    provider_id: Option<&str>,
    error_message: Option<&str>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        UPDATE notification_deliveries
        SET status = $2,
            provider_id = COALESCE($3, provider_id),
            metadata = CASE
                WHEN $4 IS NOT NULL THEN metadata || jsonb_build_object('last_error', $4::text)
                ELSE metadata
            END,
            updated_at = NOW()
        WHERE id = $1
        "#,
    )
    .bind(delivery_id)
    .bind(status)
    .bind(provider_id)
    .bind(error_message)
    .execute(pool)
    .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recipient_helpers() {
        let anon = Recipient::Anonymous {
            email: "a@b.co".into(),
        };
        assert_eq!(anon.email(), "a@b.co");
        assert!(anon.user_id().is_none());

        let u = Recipient::User {
            user_id: Uuid::nil(),
            email: "u@b.co".into(),
        };
        assert_eq!(u.email(), "u@b.co");
        assert_eq!(u.user_id(), Some(Uuid::nil()));
    }

    #[test]
    fn send_options_default_values() {
        let o = SendOptions::default();
        assert!(o.locale.is_none());
        assert!(o.category.is_none());
        assert!(o.to_name.is_none());
        assert!(o.idempotency_key.is_none());
    }
}
