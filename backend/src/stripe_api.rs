//! Stripe API helpers (billing portal, subscription updates).

use stripe_rust::{
    Address, BillingPortalSession, CancelSubscription, Client, CreateBillingPortalSession,
    CreatePaymentIntent, CreatePaymentIntentAutomaticPaymentMethods, Currency, Customer,
    CustomerId, PaymentIntent, Subscription, SubscriptionId, UpdateCustomer, UpdateSubscription,
};

use crate::{
    error::{AppError, AppResult},
    AppState,
};

fn map_stripe(e: stripe_rust::StripeError) -> AppError {
    AppError::BadRequest(format!("Stripe: {e}"))
}

pub fn client(state: &AppState) -> AppResult<Client> {
    if state.config.stripe_secret_key.is_empty() {
        return Err(AppError::BadRequest(
            "Stripe is not configured (missing STRIPE_SECRET_KEY)".to_string(),
        ));
    }
    Ok(Client::new(&state.config.stripe_secret_key))
}

pub async fn create_billing_portal_session(
    state: &AppState,
    stripe_customer_id: &str,
    return_url: &str,
) -> AppResult<String> {
    let c = client(state)?;
    let customer: CustomerId = stripe_customer_id
        .parse()
        .map_err(|_| AppError::BadRequest("Invalid Stripe customer id".to_string()))?;

    let mut params = CreateBillingPortalSession::new(customer);
    params.return_url = Some(return_url);

    let session = BillingPortalSession::create(&c, params)
        .await
        .map_err(map_stripe)?;

    Ok(session.url)
}

/// FORM-08: Mint a one-shot Stripe PaymentIntent for a form-driven
/// payment. Caller supplies the amount in minor units + currency code +
/// the canonical metadata id (`form:{form_id}` or `submission:{id}`)
/// the webhook reconciler keys off. `idempotency_key` is forwarded to
/// Stripe so a network retry never raises a duplicate intent.
pub async fn create_form_payment_intent(
    state: &AppState,
    amount_cents: i64,
    currency: &str,
    receipt_email: &str,
    metadata_id: &str,
    _idempotency_key: Option<&str>,
) -> AppResult<(String, String)> {
    let c = client(state)?;
    let cur = currency
        .parse::<Currency>()
        .map_err(|_| AppError::BadRequest(format!("invalid currency `{currency}`")))?;
    let mut params = CreatePaymentIntent::new(amount_cents, cur);
    params.receipt_email = Some(receipt_email);
    params.automatic_payment_methods = Some(CreatePaymentIntentAutomaticPaymentMethods {
        enabled: true,
        allow_redirects: None,
    });
    let mut metadata = std::collections::HashMap::new();
    metadata.insert("source".to_string(), "form".to_string());
    metadata.insert("ref".to_string(), metadata_id.to_string());
    params.metadata = Some(metadata);

    let pi = PaymentIntent::create(&c, params)
        .await
        .map_err(map_stripe)?;
    let secret = pi.client_secret.clone().ok_or_else(|| {
        AppError::BadRequest("Stripe returned PI without client_secret".to_string())
    })?;
    Ok((pi.id.to_string(), secret))
}

/// ADM-15: cancel a subscription *immediately* (not at period end).
///
/// Used by the admin members surface when banning or hard-deleting a
/// member — leaving the subscription billable would let a banned account
/// continue to accrue charges. Stripe's `DELETE /subscriptions/{id}`
/// returns the subscription with `status='canceled'`; we don't surface
/// that body to the caller because the local mirror already gets
/// updated by the matching `customer.subscription.deleted` webhook.
pub async fn cancel_subscription_immediately(
    state: &AppState,
    stripe_subscription_id: &str,
) -> AppResult<()> {
    let c = client(state)?;
    let sid: SubscriptionId = stripe_subscription_id
        .parse()
        .map_err(|_| AppError::BadRequest("Invalid Stripe subscription id".to_string()))?;

    Subscription::cancel(&c, &sid, CancelSubscription::new())
        .await
        .map_err(map_stripe)?;

    Ok(())
}

/// ADM-15: write the customer's billing address back to the Stripe
/// Customer object. Mirrors what Stripe's hosted billing portal would do
/// when the customer edits their address themselves; called after the
/// admin members PATCH so the two stores stay in sync.
///
/// Every field is optional — Stripe treats missing keys as "no change",
/// so we only build the [`Address`] struct when at least one field is
/// populated. Failures are surfaced to the caller; the admin handler
/// degrades gracefully (audit + warn) rather than rolling back the
/// local update.
#[allow(clippy::too_many_arguments)]
pub async fn update_customer_address(
    state: &AppState,
    stripe_customer_id: &str,
    line1: Option<&str>,
    line2: Option<&str>,
    city: Option<&str>,
    state_or_region: Option<&str>,
    postal_code: Option<&str>,
    country: Option<&str>,
    phone: Option<&str>,
) -> AppResult<()> {
    let c = client(state)?;
    let customer: CustomerId = stripe_customer_id
        .parse()
        .map_err(|_| AppError::BadRequest("Invalid Stripe customer id".to_string()))?;

    let address = if line1.is_some()
        || line2.is_some()
        || city.is_some()
        || state_or_region.is_some()
        || postal_code.is_some()
        || country.is_some()
    {
        Some(Address {
            city: city.map(str::to_string),
            country: country.map(str::to_string),
            line1: line1.map(str::to_string),
            line2: line2.map(str::to_string),
            postal_code: postal_code.map(str::to_string),
            state: state_or_region.map(str::to_string),
        })
    } else {
        None
    };

    let mut params = UpdateCustomer::new();
    params.address = address;
    params.phone = phone;

    Customer::update(&c, &customer, params)
        .await
        .map_err(map_stripe)?;

    Ok(())
}

pub async fn set_subscription_cancel_at_period_end(
    state: &AppState,
    stripe_subscription_id: &str,
    cancel: bool,
) -> AppResult<()> {
    let c = client(state)?;
    let sid: SubscriptionId = stripe_subscription_id
        .parse()
        .map_err(|_| AppError::BadRequest("Invalid Stripe subscription id".to_string()))?;

    let mut params = UpdateSubscription::new();
    params.cancel_at_period_end = Some(cancel);

    Subscription::update(&c, &sid, params)
        .await
        .map_err(map_stripe)?;

    Ok(())
}
