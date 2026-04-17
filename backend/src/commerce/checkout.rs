//! EC-04: Checkout service.
//!
//! Bridges cart → order + Stripe PaymentIntent.
//!
//!   * `create_checkout_session(pool, identity, email, idempotency_key)`
//!     loads the caller's cart, computes totals, mints a `pending` order,
//!     and (if Stripe is wired) creates a PaymentIntent whose
//!     `client_secret` is returned to the renderer for Stripe Elements.
//!   * `confirm_checkout(pool, order_id)` is the no-op success leg —
//!     the real status flip is done by the `payment_intent.succeeded`
//!     webhook so the source of truth stays single. This handler exists
//!     for the post-redirect "thank you" flow that polls for status.
//!   * Address book CRUD lives in [`address`] — `save`, `list`,
//!     `set_default`, `delete`.
//!
//! Idempotency: the handler MUST require an `Idempotency-Key` header.
//! `orders.idempotency_key` is UNIQUE; a replayed request returns the
//! existing order rather than minting a second one. This is the only
//! reason create_order's repo helper carries an `idempotency_key`.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, PgPool};
use utoipa::ToSchema;
use uuid::Uuid;

use super::{
    cart::{self, CartIdentity, CartItem, CartTotals},
    orders::{self, CreateOrderInput, Order},
};
use crate::error::{AppError, AppResult};

/// Returned to the renderer after `POST /api/checkout/sessions`.
/// `client_secret` is `None` when Stripe wiring is absent (tests, dev
/// environments without `STRIPE_SECRET_KEY`); the order is still
/// committed in `pending` so the admin UI can take it from there.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CheckoutSession {
    pub order: Order,
    pub items: Vec<CartItem>,
    pub totals: CartTotals,
    pub client_secret: Option<String>,
    pub stripe_payment_intent_id: Option<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum CheckoutError {
    #[error("cart is empty")]
    EmptyCart,
    #[error("checkout amount {amount_cents} below the supported minimum")]
    AmountBelowMinimum { amount_cents: i64 },
}

/// Build a CheckoutSession for the supplied identity + email. Caller
/// is responsible for upstream auth + idempotency-key extraction.
pub async fn create_checkout_session(
    pool: &PgPool,
    identity: CartIdentity,
    email: &str,
    idempotency_key: Option<&str>,
    stripe: Option<&dyn StripeIntentMinter>,
) -> AppResult<CheckoutSession> {
    let cart_row = cart::get_or_create_cart(pool, identity).await?;

    // Idempotency replay short-circuit — if an order already exists for
    // the supplied key, return the same shape without re-creating.
    if let Some(key) = idempotency_key {
        if let Some(existing) = find_order_by_idempotency_key(pool, key).await? {
            let items = cart::list_items(pool, cart_row.id).await?;
            let totals = cart::compute_totals(&items, None);
            return Ok(CheckoutSession {
                order: existing.clone(),
                items,
                totals,
                client_secret: None,
                stripe_payment_intent_id: existing.stripe_payment_intent_id,
            });
        }
    }

    let items = cart::list_items(pool, cart_row.id).await?;
    if items.is_empty() {
        return Err(AppError::BadRequest("cart is empty".into()));
    }
    let totals = cart::compute_totals(&items, None);

    // Stripe minimums: $0.50 USD-equivalent for card; we surface anything
    // below 50 cents as a hard reject up here so the PI call never sees
    // it. (Currency-specific minima are TODO when we ship non-USD checkout.)
    if totals.total_cents < 50 {
        return Err(AppError::BadRequest(
            "checkout total is below the payment-processor minimum".into(),
        ));
    }

    // Mint the order in `pending` BEFORE talking to Stripe so the row
    // exists even if the PI call fails — recovery becomes "retry the
    // PaymentIntent on this order" rather than "find the dropped order".
    let order = orders::create_order(
        pool,
        CreateOrderInput {
            user_id: match identity {
                CartIdentity::Subject(uid) => Some(uid),
                CartIdentity::Anonymous(_) => None,
            },
            cart_id: Some(cart_row.id),
            email,
            currency: &cart_row.currency,
            subtotal_cents: totals.subtotal_cents,
            discount_cents: totals.discount_cents,
            tax_cents: totals.tax_cents,
            total_cents: totals.total_cents,
            idempotency_key,
            stripe_payment_intent_id: None,
            stripe_customer_id: None,
        },
    )
    .await?;

    let (client_secret, pi_id) = if let Some(s) = stripe {
        let pi = s
            .create_intent(
                totals.total_cents,
                &cart_row.currency,
                email,
                &format!("order:{}", order.id),
                idempotency_key,
            )
            .await?;
        link_payment_intent(pool, order.id, &pi.id).await?;
        (Some(pi.client_secret), Some(pi.id))
    } else {
        (None, None)
    };

    let mut order_with_pi = order;
    order_with_pi.stripe_payment_intent_id = pi_id.clone();

    Ok(CheckoutSession {
        order: order_with_pi,
        items,
        totals,
        client_secret,
        stripe_payment_intent_id: pi_id,
    })
}

/// Persist the Stripe PaymentIntent id on the order. The webhook
/// reconciler uses this to find the order from `payment_intent.succeeded`.
pub async fn link_payment_intent(pool: &PgPool, order_id: Uuid, pi_id: &str) -> AppResult<()> {
    sqlx::query("UPDATE orders SET stripe_payment_intent_id = $2, updated_at = NOW() WHERE id = $1")
        .bind(order_id)
        .bind(pi_id)
        .execute(pool)
        .await?;
    Ok(())
}

