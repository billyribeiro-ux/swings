use axum::{
    extract::{Path, State},
    http::HeaderMap,
    routing::{get, post, put},
    Json, Router,
};
use uuid::Uuid;
use validator::Validate;

use crate::{
    error::{AppError, AppResult},
    extractors::{AdminUser, ClientInfo},
    models::*,
    services::{
        audit::audit_admin,
        pricing_rollout::rollout_after_plan_save,
    },
    AppState,
};

// ── Admin Pricing Router ──────────────────────────────────────────────

pub fn admin_router() -> Router<AppState> {
    Router::new()
        .route("/plans", get(admin_list_plans))
        .route("/plans", post(admin_create_plan))
        .route("/plans/price-log", get(admin_plan_price_change_log))
        .route("/plans/{id}", get(admin_get_plan))
        .route("/plans/{id}", put(admin_update_plan))
        .route("/plans/{id}", axum::routing::delete(admin_delete_plan))
        .route("/plans/{id}/toggle", post(admin_toggle_plan))
        .route("/plans/{id}/history", get(admin_plan_history))
}

// ── Public Pricing Router ─────────────────────────────────────────────

pub fn public_router() -> Router<AppState> {
    Router::new().route("/plans", get(public_list_plans))
}

// ── Helpers ───────────────────────────────────────────────────────────

fn slugify(s: &str) -> String {
    s.to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

#[derive(serde::Serialize)]
struct PlanWithHistory {
    #[serde(flatten)]
    plan: PricingPlan,
    change_history: Vec<PricingChangeLog>,
}

// ── Admin Handlers ────────────────────────────────────────────────────

async fn admin_list_plans(
    State(state): State<AppState>,
    _admin: AdminUser,
) -> AppResult<Json<Vec<PricingPlan>>> {
    let plans = sqlx::query_as::<_, PricingPlan>(
        r#"
        SELECT id, name, slug, description, stripe_price_id, stripe_product_id,
               amount_cents, currency, interval, interval_count, trial_days,
               features, highlight_text, is_popular, is_active, sort_order,
               created_at, updated_at
        FROM pricing_plans
        ORDER BY sort_order ASC, created_at ASC
        "#,
    )
    .fetch_all(&state.db)
    .await?;

    Ok(Json(plans))
}

#[utoipa::path(
    post,
    path = "/api/admin/pricing/plans",
    tag = "pricing",
    security(("bearer_auth" = [])),
    request_body = CreatePricingPlanRequest,
    responses(
        (status = 200, description = "Plan created", body = PricingPlan),
        (status = 403, description = "Forbidden"),
        (status = 422, description = "Validation error")
    )
)]
pub(crate) async fn admin_create_plan(
    State(state): State<AppState>,
    admin: AdminUser,
    client: ClientInfo,
    Json(req): Json<CreatePricingPlanRequest>,
) -> AppResult<Json<PricingPlan>> {
    req.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;

    let slug = req
        .slug
        .as_deref()
        .map(slugify)
        .unwrap_or_else(|| slugify(&req.name));
    let currency = req.currency.as_deref().unwrap_or("usd");
    let interval = req.interval.as_deref().unwrap_or("month");
    let interval_count = req.interval_count.unwrap_or(1);
    let trial_days = req.trial_days.unwrap_or(0);
    let features = req.features.clone().unwrap_or(serde_json::json!([]));
    let is_popular = req.is_popular.unwrap_or(false);
    let is_active = req.is_active.unwrap_or(true);
    let sort_order = req.sort_order.unwrap_or(0);

    let plan = sqlx::query_as::<_, PricingPlan>(
        r#"
        INSERT INTO pricing_plans
            (name, slug, description, stripe_price_id, stripe_product_id,
             amount_cents, currency, interval, interval_count, trial_days,
             features, highlight_text, is_popular, is_active, sort_order)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15)
        RETURNING id, name, slug, description, stripe_price_id, stripe_product_id,
                  amount_cents, currency, interval, interval_count, trial_days,
                  features, highlight_text, is_popular, is_active, sort_order,
                  created_at, updated_at
        "#,
    )
    .bind(&req.name)
    .bind(&slug)
    .bind(&req.description)
    .bind(&req.stripe_price_id)
    .bind(&req.stripe_product_id)
    .bind(req.amount_cents)
    .bind(currency)
    .bind(interval)
    .bind(interval_count)
    .bind(trial_days)
    .bind(&features)
    .bind(&req.highlight_text)
    .bind(is_popular)
    .bind(is_active)
    .bind(sort_order)
    .fetch_one(&state.db)
    .await?;

    audit_admin(
        &state.db,
        &admin,
        &client,
        "pricing_plan.create",
        "pricing_plan",
        plan.id,
        serde_json::json!({
            "slug": plan.slug,
            "amount_cents": plan.amount_cents,
            "currency": plan.currency,
            "interval": plan.interval,
        }),
    )
    .await;

    Ok(Json(plan))
}

