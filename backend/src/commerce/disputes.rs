//! EC-13: chargeback (Stripe `Dispute`) ledger.
//!
//! A dispute is opened by `charge.dispute.created`. The current scope only
//! handles `created`; the table is forward-compatible with the rest of the
//! lifecycle (`updated`, `closed`, `funds_reinstated`, `funds_withdrawn`)
//! so future expansion does not require a follow-up migration.
//!
//! **No automatic refund.** Dispute resolution is manual; the writer just
//! mirrors the dispute and (via the webhook handler) flips the related
//! order's `disputed_at` flag so admin views can surface a badge.

use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::error::AppResult;

/// Subset of the Stripe `Dispute` payload we mirror locally.
#[derive(Debug, Clone)]
pub struct DisputeFields {
    pub stripe_dispute_id: String,
    pub stripe_charge_id: Option<String>,
    pub stripe_payment_intent_id: Option<String>,
    pub stripe_customer_id: Option<String>,
    pub amount_cents: i64,
    pub currency: String,
    pub reason: Option<String>,
    pub status: String,
    pub evidence_due_by: Option<DateTime<Utc>>,
    pub is_charge_refundable: bool,
}

impl DisputeFields {
    pub fn from_payload(dispute: &serde_json::Value) -> Option<Self> {
        let stripe_dispute_id = dispute.get("id").and_then(|v| v.as_str())?.to_string();
        let stripe_charge_id = dispute
            .get("charge")
            .and_then(|v| v.as_str())
            .map(str::to_string);
        let stripe_payment_intent_id = dispute
            .get("payment_intent")
            .and_then(|v| v.as_str())
            .map(str::to_string);
        // Stripe nests `customer` under the related charge in some
        // payloads — webhooks deliver the dispute object directly so we
        // accept both shapes.
        let stripe_customer_id = dispute
            .get("customer")
            .and_then(|v| v.as_str())
            .map(str::to_string);
        let amount_cents = dispute.get("amount").and_then(|v| v.as_i64()).unwrap_or(0);
        let currency = dispute
            .get("currency")
            .and_then(|v| v.as_str())
            .unwrap_or("usd")
            .to_string();
        let reason = dispute
            .get("reason")
            .and_then(|v| v.as_str())
            .map(str::to_string);
        let status = dispute
            .get("status")
            .and_then(|v| v.as_str())
            .unwrap_or("warning_needs_response")
            .to_string();
        let evidence_due_by = dispute
            .get("evidence_details")
            .and_then(|v| v.get("due_by"))
            .and_then(|v| v.as_i64())
            .and_then(|ts| DateTime::from_timestamp(ts, 0));
        let is_charge_refundable = dispute
            .get("is_charge_refundable")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        Some(Self {
            stripe_dispute_id,
            stripe_charge_id,
            stripe_payment_intent_id,
            stripe_customer_id,
            amount_cents,
            currency,
            reason,
            status,
            evidence_due_by,
            is_charge_refundable,
        })
    }
}

/// Insert a `payment_disputes` row. Idempotent on `stripe_dispute_id`.
///
/// Returns `Some(id)` on insert, `None` when the dispute is already on
/// file (a Stripe replay).
#[allow(clippy::too_many_arguments)]
pub async fn record_dispute(
    pool: &PgPool,
    dispute: &DisputeFields,
    order_id: Option<Uuid>,
    subscription_id: Option<Uuid>,
    user_id: Option<Uuid>,
) -> AppResult<Option<Uuid>> {
    let row: Option<(Uuid,)> = sqlx::query_as(
        r#"
        INSERT INTO payment_disputes (
            stripe_dispute_id, stripe_charge_id, stripe_payment_intent_id,
            stripe_customer_id, order_id, subscription_id, user_id,
            amount_cents, currency, reason, status, evidence_due_by,
            is_charge_refundable
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
        ON CONFLICT (stripe_dispute_id) DO NOTHING
        RETURNING id
        "#,
    )
    .bind(&dispute.stripe_dispute_id)
    .bind(&dispute.stripe_charge_id)
    .bind(&dispute.stripe_payment_intent_id)
    .bind(&dispute.stripe_customer_id)
    .bind(order_id)
    .bind(subscription_id)
    .bind(user_id)
    .bind(dispute.amount_cents)
    .bind(&dispute.currency)
    .bind(&dispute.reason)
    .bind(&dispute.status)
    .bind(dispute.evidence_due_by)
    .bind(dispute.is_charge_refundable)
    .fetch_optional(pool)
    .await?;
    Ok(row.map(|(id,)| id))
}

/// Mark an order as disputed. Idempotent — re-running with the same
/// `order_id` is a no-op once `disputed_at` is set.
pub async fn flag_order_disputed(pool: &PgPool, order_id: Uuid) -> AppResult<()> {
    sqlx::query(
        r#"
        UPDATE orders
           SET disputed_at = COALESCE(disputed_at, NOW()),
               updated_at = NOW()
         WHERE id = $1
        "#,
    )
    .bind(order_id)
    .execute(pool)
    .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture_dispute() -> serde_json::Value {
        serde_json::json!({
            "id": "dp_test_1",
            "charge": "ch_test_1",
            "payment_intent": "pi_test_1",
            "customer": "cus_test_1",
            "amount": 4900,
            "currency": "usd",
            "reason": "fraudulent",
            "status": "warning_needs_response",
            "evidence_details": { "due_by": 1_712_000_000_i64 },
            "is_charge_refundable": true,
        })
    }

    #[test]
    fn from_payload_extracts_dispute_fields() {
        let d = DisputeFields::from_payload(&fixture_dispute()).expect("parse");
        assert_eq!(d.stripe_dispute_id, "dp_test_1");
        assert_eq!(d.amount_cents, 4900);
        assert_eq!(d.reason.as_deref(), Some("fraudulent"));
        assert!(d.is_charge_refundable);
        assert!(d.evidence_due_by.is_some());
    }

    #[test]
    fn from_payload_returns_none_without_id() {
        let raw = serde_json::json!({ "amount": 0 });
        assert!(DisputeFields::from_payload(&raw).is_none());
    }
}
