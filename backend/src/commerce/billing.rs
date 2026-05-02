//! EC-13: Stripe billing webhook persistence.
//!
//! This module is the writer-side of the Stripe billing event family that
//! `handlers::webhooks` dispatches to:
//!
//!   * `invoice.payment_failed`   → [`record_payment_failure`]
//!   * `invoice.paid`             → [`record_invoice_paid`]
//!   * `payment_intent.payment_failed` → [`record_payment_intent_failure`]
//!
//! Tables (all introduced by migration `077_stripe_webhook_expansion.sql`):
//!
//!   * `subscription_invoices` — mirror of Stripe invoice rows.
//!   * `payment_failures`      — per-attempt failure log; sister to
//!     `dunning_attempts` (the *scheduler*).
//!
//! Functions return [`crate::error::AppResult`] so the webhook dispatcher
//! can decide whether to retry (5xx) or absorb the error (200) per the
//! AGENTS.md hard-rule #7 contract.

use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::error::AppResult;

/// Subset of an invoice row we care about — populated from the webhook
/// payload by [`InvoiceFields::from_payload`].
#[derive(Debug, Clone)]
pub struct InvoiceFields {
    pub stripe_invoice_id: String,
    pub stripe_subscription_id: Option<String>,
    pub stripe_customer_id: Option<String>,
    pub status: String,
    pub amount_due_cents: i64,
    pub amount_paid_cents: i64,
    pub currency: String,
    pub attempt_count: i32,
    pub period_start: Option<DateTime<Utc>>,
    pub period_end: Option<DateTime<Utc>>,
    pub paid_at: Option<DateTime<Utc>>,
    /// Stripe-hosted invoice page (member-friendly receipt). Populated by
    /// Stripe once the invoice is finalized. Migration `084_*` adds the
    /// matching column to `subscription_invoices`.
    pub hosted_invoice_url: Option<String>,
    /// Direct link to the rendered PDF for the invoice. Same lifecycle as
    /// `hosted_invoice_url` above.
    pub invoice_pdf: Option<String>,
}

impl InvoiceFields {
    /// Pull the fields we persist out of the raw Stripe invoice payload.
    /// Tolerant of missing keys — Stripe occasionally omits `customer` for
    /// drafts, `paid_at` for unpaid invoices, etc.
    pub fn from_payload(invoice: &serde_json::Value) -> Option<Self> {
        let stripe_invoice_id = invoice.get("id").and_then(|v| v.as_str())?.to_string();
        let status = invoice
            .get("status")
            .and_then(|v| v.as_str())
            .unwrap_or("open")
            .to_string();
        let stripe_subscription_id = invoice
            .get("subscription")
            .and_then(|v| v.as_str())
            .map(str::to_string);
        let stripe_customer_id = invoice
            .get("customer")
            .and_then(|v| v.as_str())
            .map(str::to_string);
        let amount_due_cents = invoice
            .get("amount_due")
            .and_then(|v| v.as_i64())
            .unwrap_or(0);
        let amount_paid_cents = invoice
            .get("amount_paid")
            .and_then(|v| v.as_i64())
            .unwrap_or(0);
        let currency = invoice
            .get("currency")
            .and_then(|v| v.as_str())
            .unwrap_or("usd")
            .to_string();
        let attempt_count = invoice
            .get("attempt_count")
            .and_then(|v| v.as_i64())
            .unwrap_or(0) as i32;
        let period_start = invoice
            .get("period_start")
            .and_then(|v| v.as_i64())
            .and_then(|ts| DateTime::from_timestamp(ts, 0));
        let period_end = invoice
            .get("period_end")
            .and_then(|v| v.as_i64())
            .and_then(|ts| DateTime::from_timestamp(ts, 0));
        // Stripe sets `status_transitions.paid_at` on a paid invoice.
        let paid_at = invoice
            .get("status_transitions")
            .and_then(|v| v.get("paid_at"))
            .and_then(|v| v.as_i64())
            .and_then(|ts| DateTime::from_timestamp(ts, 0));
        // Stripe-hosted receipt + PDF. Both are top-level fields and exist
        // once the invoice is finalized; we tolerate their absence on
        // drafts.
        let hosted_invoice_url = invoice
            .get("hosted_invoice_url")
            .and_then(|v| v.as_str())
            .map(str::to_string);
        let invoice_pdf = invoice
            .get("invoice_pdf")
            .and_then(|v| v.as_str())
            .map(str::to_string);
        Some(Self {
            stripe_invoice_id,
            stripe_subscription_id,
            stripe_customer_id,
            status,
            amount_due_cents,
            amount_paid_cents,
            currency,
            attempt_count,
            period_start,
            period_end,
            paid_at,
            hosted_invoice_url,
            invoice_pdf,
        })
    }
}