async fn admin_get_plan(
    State(state): State<AppState>,
    _admin: AdminUser,
    Path(id): Path<Uuid>,
) -> AppResult<Json<PlanWithHistory>> {
    let plan = sqlx::query_as::<_, PricingPlan>(
        r#"
        SELECT id, name, slug, description, stripe_price_id, stripe_product_id,
               amount_cents, currency, interval, interval_count, trial_days,
               features, highlight_text, is_popular, is_active, sort_order,
               created_at, updated_at
        FROM pricing_plans
        WHERE id = $1
        "#,
    )
    .bind(id)
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NotFound("Pricing plan not found".to_string()))?;

    let change_history = sqlx::query_as::<_, PricingChangeLog>(
        r#"
        SELECT id, plan_id, field_changed, old_value, new_value, changed_by, changed_at
        FROM pricing_change_log
        WHERE plan_id = $1
        ORDER BY changed_at DESC
        "#,
    )
    .bind(id)
    .fetch_all(&state.db)
    .await?;

    Ok(Json(PlanWithHistory {
        plan,
        change_history,
    }))
}

#[utoipa::path(
    put,
    path = "/api/admin/pricing/plans/{id}",
    tag = "pricing",
    security(("bearer_auth" = [])),
    params(("id" = Uuid, Path, description = "Plan id")),
    request_body = UpdatePricingPlanRequest,
    responses(
        (status = 200, description = "Plan updated", body = AdminUpdatePricingPlanResponse),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Plan not found")
    )
)]
pub(crate) async fn admin_update_plan(
    State(state): State<AppState>,
    admin: AdminUser,
    client: ClientInfo,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdatePricingPlanRequest>,
) -> AppResult<Json<AdminUpdatePricingPlanResponse>> {
    let existing = sqlx::query_as::<_, PricingPlan>(
        r#"
        SELECT id, name, slug, description, stripe_price_id, stripe_product_id,
               amount_cents, currency, interval, interval_count, trial_days,
               features, highlight_text, is_popular, is_active, sort_order,
               created_at, updated_at
        FROM pricing_plans
        WHERE id = $1
        "#,
    )
    .bind(id)
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NotFound("Pricing plan not found".to_string()))?;

    // Collect changes for the change log
    let mut changes: Vec<(&str, String, String)> = Vec::new();

    if let Some(ref name) = req.name {
        if *name != existing.name {
            changes.push(("name", existing.name.clone(), name.clone()));
        }
    }
    if let Some(ref slug) = req.slug {
        let new_slug = slugify(slug);
        if new_slug != existing.slug {
            changes.push(("slug", existing.slug.clone(), new_slug));
        }
    }
    if let Some(ref description) = req.description {
        let old = existing.description.clone().unwrap_or_default();
        if *description != old {
            changes.push(("description", old, description.clone()));
        }
    }
    if let Some(ref stripe_price_id) = req.stripe_price_id {
        let old = existing.stripe_price_id.clone().unwrap_or_default();
        if *stripe_price_id != old {
            changes.push(("stripe_price_id", old, stripe_price_id.clone()));
        }
    }
    if let Some(ref stripe_product_id) = req.stripe_product_id {
        let old = existing.stripe_product_id.clone().unwrap_or_default();
        if *stripe_product_id != old {
            changes.push(("stripe_product_id", old, stripe_product_id.clone()));
        }
    }
    if let Some(amount_cents) = req.amount_cents {
        if amount_cents != existing.amount_cents {
            changes.push((
                "amount_cents",
                existing.amount_cents.to_string(),
                amount_cents.to_string(),
            ));
        }
    }
    if let Some(ref currency) = req.currency {
        if *currency != existing.currency {
            changes.push(("currency", existing.currency.clone(), currency.clone()));
        }
    }
    if let Some(ref interval) = req.interval {
        if *interval != existing.interval {
            changes.push(("interval", existing.interval.clone(), interval.clone()));
        }
    }
    if let Some(interval_count) = req.interval_count {
        if interval_count != existing.interval_count {
            changes.push((
                "interval_count",
                existing.interval_count.to_string(),
                interval_count.to_string(),
            ));
        }
    }
    if let Some(trial_days) = req.trial_days {
        if trial_days != existing.trial_days {
            changes.push((
                "trial_days",
                existing.trial_days.to_string(),
                trial_days.to_string(),
            ));
        }
    }
    if let Some(ref features) = req.features {
        let old_str = existing.features.to_string();
        let new_str = features.to_string();
        if old_str != new_str {
            changes.push(("features", old_str, new_str));
        }
    }
    if let Some(ref highlight_text) = req.highlight_text {
        let old = existing.highlight_text.clone().unwrap_or_default();
        if *highlight_text != old {
            changes.push(("highlight_text", old, highlight_text.clone()));
        }
    }
    if let Some(is_popular) = req.is_popular {
        if is_popular != existing.is_popular {
            changes.push((
                "is_popular",
                existing.is_popular.to_string(),
                is_popular.to_string(),
            ));
        }
    }
    if let Some(is_active) = req.is_active {
        if is_active != existing.is_active {
            changes.push((
                "is_active",
                existing.is_active.to_string(),
                is_active.to_string(),
            ));
        }
    }
    if let Some(sort_order) = req.sort_order {
        if sort_order != existing.sort_order {
            changes.push((
                "sort_order",
                existing.sort_order.to_string(),
                sort_order.to_string(),
            ));
        }
    }

    // Apply the update
    let name = req.name.as_deref().unwrap_or(&existing.name);
    let slug = req
        .slug
        .as_deref()
        .map(slugify)
        .unwrap_or(existing.slug.clone());
    let description = req
        .description
        .as_deref()
        .or(existing.description.as_deref());
    let stripe_price_id = req
        .stripe_price_id
        .as_deref()
        .or(existing.stripe_price_id.as_deref());
    let stripe_product_id = req
        .stripe_product_id
        .as_deref()
        .or(existing.stripe_product_id.as_deref());
    let amount_cents = req.amount_cents.unwrap_or(existing.amount_cents);
    let currency = req.currency.as_deref().unwrap_or(&existing.currency);
    let interval = req.interval.as_deref().unwrap_or(&existing.interval);
    let interval_count = req.interval_count.unwrap_or(existing.interval_count);
    let trial_days = req.trial_days.unwrap_or(existing.trial_days);
    let features = req.features.clone().unwrap_or(existing.features.clone());
    let highlight_text = req
        .highlight_text
        .as_deref()
        .or(existing.highlight_text.as_deref());
    let is_popular = req.is_popular.unwrap_or(existing.is_popular);
    let is_active = req.is_active.unwrap_or(existing.is_active);
    let sort_order = req.sort_order.unwrap_or(existing.sort_order);

    let updated = sqlx::query_as::<_, PricingPlan>(
        r#"
        UPDATE pricing_plans
        SET name = $1, slug = $2, description = $3, stripe_price_id = $4,
            stripe_product_id = $5, amount_cents = $6, currency = $7,
            interval = $8, interval_count = $9, trial_days = $10,
            features = $11, highlight_text = $12, is_popular = $13,
            is_active = $14, sort_order = $15, updated_at = NOW()
        WHERE id = $16
        RETURNING id, name, slug, description, stripe_price_id, stripe_product_id,
                  amount_cents, currency, interval, interval_count, trial_days,
                  features, highlight_text, is_popular, is_active, sort_order,
                  created_at, updated_at
        "#,
    )
    .bind(name)
    .bind(&slug)
    .bind(description)
    .bind(stripe_price_id)
    .bind(stripe_product_id)
    .bind(amount_cents)
    .bind(currency)
    .bind(interval)
    .bind(interval_count)
    .bind(trial_days)
    .bind(&features)
    .bind(highlight_text)
    .bind(is_popular)
    .bind(is_active)
    .bind(sort_order)
    .bind(id)
    .fetch_one(&state.db)
    .await?;

    // Insert change log entries
    for (field, old_val, new_val) in &changes {
        sqlx::query(
            r#"
            INSERT INTO pricing_change_log (plan_id, field_changed, old_value, new_value, changed_by)
            VALUES ($1, $2, $3, $4, $5)
            "#,
        )
        .bind(id)
        .bind(field)
        .bind(old_val)
        .bind(new_val)
        .bind(admin.user_id)
        .execute(&state.db)
        .await?;
    }

    let changed_fields: Vec<&str> = changes.iter().map(|(f, _, _)| *f).collect();

    let rollout_cfg = req.stripe_rollout.clone().unwrap_or_default();
    let price_touching_change = changes.iter().any(|(f, _, _)| {
        matches!(
            *f,
            "amount_cents"
                | "stripe_price_id"
                | "currency"
                | "interval"
                | "interval_count"
        )
    });

    let mut stripe_rollout_summary: Option<AdminStripeRolloutSummary> = None;

    if rollout_cfg.push_to_stripe_subscriptions {
        if !price_touching_change {
            return Err(AppError::BadRequest(
                "stripe_rollout.push_to_stripe_subscriptions requires a billing field change \
                 (amount_cents, stripe_price_id, currency, interval, or interval_count)"
                    .into(),
            ));
        }
        let key = headers
            .get("idempotency-key")
            .and_then(|v| v.to_str().ok())
            .map(str::trim)
            .filter(|s| (8..=255).contains(&s.len()))
            .ok_or_else(|| {
                AppError::BadRequest(
                    "Idempotency-Key header (8..=255 chars) is required when pushing prices to Stripe"
                        .into(),
                )
            })?;

        let summary = rollout_after_plan_save(&state, &updated, rollout_cfg.audience, key).await?;
        stripe_rollout_summary = Some(summary.clone());

        audit_admin(
            &state.db,
            &admin,
            &client,
            "pricing_plan.stripe_rollout",
            "pricing_plan",
            updated.id,
            serde_json::json!({
                "slug": updated.slug,
                "targeted": summary.targeted,
                "succeeded": summary.succeeded,
                "failed": summary.failed.len(),
            }),
        )
        .await;
    }

    audit_admin(
        &state.db,
        &admin,
        &client,
        "pricing_plan.update",
        "pricing_plan",
        updated.id,
        serde_json::json!({
            "slug": updated.slug,
            "fields_changed": changed_fields,
            "is_active": updated.is_active,
        }),
    )
    .await;

    Ok(Json(AdminUpdatePricingPlanResponse {
        plan: updated,
        stripe_rollout: stripe_rollout_summary,
    }))
}

