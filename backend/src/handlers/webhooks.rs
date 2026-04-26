use axum::{
    body::Bytes,
    extract::State,
    http::{HeaderMap, StatusCode},
    routing::post,
    Router,
};
use chrono::DateTime;
use hmac::{Hmac, Mac};
use rand::Rng;
use sha2::Sha256;
use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    commerce::{
        billing::{self, InvoiceFields},
        disputes::{self, DisputeFields},
        orders,
        refunds::{self, ChargeRefundFields},
        webhook_audit::record_webhook_audit_best_effort,
    },
    db,
    error::{AppError, AppResult},
    events::outbox::{publish_in_tx, Event, EventHeaders},
    models::*,
    notifications::{
        send::{send_notification, Recipient, SendOptions},
        webhooks::resend as resend_webhook,
    },
    AppState,
};

pub fn router() -> Router<AppState> {
    // FDN-08: 500/min/source. Burst-friendly (Stripe retry storms) but
    // guards against a wedged sender from flooding the pipe. The Resend
    // endpoint shares the same bucket so a burst on either provider does
    // not silently starve the other.
    Router::new()
        .route("/stripe", post(stripe_webhook))
        .route("/email/resend", post(resend_email_webhook))
        .layer(crate::middleware::rate_limit::webhooks_layer())
}

