//! Stripe API helpers (billing portal, subscription updates).

use stripe_rust::{
    BillingPortalSession, Client, CreateBillingPortalSession, CustomerId, Subscription,
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
