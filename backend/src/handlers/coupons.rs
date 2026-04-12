use axum::{
    extract::{Path, Query, State},
    routing::{delete, get, post, put},
    Json, Router,
};
use chrono::Utc;
use rand::Rng;
use rust_decimal::prelude::ToPrimitive;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

use crate::{
    error::{AppError, AppResult},
    extractors::{AuthUser, AdminUser},
    models::*,
    AppState,
};

// ── Admin Coupon Router ────────────────────────────────────────────────

pub fn admin_router() -> Router<AppState> {
    Router::new()
        .route("/coupons", get(admin_list_coupons))
        .route("/coupons", post(admin_create_coupon))
        .route("/coupons/bulk", post(admin_bulk_create_coupons))
        .route("/coupons/{id}", get(admin_get_coupon))
        .route("/coupons/{id}", put(admin_update_coupon))
        .route("/coupons/{id}", delete(admin_delete_coupon))
        .route("/coupons/{id}/toggle", post(admin_toggle_coupon))
        .route("/coupons/{id}/usages", get(admin_list_coupon_usages))
}

// ── Public Coupon Router ───────────────────────────────────────────────

pub fn public_router() -> Router<AppState> {
    Router::new()
        .route("/coupons/validate", post(public_validate_coupon))
        .route("/coupons/apply", post(public_apply_coupon))
}

// ── Query / Request / Response types ───────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct CouponListParams {
    pub page: Option<i64>,
    pub per_page: Option<i64>,
    pub filter: Option<String>, // "active", "expired", "depleted"
    pub search: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct CouponWithStats {
    #[serde(flatten)]
    pub coupon: Coupon,
    pub total_usages: i64,
    pub total_discount_cents: i64,
}

#[derive(Debug, Serialize)]
pub struct CouponUsageWithUser {
    pub id: Uuid,
    pub coupon_id: Uuid,
    pub user_id: Uuid,
    pub user_name: Option<String>,
    pub user_email: Option<String>,
    pub subscription_id: Option<Uuid>,
    pub discount_applied_cents: i32,
    pub used_at: chrono::DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct ApplyCouponRequest {
    pub code: String,
    pub plan_id: Option<Uuid>,
    pub course_id: Option<Uuid>,
    pub amount_cents: i32,
    pub subscription_id: Option<Uuid>,
}

// ── Helpers ────────────────────────────────────────────────────────────

fn generate_random_code(len: usize) -> String {
    const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
    let mut rng = rand::thread_rng();
    (0..len)
        .map(|_| {
            let idx = rng.gen_range(0..CHARSET.len());
            CHARSET[idx] as char
        })
        .collect()
}

fn generate_coupon_code(prefix: Option<&str>) -> String {
    let random_part = generate_random_code(8);
    match prefix {
        Some(p) if !p.is_empty() => format!("{}-{}", p.to_uppercase(), random_part),
        _ => random_part,
    }
}

/// Calculate the discount amount in cents for a given coupon and purchase amount.
fn calculate_discount(coupon: &Coupon, amount_cents: i32) -> Option<i32> {
    match coupon.discount_type {
        DiscountType::Percentage => {
            let value = coupon.discount_value.to_f64().unwrap_or(0.0);
            let raw = (amount_cents as f64 * value / 100.0).round() as i32;
            let capped = match coupon.max_discount_cents {
                Some(max) => raw.min(max),
                None => raw,
            };
            Some(capped.min(amount_cents).max(0))
        }
        DiscountType::FixedAmount => {
            let raw = coupon.discount_value.to_i32().unwrap_or(0);
            let capped = match coupon.max_discount_cents {
                Some(max) => raw.min(max),
                None => raw,
            };
            Some(capped.min(amount_cents).max(0))
        }
        DiscountType::FreeTrial => None,
    }
}

/// Shared validation logic. Returns the coupon if valid, or an error message.
async fn validate_coupon_inner(
    pool: &sqlx::PgPool,
    code: &str,
    plan_id: Option<Uuid>,
    course_id: Option<Uuid>,
    user_id: Option<Uuid>,
) -> Result<Coupon, String> {
    // 1. Code exists and is_active
    let coupon: Option<Coupon> = sqlx::query_as(
        "SELECT * FROM coupons WHERE UPPER(code) = UPPER($1)"
    )
    .bind(code)
    .fetch_optional(pool)
    .await
    .map_err(|e| format!("Database error: {e}"))?;

    let coupon = coupon.ok_or_else(|| "Coupon code not found".to_string())?;

    if !coupon.is_active {
        return Err("Coupon is not active".to_string());
    }

    // 2. Check time window
    let now = Utc::now();
    if let Some(starts_at) = coupon.starts_at {
        if now < starts_at {
            return Err("Coupon is not yet valid".to_string());
        }
    }
    if let Some(expires_at) = coupon.expires_at {
        if now > expires_at {
            return Err("Coupon has expired".to_string());
        }
    }

    // 3. Check global usage limit
    if let Some(limit) = coupon.usage_limit {
        if coupon.usage_count >= limit {
            return Err("Coupon usage limit has been reached".to_string());
        }
    }

    // 4. Check per-user limit
    if let Some(uid) = user_id {
        let user_usage_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM coupon_usages WHERE coupon_id = $1 AND user_id = $2"
        )
        .bind(coupon.id)
        .bind(uid)
        .fetch_one(pool)
        .await
        .map_err(|e| format!("Database error: {e}"))?;

        if user_usage_count >= coupon.per_user_limit as i64 {
            return Err("You have already used this coupon the maximum number of times".to_string());
        }
    }