#[utoipa::path(
    post,
    path = "/api/webhooks/stripe",
    tag = "webhooks",
    request_body(content_type = "application/json", description = "Raw Stripe webhook JSON payload"),
    responses(
        (status = 200, description = "Webhook processed"),
        (status = 400, description = "Invalid signature or payload"),
        (status = 500, description = "Server error")
    )
)]
pub(crate) async fn stripe_webhook(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> StatusCode {
    let signature = match headers
        .get("stripe-signature")
        .and_then(|v| v.to_str().ok())
    {
        Some(sig) => sig.to_string(),
        None => return StatusCode::BAD_REQUEST,
    };

    if state.config.stripe_webhook_secret.is_empty() {
        tracing::warn!("Stripe webhook secret not configured");
        return StatusCode::INTERNAL_SERVER_ERROR;
    }

    // Verify webhook signature before parsing the event payload.
    let payload = match std::str::from_utf8(&body) {
        Ok(p) => p,
        Err(_) => return StatusCode::BAD_REQUEST,
    };
    if !verify_stripe_signature(payload, &signature, &state.config.stripe_webhook_secret) {
        tracing::warn!("Rejected Stripe webhook due to invalid signature");
        return StatusCode::UNAUTHORIZED;
    }

    let event: serde_json::Value = match serde_json::from_str(payload) {
        Ok(e) => e,
        Err(_) => return StatusCode::BAD_REQUEST,
    };

    let Some(event_id) = event.get("id").and_then(|v| v.as_str()) else {
        tracing::warn!("Stripe event missing id");
        return StatusCode::BAD_REQUEST;
    };
    let event_type = event
        .get("type")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");

    match db::try_claim_stripe_webhook_event(&state.db, event_id, event_type).await {
        Ok(true) => {}
        Ok(false) => {
            tracing::debug!(event_id, "Duplicate Stripe webhook event — acknowledging");
            return StatusCode::OK;
        }
        Err(e) => {
            tracing::error!("Webhook idempotency insert failed: {e}");
            return StatusCode::INTERNAL_SERVER_ERROR;
        }
    }

    // ~1% of webhooks: prune old idempotency rows so the table stays bounded.
    if rand::thread_rng().gen_bool(0.01) {
        match db::cleanup_old_stripe_webhook_events(&state.db).await {
            Ok(n) if n > 0 => tracing::debug!("Cleaned up {n} old processed_webhook_events rows"),
            Err(e) => tracing::warn!("Webhook idempotency cleanup failed: {e}"),
            _ => {}
        }
    }

    tracing::info!("Stripe webhook received: {event_type} ({event_id})");

    // Dispatch on event type. Each branch calls a focused handler that
    // returns `AppResult<()>`. A handler returning `Err` triggers a 500
    // so Stripe retries the event — which is the right behaviour for
    // transient infrastructure failures (DB blip, etc.) and the wrong
    // behaviour for malformed payloads. Per AGENTS.md hard-rule #7 we
    // never `unwrap()`; handlers downgrade unrecoverable parse errors
    // to a logged 200 so Stripe doesn't retry into an endless loop.
    let dispatch = match event_type {
        "customer.subscription.created" | "customer.subscription.updated" => {
            handle_subscription_update(&state, event_id, &event).await
        }
        "customer.subscription.deleted" => {
            handle_subscription_deleted(&state, event_id, &event).await
        }
        "customer.subscription.paused" => {
            handle_subscription_paused(&state, event_id, &event).await
        }
        "customer.subscription.resumed" => {
            handle_subscription_resumed(&state, event_id, &event).await
        }
        "customer.subscription.trial_will_end" => {
            handle_subscription_trial_will_end(&state, event_id, &event).await
        }
        "checkout.session.completed" => handle_checkout_completed(&state, event_id, &event).await,
        "invoice.payment_failed" => handle_invoice_payment_failed(&state, event_id, &event).await,
        "invoice.paid" => handle_invoice_paid(&state, event_id, &event).await,
        "charge.refunded" => handle_charge_refunded(&state, event_id, &event).await,
        "payment_intent.payment_failed" => {
            handle_payment_intent_failed(&state, event_id, &event).await
        }
        "charge.dispute.created" => handle_charge_dispute_created(&state, event_id, &event).await,
        _ => {
            tracing::debug!("Unhandled webhook event: {event_type}");
            Ok(())
        }
    };

    if let Err(e) = dispatch {
        tracing::error!(event_id, event_type, error = %e, "stripe webhook handler failed");
        return StatusCode::INTERNAL_SERVER_ERROR;
    }

    StatusCode::OK
}

fn verify_stripe_signature(payload: &str, signature_header: &str, secret: &str) -> bool {
    type HmacSha256 = Hmac<Sha256>;

    let mut timestamp: Option<i64> = None;
    let mut signatures: Vec<&str> = Vec::new();
    for part in signature_header.split(',') {
        let mut kv = part.splitn(2, '=');
        let key = kv.next().unwrap_or_default().trim();
        let value = kv.next().unwrap_or_default().trim();
        match key {
            "t" => timestamp = value.parse::<i64>().ok(),
            "v1" if !value.is_empty() => signatures.push(value),
            _ => {}
        }
    }

    let Some(ts) = timestamp else {
        return false;
    };
    if signatures.is_empty() {
        return false;
    }

    let now = chrono::Utc::now().timestamp();
    // Stripe recommends a 5 minute tolerance to reduce replay risk.
    if (now - ts).abs() > 300 {
        return false;
    }

    let signed_payload = format!("{ts}.{payload}");
    let mut mac = match HmacSha256::new_from_slice(secret.as_bytes()) {
        Ok(mac) => mac,
        Err(_) => return false,
    };
    mac.update(signed_payload.as_bytes());
    let computed = mac.finalize().into_bytes();
    let computed_hex = computed
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<String>();

    signatures
        .iter()
        .any(|candidate| candidate.eq_ignore_ascii_case(&computed_hex))
}

// ─── Helpers ───────────────────────────────────────────────────────────

/// Resolve the local `subscriptions` row for a Stripe subscription id and
/// fetch the linked user in a single call. Returns `(Some(sub), Some(user))`,
/// `(Some(sub), None)`, `(None, None)` — never `(None, Some(_))`.
async fn lookup_subscription_with_user(
    pool: &PgPool,
    stripe_subscription_id: &str,
) -> AppResult<(Option<Subscription>, Option<User>)> {
    let Some(sub) = db::find_subscription_by_stripe_id(pool, stripe_subscription_id).await? else {
        return Ok((None, None));
    };
    let user = db::find_user_by_id(pool, sub.user_id).await?;
    Ok((Some(sub), user))
}

/// Convert "active"/"past_due"/"unpaid"/"paused" strings into the typed
/// enum, defaulting to `Active` for any unknown value (matches the
/// existing handler's tolerance).
fn parse_subscription_status(status: &str) -> SubscriptionStatus {
    match status {
        "canceled" => SubscriptionStatus::Canceled,
        "past_due" => SubscriptionStatus::PastDue,
        "trialing" => SubscriptionStatus::Trialing,
        "unpaid" => SubscriptionStatus::Unpaid,
        // The `paused` enum value was added in 057_subscription_status_paused.sql;
        // we route through the generic Active branch when the enum is
        // missing the variant in the typed Rust enum — caller handles
        // pause separately via `commerce::subscriptions::pause`.
        _ => SubscriptionStatus::Active,
    }
}

// ─── Existing handlers ─────────────────────────────────────────────────

async fn handle_subscription_update(
    state: &AppState,
    event_id: &str,
    event: &serde_json::Value,
) -> AppResult<()> {
    let sub = &event["data"]["object"];
    let customer_id = sub["customer"].as_str().unwrap_or_default();
    let sub_id = sub["id"].as_str().unwrap_or_default();
    let status_str = sub["status"].as_str().unwrap_or("active");
    let status = parse_subscription_status(status_str);

    // Determine plan from price interval
    let plan = if let Some(items) = sub["items"]["data"].as_array() {
        if let Some(first) = items.first() {
            match first["price"]["recurring"]["interval"].as_str() {
                Some("year") => SubscriptionPlan::Annual,
                _ => SubscriptionPlan::Monthly,
            }
        } else {
            SubscriptionPlan::Monthly
        }
    } else {
        SubscriptionPlan::Monthly
    };

    let period_start =
        DateTime::from_timestamp(sub["current_period_start"].as_i64().unwrap_or(0), 0)
            .unwrap_or_default();
    let period_end = DateTime::from_timestamp(sub["current_period_end"].as_i64().unwrap_or(0), 0)
        .unwrap_or_default();

    let pricing_plan_id = sub["metadata"]["swings_pricing_plan_id"]
        .as_str()
        .and_then(|s| Uuid::parse_str(s).ok());

    if let Some(user) = db::find_user_by_stripe_customer(&state.db, customer_id).await? {
        let upserted = db::upsert_subscription(
            &state.db,
            user.id,
            customer_id,
            sub_id,
            &plan,
            &status,
            period_start,
            period_end,
            pricing_plan_id,
        )
        .await?;
        record_webhook_audit_best_effort(
            &state.db,
            event_id,
            "customer.subscription.updated",
            Some("subscription"),
            Some(&upserted.id.to_string()),
            serde_json::json!({
                "stripe_subscription_id": sub_id,
                "status": status_str,
            }),
        )
        .await;
    }

    Ok(())
}

async fn handle_subscription_deleted(
    state: &AppState,
    event_id: &str,
    event: &serde_json::Value,
) -> AppResult<()> {
    let sub = &event["data"]["object"];
    let sub_id = sub["id"].as_str().unwrap_or_default();

    if let Some(existing) = db::find_subscription_by_stripe_id(&state.db, sub_id).await? {
        let customer_id = &existing.stripe_customer_id;
        db::upsert_subscription(
            &state.db,
            existing.user_id,
            customer_id,
            sub_id,
            &existing.plan,
            &SubscriptionStatus::Canceled,
            existing.current_period_start,
            existing.current_period_end,
            existing.pricing_plan_id,
        )
        .await?;

        // FDN-05: notify the member that their subscription is cancelled.
        if let Some(user) = db::find_user_by_id(&state.db, existing.user_id).await? {
            let end_date = existing.current_period_end.format("%B %-d, %Y").to_string();
            let ctx = serde_json::json!({
                "name": user.name,
                "end_date": end_date,
                "app_url": state.config.app_url,
                "year": chrono::Utc::now().format("%Y").to_string(),
            });
            if let Err(e) = send_notification(
                &state.db,
                "subscription.cancelled",
                &Recipient::User {
                    user_id: user.id,
                    email: user.email.clone(),
                },
                ctx,
                SendOptions::default(),
            )
            .await
            {
                tracing::warn!(user_id = %user.id, error = %e, "failed to enqueue subscription.cancelled email");
            }
        }

        record_webhook_audit_best_effort(
            &state.db,
            event_id,
            "customer.subscription.deleted",
            Some("subscription"),
            Some(&existing.id.to_string()),
            serde_json::json!({ "stripe_subscription_id": sub_id }),
        )
        .await;
    }

    Ok(())
}

async fn handle_checkout_completed(
    state: &AppState,
    event_id: &str,
    event: &serde_json::Value,
) -> AppResult<()> {
    let session = &event["data"]["object"];
    let customer_email = session["customer_details"]["email"]
        .as_str()
        .unwrap_or_default();
    let customer_id = session["customer"].as_str().unwrap_or_default();
    let sub_id = session["subscription"].as_str().unwrap_or_default();

    // SECURITY: do not log `customer_email` — it's PII and retention
    // windows for application logs are not the same as our PII store.
    // Stripe's customer/subscription IDs are opaque and safe to log.
    tracing::info!(
        customer_id,
        subscription_id = sub_id,
        "stripe checkout.session.completed received"
    );

    if let Some(user) = db::find_user_by_email(&state.db, customer_email).await? {
        let now = chrono::Utc::now();
        let upserted = db::upsert_subscription(
            &state.db,
            user.id,
            customer_id,
            sub_id,
            &SubscriptionPlan::Monthly, // Will be corrected by subscription.updated event
            &SubscriptionStatus::Active,
            now,
            now + chrono::Duration::days(30),
            None,
        )
        .await?;

        // ADM-15: persist the billing profile Stripe collected during
        // checkout so the admin members surface always reflects the
        // latest data. Every field is optional — Stripe omits unset
        // ones — and the UPDATE is COALESCE-based, so existing values
        // survive a sparse webhook.
        let phone = session["customer_details"]["phone"].as_str();
        let address = &session["customer_details"]["address"];
        if let Err(e) = db::update_user_checkout_profile(
            &state.db,
            user.id,
            phone,
            address["line1"].as_str(),
            address["line2"].as_str(),
            address["city"].as_str(),
            address["state"].as_str(),
            address["postal_code"].as_str(),
            address["country"].as_str(),
        )
        .await
        {
            // Logged but not fatal — the subscription itself is the
            // primary effect of this event; address sync is observability.
            tracing::warn!(
                user_id = %user.id,
                error = %e,
                "failed to persist checkout customer_details to users row"
            );
        }

        // FDN-05: send confirmation. Plan name is best-effort — the authoritative
        // plan arrives on `customer.subscription.updated` so we use "Monthly"
        // here to match the upsert above.
        let ctx = serde_json::json!({
            "name": user.name,
            "plan_name": "Monthly",
            "app_url": state.config.app_url,
            "year": chrono::Utc::now().format("%Y").to_string(),
        });
        if let Err(e) = send_notification(
            &state.db,
            "subscription.confirmed",
            &Recipient::User {
                user_id: user.id,
                email: user.email.clone(),
            },
            ctx,
            SendOptions::default(),
        )
        .await
        {
            tracing::warn!(user_id = %user.id, error = %e, "failed to enqueue subscription.confirmed email");
        }

        record_webhook_audit_best_effort(
            &state.db,
            event_id,
            "checkout.session.completed",
            Some("subscription"),
            Some(&upserted.id.to_string()),
            serde_json::json!({
                "stripe_subscription_id": sub_id,
                "stripe_customer_id": customer_id,
            }),
        )
        .await;
    }

    Ok(())
}

// ─── New handlers ──────────────────────────────────────────────────────

/// `invoice.payment_failed` — flips the subscription to `past_due`
/// (or `unpaid` once Stripe has exhausted smart retries), records a
/// dunning event, and queues a notification email.
async fn handle_invoice_payment_failed(
    state: &AppState,
    event_id: &str,
    event: &serde_json::Value,
) -> AppResult<()> {
    let invoice = &event["data"]["object"];
    let Some(fields) = InvoiceFields::from_payload(invoice) else {
        tracing::warn!(
            event_id,
            "invoice.payment_failed missing invoice id; skipping"
        );
        return Ok(());
    };

    // Resolve the local subscription + user for cross-linking.
    let (sub, user) = match fields.stripe_subscription_id.as_deref() {
        Some(sub_id) => lookup_subscription_with_user(&state.db, sub_id).await?,
        None => (None, None),
    };
    let sub_uuid = sub.as_ref().map(|s| s.id);
    let user_uuid = user.as_ref().map(|u| u.id);

    // Persist the invoice mirror up front so a later `invoice.paid`
    // overlay (which calls `upsert_invoice` again) cleanly transitions
    // status from `open` -> `paid`.
    billing::upsert_invoice(&state.db, &fields, sub_uuid, user_uuid).await?;

    // `next_payment_attempt` is null on the *final* failure (Stripe has
    // given up retrying); we mirror that as `final = true`.
    let next_payment_attempt = invoice
        .get("next_payment_attempt")
        .and_then(|v| v.as_i64())
        .and_then(|ts| DateTime::from_timestamp(ts, 0));
    let final_attempt = next_payment_attempt.is_none() && fields.attempt_count >= 1;

    let new_status = if final_attempt { "unpaid" } else { "past_due" };
    if let Some(sub_id) = fields.stripe_subscription_id.as_deref() {
        billing::set_subscription_status_by_stripe_id(&state.db, sub_id, new_status).await?;
    }

    // Failure code/message live on the embedded `last_payment_error` or
    // (in the older payload shape) at the invoice top level.
    let failure_code = invoice
        .get("last_finalization_error")
        .and_then(|v| v.get("code"))
        .and_then(|v| v.as_str())
        .or_else(|| {
            invoice
                .get("charge")
                .and_then(|v| v.get("failure_code"))
                .and_then(|v| v.as_str())
        });
    let failure_message = invoice
        .get("last_finalization_error")
        .and_then(|v| v.get("message"))
        .and_then(|v| v.as_str())
        .or_else(|| {
            invoice
                .get("charge")
                .and_then(|v| v.get("failure_message"))
                .and_then(|v| v.as_str())
        });

    billing::record_payment_failure(
        &state.db,
        event_id,
        Some(&fields.stripe_invoice_id),
        invoice.get("payment_intent").and_then(|v| v.as_str()),
        fields.stripe_customer_id.as_deref(),
        sub_uuid,
        user_uuid,
        Some(fields.amount_due_cents),
        Some(&fields.currency),
        failure_code,
        failure_message,
        fields.attempt_count.max(1),
        next_payment_attempt,
        final_attempt,
    )
    .await?;

    // Email the member. Best-effort — never fail the webhook on a
    // missing template.
    if let Some(ref user) = user {
        let next_attempt_pretty = next_payment_attempt
            .map(|t| t.format("%B %-d, %Y").to_string())
            .unwrap_or_else(|| "shortly".to_string());
        let ctx = serde_json::json!({
            "name": user.name,
            "next_attempt": next_attempt_pretty,
            "billing_portal_url": format!("{}/account/billing", state.config.app_url),
            "app_url": state.config.app_url,
            "year": chrono::Utc::now().format("%Y").to_string(),
        });
        if let Err(e) = send_notification(
            &state.db,
            "subscription.payment_failed",
            &Recipient::User {
                user_id: user.id,
                email: user.email.clone(),
            },
            ctx,
            SendOptions::default(),
        )
        .await
        {
            tracing::warn!(user_id = %user.id, error = %e,
                "failed to enqueue subscription.payment_failed email");
        }
    }

    record_webhook_audit_best_effort(
        &state.db,
        event_id,
        "invoice.payment_failed",
        Some("subscription"),
        sub_uuid.map(|u| u.to_string()).as_deref(),
        serde_json::json!({
            "stripe_invoice_id": fields.stripe_invoice_id,
            "stripe_subscription_id": fields.stripe_subscription_id,
            "attempt_count": fields.attempt_count,
            "amount_due_cents": fields.amount_due_cents,
            "currency": fields.currency,
            "new_status": new_status,
            "final": final_attempt,
        }),
    )
    .await;

    Ok(())
}

/// `invoice.paid` — invoice cleared. Restores the subscription to
/// `active` if it was previously dunning, and records the invoice mirror.
async fn handle_invoice_paid(
    state: &AppState,
    event_id: &str,
    event: &serde_json::Value,
) -> AppResult<()> {
    let invoice = &event["data"]["object"];
    let Some(fields) = InvoiceFields::from_payload(invoice) else {
        tracing::warn!(event_id, "invoice.paid missing invoice id; skipping");
        return Ok(());
    };

    let (sub, user) = match fields.stripe_subscription_id.as_deref() {
        Some(sub_id) => lookup_subscription_with_user(&state.db, sub_id).await?,
        None => (None, None),
    };
    let sub_uuid = sub.as_ref().map(|s| s.id);
    let user_uuid = user.as_ref().map(|u| u.id);

    billing::upsert_invoice(&state.db, &fields, sub_uuid, user_uuid).await?;

    // If the subscription was previously past_due / unpaid, flip it back
    // to active. We only do this when the local DB shows a recovering
    // state to avoid trampling a concurrent admin cancel.
    let mut recovered = false;
    if let Some(ref existing) = sub {
        if matches!(
            existing.status,
            SubscriptionStatus::PastDue | SubscriptionStatus::Unpaid
        ) {
            if let Some(sub_id) = fields.stripe_subscription_id.as_deref() {
                billing::set_subscription_status_by_stripe_id(&state.db, sub_id, "active").await?;
                recovered = true;
            }
        }
    }

    // Send a recovery email only on the recovery transition; a fresh
    // monthly renewal already gets `subscription.confirmed` via
    // checkout/subscription.updated, so duplicating here would spam.
    if recovered {
        if let Some(ref user) = user {
            let ctx = serde_json::json!({
                "name": user.name,
                "app_url": state.config.app_url,
                "year": chrono::Utc::now().format("%Y").to_string(),
            });
            if let Err(e) = send_notification(
                &state.db,
                "subscription.payment_recovered",
                &Recipient::User {
                    user_id: user.id,
                    email: user.email.clone(),
                },
                ctx,
                SendOptions::default(),
            )
            .await
            {
                tracing::warn!(user_id = %user.id, error = %e,
                    "failed to enqueue subscription.payment_recovered email");
            }
        }
    }

    record_webhook_audit_best_effort(
        &state.db,
        event_id,
        "invoice.paid",
        Some("subscription"),
        sub_uuid.map(|u| u.to_string()).as_deref(),
        serde_json::json!({
            "stripe_invoice_id": fields.stripe_invoice_id,
            "stripe_subscription_id": fields.stripe_subscription_id,
            "amount_paid_cents": fields.amount_paid_cents,
            "currency": fields.currency,
            "recovered_from_dunning": recovered,
        }),
    )
    .await;

    Ok(())
}

/// `charge.refunded` — record the refund mirror, cross-link to the order
/// when there is one, and decrement reportable revenue (the refund row
/// is the source-of-truth for that calculation).
async fn handle_charge_refunded(
    state: &AppState,
    event_id: &str,
    event: &serde_json::Value,
) -> AppResult<()> {
    let charge = &event["data"]["object"];
    let Some(fields) = ChargeRefundFields::latest_from_charge(charge) else {
        tracing::warn!(event_id, "charge.refunded missing refunds.data[]; skipping");
        return Ok(());
    };

    // Cross-link to a local order, subscription, and user where possible.
    let order = match fields.stripe_payment_intent_id.as_deref() {
        Some(pi) => orders::get_order_by_payment_intent(&state.db, pi).await?,
        None => None,
    };
    let order_id = order.as_ref().map(|o| o.id);
    let user_uuid = match (order.as_ref(), fields.stripe_customer_id.as_deref()) {
        (Some(o), _) => o.user_id,
        (None, Some(cid)) => db::find_user_by_stripe_customer(&state.db, cid)
            .await?
            .map(|u| u.id),
        _ => None,
    };
    let subscription_id = if let Some(invoice_id) = fields.stripe_invoice_id.as_deref() {
        let row: Option<(Option<Uuid>,)> = sqlx::query_as(
            r#"
            SELECT subscription_id
              FROM subscription_invoices
             WHERE stripe_invoice_id = $1
            "#,
        )
        .bind(invoice_id)
        .fetch_optional(&state.db)
        .await?;
        row.and_then(|(opt,)| opt)
    } else {
        None
    };

    let inserted =
        refunds::record_charge_refund(&state.db, &fields, order_id, subscription_id, user_uuid)
            .await?;

    // If there's an order linked, transition it to `refunded` (only
    // valid from `processing` or `completed`). We deliberately swallow
    // an illegal-transition error rather than fail the webhook — the
    // admin may have already moved the order to refunded.
    if let Some(order) = order {
        if let Some(parsed) = orders::OrderStatus::parse(&order.status) {
            if parsed == orders::OrderStatus::Processing || parsed == orders::OrderStatus::Completed
            {
                if let Err(e) = orders::transition(
                    &state.db,
                    order.id,
                    orders::OrderStatus::Refunded,
                    None,
                    Some("stripe charge.refunded webhook"),
                )
                .await
                {
                    tracing::warn!(order_id = %order.id, error = %e,
                        "could not transition order to refunded; admin may need to reconcile");
                }
            }
        }
    }

    record_webhook_audit_best_effort(
        &state.db,
        event_id,
        "charge.refunded",
        Some("refund"),
        inserted.map(|u| u.to_string()).as_deref(),
        serde_json::json!({
            "stripe_refund_id": fields.stripe_refund_id,
            "stripe_charge_id": fields.stripe_charge_id,
            "stripe_invoice_id": fields.stripe_invoice_id,
            "amount_cents": fields.amount_cents,
            "currency": fields.currency,
            "order_id": order_id,
        }),
    )
    .await;

    Ok(())
}

/// `payment_intent.payment_failed` — typically fires for one-shot
/// PaymentIntents (cart checkout / form payments) that aren't backed by
/// a subscription invoice. We mark the linked order as `failed` and log
/// the dunning row. Subscription invoice failures are de-duplicated via
/// the unique `stripe_event_id` constraint on `payment_failures`.
async fn handle_payment_intent_failed(
    state: &AppState,
    event_id: &str,
    event: &serde_json::Value,
) -> AppResult<()> {
    let pi = &event["data"]["object"];
    let pi_id = pi.get("id").and_then(|v| v.as_str());
    let customer_id = pi.get("customer").and_then(|v| v.as_str());
    let amount = pi.get("amount").and_then(|v| v.as_i64());
    let currency = pi.get("currency").and_then(|v| v.as_str());
    let failure_code = pi
        .get("last_payment_error")
        .and_then(|v| v.get("code"))
        .and_then(|v| v.as_str());
    let failure_message = pi
        .get("last_payment_error")
        .and_then(|v| v.get("message"))
        .and_then(|v| v.as_str());

    // Find the order this PI relates to (if any). PI-driven subscription
    // renewals don't surface here — those come through `invoice.payment_failed`.
    let order = match pi_id {
        Some(id) => orders::get_order_by_payment_intent(&state.db, id).await?,
        None => None,
    };
    let user_uuid = match (order.as_ref(), customer_id) {
        (Some(o), _) => o.user_id,
        (None, Some(cid)) => db::find_user_by_stripe_customer(&state.db, cid)
            .await?
            .map(|u| u.id),
        _ => None,
    };

    billing::record_payment_failure(
        &state.db,
        event_id,
        None,
        pi_id,
        customer_id,
        None,
        user_uuid,
        amount,
        currency,
        failure_code,
        failure_message,
        1,
        None,
        true,
    )
    .await?;

    if let Some(order) = order {
        if let Some(parsed) = orders::OrderStatus::parse(&order.status) {
            // `Pending → Failed` and `Processing → Failed` are both legal.
            if matches!(
                parsed,
                orders::OrderStatus::Pending | orders::OrderStatus::Processing
            ) {
                if let Err(e) = orders::transition(
                    &state.db,
                    order.id,
                    orders::OrderStatus::Failed,
                    None,
                    Some("stripe payment_intent.payment_failed webhook"),
                )
                .await
                {
                    tracing::warn!(order_id = %order.id, error = %e,
                        "could not transition order to failed; admin reconcile required");
                }
            }
        }
    }

    record_webhook_audit_best_effort(
        &state.db,
        event_id,
        "payment_intent.payment_failed",
        Some("payment_intent"),
        pi_id,
        serde_json::json!({
            "stripe_customer_id": customer_id,
            "amount_cents": amount,
            "currency": currency,
            "failure_code": failure_code,
        }),
    )
    .await;

    Ok(())
}

/// `charge.dispute.created` — chargeback opened. Records the dispute,
/// flags the related order, and emits an outbox event for ops alerting.
/// **No automatic refund**; resolution is manual.
async fn handle_charge_dispute_created(
    state: &AppState,
    event_id: &str,
    event: &serde_json::Value,
) -> AppResult<()> {
    let dispute_obj = &event["data"]["object"];
    let Some(fields) = DisputeFields::from_payload(dispute_obj) else {
        tracing::warn!(
            event_id,
            "charge.dispute.created missing dispute id; skipping"
        );
        return Ok(());
    };

    // Cross-link to a local order / sub / user when we can.
    let order = match fields.stripe_payment_intent_id.as_deref() {
        Some(pi) => orders::get_order_by_payment_intent(&state.db, pi).await?,
        None => None,
    };
    let order_id = order.as_ref().map(|o| o.id);
    let user_uuid = match (order.as_ref(), fields.stripe_customer_id.as_deref()) {
        (Some(o), _) => o.user_id,
        (None, Some(cid)) => db::find_user_by_stripe_customer(&state.db, cid)
            .await?
            .map(|u| u.id),
        _ => None,
    };

    let inserted = disputes::record_dispute(&state.db, &fields, order_id, None, user_uuid).await?;

    if let Some(order_id) = order_id {
        disputes::flag_order_disputed(&state.db, order_id).await?;
    }

    // Ops alert on the outbox so the dispatcher routes it to whichever
    // channel the operator has wired up (email + Slack via webhook_out).
    let mut tx = state.db.begin().await?;
    let evt = Event {
        aggregate_type: "dispute".into(),
        aggregate_id: fields.stripe_dispute_id.clone(),
        event_type: "ops.dispute_opened".into(),
        payload: serde_json::json!({
            "stripe_dispute_id": fields.stripe_dispute_id,
            "stripe_charge_id": fields.stripe_charge_id,
            "amount_cents": fields.amount_cents,
            "currency": fields.currency,
            "reason": fields.reason,
            "status": fields.status,
            "evidence_due_by": fields.evidence_due_by,
            "order_id": order_id,
            "user_id": user_uuid,
        }),
        headers: EventHeaders {
            idempotency_key: Some(format!("dispute:{}", fields.stripe_dispute_id)),
            correlation_id: Some(event_id.to_string()),
            tenant: None,
        },
    };
    publish_in_tx(&mut tx, &evt)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("ops outbox publish failed: {e}")))?;
    tx.commit().await?;

    record_webhook_audit_best_effort(
        &state.db,
        event_id,
        "charge.dispute.created",
        Some("dispute"),
        inserted.map(|u| u.to_string()).as_deref(),
        serde_json::json!({
            "stripe_dispute_id": fields.stripe_dispute_id,
            "stripe_charge_id": fields.stripe_charge_id,
            "amount_cents": fields.amount_cents,
            "currency": fields.currency,
            "reason": fields.reason,
            "status": fields.status,
            "order_id": order_id,
        }),
    )
    .await;

    Ok(())
}

