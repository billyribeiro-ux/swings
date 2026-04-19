//! ADM-12: orders admin surface — list / read / manual create / void /
//! partial refund / CSV export.
//!
//! Mounted at `/api/admin/orders`. The historical Stripe-driven
//! checkout pipeline is the source of truth for happy-path orders;
//! this handler is the operator escape hatch for:
//!
//!   * `POST /`                         — manual create (wholesale,
//!     comp follow-up, fixed-price agreement) without a cart.
//!   * `POST /{id}/void`                — cancel a non-terminal order.
//!   * `POST /{id}/refund`              — record a partial or full
//!     refund. When the running refund total reaches the order total
//!     the engine transitions the order to `refunded`; otherwise the
//!     order keeps its current state and the refund total accumulates
//!     in `order_refunds`.
//!   * `GET /export.csv?…`              — `text/csv` dump of the
//!     filtered table for finance reconciliation.
//!
//! Refunds are recorded in `order_refunds` and surfaced via
//! `admin_actions`; **Stripe-side refunds are out of scope** for this
//! handler and remain the operator's responsibility (set
//! `stripe_refund_id` via the optional request field once Stripe has
//! confirmed the off-band refund).

use std::fmt::Write as _;

use axum::{
    extract::{Path, Query, State},
    http::{header, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::Row;
use utoipa::ToSchema;
use uuid::Uuid;
use validator::Validate;

use crate::{
    commerce::orders::{self as orders_repo, OrderStatus},
    error::{AppError, AppResult},
    extractors::{ClientInfo, PrivilegedUser},
    services::audit::audit_admin_priv,
    AppState,
};

const PERM_READ: &str = "admin.order.read";
const PERM_CREATE: &str = "admin.order.create";
const PERM_VOID: &str = "admin.order.void";
const PERM_REFUND: &str = "admin.order.refund";
const PERM_EXPORT: &str = "admin.order.export";

const DEFAULT_LIMIT: i64 = 25;
const MAX_LIMIT: i64 = 200;
const MAX_EXPORT_ROWS: i64 = 100_000;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(list).post(create_manual))
        .route("/export.csv", get(export_csv))
        .route("/{id}", get(read_one))
        .route("/{id}/void", post(void_order))
        .route("/{id}/refund", post(refund_order))
}