    // 5. Check applies_to scope
    match coupon.applies_to.as_str() {
        "all" => { /* applies to everything */ }
        "plan" => {
            if let Some(pid) = plan_id {
                if !coupon.applicable_plan_ids.is_empty() && !coupon.applicable_plan_ids.contains(&pid) {
                    return Err("Coupon does not apply to this plan".to_string());
                }
            }
        }
        "course" => {
            if let Some(cid) = course_id {
                if !coupon.applicable_course_ids.is_empty() && !coupon.applicable_course_ids.contains(&cid) {
                    return Err("Coupon does not apply to this course".to_string());
                }
            }
        }
        _ => { /* unknown scope, allow */ }
    }

    Ok(coupon)
}

// ══════════════════════════════════════════════════════════════════════
// ADMIN HANDLERS
// ══════════════════════════════════════════════════════════════════════

async fn admin_list_coupons(
    State(state): State<AppState>,
    _admin: AdminUser,
    Query(params): Query<CouponListParams>,
) -> AppResult<Json<PaginatedResponse<Coupon>>> {
    let per_page = params.per_page.unwrap_or(20).min(100).max(1);
    let page = params.page.unwrap_or(1).max(1);
    let offset = (page - 1) * per_page;

    let filter = params.filter.as_deref().unwrap_or("all");
    let search_pattern = params.search.as_deref().map(|s| format!("%{}%", s));

    let (coupons, total): (Vec<Coupon>, i64) = match filter {
        "active" => {
            let rows: Vec<Coupon> = sqlx::query_as(
                r#"
                SELECT * FROM coupons
                WHERE is_active = true
                  AND (expires_at IS NULL OR expires_at > NOW())
                  AND (usage_limit IS NULL OR usage_count < usage_limit)
                  AND ($1::text IS NULL OR code ILIKE $1 OR description ILIKE $1)
                ORDER BY created_at DESC
                LIMIT $2 OFFSET $3
                "#,
            )
            .bind(&search_pattern)
            .bind(per_page)
            .bind(offset)
            .fetch_all(&state.db)
            .await?;

            let count: i64 = sqlx::query_scalar(
                r#"
                SELECT COUNT(*) FROM coupons
                WHERE is_active = true
                  AND (expires_at IS NULL OR expires_at > NOW())
                  AND (usage_limit IS NULL OR usage_count < usage_limit)
                  AND ($1::text IS NULL OR code ILIKE $1 OR description ILIKE $1)
                "#,
            )
            .bind(&search_pattern)
            .fetch_one(&state.db)
            .await?;

            (rows, count)
        }
        "expired" => {
            let rows: Vec<Coupon> = sqlx::query_as(
                r#"
                SELECT * FROM coupons
                WHERE expires_at IS NOT NULL AND expires_at <= NOW()
                  AND ($1::text IS NULL OR code ILIKE $1 OR description ILIKE $1)
                ORDER BY created_at DESC
                LIMIT $2 OFFSET $3
                "#,
            )
            .bind(&search_pattern)
            .bind(per_page)
            .bind(offset)
            .fetch_all(&state.db)
            .await?;

            let count: i64 = sqlx::query_scalar(
                r#"
                SELECT COUNT(*) FROM coupons
                WHERE expires_at IS NOT NULL AND expires_at <= NOW()
                  AND ($1::text IS NULL OR code ILIKE $1 OR description ILIKE $1)
                "#,
            )
            .bind(&search_pattern)
            .fetch_one(&state.db)
            .await?;

            (rows, count)
        }
        "depleted" => {
            let rows: Vec<Coupon> = sqlx::query_as(
                r#"
                SELECT * FROM coupons
                WHERE usage_limit IS NOT NULL AND usage_count >= usage_limit
                  AND ($1::text IS NULL OR code ILIKE $1 OR description ILIKE $1)
                ORDER BY created_at DESC
                LIMIT $2 OFFSET $3
                "#,
            )
            .bind(&search_pattern)
            .bind(per_page)
            .bind(offset)
            .fetch_all(&state.db)
            .await?;

            let count: i64 = sqlx::query_scalar(
                r#"
                SELECT COUNT(*) FROM coupons
                WHERE usage_limit IS NOT NULL AND usage_count >= usage_limit
                  AND ($1::text IS NULL OR code ILIKE $1 OR description ILIKE $1)
                "#,
            )
            .bind(&search_pattern)
            .fetch_one(&state.db)
            .await?;

            (rows, count)
        }
        _ => {
            // "all" or unrecognized
            let rows: Vec<Coupon> = sqlx::query_as(
                r#"
                SELECT * FROM coupons
                WHERE ($1::text IS NULL OR code ILIKE $1 OR description ILIKE $1)
                ORDER BY created_at DESC
                LIMIT $2 OFFSET $3
                "#,
            )
            .bind(&search_pattern)
            .bind(per_page)
            .bind(offset)
            .fetch_all(&state.db)
            .await?;

            let count: i64 = sqlx::query_scalar(
                r#"
                SELECT COUNT(*) FROM coupons
                WHERE ($1::text IS NULL OR code ILIKE $1 OR description ILIKE $1)
                "#,
            )
            .bind(&search_pattern)
            .fetch_one(&state.db)
            .await?;

            (rows, count)
        }
    };

    let total_pages = (total as f64 / per_page as f64).ceil() as i64;

    Ok(Json(PaginatedResponse {
        data: coupons,
        total,
        page,
        per_page,
        total_pages,
    }))
}

