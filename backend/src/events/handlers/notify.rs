//! Notifications fan-out: bridges outbox `notification.queued` events to the
//! [`crate::notifications`] channel registry.
//!
//! The handler's job is narrow: given an event payload that carries a
//! `delivery_id`, load the corresponding `notification_deliveries` row, look
//! up the right channel by its DB-stored label, call `Channel::send`, and
//! rewrite the delivery row's status. Transient / permanent errors are
//! propagated through [`DispatchError`] so the outbox worker can apply its
//! backoff + DLQ logic.
//!
//! # Why not call the channel directly from `send_notification`?
//!
//! The transactional outbox (FDN-04) is the durability boundary between
//! "domain mutation" and "flaky third-party network call". Delivering inline
//! would violate that separation the moment the SMTP / Resend provider
//! hiccups. By going through the outbox we get at-least-once retries, the
//! admin ops surface (`/api/admin/outbox/{id}/retry`), and metrics for free.

use std::sync::Arc;

use sqlx::PgPool;
use tracing::{debug, warn};
use uuid::Uuid;

use crate::notifications::{send, ChannelRegistry, DeliveryRequest};

use super::super::dispatcher::DispatchError;
use super::super::outbox::OutboxRecord;
use super::{BoxFuture, EventHandler};

/// Event subscriber wired against the live [`ChannelRegistry`].
///
/// The handler matches the outbox event type `"notification.queued"`; see
/// `main.rs` for the registration call.
pub struct NotifyHandler {
    pool: PgPool,
    channels: Arc<ChannelRegistry>,
}

impl NotifyHandler {
    #[must_use]
    pub fn new(pool: PgPool, channels: Arc<ChannelRegistry>) -> Self {
        Self { pool, channels }
    }
}

impl EventHandler for NotifyHandler {
    fn handle<'a>(&'a self, event: &'a OutboxRecord) -> BoxFuture<'a, Result<(), DispatchError>> {
        Box::pin(async move { dispatch(self, event).await })
    }
}

async fn dispatch(h: &NotifyHandler, event: &OutboxRecord) -> Result<(), DispatchError> {
    // Only events whose payload carries a delivery_id are our business; the
    // event_type filter (registered in main.rs) narrows to `notification.*`
    // but payload shape is still runtime-checked so a schema drift cannot
    // panic the worker.
    let delivery_id = event
        .payload
        .get("delivery_id")
        .and_then(|v| v.as_str())
        .and_then(|s| Uuid::parse_str(s).ok())
        .ok_or_else(|| DispatchError::Permanent("notification event missing delivery_id".into()))?;

    let row = sqlx::query_as::<_, DeliveryRow>(
        r#"
        SELECT id, user_id, anonymous_email, template_key, channel, status,
               subject, rendered_body
        FROM notification_deliveries
        WHERE id = $1
        "#,
    )
    .bind(delivery_id)
    .fetch_optional(&h.pool)
    .await
    .map_err(|e| DispatchError::Transient(format!("load delivery row: {e}")))?
    .ok_or_else(|| {
        DispatchError::Permanent(format!("delivery row {delivery_id} no longer exists"))
    })?;

    if row.status != "queued" {
        // Already settled (e.g. admin retried after the original worker
        // delivered). Treat as a no-op success so the outbox does not spin.
        debug!(
            delivery_id = %delivery_id,
            status = %row.status,
            "notification delivery not queued; skipping"
        );
        return Ok(());
    }

    let Some(recipient) = row.anonymous_email.as_deref() else {
        return Err(DispatchError::Permanent(
            "delivery row has no recipient e-mail".into(),
        ));
    };

    let channel = h.channels.get(&row.channel).ok_or_else(|| {
        DispatchError::Permanent(format!("no channel registered for `{}`", row.channel))
    })?;

    let payload_idempotency = event
        .headers
        .get("idempotency_key")
        .and_then(|v| v.as_str());

    let locale = event
        .payload
        .get("locale")
        .and_then(|v| v.as_str())
        .unwrap_or("en");

    let req = DeliveryRequest {
        to: recipient,
        to_name: None,
        template_key: &row.template_key,
        subject: row.subject.as_deref(),
        body: &row.rendered_body,
        locale,
        idempotency_key: payload_idempotency,
    };

    match channel.send(&req).await {
        Ok(provider_id) => {
            if let Err(e) =
                send::mark_delivery_status(&h.pool, delivery_id, "sent", Some(&provider_id), None)
                    .await
            {
                warn!(
                    delivery_id = %delivery_id,
                    error = %e,
                    "failed to mark delivery row as sent"
                );
            }
            Ok(())
        }
        Err(crate::notifications::ChannelError::Permanent(msg)) => {
            if let Err(e) =
                send::mark_delivery_status(&h.pool, delivery_id, "failed", None, Some(&msg)).await
            {
                warn!(
                    delivery_id = %delivery_id,
                    error = %e,
                    "failed to mark delivery row as failed"
                );
            }
            Err(DispatchError::Permanent(msg))
        }
        Err(crate::notifications::ChannelError::Transient(msg)) => {
            Err(DispatchError::Transient(msg))
        }
    }
}

#[derive(Debug, sqlx::FromRow)]
struct DeliveryRow {
    #[allow(dead_code)]
    id: Uuid,
    #[allow(dead_code)]
    user_id: Option<Uuid>,
    anonymous_email: Option<String>,
    template_key: String,
    channel: String,
    status: String,
    subject: Option<String>,
    rendered_body: String,
}
