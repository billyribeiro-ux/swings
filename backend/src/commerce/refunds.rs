//! EC-13: Stripe-driven refund mirror.
//!
//! Distinct from [`crate::commerce::orders::OrderRefund`] (admin-driven, EC-05)
//! because not every charge corresponds to a row in `orders` —
//! subscription invoices skip the orders table entirely. When the charge
//! does correspond to an order, [`record_charge_refund`] populates
//! `payment_refunds.order_id` so revenue reports can union both sources.
//!
//! The webhook handler resolves any `order_id`, `subscription_id`, and
//! `user_id` correlation it can before calling [`record_charge_refund`];
//! the writer is otherwise agnostic to which path produced the row.

use sqlx::PgPool;
use uuid::Uuid;

use crate::error::AppResult;

/// Subset of the Stripe `Charge` payload (and its embedded `refunds`) we
/// need to mirror locally for one refund.
#[derive(Debug, Clone)]
pub struct ChargeRefundFields {
    pub stripe_refund_id: String,
    pub stripe_charge_id: Option<String>,
    pub stripe_payment_intent_id: Option<String>,
    pub stripe_customer_id: Option<String>,
    pub stripe_invoice_id: Option<String>,
    pub amount_cents: i64,
    pub currency: String,
    pub reason: Option<String>,
    pub status: String,
}

impl ChargeRefundFields {
    /// Walk the `charge.refunded` event payload (`event.data.object` is a
    /// `charge` whose `refunds.data[]` carries the individual refund rows).
    /// Returns the *latest* refund — the one that triggered this event —
    /// per Stripe's [`charge.refunded`](https://docs.stripe.com/api/events/types#event_types-charge.refunded)
    /// contract.
    pub fn latest_from_charge(charge: &serde_json::Value) -> Option<Self> {
        let stripe_charge_id = charge
            .get("id")
            .and_then(|v| v.as_str())
            .map(str::to_string);
        let stripe_payment_intent_id = charge
            .get("payment_intent")
            .and_then(|v| v.as_str())
            .map(str::to_string);
        let stripe_customer_id = charge
            .get("customer")
            .and_then(|v| v.as_str())
            .map(str::to_string);
        let stripe_invoice_id = charge
            .get("invoice")
            .and_then(|v| v.as_str())
            .map(str::to_string);
        let currency = charge
            .get("currency")
            .and_then(|v| v.as_str())
            .unwrap_or("usd")
            .to_string();
        let refunds = charge
            .get("refunds")
            .and_then(|v| v.get("data"))
            .and_then(|v| v.as_array())?;
        // Stripe orders refunds with the most recent first, but we
        // defensively pick the highest `created` timestamp anyway.
        let latest = refunds
            .iter()
            .max_by_key(|r| r.get("created").and_then(|v| v.as_i64()).unwrap_or(0))?;
        let stripe_refund_id = latest.get("id").and_then(|v| v.as_str())?.to_string();
        let amount_cents = latest.get("amount").and_then(|v| v.as_i64())?;
        let reason = latest
            .get("reason")
            .and_then(|v| v.as_str())
            .map(str::to_string);
        let status = latest
            .get("status")
            .and_then(|v| v.as_str())
            .unwrap_or("succeeded")
            .to_string();
        Some(Self {
            stripe_refund_id,
            stripe_charge_id,
            stripe_payment_intent_id,
            stripe_customer_id,
            stripe_invoice_id,
            amount_cents,
            currency,
            reason,
            status,
        })
    }

