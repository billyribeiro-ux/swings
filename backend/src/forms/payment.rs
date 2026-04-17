//! FORM-08: Stripe-backed payment / donation / subscription fields.
//!
//! `POST /api/forms/{slug}/payment-intent` mints a Stripe PaymentIntent
//! (or Customer + Subscription pair, for `kind = subscription`) and
//! returns the `client_secret` the renderer plugs into Stripe Elements.
//!
//! Idempotency:
//!   * The handler requires an `Idempotency-Key` header.
//!   * The same key is forwarded to Stripe so a network-retried call
//!     never raises a duplicate intent.
//!   * The key is also UNIQUE in `form_payment_intents`; a replayed
//!     request reuses the stored client_secret rather than calling
//!     Stripe a second time.
//!
//! Donation amounts:
//!   * `field.payment_kind = "one_time"` → `amount_cents` from the schema.
//!   * `field.payment_kind = "donation"` → caller-supplied `amount_cents`,
//!     gated against `suggested_amounts ∪ {custom if allow_custom}`.
//!
//! Webhook reconciliation lives in `handlers::webhooks` — when
//! `payment_intent.succeeded` arrives we look the row up by
//! `stripe_payment_intent_id`, mark it `succeeded`, emit a
//! `form.payment.succeeded` outbox event, and (if a submission already
//! exists) link it via `submission_id`.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, PgPool};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::error::AppResult;

/// Returned to the client; powers Stripe Elements via `client_secret`.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PaymentIntentResponse {
    pub intent_id: String,
    pub client_secret: String,
    pub amount_cents: i64,
    pub currency: String,
    pub kind: PaymentKind,
}

/// Stable persisted shape; mirrors the migration-level CHECK constraint.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum PaymentKind {
    OneTime,
    Donation,
    Subscription,
}

