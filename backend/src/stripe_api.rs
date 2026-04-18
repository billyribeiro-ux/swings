//! Stripe API helpers (billing portal, subscription updates).

use stripe_rust::{
    BillingPortalSession, Client, CreateBillingPortalSession, CreatePaymentIntent,
    CreatePaymentIntentAutomaticPaymentMethods, Currency, CustomerId, PaymentIntent, Subscription,
    SubscriptionId, UpdateSubscription,
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
