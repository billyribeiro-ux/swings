//! Push catalog pricing changes to Stripe subscriptions (ADM-14 extension).

use stripe_rust::{
    Currency, RequestStrategy, Subscription as StripeSubscription, SubscriptionId,
    SubscriptionInterval, SubscriptionPriceData, SubscriptionPriceDataRecurring,
    UpdateSubscription, UpdateSubscriptionItems,
};

use crate::{
    db,
    error::{AppError, AppResult},
    models::{
        AdminStripeRolloutFailure, AdminStripeRolloutSummary, PricingPlan,
        PricingStripeRolloutAudience, Subscription, SubscriptionPlan,
    },
    stripe_api, AppState,
};

pub(crate) fn subscription_cadence_for_plan(plan: &PricingPlan) -> AppResult<SubscriptionPlan> {
    match plan.interval.as_str() {
        "year" => Ok(SubscriptionPlan::Annual),
        "month" => Ok(SubscriptionPlan::Monthly),
        "one_time" => Err(AppError::BadRequest(
            "Stripe subscription rollout is not supported for one_time catalog plans".into(),
        )),
        _ => Err(AppError::BadRequest(format!(
            "Unknown catalog interval `{}`",
            plan.interval
        ))),
    }
}

fn stripe_currency(plan: &PricingPlan) -> AppResult<Currency> {
    plan.currency
        .parse::<Currency>()
        .map_err(|_| AppError::BadRequest(format!("invalid currency `{}`", plan.currency)))
}

fn stripe_plan_interval(plan: &PricingPlan) -> AppResult<SubscriptionInterval> {
    match plan.interval.as_str() {
        "year" => Ok(SubscriptionInterval::Year),
        "month" => Ok(SubscriptionInterval::Month),
        _ => Err(AppError::BadRequest(format!(
            "Unsupported interval `{}` for Stripe price_data",
            plan.interval
        ))),
    }
}

async fn update_one_stripe_subscription(
    state: &AppState,
    catalog: &PricingPlan,
    local: &Subscription,
    stripe_idempotency_key: &str,
) -> AppResult<()> {
    let c = stripe_api::client(state)?.with_strategy(RequestStrategy::Idempotent(
        stripe_idempotency_key.to_string(),
    ));
    let sid: SubscriptionId = local
        .stripe_subscription_id
        .parse()
        .map_err(|_| AppError::BadRequest("Invalid Stripe subscription id".into()))?;

    let stripe_sub = StripeSubscription::retrieve(&c, &sid, &["items.data"])
        .await
        .map_err(|e| AppError::BadRequest(format!("Stripe: {e}")))?;

    let items = &stripe_sub.items.data;
    if items.len() != 1 {
        return Err(AppError::BadRequest(format!(
            "Subscription {} has {} line items; rollout supports exactly one",
            local.stripe_subscription_id,
            items.len()
        )));
    }
    let item_id = items[0].id.to_string();

    let mut params = UpdateSubscription::new();
    // Proration: rely on Stripe’s default for subscription updates (`create_prorations`).
    // `async-stripe` exposes duplicate `SubscriptionProrationBehavior` types on the
    // subscription vs subscription_item modules; we avoid wiring the wrong enum here.

    if let Some(ref price_id) = catalog.stripe_price_id {
        if price_id.is_empty() {
            return Err(AppError::BadRequest(
                "stripe_price_id is set but empty — clear it or paste a valid price_ id".into(),
            ));
        }
        params.items = Some(vec![UpdateSubscriptionItems {
            id: Some(item_id),
            price: Some(price_id.clone()),
            ..UpdateSubscriptionItems::default()
        }]);
    } else {
        let product_id = catalog.stripe_product_id.clone().ok_or_else(|| {
            AppError::BadRequest(
                "Dynamic catalog price (no stripe_price_id) requires stripe_product_id \
                 on the plan so Stripe can attach an inline recurring price."
                    .into(),
            )
        })?;
        if product_id.is_empty() {
            return Err(AppError::BadRequest(
                "stripe_product_id is required for dynamic-price rollout".into(),
            ));
        }
        let cur = stripe_currency(catalog)?;
        let interval = stripe_plan_interval(catalog)?;
        let recurring = SubscriptionPriceDataRecurring {
            interval,
            interval_count: Some(catalog.interval_count.max(1) as u64),
        };
        let price_data = SubscriptionPriceData {
            currency: cur,
            product: product_id,
            recurring,
            unit_amount: Some(i64::from(catalog.amount_cents)),
            ..SubscriptionPriceData::default()
        };
        params.items = Some(vec![UpdateSubscriptionItems {
            id: Some(item_id),
            price_data: Some(price_data),
            ..UpdateSubscriptionItems::default()
        }]);
    }

    StripeSubscription::update(&c, &sid, params)
        .await
        .map_err(|e| AppError::BadRequest(format!("Stripe: {e}")))?;
    Ok(())
}

/// After `pricing_plans` has been updated in Postgres, optionally push the new
/// commercial terms to Stripe for every targeted member subscription.
pub async fn rollout_after_plan_save(
    state: &AppState,
    catalog: &PricingPlan,
    audience: PricingStripeRolloutAudience,
    admin_idempotency_key: &str,
) -> AppResult<AdminStripeRolloutSummary> {
    let cadence = subscription_cadence_for_plan(catalog)?;
    let include_legacy = matches!(
        audience,
        PricingStripeRolloutAudience::LinkedAndUnlinkedLegacySameCadence
    );
    let targets =
        db::list_subscriptions_for_pricing_rollout(&state.db, catalog.id, include_legacy, &cadence)
            .await?;

    let mut failed = Vec::new();
    let mut succeeded: usize = 0;

    for (idx, row) in targets.iter().enumerate() {
        let key = format!("{}-rollout-{}-{}", admin_idempotency_key, catalog.id, idx);
        match update_one_stripe_subscription(state, catalog, row, &key).await {
            Ok(()) => succeeded += 1,
            Err(e) => failed.push(AdminStripeRolloutFailure {
                stripe_subscription_id: row.stripe_subscription_id.clone(),
                user_id: row.user_id,
                error: e.to_string(),
            }),
        }
    }

    Ok(AdminStripeRolloutSummary {
        targeted: targets.len(),
        succeeded,
        failed,
    })
}