/// `customer.subscription.trial_will_end` — fires 3 days before a trial
/// expires (configurable in Stripe). We send the trial-ending email and
/// dedupe on `(subscription_id, trial_end)` so a Stripe re-delivery
/// doesn't double-email.
async fn handle_subscription_trial_will_end(
    state: &AppState,
    event_id: &str,
    event: &serde_json::Value,
) -> AppResult<()> {
    let sub = &event["data"]["object"];
    let stripe_sub_id = sub["id"].as_str().unwrap_or_default();
    let trial_end = sub
        .get("trial_end")
        .and_then(|v| v.as_i64())
        .and_then(|ts| DateTime::from_timestamp(ts, 0));
    let Some(trial_end) = trial_end else {
        tracing::warn!(event_id, "trial_will_end missing trial_end; skipping");
        return Ok(());
    };

    let (existing, user) = lookup_subscription_with_user(&state.db, stripe_sub_id).await?;
    let Some(existing) = existing else {
        tracing::debug!(
            stripe_sub_id,
            "trial_will_end for unknown subscription; skipping"
        );
        return Ok(());
    };

    // Per-trial-end dedupe row. The (subscription_id, trial_end) PK
    // means a Stripe redelivery is a no-op.
    let inserted: Option<(Uuid,)> = sqlx::query_as(
        r#"
        INSERT INTO subscription_trial_events (subscription_id, trial_end)
        VALUES ($1, $2)
        ON CONFLICT (subscription_id, trial_end) DO NOTHING
        RETURNING subscription_id
        "#,
    )
    .bind(existing.id)
    .bind(trial_end)
    .fetch_optional(&state.db)
    .await?;

    if inserted.is_some() {
        if let Some(user) = user {
            let ctx = serde_json::json!({
                "name": user.name,
                "trial_end_date": trial_end.format("%B %-d, %Y").to_string(),
                "billing_portal_url": format!("{}/account/billing", state.config.app_url),
                "app_url": state.config.app_url,
                "year": chrono::Utc::now().format("%Y").to_string(),
            });
            if let Err(e) = send_notification(
                &state.db,
                "subscription.trial_ending",
                &Recipient::User {
                    user_id: user.id,
                    email: user.email.clone(),
                },
                ctx,
                SendOptions::default(),
            )
            .await
            {
                tracing::warn!(user_id = %user.id, error = %e,
                    "failed to enqueue subscription.trial_ending email");
            }
        }
    }

    record_webhook_audit_best_effort(
        &state.db,
        event_id,
        "customer.subscription.trial_will_end",
        Some("subscription"),
        Some(&existing.id.to_string()),
        serde_json::json!({
            "stripe_subscription_id": stripe_sub_id,
            "trial_will_end_at": trial_end,
            "notified": inserted.is_some(),
        }),
    )
    .await;

    Ok(())
}

