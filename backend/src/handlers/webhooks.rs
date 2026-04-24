use axum::{
    body::Bytes,
    extract::State,
    http::{HeaderMap, StatusCode},
    routing::post,
    Router,
};
use hmac::{Hmac, Mac};
use rand::Rng;
use sha2::Sha256;

use crate::{
    db,
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

    match event_type {
        "customer.subscription.created" | "customer.subscription.updated" => {
            if let Err(e) = handle_subscription_update(&state, &event).await {
                tracing::error!("Failed to handle subscription update: {e}");
                return StatusCode::INTERNAL_SERVER_ERROR;
            }
        }
        "customer.subscription.deleted" => {
            if let Err(e) = handle_subscription_deleted(&state, &event).await {
                tracing::error!("Failed to handle subscription deletion: {e}");
                return StatusCode::INTERNAL_SERVER_ERROR;
            }
        }
        "checkout.session.completed" => {
            if let Err(e) = handle_checkout_completed(&state, &event).await {
                tracing::error!("Failed to handle checkout: {e}");
                return StatusCode::INTERNAL_SERVER_ERROR;
            }
        }
        _ => {
            tracing::debug!("Unhandled webhook event: {event_type}");
        }
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

async fn handle_subscription_update(
    state: &AppState,
    event: &serde_json::Value,
) -> Result<(), Box<dyn std::error::Error>> {
    let sub = &event["data"]["object"];
    let customer_id = sub["customer"].as_str().unwrap_or_default();
    let sub_id = sub["id"].as_str().unwrap_or_default();
    let status_str = sub["status"].as_str().unwrap_or("active");

    let status = match status_str {
        "active" => SubscriptionStatus::Active,
        "canceled" => SubscriptionStatus::Canceled,
        "past_due" => SubscriptionStatus::PastDue,
        "trialing" => SubscriptionStatus::Trialing,
        "unpaid" => SubscriptionStatus::Unpaid,
        _ => SubscriptionStatus::Active,
    };

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
        chrono::DateTime::from_timestamp(sub["current_period_start"].as_i64().unwrap_or(0), 0)
            .unwrap_or_default();

    let period_end =
        chrono::DateTime::from_timestamp(sub["current_period_end"].as_i64().unwrap_or(0), 0)
            .unwrap_or_default();

    // Find user by stripe customer id
    if let Some(user) = db::find_user_by_stripe_customer(&state.db, customer_id).await? {
        db::upsert_subscription(
            &state.db,
            user.id,
            customer_id,
            sub_id,
            &plan,
            &status,
            period_start,
            period_end,
        )
        .await?;
    }

    Ok(())
}

async fn handle_subscription_deleted(
    state: &AppState,
    event: &serde_json::Value,
) -> Result<(), Box<dyn std::error::Error>> {
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
    }

    Ok(())
}

async fn handle_checkout_completed(
    state: &AppState,
    event: &serde_json::Value,
) -> Result<(), Box<dyn std::error::Error>> {
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

    // If user exists, link their subscription
    if let Some(user) = db::find_user_by_email(&state.db, customer_email).await? {
        let now = chrono::Utc::now();
        db::upsert_subscription(
            &state.db,
            user.id,
            customer_id,
            sub_id,
            &SubscriptionPlan::Monthly, // Will be corrected by subscription.updated event
            &SubscriptionStatus::Active,
            now,
            now + chrono::Duration::days(30),
        )
        .await?;

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
    }

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