// ── DTOs ───────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct ListQuery {
    /// Free-text — substring match against email + order number.
    #[serde(default)]
    pub q: Option<String>,
    /// Status filter — must parse as one of the canonical
    /// `OrderStatus` values; unknown rejected with `400`.
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub limit: Option<i64>,
    #[serde(default)]
    pub offset: Option<i64>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct OrderListEnvelope {
    pub data: Vec<orders_repo::Order>,
    pub total: i64,
    pub page: i64,
    pub per_page: i64,
    pub total_pages: i64,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct OrderDetail {
    pub order: orders_repo::Order,
    pub items: Vec<orders_repo::OrderItem>,
    pub refunds: Vec<orders_repo::OrderRefund>,
    pub notes: Vec<orders_repo::OrderNote>,
    /// Cents already refunded (sum of `order_refunds.amount_cents`).
    pub refunded_cents: i64,
    pub remaining_refundable_cents: i64,
}

#[derive(Debug, Deserialize, Serialize, Validate, ToSchema)]
pub struct ManualOrderItem {
    pub product_id: Uuid,
    #[validate(range(min = 1, max = 1_000))]
    pub quantity: i32,
    /// Cents per unit. Operator-supplied so wholesale / comp pricing
    /// can deviate from the public catalogue.
    #[validate(range(min = 0, max = 1_000_000_000))]
    pub unit_price_cents: i64,
    #[validate(length(min = 1, max = 200))]
    pub name: String,
    pub sku: Option<String>,
}

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct ManualOrderRequest {
    #[validate(email)]
    pub email: String,
    pub user_id: Option<Uuid>,
    #[serde(default = "default_currency")]
    pub currency: String,
    #[validate(length(min = 1, max = 50, message = "at least one item required"))]
    pub items: Vec<ManualOrderItem>,
    #[validate(range(min = 0))]
    pub discount_cents: Option<i64>,
    #[validate(range(min = 0))]
    pub tax_cents: Option<i64>,
    /// `true` ⇒ skip the pending state and mark the order completed
    /// immediately (the operator already collected payment off-band).
    /// `false` (default) leaves the order in `pending` so the standard
    /// reconciliation pipeline can advance it.
    #[serde(default)]
    pub mark_completed: bool,
    pub notes: Option<String>,
}

fn default_currency() -> String {
    "usd".to_string()
}

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct VoidRequest {
    #[validate(length(max = 500))]
    pub reason: Option<String>,
}

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct RefundRequest {
    #[validate(range(min = 1))]
    pub amount_cents: i64,
    #[validate(length(max = 500))]
    pub reason: Option<String>,
    /// Operator-supplied Stripe refund id when the refund was issued
    /// out-of-band via the dashboard. Persisted on `order_refunds`
    /// so reconciliation can match.
    #[validate(length(max = 200))]
    pub stripe_refund_id: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct RefundResponse {
    pub refund: orders_repo::OrderRefund,
    /// `true` when the cumulative refund total reached the order
    /// total and the engine flipped the order to `refunded`.
    pub order_marked_refunded: bool,
    pub remaining_refundable_cents: i64,
}

// ── Handlers ───────────────────────────────────────────────────────────

#[utoipa::path(
    get,
    path = "/api/admin/orders",
    tag = "admin-orders",
    security(("bearer_auth" = [])),
    params(
        ("q"      = Option<String>, Query, description = "Substring across email + number"),
        ("status" = Option<String>, Query, description = "Order status filter"),
        ("limit"  = Option<i64>,    Query, description = "Page size (1-200, default 25)"),
        ("offset" = Option<i64>,    Query, description = "Cursor offset"),
    ),
    responses(
        (status = 200, description = "Paginated orders", body = OrderListEnvelope),
        (status = 400, description = "Invalid filter"),
        (status = 401, description = "Unauthenticated"),
        (status = 403, description = "Forbidden")
    )
)]
pub async fn list(
    State(state): State<AppState>,
    privileged: PrivilegedUser,
    Query(q): Query<ListQuery>,
) -> AppResult<Json<OrderListEnvelope>> {
    privileged.require(&state.policy, PERM_READ)?;

    let limit = q.limit.unwrap_or(DEFAULT_LIMIT).clamp(1, MAX_LIMIT);
    let offset = q.offset.unwrap_or(0).max(0);
    let pattern = q
        .q
        .as_deref()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(|s| format!("%{}%", s.to_lowercase()));

    let status = match q.status.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        None => None,
        Some(s) => Some(
            OrderStatus::parse(&s.to_lowercase())
                .ok_or_else(|| AppError::BadRequest(format!("unknown status: {s}")))?,
        ),
    };
    let status_str = status.map(|s| s.as_str().to_string());

    let where_sql = r#"
        WHERE ($1::text IS NULL OR lower(email) LIKE $1 OR lower(number) LIKE $1)
          AND ($2::text IS NULL OR status::text = $2)
    "#;

    let list_sql = format!(
        r#"
        SELECT id, number, user_id, cart_id, status::text AS status, currency,
               subtotal_cents, discount_cents, tax_cents, total_cents, email,
               stripe_payment_intent_id, stripe_customer_id, idempotency_key,
               metadata, placed_at, completed_at, created_at, updated_at
          FROM orders {where_sql}
         ORDER BY created_at DESC
         LIMIT $3 OFFSET $4
        "#,
    );
    let count_sql = format!("SELECT COUNT(*) FROM orders {where_sql}");

    let data = sqlx::query_as::<_, orders_repo::Order>(&list_sql)
        .bind(pattern.as_deref())
        .bind(status_str.as_deref())
        .bind(limit)
        .bind(offset)
        .fetch_all(&state.db)
        .await?;

    let total: (i64,) = sqlx::query_as(&count_sql)
        .bind(pattern.as_deref())
        .bind(status_str.as_deref())
        .fetch_one(&state.db)
        .await?;

    let per_page = limit;
    let page = (offset / per_page.max(1)) + 1;
    let total_pages = (total.0 as f64 / per_page as f64).ceil() as i64;

    Ok(Json(OrderListEnvelope {
        data,
        total: total.0,
        page,
        per_page,
        total_pages,
    }))
}