/// `customer.subscription.paused` — Stripe pause_collection enabled.
/// We flip the local row to `paused` so member-side guards see the
/// inactive state.
async fn handle_subscription_paused(
    state: &AppState,
    event_id: &str,
    event: &serde_json::Value,
) -> AppResult<()> {
    let sub = &event["data"]["object"];
    let stripe_sub_id = sub["id"].as_str().unwrap_or_default();
    let Some(existing) = db::find_subscription_by_stripe_id(&state.db, stripe_sub_id).await? else {
        tracing::debug!(
            stripe_sub_id,
            "paused event for unknown subscription; skipping"
        );
        return Ok(());
    };

    // The `paused` enum value was added in 057_subscription_status_paused.sql.
    // The typed `SubscriptionStatus` enum doesn't carry it; we go through
    // the raw enum cast here.
    sqlx::query(
        r#"
        UPDATE subscriptions
           SET status = 'paused'::subscription_status,
               paused_at = COALESCE(paused_at, NOW()),
               updated_at = NOW()
         WHERE id = $1
        "#,
    )
    .bind(existing.id)
    .execute(&state.db)
    .await?;

    record_webhook_audit_best_effort(
        &state.db,
        event_id,
        "customer.subscription.paused",
        Some("subscription"),
        Some(&existing.id.to_string()),
        serde_json::json!({ "stripe_subscription_id": stripe_sub_id }),
    )
    .await;
    Ok(())
}