    /// Parse a top-level `refund` event payload (`refund.created`,
    /// `refund.updated`). Stripe's modern API surfaces refund details on
    /// the standalone `refund` object — newer accounts (or accounts on
    /// API versions where the embedded `charge.refunds.data[]` array is
    /// not always populated on `charge.refunded`) MUST listen on
    /// `refund.created` to receive every refund.
    ///
    /// Why we need both this AND `latest_from_charge`: Stripe is
    /// migrating refund delivery from the old "embed refunds inside
    /// the charge.refunded event" model to the new "fire dedicated
    /// refund.* events" model. We listen on both so live deployments
    /// don't lose data regardless of which API version their account is
    /// pinned to.
    pub fn from_refund_object(refund: &serde_json::Value) -> Option<Self> {
        let stripe_refund_id = refund.get("id").and_then(|v| v.as_str())?.to_string();
        let amount_cents = refund.get("amount").and_then(|v| v.as_i64())?;
        let currency = refund
            .get("currency")
            .and_then(|v| v.as_str())
            .unwrap_or("usd")
            .to_string();
        let stripe_charge_id = refund
            .get("charge")
            .and_then(|v| v.as_str())
            .map(str::to_string);
        let stripe_payment_intent_id = refund
            .get("payment_intent")
            .and_then(|v| v.as_str())
            .map(str::to_string);
        // The standalone refund object does NOT carry `customer` or
        // `invoice` directly — those live on the parent charge. The
        // webhook handler will hydrate them via Stripe API on demand if
        // the correlation matters; for now they default to None and the
        // refund still gets recorded with whatever IDs the event carried.
        let stripe_customer_id = None;
        let stripe_invoice_id = None;
        let reason = refund
            .get("reason")
            .and_then(|v| v.as_str())
            .map(str::to_string);
        let status = refund
            .get("status")
            .and_then(|v| v.as_str())
            .unwrap_or("succeeded")
            .to_string();
        Some(Self {
            stripe_refund_id,
            stripe_charge_id,
            stripe_payment_intent_id,
            stripe_customer_id,
            stripe_invoice_id,
            amount_cents,
            currency,
            reason,
            status,
        })
    }
}

/// Insert a `payment_refunds` row. Idempotent on `stripe_refund_id`.
///
/// Returns `Some(id)` on insert, `None` when the refund was already
/// recorded (a Stripe replay).
#[allow(clippy::too_many_arguments)]
pub async fn record_charge_refund(
    pool: &PgPool,
    refund: &ChargeRefundFields,
    order_id: Option<Uuid>,
    subscription_id: Option<Uuid>,
    user_id: Option<Uuid>,
) -> AppResult<Option<Uuid>> {
    let row: Option<(Uuid,)> = sqlx::query_as(
        r#"
        INSERT INTO payment_refunds (
            stripe_refund_id, stripe_charge_id, stripe_payment_intent_id,
            stripe_customer_id, stripe_invoice_id, order_id, subscription_id,
            user_id, amount_cents, currency, reason, status
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
        ON CONFLICT (stripe_refund_id) DO NOTHING
        RETURNING id
        "#,
    )
    .bind(&refund.stripe_refund_id)
    .bind(&refund.stripe_charge_id)
    .bind(&refund.stripe_payment_intent_id)
    .bind(&refund.stripe_customer_id)
    .bind(&refund.stripe_invoice_id)
    .bind(order_id)
    .bind(subscription_id)
    .bind(user_id)
    .bind(refund.amount_cents)
    .bind(&refund.currency)
    .bind(&refund.reason)
    .bind(&refund.status)
    .fetch_optional(pool)
    .await?;
    Ok(row.map(|(id,)| id))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture_charge() -> serde_json::Value {
        serde_json::json!({
            "id": "ch_test_1",
            "payment_intent": "pi_test_1",
            "customer": "cus_test_1",
            "invoice": "in_test_1",
            "currency": "usd",
            "refunds": {
                "data": [
                    {
                        "id": "re_test_1",
                        "amount": 1000,
                        "reason": "requested_by_customer",
                        "status": "succeeded",
                        "created": 1_710_000_000_i64,
                    },
                    {
                        "id": "re_test_2",
                        "amount": 500,
                        "reason": "duplicate",
                        "status": "succeeded",
                        "created": 1_710_500_000_i64,
                    }
                ]
            }
        })
    }

    #[test]
    fn latest_from_charge_picks_most_recent() {
        let r = ChargeRefundFields::latest_from_charge(&fixture_charge()).expect("parse");
        assert_eq!(r.stripe_refund_id, "re_test_2");
        assert_eq!(r.amount_cents, 500);
        assert_eq!(r.reason.as_deref(), Some("duplicate"));
    }

    #[test]
    fn latest_from_charge_returns_none_without_refunds_array() {
        let raw = serde_json::json!({ "id": "ch_test_2" });
        assert!(ChargeRefundFields::latest_from_charge(&raw).is_none());
    }
}