#[utoipa::path(
    get,
    path = "/api/admin/orders/{id}",
    tag = "admin-orders",
    security(("bearer_auth" = [])),
    params(("id" = Uuid, Path, description = "Order id")),
    responses(
        (status = 200, description = "Order detail", body = OrderDetail),
        (status = 401, description = "Unauthenticated"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Order not found")
    )
)]
pub async fn read_one(
    State(state): State<AppState>,
    privileged: PrivilegedUser,
    Path(id): Path<Uuid>,
) -> AppResult<Json<OrderDetail>> {
    privileged.require(&state.policy, PERM_READ)?;

    let order = orders_repo::get_order(&state.db, id)
        .await?
        .ok_or_else(|| AppError::NotFound("Order not found".to_string()))?;

    let items = sqlx::query_as::<_, orders_repo::OrderItem>(
        "SELECT * FROM order_items WHERE order_id = $1 ORDER BY created_at",
    )
    .bind(id)
    .fetch_all(&state.db)
    .await?;
    let refunds = sqlx::query_as::<_, orders_repo::OrderRefund>(
        "SELECT * FROM order_refunds WHERE order_id = $1 ORDER BY created_at",
    )
    .bind(id)
    .fetch_all(&state.db)
    .await?;
    let notes = sqlx::query_as::<_, orders_repo::OrderNote>(
        "SELECT * FROM order_notes WHERE order_id = $1 ORDER BY created_at",
    )
    .bind(id)
    .fetch_all(&state.db)
    .await?;

    let refunded_cents: i64 = refunds.iter().map(|r| r.amount_cents).sum();
    let remaining = (order.total_cents - refunded_cents).max(0);

    Ok(Json(OrderDetail {
        order,
        items,
        refunds,
        notes,
        refunded_cents,
        remaining_refundable_cents: remaining,
    }))
}

#[utoipa::path(
    post,
    path = "/api/admin/orders",
    tag = "admin-orders",
    security(("bearer_auth" = [])),
    request_body = ManualOrderRequest,
    responses(
        (status = 201, description = "Manual order created", body = OrderDetail),
        (status = 400, description = "Validation failed"),
        (status = 401, description = "Unauthenticated"),
        (status = 403, description = "Forbidden")
    )
)]
pub async fn create_manual(
    State(state): State<AppState>,
    privileged: PrivilegedUser,
    client: ClientInfo,
    Json(req): Json<ManualOrderRequest>,
) -> AppResult<(StatusCode, Json<OrderDetail>)> {
    privileged.require(&state.policy, PERM_CREATE)?;
    req.validate().map_err(|e| AppError::BadRequest(e.to_string()))?;
    for item in &req.items {
        item.validate()
            .map_err(|e| AppError::BadRequest(e.to_string()))?;
    }

    let discount_cents = req.discount_cents.unwrap_or(0);
    let tax_cents = req.tax_cents.unwrap_or(0);
    let subtotal_cents: i64 = req
        .items
        .iter()
        .map(|i| i.unit_price_cents * i64::from(i.quantity))
        .sum();
    let total_cents = (subtotal_cents - discount_cents + tax_cents).max(0);

    // Mint a number outside the TX so a serialization retry doesn't
    // burn sequence values.
    let number = orders_repo::next_order_number(&state.db).await?;

    let mut tx = state.db.begin().await?;

    let order_row = sqlx::query_as::<_, orders_repo::Order>(
        r#"
        INSERT INTO orders
            (number, user_id, status, currency,
             subtotal_cents, discount_cents, tax_cents, total_cents, email,
             metadata, placed_at)
        VALUES ($1, $2, 'pending'::order_status, $3, $4, $5, $6, $7, $8, $9, NOW())
        RETURNING id, number, user_id, cart_id, status::text AS status, currency,
                  subtotal_cents, discount_cents, tax_cents, total_cents, email,
                  stripe_payment_intent_id, stripe_customer_id, idempotency_key,
                  metadata, placed_at, completed_at, created_at, updated_at
        "#,
    )
    .bind(number)
    .bind(req.user_id)
    .bind(&req.currency)
    .bind(subtotal_cents)
    .bind(discount_cents)
    .bind(tax_cents)
    .bind(total_cents)
    .bind(&req.email)
    .bind(serde_json::json!({"manual": true}))
    .fetch_one(&mut *tx)
    .await?;

    for item in &req.items {
        sqlx::query(
            r#"
            INSERT INTO order_items
                (order_id, product_id, sku, name, quantity, unit_price_cents, line_total_cents)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            "#,
        )
        .bind(order_row.id)
        .bind(item.product_id)
        .bind(item.sku.as_deref())
        .bind(&item.name)
        .bind(item.quantity)
        .bind(item.unit_price_cents)
        .bind(item.unit_price_cents * i64::from(item.quantity))
        .execute(&mut *tx)
        .await
        .map_err(|e| match &e {
            sqlx::Error::Database(db_err) if db_err.code().as_deref() == Some("23503") => {
                AppError::BadRequest(format!("unknown product id: {}", item.product_id))
            }
            _ => AppError::Database(e),
        })?;
    }

    if let Some(note) = req.notes.as_deref() {
        sqlx::query(
            "INSERT INTO order_notes (order_id, author_id, kind, body) VALUES ($1, $2, 'internal', $3)",
        )
        .bind(order_row.id)
        .bind(privileged.user_id)
        .bind(note)
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;

    // If the operator marked the order completed, walk the FSM out of
    // the TX so each transition writes its own state-log row exactly
    // like the webhook reconciler does.
    if req.mark_completed {
        orders_repo::transition(
            &state.db,
            order_row.id,
            OrderStatus::Processing,
            Some(privileged.user_id),
            Some("manual:create"),
        )
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("transition pending→processing: {e}")))?;
        orders_repo::transition(
            &state.db,
            order_row.id,
            OrderStatus::Completed,
            Some(privileged.user_id),
            Some("manual:create"),
        )
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("transition processing→completed: {e}")))?;
    }

    audit_admin_priv(
        &state.db,
        &privileged,
        &client,
        "admin.order.create",
        "order",
        order_row.id.to_string(),
        serde_json::json!({
            "number":         order_row.number,
            "email":          order_row.email,
            "total_cents":    total_cents,
            "items":          req.items.len(),
            "mark_completed": req.mark_completed,
        }),
    )
    .await;

    let detail = read_one_inner(&state, order_row.id).await?;
    Ok((StatusCode::CREATED, Json(detail)))
}