/// `customer.subscription.resumed` — pause_collection cleared. We flip
/// the local row back to `active`.
async fn handle_subscription_resumed(
    state: &AppState,
    event_id: &str,
    event: &serde_json::Value,
) -> AppResult<()> {
    let sub = &event["data"]["object"];
    let stripe_sub_id = sub["id"].as_str().unwrap_or_default();
    let Some(existing) = db::find_subscription_by_stripe_id(&state.db, stripe_sub_id).await? else {
        tracing::debug!(
            stripe_sub_id,
            "resumed event for unknown subscription; skipping"
        );
        return Ok(());
    };

    sqlx::query(
        r#"
        UPDATE subscriptions
           SET status = 'active'::subscription_status,
               paused_at = NULL,
               pause_resumes_at = NULL,
               updated_at = NOW()
         WHERE id = $1
        "#,
    )
    .bind(existing.id)
    .execute(&state.db)
    .await?;

    record_webhook_audit_best_effort(
        &state.db,
        event_id,
        "customer.subscription.resumed",
        Some("subscription"),
        Some(&existing.id.to_string()),
        serde_json::json!({ "stripe_subscription_id": stripe_sub_id }),
    )
    .await;
    Ok(())
}

// ────────────────────────────────────────────────────────────────────────
// FDN-09: Resend delivery-status webhook.
// ────────────────────────────────────────────────────────────────────────