#[utoipa::path(
    get,
    path = "/api/admin/pricing/plans/price-log",
    tag = "pricing",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Recent amount_cents changes", body = Vec<PricingPlanAmountChangeLogEntry>),
        (status = 403, description = "Forbidden")
    )
)]
pub(crate) async fn admin_plan_price_change_log(
    State(state): State<AppState>,
    _admin: AdminUser,
) -> AppResult<Json<Vec<PricingPlanAmountChangeLogEntry>>> {
    let rows = sqlx::query_as::<_, PricingPlanAmountChangeLogEntry>(
        r#"
        SELECT l.id,
               p.name AS plan_name,
               CAST(l.old_value AS INTEGER) AS old_amount_cents,
               CAST(l.new_value AS INTEGER) AS new_amount_cents,
               l.changed_at,
               u.email AS changed_by
        FROM pricing_change_log l
        JOIN pricing_plans p ON p.id = l.plan_id
        JOIN users u ON u.id = l.changed_by
        WHERE l.field_changed = 'amount_cents'
          AND l.old_value IS NOT NULL
          AND l.new_value IS NOT NULL
        ORDER BY l.changed_at DESC
        LIMIT 500
        "#,
    )
    .fetch_all(&state.db)
    .await?;

    Ok(Json(rows))
}