#[utoipa::path(
    post,
    path = "/api/admin/orders/{id}/void",
    tag = "admin-orders",
    security(("bearer_auth" = [])),
    params(("id" = Uuid, Path, description = "Order id")),
    request_body = VoidRequest,
    responses(
        (status = 200, description = "Order cancelled", body = OrderDetail),
        (status = 400, description = "Order is in a terminal state"),
        (status = 401, description = "Unauthenticated"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Order not found")
    )
)]
pub async fn void_order(
    State(state): State<AppState>,
    privileged: PrivilegedUser,
    client: ClientInfo,
    Path(id): Path<Uuid>,
    Json(req): Json<VoidRequest>,
) -> AppResult<Json<OrderDetail>> {
    privileged.require(&state.policy, PERM_VOID)?;
    req.validate().map_err(|e| AppError::BadRequest(e.to_string()))?;

    orders_repo::transition(
        &state.db,
        id,
        OrderStatus::Cancelled,
        Some(privileged.user_id),
        req.reason.as_deref(),
    )
    .await
    .map_err(|e| match e {
        orders_repo::OrderError::NotFound(_) => AppError::NotFound("Order not found".into()),
        orders_repo::OrderError::IllegalTransition { from, to } => AppError::BadRequest(format!(
            "cannot void order in state {from:?} (target {to:?})"
        )),
        orders_repo::OrderError::RefundOverbalance { .. } => AppError::BadRequest(e.to_string()),
    })?;

    audit_admin_priv(
        &state.db,
        &privileged,
        &client,
        "admin.order.void",
        "order",
        id.to_string(),
        serde_json::json!({ "reason": req.reason }),
    )
    .await;

    let detail = read_one_inner(&state, id).await?;
    Ok(Json(detail))
}