/// Configured secret for Resend webhooks. We purposefully read from
/// `RESEND_WEBHOOK_SECRET` via `std::env` at request time (rather than
/// plumbing it onto `Config`) so operators can rotate the secret without a
/// binary redeploy — `assert_production_ready` still enforces presence.
fn resend_webhook_secret() -> Option<String> {
    std::env::var("RESEND_WEBHOOK_SECRET")
        .ok()
        .filter(|v| !v.trim().is_empty())
}

#[utoipa::path(
    post,
    path = "/api/webhooks/email/resend",
    tag = "webhooks",
    request_body(
        content_type = "application/json",
        description = "Resend webhook JSON payload (email.sent, email.delivered, email.bounced, email.complained, email.opened, email.clicked, email.delivery_delayed)"
    ),
    responses(
        (status = 200, description = "Webhook processed (or duplicate)"),
        (status = 400, description = "Invalid payload"),
        (status = 401, description = "Invalid or missing signature"),
        (status = 500, description = "Server error")
    )
)]
pub(crate) async fn resend_email_webhook(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> StatusCode {
    let Some(secret) = resend_webhook_secret() else {
        tracing::warn!("Rejected Resend webhook — RESEND_WEBHOOK_SECRET is not configured");
        return StatusCode::INTERNAL_SERVER_ERROR;
    };

    // Resend ships the Svix header trio; the optional `resend-*` variants
    // accommodate early-alpha tenants and our own fixture tests.
    let sig_header = headers
        .get("svix-signature")
        .or_else(|| headers.get("resend-signature"))
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    let timestamp = headers
        .get("svix-timestamp")
        .or_else(|| headers.get("resend-timestamp"))
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.parse::<i64>().ok());
    let svix_id = headers
        .get("svix-id")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    let Some(ts) = timestamp else {
        tracing::warn!("Rejected Resend webhook — missing svix-timestamp header");
        return StatusCode::BAD_REQUEST;
    };
    if sig_header.is_empty() {
        tracing::warn!("Rejected Resend webhook — missing svix-signature header");
        return StatusCode::BAD_REQUEST;
    }

    if !resend_webhook::verify_signature(&body, secret.as_bytes(), svix_id, ts, sig_header) {
        tracing::warn!("Rejected Resend webhook due to invalid signature");
        return StatusCode::UNAUTHORIZED;
    }

    let envelope: resend_webhook::ResendWebhookEnvelope = match serde_json::from_slice(&body) {
        Ok(e) => e,
        Err(e) => {
            tracing::warn!("Invalid Resend webhook JSON: {e}");
            return StatusCode::BAD_REQUEST;
        }
    };

    tracing::info!(
        event_id = %envelope.event_id,
        event_type = %envelope.event_type,
        "Resend webhook received"
    );

    match resend_webhook::process_event(&state.db, &envelope).await {
        Ok(outcome) => {
            tracing::debug!(outcome = ?outcome, "resend webhook processed");
            StatusCode::OK
        }
        Err(e) => {
            tracing::error!(error = %e, "resend webhook processing failed");
            StatusCode::INTERNAL_SERVER_ERROR
        }
    }
}