async fn admin_create_coupon(
    State(state): State<AppState>,
    admin: AdminUser,
    Json(req): Json<CreateCouponRequest>,
) -> AppResult<Json<Coupon>> {
    req.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;

    let code = match req.code {
        Some(ref c) if !c.is_empty() => c.to_uppercase(),
        _ => generate_coupon_code(None),
    };

    // Check for duplicate code
    let existing: Option<(Uuid,)> = sqlx::query_as(
        "SELECT id FROM coupons WHERE UPPER(code) = UPPER($1)"
    )
    .bind(&code)
    .fetch_optional(&state.db)
    .await?;

    if existing.is_some() {
        return Err(AppError::Conflict(format!("Coupon code '{}' already exists", code)));
    }

    let coupon: Coupon = sqlx::query_as(
        r#"
        INSERT INTO coupons (
            id, code, description, discount_type, discount_value,
            min_purchase_cents, max_discount_cents, applies_to,
            applicable_plan_ids, applicable_course_ids,
            usage_limit, usage_count, per_user_limit,
            starts_at, expires_at, is_active, stackable, first_purchase_only,
            created_by, created_at, updated_at
        ) VALUES (
            $1, $2, $3, $4, $5,
            $6, $7, $8,
            $9, $10,
            $11, 0, $12,
            $13, $14, $15, $16, $17,
            $18, NOW(), NOW()
        )
        RETURNING *
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(&code)
    .bind(&req.description)
    .bind(&req.discount_type)
    .bind(rust_decimal::Decimal::from_f64_retain(req.discount_value).unwrap_or_default())
    .bind(req.min_purchase_cents)
    .bind(req.max_discount_cents)
    .bind(req.applies_to.as_deref().unwrap_or("all"))
    .bind(req.applicable_plan_ids.as_deref().unwrap_or(&[]))
    .bind(req.applicable_course_ids.as_deref().unwrap_or(&[]))
    .bind(req.usage_limit)
    .bind(req.per_user_limit.unwrap_or(1))
    .bind(req.starts_at)
    .bind(req.expires_at)
    .bind(req.is_active.unwrap_or(true))
    .bind(req.stackable.unwrap_or(false))
    .bind(req.first_purchase_only.unwrap_or(false))
    .bind(admin.user_id)
    .fetch_one(&state.db)
    .await?;

    Ok(Json(coupon))
}

async fn admin_get_coupon(
    State(state): State<AppState>,
    _admin: AdminUser,
    Path(id): Path<Uuid>,
) -> AppResult<Json<CouponWithStats>> {
    let coupon: Coupon = sqlx::query_as(
        "SELECT * FROM coupons WHERE id = $1"
    )
    .bind(id)
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NotFound("Coupon not found".to_string()))?;

    let stats: (i64, Option<i64>) = sqlx::query_as(
        r#"
        SELECT COUNT(*), COALESCE(SUM(discount_applied_cents::bigint), 0)
        FROM coupon_usages
        WHERE coupon_id = $1
        "#,
    )
    .bind(id)
    .fetch_one(&state.db)
    .await?;

    Ok(Json(CouponWithStats {
        coupon,
        total_usages: stats.0,
        total_discount_cents: stats.1.unwrap_or(0),
    }))
}