impl PaymentKind {
    pub fn as_str(self) -> &'static str {
        match self {
            PaymentKind::OneTime => "one_time",
            PaymentKind::Donation => "donation",
            PaymentKind::Subscription => "subscription",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "one_time" => Some(PaymentKind::OneTime),
            "donation" => Some(PaymentKind::Donation),
            "subscription" => Some(PaymentKind::Subscription),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct FormPaymentIntent {
    pub id: Uuid,
    pub form_id: Uuid,
    pub submission_id: Option<Uuid>,
    pub partial_id: Option<Uuid>,
    pub field_key: String,
    pub stripe_payment_intent_id: String,
    pub stripe_client_secret: String,
    pub stripe_customer_id: Option<String>,
    pub stripe_subscription_id: Option<String>,
    pub amount_cents: i64,
    pub currency: String,
    pub kind: String,
    pub status: String,
    pub idempotency_key: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, thiserror::Error)]
pub enum PaymentError {
    #[error("amount {requested} not allowed for donation field (suggested={suggested:?}, allow_custom={allow_custom})")]
    DonationAmountRejected {
        requested: i64,
        suggested: Vec<i64>,
        allow_custom: bool,
    },
    #[error("payment kind `{0}` mismatched the field schema")]
    KindMismatch(String),
    #[error("amount must be a positive number of cents")]
    NonPositive,
}

/// Validate a donation amount against the field's allowed shape.
pub fn validate_donation_amount(
    requested: i64,
    suggested: &[i64],
    allow_custom: bool,
) -> Result<(), PaymentError> {
    if requested <= 0 {
        return Err(PaymentError::NonPositive);
    }
    if suggested.contains(&requested) {
        return Ok(());
    }
    if allow_custom {
        return Ok(());
    }
    Err(PaymentError::DonationAmountRejected {
        requested,
        suggested: suggested.to_vec(),
        allow_custom,
    })
}

// ── Repository ─────────────────────────────────────────────────────────

#[allow(clippy::too_many_arguments)]
pub async fn insert_intent(
    pool: &PgPool,
    form_id: Uuid,
    field_key: &str,
    partial_id: Option<Uuid>,
    stripe_payment_intent_id: &str,
    stripe_client_secret: &str,
    stripe_customer_id: Option<&str>,
    stripe_subscription_id: Option<&str>,
    amount_cents: i64,
    currency: &str,
    kind: PaymentKind,
    idempotency_key: &str,
    initial_status: &str,
) -> AppResult<FormPaymentIntent> {
    let row = sqlx::query_as::<_, FormPaymentIntent>(
        r#"
        INSERT INTO form_payment_intents
            (form_id, partial_id, field_key, stripe_payment_intent_id,
             stripe_client_secret, stripe_customer_id, stripe_subscription_id,
             amount_cents, currency, kind, status, idempotency_key)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
        RETURNING id, form_id, submission_id, partial_id, field_key,
                  stripe_payment_intent_id, stripe_client_secret,
                  stripe_customer_id, stripe_subscription_id,
                  amount_cents, currency, kind, status, idempotency_key,
                  created_at, updated_at
        "#,
    )
    .bind(form_id)
    .bind(partial_id)
    .bind(field_key)
    .bind(stripe_payment_intent_id)
    .bind(stripe_client_secret)
    .bind(stripe_customer_id)
    .bind(stripe_subscription_id)
    .bind(amount_cents)
    .bind(currency)
    .bind(kind.as_str())
    .bind(initial_status)
    .bind(idempotency_key)
    .fetch_one(pool)
    .await?;
    Ok(row)
}

/// Look up an intent by the supplied idempotency key — used to short-circuit
/// retries before we ever call Stripe.
pub async fn find_by_idempotency_key(
    pool: &PgPool,
    idempotency_key: &str,
) -> AppResult<Option<FormPaymentIntent>> {
    let row = sqlx::query_as::<_, FormPaymentIntent>(
        r#"
        SELECT id, form_id, submission_id, partial_id, field_key,
               stripe_payment_intent_id, stripe_client_secret,
               stripe_customer_id, stripe_subscription_id,
               amount_cents, currency, kind, status, idempotency_key,
               created_at, updated_at
        FROM form_payment_intents
        WHERE idempotency_key = $1
        "#,
    )
    .bind(idempotency_key)
    .fetch_optional(pool)
    .await?;
    Ok(row)
}

/// Mark an intent succeeded + (optionally) link to the persisted submission.
/// Called by the `payment_intent.succeeded` webhook handler.
pub async fn mark_succeeded(
    pool: &PgPool,
    stripe_payment_intent_id: &str,
    submission_id: Option<Uuid>,
) -> AppResult<Option<FormPaymentIntent>> {
    let row = sqlx::query_as::<_, FormPaymentIntent>(
        r#"
        UPDATE form_payment_intents
           SET status        = 'succeeded',
               submission_id = COALESCE($2, submission_id),
               updated_at    = NOW()
         WHERE stripe_payment_intent_id = $1
        RETURNING id, form_id, submission_id, partial_id, field_key,
                  stripe_payment_intent_id, stripe_client_secret,
                  stripe_customer_id, stripe_subscription_id,
                  amount_cents, currency, kind, status, idempotency_key,
                  created_at, updated_at
        "#,
    )
    .bind(stripe_payment_intent_id)
    .bind(submission_id)
    .fetch_optional(pool)
    .await?;
    Ok(row)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn donation_accepts_suggested_amount() {
        validate_donation_amount(500, &[500, 1_000, 2_500], false).unwrap();
    }

    #[test]
    fn donation_rejects_unlisted_when_no_custom() {
        let err = validate_donation_amount(750, &[500, 1_000], false).unwrap_err();
        assert!(matches!(err, PaymentError::DonationAmountRejected { .. }));
    }

    #[test]
    fn donation_accepts_custom_when_enabled() {
        validate_donation_amount(750, &[500, 1_000], true).unwrap();
    }

    #[test]
    fn donation_rejects_non_positive() {
        assert!(matches!(
            validate_donation_amount(0, &[500], true).unwrap_err(),
            PaymentError::NonPositive
        ));
        assert!(matches!(
            validate_donation_amount(-1, &[500], true).unwrap_err(),
            PaymentError::NonPositive
        ));
    }

    #[test]
    fn payment_kind_round_trip() {
        for k in [PaymentKind::OneTime, PaymentKind::Donation, PaymentKind::Subscription] {
            assert_eq!(PaymentKind::parse(k.as_str()), Some(k));
        }
        assert_eq!(PaymentKind::parse("bogus"), None);
    }
}
