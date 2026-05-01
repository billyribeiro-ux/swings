//! Push catalog pricing changes to Stripe subscriptions (ADM-14 extension).

use crate::{
    db,
    error::{AppError, AppResult},
    models::{
        AdminStripeRolloutFailure, AdminStripeRolloutSummary, PricingPlan,
        PricingStripeRolloutAudience, Subscription, SubscriptionPlan,
    },
    stripe_api::{self, PriceUpdate, PriceUpdateKind, SubscriptionInterval},
    AppState,
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

/// Translate the catalog's free-form `interval` string into the typed
/// Stripe interval enum the wrapper consumes. We accept only the two
/// recurring shapes the rollout supports — `one_time` is rejected here
/// (and earlier, in `subscription_cadence_for_plan`).
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

/// Light validation on the catalog's currency before we hand it to the
/// Stripe wrapper. Kept here (rather than only inside `stripe_api`) so
/// the rollout summary surfaces a meaningful BadRequest rather than a
/// per-target failure burst.
fn validate_currency(plan: &PricingPlan) -> AppResult<()> {
    let lc = plan.currency.trim().to_ascii_lowercase();
    if lc.len() != 3 || !lc.chars().all(|c| c.is_ascii_alphabetic()) {
        return Err(AppError::BadRequest(format!(
            "invalid currency `{}`",
            plan.currency
        )));
    }
    Ok(())
}

async fn update_one_stripe_subscription(
    state: &AppState,
    catalog: &PricingPlan,
    local: &Subscription,
    stripe_idempotency_key: &str,
) -> AppResult<()> {
    let stripe_sub = stripe_api::retrieve_subscription(state, &local.stripe_subscription_id)
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
    let item_id = &items[0].id;

    // Decide which `price_*` we're attaching: a static, pre-created
    // Stripe price (cheaper, preferred path) or an inline price_data
    // shape that lets Stripe author a brand-new price under the hood.
    let update = if let Some(ref price_id) = catalog.stripe_price_id {
        if price_id.is_empty() {
            return Err(AppError::BadRequest(
                "stripe_price_id is set but empty — clear it or paste a valid price_ id".into(),
            ));
        }
        PriceUpdate {
            item_id,
            kind: PriceUpdateKind::StaticPrice { price_id },
        }
    } else {
        let product_id = catalog.stripe_product_id.as_deref().ok_or_else(|| {
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
        validate_currency(catalog)?;
        let interval = stripe_plan_interval(catalog)?;
        PriceUpdate {
            item_id,
            kind: PriceUpdateKind::Inline {
                currency: &catalog.currency,
                product_id,
                interval,
                interval_count: catalog.interval_count.max(1) as u64,
                unit_amount_cents: i64::from(catalog.amount_cents),
            },
        }
    };

    stripe_api::update_subscription_item_price(
        state,
        &local.stripe_subscription_id,
        &update,
        stripe_idempotency_key,
    )
    .await
    .map_err(|e| AppError::BadRequest(format!("Stripe: {e}")))?;
    Ok(())
}

/// After `pricing_plans` has been updated in Postgres, optionally push the new
/// commercial terms to Stripe for every targeted member subscription.
///
/// Subscriptions with `price_protection_enabled = TRUE` are always skipped —
/// those members keep their grandfathered rate regardless of `audience`.
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
    let mut skipped_grandfathered: usize = 0;

    for (idx, row) in targets.iter().enumerate() {
        // Always honour the per-subscription grandfather flag.
        if row.price_protection_enabled {
            skipped_grandfathered += 1;
            continue;
        }
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
        skipped_grandfathered,
        failed,
    })
}

/// Dry-run preview: returns counts without touching Stripe.
/// Used by `GET /api/admin/pricing/plans/{id}/rollout-preview`.
pub async fn preview_rollout(
    state: &AppState,
    catalog: &PricingPlan,
    audience: PricingStripeRolloutAudience,
) -> AppResult<crate::models::PricingRolloutPreview> {
    let cadence = subscription_cadence_for_plan(catalog)?;
    let include_legacy = matches!(
        audience,
        PricingStripeRolloutAudience::LinkedAndUnlinkedLegacySameCadence
    );
    let targets =
        db::list_subscriptions_for_pricing_rollout(&state.db, catalog.id, include_legacy, &cadence)
            .await?;

    let total_in_audience = targets.len();
    let would_skip_grandfathered = targets
        .iter()
        .filter(|s| s.price_protection_enabled)
        .count();
    let would_update = total_in_audience - would_skip_grandfathered;

    Ok(crate::models::PricingRolloutPreview {
        total_in_audience,
        would_update,
        would_skip_grandfathered,
        current_amount_cents: catalog.amount_cents,
        currency: catalog.currency.clone(),
    })
}

#[cfg(test)]
mod tests {
    //! Phase 8.11 — pure-helper coverage. Mocking the Stripe HTTP client
    //! for `rollout_after_plan_save` end-to-end is tracked separately;
    //! this module asserts the deterministic mapping helpers that gate
    //! that workflow, so a regression in cadence/currency/interval
    //! parsing is caught without spinning up the network.
    use super::*;
    use crate::models::SubscriptionPlan;
    use chrono::Utc;
    use uuid::Uuid;

    fn make_plan(interval: &str, currency: &str) -> PricingPlan {
        PricingPlan {
            id: Uuid::nil(),
            name: "Test".into(),
            slug: "test".into(),
            description: None,
            stripe_price_id: None,
            stripe_product_id: None,
            amount_cents: 1000,
            currency: currency.into(),
            interval: interval.into(),
            interval_count: 1,
            trial_days: 0,
            features: serde_json::json!({}),
            highlight_text: None,
            is_popular: false,
            is_active: true,
            sort_order: 0,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    #[test]
    fn cadence_maps_year_to_annual() {
        let plan = make_plan("year", "USD");
        let cadence = subscription_cadence_for_plan(&plan).expect("year is valid");
        assert!(matches!(cadence, SubscriptionPlan::Annual));
    }

    #[test]
    fn cadence_maps_month_to_monthly() {
        let plan = make_plan("month", "USD");
        let cadence = subscription_cadence_for_plan(&plan).expect("month is valid");
        assert!(matches!(cadence, SubscriptionPlan::Monthly));
    }

    #[test]
    fn cadence_rejects_one_time() {
        let plan = make_plan("one_time", "USD");
        let err = subscription_cadence_for_plan(&plan).expect_err("one_time not supported");
        match err {
            AppError::BadRequest(msg) => {
                assert!(msg.contains("one_time"), "msg: {msg}");
            }
            other => panic!("expected BadRequest, got {other:?}"),
        }
    }

    #[test]
    fn cadence_rejects_unknown_interval() {
        let plan = make_plan("decade", "USD");
        let err = subscription_cadence_for_plan(&plan).expect_err("decade is invalid");
        assert!(matches!(err, AppError::BadRequest(_)));
    }

    #[test]
    fn currency_parses_uppercase_iso() {
        let plan = make_plan("month", "usd");
        validate_currency(&plan).expect("usd parses");
    }

    #[test]
    fn currency_rejects_garbage() {
        let plan = make_plan("month", "ZZZZ");
        let err = validate_currency(&plan).expect_err("ZZZZ should fail");
        match err {
            AppError::BadRequest(msg) => assert!(msg.contains("currency"), "msg: {msg}"),
            other => panic!("expected BadRequest, got {other:?}"),
        }
    }

    #[test]
    fn plan_interval_maps_year() {
        let plan = make_plan("year", "USD");
        let i = stripe_plan_interval(&plan).expect("year");
        assert!(matches!(i, SubscriptionInterval::Year));
    }

    #[test]
    fn plan_interval_maps_month() {
        let plan = make_plan("month", "USD");
        let i = stripe_plan_interval(&plan).expect("month");
        assert!(matches!(i, SubscriptionInterval::Month));
    }

    #[test]
    fn plan_interval_rejects_one_time_for_inline_price_data() {
        let plan = make_plan("one_time", "USD");
        let err = stripe_plan_interval(&plan).expect_err("one_time has no recurring shape");
        assert!(matches!(err, AppError::BadRequest(_)));
    }
}