/// Upsert a subscription_invoices row. Returns the persisted row's id.
///
/// Idempotent on `stripe_invoice_id`; replays update mutable fields
/// (status, amount_paid, attempt_count, paid_at) so a later
/// `invoice.paid` correctly overlays an earlier `invoice.payment_failed`.
pub async fn upsert_invoice(
    pool: &PgPool,
    invoice: &InvoiceFields,
    subscription_id: Option<Uuid>,
    user_id: Option<Uuid>,
) -> AppResult<Uuid> {
    let row: (Uuid,) = sqlx::query_as(
        r#"
        INSERT INTO subscription_invoices (
            stripe_invoice_id, stripe_subscription_id, stripe_customer_id,
            subscription_id, user_id, status, amount_due_cents, amount_paid_cents,
            currency, attempt_count, period_start, period_end, paid_at,
            hosted_invoice_url, invoice_pdf
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15)
        ON CONFLICT (stripe_invoice_id) DO UPDATE
           SET status             = EXCLUDED.status,
               amount_paid_cents  = EXCLUDED.amount_paid_cents,
               amount_due_cents   = EXCLUDED.amount_due_cents,
               attempt_count      = GREATEST(subscription_invoices.attempt_count,
                                             EXCLUDED.attempt_count),
               paid_at            = COALESCE(EXCLUDED.paid_at, subscription_invoices.paid_at),
               subscription_id    = COALESCE(EXCLUDED.subscription_id, subscription_invoices.subscription_id),
               user_id            = COALESCE(EXCLUDED.user_id, subscription_invoices.user_id),
               hosted_invoice_url = COALESCE(EXCLUDED.hosted_invoice_url,
                                             subscription_invoices.hosted_invoice_url),
               invoice_pdf        = COALESCE(EXCLUDED.invoice_pdf,
                                             subscription_invoices.invoice_pdf),
               updated_at         = NOW()
        RETURNING id
        "#,
    )
    .bind(&invoice.stripe_invoice_id)
    .bind(&invoice.stripe_subscription_id)
    .bind(&invoice.stripe_customer_id)
    .bind(subscription_id)
    .bind(user_id)
    .bind(&invoice.status)
    .bind(invoice.amount_due_cents)
    .bind(invoice.amount_paid_cents)
    .bind(&invoice.currency)
    .bind(invoice.attempt_count)
    .bind(invoice.period_start)
    .bind(invoice.period_end)
    .bind(invoice.paid_at)
    .bind(&invoice.hosted_invoice_url)
    .bind(&invoice.invoice_pdf)
    .fetch_one(pool)
    .await?;
    Ok(row.0)
}