#[cfg(test)]
mod tests {
    use super::verify_stripe_signature;
    use chrono::Utc;
    use hmac::{Hmac, Mac};
    use sha2::Sha256;

    fn make_signature(secret: &str, payload: &str, timestamp: i64) -> String {
        type HmacSha256 = Hmac<Sha256>;
        let mut mac = HmacSha256::new_from_slice(secret.as_bytes()).expect("valid hmac key");
        mac.update(format!("{timestamp}.{payload}").as_bytes());
        let digest = mac.finalize().into_bytes();
        let hex = digest
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>();
        format!("t={timestamp},v1={hex}")
    }

    #[test]
    fn accepts_valid_signature() {
        let secret = "whsec_test_secret";
        let payload = r#"{"type":"checkout.session.completed"}"#;
        let timestamp = Utc::now().timestamp();
        let header = make_signature(secret, payload, timestamp);
        assert!(verify_stripe_signature(payload, &header, secret));
    }

    #[test]
    fn rejects_tampered_payload() {
        let secret = "whsec_test_secret";
        let payload = r#"{"type":"checkout.session.completed"}"#;
        let timestamp = Utc::now().timestamp();
        let header = make_signature(secret, payload, timestamp);
        assert!(!verify_stripe_signature(
            r#"{"type":"customer.subscription.deleted"}"#,
            &header,
            secret
        ));
    }
}