#[utoipa::path(
    delete,
    path = "/api/admin/pricing/plans/{id}",
    tag = "pricing",
    security(("bearer_auth" = [])),
    params(("id" = Uuid, Path, description = "Plan id")),
    responses(
        (status = 200, description = "Plan deleted"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Plan not found")
    )
)]
pub(crate) async fn admin_delete_plan(
    State(state): State<AppState>,
    admin: AdminUser,
    client: ClientInfo,
    Path(id): Path<Uuid>,
) -> AppResult<Json<serde_json::Value>> {
    let snapshot: Option<(String,)> =
        sqlx::query_as("SELECT slug FROM pricing_plans WHERE id = $1")
            .bind(id)
            .fetch_optional(&state.db)
            .await?;
    let slug = snapshot
        .ok_or(AppError::NotFound("Pricing plan not found".to_string()))?
        .0;

    let result = sqlx::query(
        r#"
        UPDATE pricing_plans
        SET is_active = false, updated_at = NOW()
        WHERE id = $1
        "#,
    )
    .bind(id)
    .execute(&state.db)
    .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound("Pricing plan not found".to_string()));
    }

    audit_admin(
        &state.db,
        &admin,
        &client,
        "pricing_plan.deactivate",
        "pricing_plan",
        id,
        serde_json::json!({ "slug": slug }),
    )
    .await;

    Ok(Json(
        serde_json::json!({ "message": "Pricing plan deactivated" }),
    ))
}