async fn admin_update_coupon(
    State(state): State<AppState>,
    _admin: AdminUser,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateCouponRequest>,
) -> AppResult<Json<Coupon>> {
    let existing: Coupon = sqlx::query_as(
        "SELECT * FROM coupons WHERE id = $1"
    )
    .bind(id)
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NotFound("Coupon not found".to_string()))?;

    let discount_type = req.discount_type.unwrap_or(existing.discount_type);
    let discount_value = req.discount_value
        .map(|v| rust_decimal::Decimal::from_f64_retain(v).unwrap_or_default())
        .unwrap_or(existing.discount_value);
    let description = req.description.or(existing.description);
    let min_purchase_cents = req.min_purchase_cents.or(existing.min_purchase_cents);
    let max_discount_cents = req.max_discount_cents.or(existing.max_discount_cents);
    let applies_to = req.applies_to.unwrap_or(existing.applies_to);
    let applicable_plan_ids = req.applicable_plan_ids.unwrap_or(existing.applicable_plan_ids);
    let applicable_course_ids = req.applicable_course_ids.unwrap_or(existing.applicable_course_ids);
    let usage_limit = req.usage_limit.or(existing.usage_limit);
    let per_user_limit = req.per_user_limit.unwrap_or(existing.per_user_limit);
    let starts_at = req.starts_at.or(existing.starts_at);
    let expires_at = req.expires_at.or(existing.expires_at);
    let is_active = req.is_active.unwrap_or(existing.is_active);
    let stackable = req.stackable.unwrap_or(existing.stackable);
    let first_purchase_only = req.first_purchase_only.unwrap_or(existing.first_purchase_only);

    let coupon: Coupon = sqlx::query_as(
        r#"
        UPDATE coupons SET
            description = $1,
            discount_type = $2,
            discount_value = $3,
            min_purchase_cents = $4,
            max_discount_cents = $5,
            applies_to = $6,
            applicable_plan_ids = $7,
            applicable_course_ids = $8,
            usage_limit = $9,
            per_user_limit = $10,
            starts_at = $11,
            expires_at = $12,
            is_active = $13,
            stackable = $14,
            first_purchase_only = $15,
            updated_at = NOW()
        WHERE id = $16
        RETURNING *
        "#,
    )
    .bind(&description)
    .bind(&discount_type)
    .bind(discount_value)
    .bind(min_purchase_cents)
    .bind(max_discount_cents)
    .bind(&applies_to)
    .bind(&applicable_plan_ids)
    .bind(&applicable_course_ids)
    .bind(usage_limit)
    .bind(per_user_limit)
    .bind(starts_at)
    .bind(expires_at)
    .bind(is_active)
    .bind(stackable)
    .bind(first_purchase_only)
    .bind(id)
    .fetch_one(&state.db)
    .await?;

    Ok(Json(coupon))
}

