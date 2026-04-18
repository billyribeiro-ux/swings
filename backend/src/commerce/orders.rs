//! EC-05: Order state machine + repository.
//!
//! The transition table is small and explicit:
//!
//!   pending    → processing | cancelled | failed
//!   processing → on_hold | completed | failed | refunded
//!   on_hold    → processing | cancelled
//!   completed  → refunded
//!   refunded   → (terminal)
//!   cancelled  → (terminal)
//!   failed     → pending     -- retry path; cleared by checkout if user re-pays
//!
//! Anything else is rejected with [`OrderError::IllegalTransition`]. The
//! checkout handler (EC-04) creates orders in `pending`; the webhook
//! reconciler bumps `processing → completed` on
//! `payment_intent.succeeded`. Admin actions (refund / cancel / put on
//! hold) all flow through [`transition`] so the audit trail is uniform.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, PgPool, Postgres, Transaction};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::error::AppResult;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum OrderStatus {
    Pending,
    Processing,
    OnHold,
    Completed,
    Refunded,
    Cancelled,
    Failed,
}

impl OrderStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            OrderStatus::Pending => "pending",
            OrderStatus::Processing => "processing",
            OrderStatus::OnHold => "on_hold",
            OrderStatus::Completed => "completed",
            OrderStatus::Refunded => "refunded",
            OrderStatus::Cancelled => "cancelled",
            OrderStatus::Failed => "failed",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        Some(match s {
            "pending" => OrderStatus::Pending,
            "processing" => OrderStatus::Processing,
            "on_hold" => OrderStatus::OnHold,
            "completed" => OrderStatus::Completed,
            "refunded" => OrderStatus::Refunded,
            "cancelled" => OrderStatus::Cancelled,
            "failed" => OrderStatus::Failed,
            _ => return None,
        })
    }

    pub fn is_terminal(self) -> bool {
        matches!(self, OrderStatus::Refunded | OrderStatus::Cancelled)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum OrderError {
    #[error("order not found: {0}")]
    NotFound(Uuid),
    #[error("illegal transition {from:?} → {to:?}")]
    IllegalTransition { from: OrderStatus, to: OrderStatus },
    #[error("refund {requested_cents} exceeds remaining balance {remaining_cents}")]
    RefundOverbalance {
        requested_cents: i64,
        remaining_cents: i64,
    },
}

/// Returns true if the supplied transition is permitted. Pure function so
/// it can be unit-tested without a DB.
pub fn can_transition(from: OrderStatus, to: OrderStatus) -> bool {
    use OrderStatus::*;
    matches!(
        (from, to),
        (Pending, Processing)
            | (Pending, Cancelled)
            | (Pending, Failed)
            | (Processing, OnHold)
            | (Processing, Completed)
            | (Processing, Failed)
            | (Processing, Refunded)
            | (OnHold, Processing)
            | (OnHold, Cancelled)
            | (Completed, Refunded)
            | (Failed, Pending)
    )
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct Order {
    pub id: Uuid,
    pub number: String,
    pub user_id: Option<Uuid>,
    pub cart_id: Option<Uuid>,
    pub status: String,
    pub currency: String,
    pub subtotal_cents: i64,
    pub discount_cents: i64,
    pub tax_cents: i64,
    pub total_cents: i64,
    pub email: String,
    pub stripe_payment_intent_id: Option<String>,
    pub stripe_customer_id: Option<String>,
    pub idempotency_key: Option<String>,
    pub metadata: serde_json::Value,
    pub placed_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct OrderItem {
    pub id: Uuid,
    pub order_id: Uuid,
    pub product_id: Uuid,
    pub variant_id: Option<Uuid>,
    pub sku: Option<String>,
    pub name: String,
    pub quantity: i32,
    pub unit_price_cents: i64,
    pub line_total_cents: i64,
    pub metadata: serde_json::Value,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct OrderNote {
    pub id: Uuid,
    pub order_id: Uuid,
    pub author_id: Option<Uuid>,
    pub kind: String,
    pub body: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct OrderRefund {
    pub id: Uuid,
    pub order_id: Uuid,
    pub amount_cents: i64,
    pub reason: Option<String>,
    pub stripe_refund_id: Option<String>,
    pub created_by: Option<Uuid>,
    pub created_at: DateTime<Utc>,
}

// ── Sequence helpers ───────────────────────────────────────────────────

/// Mint the next `ORD-YYYY-NNNNNNNN` number. Reads from the per-year
/// sequence created by migration 035; falls back to a 1-based counter
/// when the sequence for the current year is missing (cron creates new
/// sequences each January — the fallback exists to keep tests running
/// without invoking the cron job).
pub async fn next_order_number(pool: &PgPool) -> AppResult<String> {
    let year = Utc::now().format("%Y").to_string();
    let seq = format!("orders_number_seq_{year}");
    let n: i64 = match sqlx::query_scalar::<_, i64>(&format!("SELECT nextval('{seq}')"))
        .fetch_one(pool)
        .await
    {
        Ok(n) => n,
        Err(_) => {
            // Sequence missing — create on the fly + retry. Keeps year-rollover
            // safe even when the cron didn't run.
            sqlx::query(&format!(
                "CREATE SEQUENCE IF NOT EXISTS {seq} INCREMENT 1 START 1 CACHE 50"
            ))
            .execute(pool)
            .await?;
            sqlx::query_scalar::<_, i64>(&format!("SELECT nextval('{seq}')"))
                .fetch_one(pool)
                .await?
        }
    };
    Ok(format!("ORD-{year}-{n:08}"))
}

// ── Repository ─────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct CreateOrderInput<'a> {
    pub user_id: Option<Uuid>,
    pub cart_id: Option<Uuid>,
    pub email: &'a str,
    pub currency: &'a str,
    pub subtotal_cents: i64,
    pub discount_cents: i64,
    pub tax_cents: i64,
    pub total_cents: i64,
    pub idempotency_key: Option<&'a str>,
    pub stripe_payment_intent_id: Option<&'a str>,
    pub stripe_customer_id: Option<&'a str>,
}

pub async fn create_order(pool: &PgPool, input: CreateOrderInput<'_>) -> AppResult<Order> {
    let number = next_order_number(pool).await?;
    let row = sqlx::query_as::<_, Order>(
        r#"
        INSERT INTO orders
            (number, user_id, cart_id, status, currency,
             subtotal_cents, discount_cents, tax_cents, total_cents, email,
             stripe_payment_intent_id, stripe_customer_id, idempotency_key, placed_at)
        VALUES ($1, $2, $3, 'pending', $4, $5, $6, $7, $8, $9, $10, $11, $12, NOW())
        RETURNING id, number, user_id, cart_id, status::text AS status, currency,
                  subtotal_cents, discount_cents, tax_cents, total_cents, email,
                  stripe_payment_intent_id, stripe_customer_id, idempotency_key,
                  metadata, placed_at, completed_at, created_at, updated_at
        "#,
    )
    .bind(number)
    .bind(input.user_id)
    .bind(input.cart_id)
    .bind(input.currency)
    .bind(input.subtotal_cents)
    .bind(input.discount_cents)
    .bind(input.tax_cents)
    .bind(input.total_cents)
    .bind(input.email)
    .bind(input.stripe_payment_intent_id)
    .bind(input.stripe_customer_id)
    .bind(input.idempotency_key)
    .fetch_one(pool)
    .await?;
    Ok(row)
}

pub async fn get_order(pool: &PgPool, id: Uuid) -> AppResult<Option<Order>> {
    let row = sqlx::query_as::<_, Order>(
        r#"
        SELECT id, number, user_id, cart_id, status::text AS status, currency,
               subtotal_cents, discount_cents, tax_cents, total_cents, email,
               stripe_payment_intent_id, stripe_customer_id, idempotency_key,
               metadata, placed_at, completed_at, created_at, updated_at
        FROM orders WHERE id = $1
        "#,
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;
    Ok(row)
}

pub async fn get_order_by_payment_intent(pool: &PgPool, pi: &str) -> AppResult<Option<Order>> {
    let row = sqlx::query_as::<_, Order>(
        r#"
        SELECT id, number, user_id, cart_id, status::text AS status, currency,
               subtotal_cents, discount_cents, tax_cents, total_cents, email,
               stripe_payment_intent_id, stripe_customer_id, idempotency_key,
               metadata, placed_at, completed_at, created_at, updated_at
        FROM orders WHERE stripe_payment_intent_id = $1
        "#,
    )
    .bind(pi)
    .fetch_optional(pool)
    .await?;
    Ok(row)
}

/// Move an order from one status to another, writing a row to
/// `order_state_transitions` in the same TX. Returns the updated order.
pub async fn transition(
    pool: &PgPool,
    order_id: Uuid,
    to: OrderStatus,
    actor_id: Option<Uuid>,
    reason: Option<&str>,
) -> Result<Order, OrderError> {
    let mut tx: Transaction<'_, Postgres> = pool
        .begin()
        .await
        .map_err(|_| OrderError::NotFound(order_id))?;

    let from: String =
        sqlx::query_scalar("SELECT status::text FROM orders WHERE id = $1 FOR UPDATE")
            .bind(order_id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(|_| OrderError::NotFound(order_id))?
            .ok_or(OrderError::NotFound(order_id))?;
    let from_status = OrderStatus::parse(&from).ok_or(OrderError::NotFound(order_id))?;

    if !can_transition(from_status, to) {
        return Err(OrderError::IllegalTransition {
            from: from_status,
            to,
        });
    }

    sqlx::query("UPDATE orders SET status = $2::order_status, updated_at = NOW(), completed_at = CASE WHEN $2 = 'completed' THEN NOW() ELSE completed_at END WHERE id = $1")
        .bind(order_id)
        .bind(to.as_str())
        .execute(&mut *tx)
        .await
        .map_err(|_| OrderError::NotFound(order_id))?;

    sqlx::query(
        "INSERT INTO order_state_transitions (order_id, from_status, to_status, actor_id, reason)
         VALUES ($1, $2::order_status, $3::order_status, $4, $5)",
    )
    .bind(order_id)
    .bind(from_status.as_str())
    .bind(to.as_str())
    .bind(actor_id)
    .bind(reason)
    .execute(&mut *tx)
    .await
    .map_err(|_| OrderError::NotFound(order_id))?;

    let updated: Order = sqlx::query_as::<_, Order>(
        r#"
        SELECT id, number, user_id, cart_id, status::text AS status, currency,
               subtotal_cents, discount_cents, tax_cents, total_cents, email,
               stripe_payment_intent_id, stripe_customer_id, idempotency_key,
               metadata, placed_at, completed_at, created_at, updated_at
        FROM orders WHERE id = $1
        "#,
    )
    .bind(order_id)
    .fetch_one(&mut *tx)
    .await
    .map_err(|_| OrderError::NotFound(order_id))?;

    tx.commit()
        .await
        .map_err(|_| OrderError::NotFound(order_id))?;
    Ok(updated)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pending_can_advance_to_processing() {
        assert!(can_transition(
            OrderStatus::Pending,
            OrderStatus::Processing
        ));
    }

    #[test]
    fn completed_can_only_refund() {
        assert!(can_transition(
            OrderStatus::Completed,
            OrderStatus::Refunded
        ));
        assert!(!can_transition(
            OrderStatus::Completed,
            OrderStatus::Pending
        ));
        assert!(!can_transition(
            OrderStatus::Completed,
            OrderStatus::Cancelled
        ));
    }

    #[test]
    fn refunded_is_terminal() {
        assert!(OrderStatus::Refunded.is_terminal());
        for to in [
            OrderStatus::Pending,
            OrderStatus::Processing,
            OrderStatus::OnHold,
            OrderStatus::Completed,
            OrderStatus::Cancelled,
            OrderStatus::Failed,
        ] {
            assert!(!can_transition(OrderStatus::Refunded, to));
        }
    }

    #[test]
    fn failed_can_retry_back_to_pending() {
        assert!(can_transition(OrderStatus::Failed, OrderStatus::Pending));
    }

    #[test]
    fn status_string_round_trip() {
        for s in [
            OrderStatus::Pending,
            OrderStatus::Processing,
            OrderStatus::OnHold,
            OrderStatus::Completed,
            OrderStatus::Refunded,
            OrderStatus::Cancelled,
            OrderStatus::Failed,
        ] {
            assert_eq!(OrderStatus::parse(s.as_str()), Some(s));
        }
    }
}