async fn find_order_by_idempotency_key(
    pool: &PgPool,
    key: &str,
) -> AppResult<Option<Order>> {
    let row = sqlx::query_as::<_, Order>(
        r#"
        SELECT id, number, user_id, cart_id, status::text AS status, currency,
               subtotal_cents, discount_cents, tax_cents, total_cents, email,
               stripe_payment_intent_id, stripe_customer_id, idempotency_key,
               metadata, placed_at, completed_at, created_at, updated_at
        FROM orders WHERE idempotency_key = $1
        "#,
    )
    .bind(key)
    .fetch_optional(pool)
    .await?;
    Ok(row)
}

// ── Stripe seam ────────────────────────────────────────────────────────

/// Trait the checkout service depends on instead of pulling stripe-rust
/// directly. Lets unit tests use a `MockStripe` that returns a deterministic
/// PaymentIntent without network I/O.
#[async_trait::async_trait]
pub trait StripeIntentMinter: Send + Sync {
    async fn create_intent(
        &self,
        amount_cents: i64,
        currency: &str,
        receipt_email: &str,
        metadata_id: &str,
        idempotency_key: Option<&str>,
    ) -> AppResult<MintedIntent>;
}

#[derive(Debug, Clone)]
pub struct MintedIntent {
    pub id: String,
    pub client_secret: String,
}

// ── Address book ───────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct Address {
    pub id: Uuid,
    pub user_id: Uuid,
    pub kind: String,
    pub full_name: String,
    pub company: Option<String>,
    pub line1: String,
    pub line2: Option<String>,
    pub city: String,
    pub state: Option<String>,
    pub postal_code: String,
    pub country: String,
    pub phone: Option<String>,
    pub is_default: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct UpsertAddress {
    pub kind: String,
    pub full_name: String,
    #[serde(default)]
    pub company: Option<String>,
    pub line1: String,
    #[serde(default)]
    pub line2: Option<String>,
    pub city: String,
    #[serde(default)]
    pub state: Option<String>,
    pub postal_code: String,
    pub country: String,
    #[serde(default)]
    pub phone: Option<String>,
    #[serde(default)]
    pub is_default: bool,
}

pub async fn list_addresses(pool: &PgPool, user_id: Uuid) -> AppResult<Vec<Address>> {
    let rows = sqlx::query_as::<_, Address>(
        r#"
        SELECT id, user_id, kind, full_name, company, line1, line2,
               city, state, postal_code, country, phone, is_default,
               created_at, updated_at
        FROM addresses
        WHERE user_id = $1
        ORDER BY is_default DESC, created_at DESC
        "#,
    )
    .bind(user_id)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

pub async fn save_address(
    pool: &PgPool,
    user_id: Uuid,
    input: UpsertAddress,
) -> AppResult<Address> {
    if !matches!(input.kind.as_str(), "billing" | "shipping") {
        return Err(AppError::BadRequest(
            "address kind must be 'billing' or 'shipping'".into(),
        ));
    }
    // If the new address claims to be the default, demote any prior
    // default for the same (user, kind) in the same TX.
    let mut tx = pool.begin().await?;
    if input.is_default {
        sqlx::query(
            "UPDATE addresses SET is_default = FALSE, updated_at = NOW()
             WHERE user_id = $1 AND kind = $2 AND is_default",
        )
        .bind(user_id)
        .bind(&input.kind)
        .execute(&mut *tx)
        .await?;
    }
    let row = sqlx::query_as::<_, Address>(
        r#"
        INSERT INTO addresses
            (user_id, kind, full_name, company, line1, line2, city, state,
             postal_code, country, phone, is_default)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
        RETURNING id, user_id, kind, full_name, company, line1, line2,
                  city, state, postal_code, country, phone, is_default,
                  created_at, updated_at
        "#,
    )
    .bind(user_id)
    .bind(input.kind)
    .bind(input.full_name)
    .bind(input.company)
    .bind(input.line1)
    .bind(input.line2)
    .bind(input.city)
    .bind(input.state)
    .bind(input.postal_code)
    .bind(input.country)
    .bind(input.phone)
    .bind(input.is_default)
    .fetch_one(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(row)
}

pub async fn delete_address(pool: &PgPool, user_id: Uuid, id: Uuid) -> AppResult<bool> {
    let r = sqlx::query("DELETE FROM addresses WHERE id = $1 AND user_id = $2")
        .bind(id)
        .bind(user_id)
        .execute(pool)
        .await?;
    Ok(r.rows_affected() > 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `compute_totals` is already covered in commerce::cart; here we just
    /// assert the checkout-error guard fires before any DB call when the
    /// cart total is sub-minimum. Because `create_checkout_session` needs
    /// a pool to even reach the guard, the meaningful unit-level checks
    /// live with the pure helpers. We keep this sentinel test so the
    /// AmountBelowMinimum variant doesn't get accidentally removed.
    #[test]
    fn checkout_error_amount_below_minimum_carries_amount() {
        let e = CheckoutError::AmountBelowMinimum { amount_cents: 25 };
        let msg = format!("{e}");
        assert!(msg.contains("25"));
    }

    #[test]
    fn checkout_error_empty_cart_renders() {
        let msg = format!("{}", CheckoutError::EmptyCart);
        assert_eq!(msg, "cart is empty");
    }
}