async fn admin_delete_coupon(
    State(state): State<AppState>,
    _admin: AdminUser,
    Path(id): Path<Uuid>,
) -> AppResult<Json<serde_json::Value>> {
    let result = sqlx::query("DELETE FROM coupons WHERE id = $1")
        .bind(id)
        .execute(&state.db)
        .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound("Coupon not found".to_string()));
    }

    Ok(Json(serde_json::json!({ "deleted": true })))
}

async fn admin_toggle_coupon(
    State(state): State<AppState>,
    _admin: AdminUser,
    Path(id): Path<Uuid>,
) -> AppResult<Json<Coupon>> {
    let coupon: Coupon = sqlx::query_as(
        r#"
        UPDATE coupons
        SET is_active = NOT is_active, updated_at = NOW()
        WHERE id = $1
        RETURNING *
        "#,
    )
    .bind(id)
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NotFound("Coupon not found".to_string()))?;

    Ok(Json(coupon))
}

async fn admin_bulk_create_coupons(
    State(state): State<AppState>,
    admin: AdminUser,
    Json(req): Json<BulkCouponRequest>,
) -> AppResult<Json<Vec<Coupon>>> {
    if req.count < 1 || req.count > 1000 {
        return Err(AppError::BadRequest(
            "Count must be between 1 and 1000".to_string(),
        ));
    }

    let discount_value = rust_decimal::Decimal::from_f64_retain(req.discount_value)
        .unwrap_or_default();

    let mut created: Vec<Coupon> = Vec::with_capacity(req.count as usize);

    for _ in 0..req.count {
        let code = generate_coupon_code(req.prefix.as_deref());

        let coupon: Coupon = sqlx::query_as(
            r#"
            INSERT INTO coupons (
                id, code, description, discount_type, discount_value,
                min_purchase_cents, max_discount_cents, applies_to,
                applicable_plan_ids, applicable_course_ids,
                usage_limit, usage_count, per_user_limit,
                starts_at, expires_at, is_active, stackable, first_purchase_only,
                created_by, created_at, updated_at
            ) VALUES (
                $1, $2, NULL, $3, $4,
                NULL, NULL, 'all',
                '{}', '{}',
                $5, 0, 1,
                NULL, $6, true, false, false,
                $7, NOW(), NOW()
            )
            RETURNING *
            "#,
        )
        .bind(Uuid::new_v4())
        .bind(&code)
        .bind(&req.discount_type)
        .bind(discount_value)
        .bind(req.usage_limit)
        .bind(req.expires_at)
        .bind(admin.user_id)
        .fetch_one(&state.db)
        .await?;

        created.push(coupon);
    }

    Ok(Json(created))
}

async fn admin_list_coupon_usages(
    State(state): State<AppState>,
    _admin: AdminUser,
    Path(id): Path<Uuid>,
    Query(params): Query<PaginationParams>,
) -> AppResult<Json<PaginatedResponse<CouponUsageWithUser>>> {
    // Verify coupon exists
    let exists: Option<(Uuid,)> = sqlx::query_as(
        "SELECT id FROM coupons WHERE id = $1"
    )
    .bind(id)
    .fetch_optional(&state.db)
    .await?;

    if exists.is_none() {
        return Err(AppError::NotFound("Coupon not found".to_string()));
    }

    let per_page = params.per_page();
    let offset = params.offset();
    let page = params.page.unwrap_or(1).max(1);

    let rows = sqlx::query_as::<_, (Uuid, Uuid, Uuid, Option<String>, Option<String>, Option<Uuid>, i32, chrono::DateTime<Utc>)>(
        r#"
        SELECT
            cu.id,
            cu.coupon_id,
            cu.user_id,
            u.name AS user_name,
            u.email AS user_email,
            cu.subscription_id,
            cu.discount_applied_cents,
            cu.used_at
        FROM coupon_usages cu
        LEFT JOIN users u ON u.id = cu.user_id
        WHERE cu.coupon_id = $1
        ORDER BY cu.used_at DESC
        LIMIT $2 OFFSET $3
        "#,
    )
    .bind(id)
    .bind(per_page)
    .bind(offset)
    .fetch_all(&state.db)
    .await?;

    let usages: Vec<CouponUsageWithUser> = rows
        .into_iter()
        .map(|r| CouponUsageWithUser {
            id: r.0,
            coupon_id: r.1,
            user_id: r.2,
            user_name: r.3,
            user_email: r.4,
            subscription_id: r.5,
            discount_applied_cents: r.6,
            used_at: r.7,
        })
        .collect();

    let total: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM coupon_usages WHERE coupon_id = $1"
    )
    .bind(id)
    .fetch_one(&state.db)
    .await?;

    let total_pages = (total as f64 / per_page as f64).ceil() as i64;

    Ok(Json(PaginatedResponse {
        data: usages,
        total,
        page,
        per_page,
        total_pages,
    }))
}