#[utoipa::path(
    post,
    path = "/api/admin/orders/{id}/refund",
    tag = "admin-orders",
    security(("bearer_auth" = [])),
    params(("id" = Uuid, Path, description = "Order id")),
    request_body = RefundRequest,
    responses(
        (status = 200, description = "Refund recorded", body = RefundResponse),
        (status = 400, description = "Refund exceeds remaining balance"),
        (status = 401, description = "Unauthenticated"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Order not found")
    )
)]
pub async fn refund_order(
    State(state): State<AppState>,
    privileged: PrivilegedUser,
    client: ClientInfo,
    Path(id): Path<Uuid>,
    Json(req): Json<RefundRequest>,
) -> AppResult<Json<RefundResponse>> {
    privileged.require(&state.policy, PERM_REFUND)?;
    req.validate().map_err(|e| AppError::BadRequest(e.to_string()))?;

    let mut tx = state.db.begin().await?;

    let order_row = sqlx::query(
        "SELECT total_cents, status::text AS status FROM orders WHERE id = $1 FOR UPDATE",
    )
    .bind(id)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| AppError::NotFound("Order not found".into()))?;
    let total_cents: i64 = order_row.try_get("total_cents").map_err(|e| AppError::Internal(e.into()))?;
    let status_str: String = order_row.try_get("status").map_err(|e| AppError::Internal(e.into()))?;
    let status = OrderStatus::parse(&status_str).ok_or_else(|| {
        AppError::Internal(anyhow::anyhow!("unknown status string: {status_str}"))
    })?;

    let refunded_cents: i64 = sqlx::query_scalar(
        "SELECT COALESCE(SUM(amount_cents), 0)::bigint FROM order_refunds WHERE order_id = $1",
    )
    .bind(id)
    .fetch_one(&mut *tx)
    .await?;

    let remaining = total_cents - refunded_cents;
    if req.amount_cents > remaining {
        return Err(AppError::BadRequest(format!(
            "refund {} exceeds remaining balance {remaining}",
            req.amount_cents
        )));
    }
    if matches!(status, OrderStatus::Cancelled | OrderStatus::Failed) {
        return Err(AppError::BadRequest(format!(
            "cannot refund order in {status:?} state"
        )));
    }

    let refund = sqlx::query_as::<_, orders_repo::OrderRefund>(
        r#"
        INSERT INTO order_refunds (order_id, amount_cents, reason, stripe_refund_id, created_by)
        VALUES ($1, $2, $3, $4, $5)
        RETURNING *
        "#,
    )
    .bind(id)
    .bind(req.amount_cents)
    .bind(req.reason.as_deref())
    .bind(req.stripe_refund_id.as_deref())
    .bind(privileged.user_id)
    .fetch_one(&mut *tx)
    .await?;

    tx.commit().await?;

    let new_total_refunded = refunded_cents + req.amount_cents;
    let new_remaining = total_cents - new_total_refunded;

    // If the order is fully refunded and currently in Completed,
    // transition it to Refunded. Anything else (Pending / Processing
    // / OnHold etc.) is left alone — the partial refund is recorded
    // but the FSM hasn't reached its terminal "all paid back" state.
    let mut order_marked_refunded = false;
    if new_remaining == 0 && matches!(status, OrderStatus::Completed) {
        if let Err(e) = orders_repo::transition(
            &state.db,
            id,
            OrderStatus::Refunded,
            Some(privileged.user_id),
            Some("admin:refund"),
        )
        .await
        {
            tracing::warn!(?e, %id, "refund FSM transition completed→refunded failed");
        } else {
            order_marked_refunded = true;
        }
    }

    audit_admin_priv(
        &state.db,
        &privileged,
        &client,
        "admin.order.refund",
        "order",
        id.to_string(),
        serde_json::json!({
            "amount_cents":    req.amount_cents,
            "reason":          req.reason,
            "stripe_refund_id": req.stripe_refund_id,
            "fully_refunded":  order_marked_refunded,
        }),
    )
    .await;

    Ok(Json(RefundResponse {
        refund,
        order_marked_refunded,
        remaining_refundable_cents: new_remaining,
    }))
}