#[utoipa::path(
    post,
    path = "/api/admin/pricing/plans/{id}/toggle",
    tag = "pricing",
    security(("bearer_auth" = [])),
    params(("id" = Uuid, Path, description = "Plan id")),
    responses(
        (status = 200, description = "Plan active flag toggled", body = PricingPlan),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Plan not found")
    )
)]
pub(crate) async fn admin_toggle_plan(
    State(state): State<AppState>,
    admin: AdminUser,
    client: ClientInfo,
    Path(id): Path<Uuid>,
) -> AppResult<Json<PricingPlan>> {
    let plan = sqlx::query_as::<_, PricingPlan>(
        r#"
        UPDATE pricing_plans
        SET is_active = NOT is_active, updated_at = NOW()
        WHERE id = $1
        RETURNING id, name, slug, description, stripe_price_id, stripe_product_id,
                  amount_cents, currency, interval, interval_count, trial_days,
                  features, highlight_text, is_popular, is_active, sort_order,
                  created_at, updated_at
        "#,
    )
    .bind(id)
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NotFound("Pricing plan not found".to_string()))?;

    audit_admin(
        &state.db,
        &admin,
        &client,
        "pricing_plan.toggle",
        "pricing_plan",
        plan.id,
        serde_json::json!({
            "slug": plan.slug,
            "is_active": plan.is_active,
        }),
    )
    .await;

    Ok(Json(plan))
}

async fn admin_plan_history(
    State(state): State<AppState>,
    _admin: AdminUser,
    Path(id): Path<Uuid>,
) -> AppResult<Json<Vec<PricingChangeLog>>> {
    // Verify plan exists
    let exists =
        sqlx::query_scalar::<_, bool>("SELECT EXISTS(SELECT 1 FROM pricing_plans WHERE id = $1)")
            .bind(id)
            .fetch_one(&state.db)
            .await?;

    if !exists {
        return Err(AppError::NotFound("Pricing plan not found".to_string()));
    }

    let history = sqlx::query_as::<_, PricingChangeLog>(
        r#"
        SELECT id, plan_id, field_changed, old_value, new_value, changed_by, changed_at
        FROM pricing_change_log
        WHERE plan_id = $1
        ORDER BY changed_at DESC
        "#,
    )
    .bind(id)
    .fetch_all(&state.db)
    .await?;

    Ok(Json(history))
}

// ── Public Handlers ───────────────────────────────────────────────────

async fn public_list_plans(State(state): State<AppState>) -> AppResult<Json<Vec<PricingPlan>>> {
    let plans = sqlx::query_as::<_, PricingPlan>(
        r#"
        SELECT id, name, slug, description, stripe_price_id, stripe_product_id,
               amount_cents, currency, interval, interval_count, trial_days,
               features, highlight_text, is_popular, is_active, sort_order,
               created_at, updated_at
        FROM pricing_plans
        WHERE is_active = true
        ORDER BY sort_order ASC, created_at ASC
        "#,
    )
    .fetch_all(&state.db)
    .await?;

    Ok(Json(plans))
}