/// Insert a `payment_failures` row.
///
/// `stripe_event_id` is the dedupe key — the column is `UNIQUE` so a replay
/// of the same event never duplicates the dunning row.
#[allow(clippy::too_many_arguments)]
pub async fn record_payment_failure(
    pool: &PgPool,
    stripe_event_id: &str,
    stripe_invoice_id: Option<&str>,
    stripe_payment_intent_id: Option<&str>,
    stripe_customer_id: Option<&str>,
    subscription_id: Option<Uuid>,
    user_id: Option<Uuid>,
    amount_cents: Option<i64>,
    currency: Option<&str>,
    failure_code: Option<&str>,
    failure_message: Option<&str>,
    attempt_count: i32,
    next_payment_attempt: Option<DateTime<Utc>>,
    final_attempt: bool,
) -> AppResult<Option<Uuid>> {
    let row: Option<(Uuid,)> = sqlx::query_as(
        r#"
        INSERT INTO payment_failures (
            stripe_event_id, stripe_invoice_id, stripe_payment_intent_id,
            stripe_customer_id, subscription_id, user_id, amount_cents,
            currency, failure_code, failure_message, attempt_count,
            next_payment_attempt, final
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
        ON CONFLICT (stripe_event_id) DO NOTHING
        RETURNING id
        "#,
    )
    .bind(stripe_event_id)
    .bind(stripe_invoice_id)
    .bind(stripe_payment_intent_id)
    .bind(stripe_customer_id)
    .bind(subscription_id)
    .bind(user_id)
    .bind(amount_cents)
    .bind(currency)
    .bind(failure_code)
    .bind(failure_message)
    .bind(attempt_count)
    .bind(next_payment_attempt)
    .bind(final_attempt)
    .fetch_optional(pool)
    .await?;
    Ok(row.map(|(id,)| id))
}

/// Bump a subscription's status. Used by the invoice handlers to swap
/// between active / past_due / unpaid as Stripe walks through retries.
///
/// Returns `true` when the row existed and was updated, `false` otherwise.
pub async fn set_subscription_status_by_stripe_id(
    pool: &PgPool,
    stripe_subscription_id: &str,
    status: &str,
) -> AppResult<bool> {
    let res = sqlx::query(
        r#"
        UPDATE subscriptions
           SET status = $2::subscription_status, updated_at = NOW()
         WHERE stripe_subscription_id = $1
        "#,
    )
    .bind(stripe_subscription_id)
    .bind(status)
    .execute(pool)
    .await?;
    Ok(res.rows_affected() > 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture_invoice() -> serde_json::Value {
        serde_json::json!({
            "id": "in_test_1",
            "subscription": "sub_test_1",
            "customer": "cus_test_1",
            "status": "paid",
            "amount_due": 4900,
            "amount_paid": 4900,
            "currency": "usd",
            "attempt_count": 1,
            "period_start": 1_710_000_000,
            "period_end":   1_712_592_000,
            "status_transitions": { "paid_at": 1_710_500_000 },
        })
    }

    #[test]
    fn invoice_fields_from_payload_extracts_paid_invoice() {
        let inv = InvoiceFields::from_payload(&fixture_invoice()).expect("parse");
        assert_eq!(inv.stripe_invoice_id, "in_test_1");
        assert_eq!(inv.status, "paid");
        assert_eq!(inv.amount_due_cents, 4900);
        assert_eq!(inv.amount_paid_cents, 4900);
        assert_eq!(inv.attempt_count, 1);
        assert!(inv.paid_at.is_some());
    }

    #[test]
    fn invoice_fields_tolerates_missing_optional_fields() {
        let raw = serde_json::json!({
            "id": "in_test_2",
            "amount_due": 0,
            "currency": "usd",
        });
        let inv = InvoiceFields::from_payload(&raw).expect("parse");
        assert_eq!(inv.stripe_invoice_id, "in_test_2");
        assert_eq!(inv.status, "open");
        assert!(inv.stripe_customer_id.is_none());
        assert!(inv.paid_at.is_none());
    }

    #[test]
    fn invoice_fields_returns_none_when_id_missing() {
        let raw = serde_json::json!({ "status": "open" });
        assert!(InvoiceFields::from_payload(&raw).is_none());
    }
}