#[utoipa::path(
    get,
    path = "/api/admin/orders/export.csv",
    tag = "admin-orders",
    security(("bearer_auth" = [])),
    params(
        ("status" = Option<String>, Query, description = "Order status filter"),
        ("q"      = Option<String>, Query, description = "Substring across email + number"),
    ),
    responses(
        (status = 200, description = "CSV stream", content_type = "text/csv"),
        (status = 401, description = "Unauthenticated"),
        (status = 403, description = "Forbidden")
    )
)]
pub async fn export_csv(
    State(state): State<AppState>,
    privileged: PrivilegedUser,
    client: ClientInfo,
    Query(q): Query<ListQuery>,
) -> AppResult<Response> {
    privileged.require(&state.policy, PERM_EXPORT)?;

    let pattern = q
        .q
        .as_deref()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(|s| format!("%{}%", s.to_lowercase()));
    let status = match q.status.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        None => None,
        Some(s) => Some(
            OrderStatus::parse(&s.to_lowercase())
                .ok_or_else(|| AppError::BadRequest(format!("unknown status: {s}")))?,
        ),
    };
    let status_str = status.map(|s| s.as_str().to_string());

    // We cap at MAX_EXPORT_ROWS so a runaway filter never holds a
    // connection forever; finance can pull subsequent pages with
    // a `created_at` lower-bound filter (out-of-scope for ADM-12).
    let rows = sqlx::query_as::<_, orders_repo::Order>(
        r#"
        SELECT id, number, user_id, cart_id, status::text AS status, currency,
               subtotal_cents, discount_cents, tax_cents, total_cents, email,
               stripe_payment_intent_id, stripe_customer_id, idempotency_key,
               metadata, placed_at, completed_at, created_at, updated_at
          FROM orders
         WHERE ($1::text IS NULL OR lower(email) LIKE $1 OR lower(number) LIKE $1)
           AND ($2::text IS NULL OR status::text = $2)
         ORDER BY created_at DESC
         LIMIT $3
        "#,
    )
    .bind(pattern.as_deref())
    .bind(status_str.as_deref())
    .bind(MAX_EXPORT_ROWS)
    .fetch_all(&state.db)
    .await?;

    let mut buf = String::with_capacity(4096 + rows.len() * 256);
    buf.push_str(
        "id,number,user_id,status,currency,subtotal_cents,discount_cents,tax_cents,total_cents,email,placed_at,completed_at,created_at\n",
    );
    for r in &rows {
        let _ = writeln!(
            buf,
            "{id},{number},{user},{status},{currency},{subtotal},{discount},{tax},{total},{email},{placed},{completed},{created}",
            id        = r.id,
            number    = csv_escape(&r.number),
            user      = r.user_id.map(|u| u.to_string()).unwrap_or_default(),
            status    = r.status,
            currency  = r.currency,
            subtotal  = r.subtotal_cents,
            discount  = r.discount_cents,
            tax       = r.tax_cents,
            total     = r.total_cents,
            email     = csv_escape(&r.email),
            placed    = r.placed_at.map(format_ts).unwrap_or_default(),
            completed = r.completed_at.map(format_ts).unwrap_or_default(),
            created   = format_ts(r.created_at),
        );
    }

    audit_admin_priv(
        &state.db,
        &privileged,
        &client,
        "admin.order.export",
        "order",
        "*".to_string(),
        serde_json::json!({
            "rows":   rows.len(),
            "filter": { "q": q.q, "status": q.status },
        }),
    )
    .await;

    let body = buf.into_bytes();
    Ok((
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, "text/csv; charset=utf-8".to_string()),
            (
                header::CONTENT_DISPOSITION,
                "attachment; filename=\"orders.csv\"".to_string(),
            ),
        ],
        body,
    )
        .into_response())
}

// ── Internals ──────────────────────────────────────────────────────────

async fn read_one_inner(state: &AppState, id: Uuid) -> AppResult<OrderDetail> {
    let order = orders_repo::get_order(&state.db, id)
        .await?
        .ok_or_else(|| AppError::NotFound("Order not found".to_string()))?;
    let items = sqlx::query_as::<_, orders_repo::OrderItem>(
        "SELECT * FROM order_items WHERE order_id = $1 ORDER BY created_at",
    )
    .bind(id)
    .fetch_all(&state.db)
    .await?;
    let refunds = sqlx::query_as::<_, orders_repo::OrderRefund>(
        "SELECT * FROM order_refunds WHERE order_id = $1 ORDER BY created_at",
    )
    .bind(id)
    .fetch_all(&state.db)
    .await?;
    let notes = sqlx::query_as::<_, orders_repo::OrderNote>(
        "SELECT * FROM order_notes WHERE order_id = $1 ORDER BY created_at",
    )
    .bind(id)
    .fetch_all(&state.db)
    .await?;
    let refunded_cents: i64 = refunds.iter().map(|r| r.amount_cents).sum();
    let remaining = (order.total_cents - refunded_cents).max(0);
    Ok(OrderDetail {
        order,
        items,
        refunds,
        notes,
        refunded_cents,
        remaining_refundable_cents: remaining,
    })
}

fn csv_escape(s: &str) -> String {
    if s.contains(',') || s.contains('"') || s.contains('\n') {
        let escaped = s.replace('"', "\"\"");
        format!("\"{escaped}\"")
    } else {
        s.to_string()
    }
}

fn format_ts(ts: DateTime<Utc>) -> String {
    ts.to_rfc3339()
}
