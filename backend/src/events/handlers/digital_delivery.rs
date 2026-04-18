//! EC-07: Digital-delivery outbox subscriber.
//!
//! Listens on `order.completed`. For every `order_items` row whose
//! corresponding product is typed `downloadable`, the handler mints a
//! `user_downloads` grant (via [`crate::commerce::downloads::grant_download`])
//! and emits a `user.download.granted` event per grant so downstream
//! mailers can notify the customer.
//!
//! Missing `order_id` in the payload is a [`DispatchError::Permanent`] — the
//! producer contract mandates it. DB errors on the lookup / write path are
//! [`DispatchError::Transient`] so the worker applies backoff + retries;
//! after `max_attempts` the row rolls into DLQ for investigation.

use chrono::Duration;
use sqlx::PgPool;
use uuid::Uuid;

use super::super::dispatcher::DispatchError;
use super::super::outbox::{publish_in_tx, Event, EventHeaders, OutboxRecord};
use super::{BoxFuture, EventHandler};
use crate::commerce::downloads::{self, GrantInput};

/// Default grant window + download quota. Overridable via env for ops
/// tuning.
const DEFAULT_TTL_HOURS: i64 = 24 * 30; // 30 days
const DEFAULT_DOWNLOADS_ALLOWED: i32 = 5;

/// Event subscriber wired against a live `PgPool`. Registered in `main.rs`
/// under the `order.completed` pattern.
#[derive(Debug)]
pub struct DigitalDeliveryHandler {
    pool: PgPool,
    ttl: Duration,
    downloads_allowed: i32,
}

impl DigitalDeliveryHandler {
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        let ttl_hours = std::env::var("DOWNLOAD_GRANT_TTL_HOURS")
            .ok()
            .and_then(|s| s.parse::<i64>().ok())
            .filter(|v| *v > 0)
            .unwrap_or(DEFAULT_TTL_HOURS);
        let downloads_allowed = std::env::var("DOWNLOAD_GRANT_QUOTA")
            .ok()
            .and_then(|s| s.parse::<i32>().ok())
            .filter(|v| *v > 0)
            .unwrap_or(DEFAULT_DOWNLOADS_ALLOWED);
        Self {
            pool,
            ttl: Duration::hours(ttl_hours),
            downloads_allowed,
        }
    }
}

impl EventHandler for DigitalDeliveryHandler {
    fn handle<'a>(&'a self, event: &'a OutboxRecord) -> BoxFuture<'a, Result<(), DispatchError>> {
        Box::pin(async move { dispatch(self, event).await })
    }
}

async fn dispatch(h: &DigitalDeliveryHandler, event: &OutboxRecord) -> Result<(), DispatchError> {
    if event.event_type != "order.completed" {
        return Ok(());
    }

    let order_id = event
        .payload
        .get("order_id")
        .and_then(|v| v.as_str())
        .and_then(|s| Uuid::parse_str(s).ok())
        .ok_or_else(|| DispatchError::Permanent("order.completed missing order_id".into()))?;

    let user_id = event
        .payload
        .get("user_id")
        .and_then(|v| v.as_str())
        .and_then(|s| Uuid::parse_str(s).ok());

    // Resolve user id from the order when the payload does not carry one —
    // guest checkouts have `user_id = NULL` on the order and therefore no
    // download grants to mint. We still return Ok so the outbox advances.
    let user_id = match user_id {
        Some(u) => u,
        None => {
            let row: Option<Uuid> = sqlx::query_scalar(
                r#"SELECT user_id FROM orders WHERE id = $1 AND user_id IS NOT NULL"#,
            )
            .bind(order_id)
            .fetch_optional(&h.pool)
            .await
            .map_err(|e| DispatchError::Transient(format!("load order user_id: {e}")))?;
            match row {
                Some(u) => u,
                None => return Ok(()),
            }
        }
    };

    // Pull every downloadable asset for every order item in a single query.
    let rows = sqlx::query_as::<_, AssetLink>(
        r#"
        SELECT oi.product_id,
               a.id         AS asset_id,
               a.storage_key,
               a.mime_type
        FROM order_items oi
        JOIN products p ON p.id = oi.product_id
        JOIN downloadable_assets a ON a.product_id = oi.product_id
        WHERE oi.order_id = $1
          AND p.product_type = 'downloadable'
        "#,
    )
    .bind(order_id)
    .fetch_all(&h.pool)
    .await
    .map_err(|e| DispatchError::Transient(format!("load downloadable assets: {e}")))?;

    for link in rows {
        let granted = downloads::grant_download(
            &h.pool,
            GrantInput {
                user_id,
                order_id: Some(order_id),
                product_id: link.product_id,
                asset_id: link.asset_id,
                storage_key: &link.storage_key,
                mime_type: &link.mime_type,
                downloads_allowed: h.downloads_allowed,
                ttl: h.ttl,
            },
        )
        .await
        .map_err(|e| DispatchError::Transient(format!("grant_download: {e}")))?;

        // Emit a follow-up event so the mailer / analytics handlers pick
        // this up without tight-coupling. Published in its own transaction
        // so a partial batch still commits the successful grants.
        let mut tx = h
            .pool
            .begin()
            .await
            .map_err(|e| DispatchError::Transient(format!("begin tx: {e}")))?;
        let payload = serde_json::json!({
            "user_id": user_id,
            "order_id": order_id,
            "product_id": link.product_id,
            "asset_id": link.asset_id,
            "grant_id": granted.download.id,
            "token": granted.token,
            "expires_at": granted.download.expires_at,
            "downloads_remaining": granted.download.downloads_remaining,
        });
        let evt = Event {
            aggregate_type: "user_download".into(),
            aggregate_id: granted.download.id.to_string(),
            event_type: "user.download.granted".into(),
            payload,
            headers: EventHeaders::default(),
        };
        publish_in_tx(&mut tx, &evt)
            .await
            .map_err(|e| DispatchError::Transient(format!("publish grant event: {e}")))?;
        tx.commit()
            .await
            .map_err(|e| DispatchError::Transient(format!("commit grant event: {e}")))?;
    }

    Ok(())
}

#[derive(Debug, sqlx::FromRow)]
struct AssetLink {
    product_id: Uuid,
    asset_id: Uuid,
    storage_key: String,
    mime_type: String,
}