// ══════════════════════════════════════════════════════════════════════
// PUBLIC HANDLERS
// ══════════════════════════════════════════════════════════════════════

async fn public_validate_coupon(
    State(state): State<AppState>,
    Json(req): Json<ValidateCouponRequest>,
) -> AppResult<Json<CouponValidationResponse>> {
    match validate_coupon_inner(&state.db, &req.code, req.plan_id, req.course_id, None).await {
        Ok(coupon) => {
            let message = match coupon.discount_type {
                DiscountType::Percentage => {
                    let val = coupon.discount_value.to_f64().unwrap_or(0.0);
                    format!("{}% discount", val)
                }
                DiscountType::FixedAmount => {
                    let val = coupon.discount_value.to_i32().unwrap_or(0);
                    format!("${:.2} discount", val as f64 / 100.0)
                }
                DiscountType::FreeTrial => "Free trial period".to_string(),
            };

            Ok(Json(CouponValidationResponse {
                valid: true,
                coupon: Some(coupon),
                discount_amount_cents: None,
                message,
            }))
        }
        Err(msg) => Ok(Json(CouponValidationResponse {
            valid: false,
            coupon: None,
            discount_amount_cents: None,
            message: msg,
        })),
    }
}

async fn public_apply_coupon(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<ApplyCouponRequest>,
) -> AppResult<Json<CouponUsage>> {
    // Validate the coupon with user context
    let coupon = validate_coupon_inner(
        &state.db,
        &req.code,
        req.plan_id,
        req.course_id,
        Some(auth.user_id),
    )
    .await
    .map_err(|msg| AppError::BadRequest(msg))?;

    // Check min purchase
    if let Some(min) = coupon.min_purchase_cents {
        if req.amount_cents < min {
            return Err(AppError::BadRequest(format!(
                "Minimum purchase of ${:.2} required",
                min as f64 / 100.0
            )));
        }
    }

    // Check first_purchase_only
    if coupon.first_purchase_only {
        let has_prior: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM coupon_usages WHERE user_id = $1"
        )
        .bind(auth.user_id)
        .fetch_one(&state.db)
        .await?;

        if has_prior > 0 {
            return Err(AppError::BadRequest(
                "This coupon is only valid for first-time purchases".to_string(),
            ));
        }
    }

    // Calculate discount
    let discount_applied_cents = calculate_discount(&coupon, req.amount_cents).unwrap_or(0);

    // Insert usage record and increment usage_count in a transaction
    let mut tx = state.db.begin().await?;

    let usage: CouponUsage = sqlx::query_as(
        r#"
        INSERT INTO coupon_usages (id, coupon_id, user_id, subscription_id, discount_applied_cents, used_at)
        VALUES ($1, $2, $3, $4, $5, NOW())
        RETURNING *
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(coupon.id)
    .bind(auth.user_id)
    .bind(req.subscription_id)
    .bind(discount_applied_cents)
    .fetch_one(&mut *tx)
    .await?;

    sqlx::query("UPDATE coupons SET usage_count = usage_count + 1, updated_at = NOW() WHERE id = $1")
        .bind(coupon.id)
        .execute(&mut *tx)
        .await?;

    tx.commit().await?;

    Ok(Json(usage))
}
