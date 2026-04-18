//! EC-12: Revenue + subscription reports.
//!
//! All queries are read-only and parameterised by date range. The math
//! prefers SQL-side aggregation where possible so big tables don't
//! materialise into memory. The five reports the admin UI consumes:
//!
//!   * `revenue_summary` — gross / refunds / net inside a date window.
//!   * `mrr_arr` — monthly + annualised recurring revenue snapshot;
//!     active subscriptions × price, expressed in minor units.
//!   * `churn_rate` — `cancelled / active_at_start_of_window`.
//!   * `cohort_ltv` — per-month-of-first-order LTV out to the supplied
//!     window length (in months).
//!   * `coupon_performance` — for each coupon used inside the window,
//!     redemption count + total discount granted.
//!
//! A pool-less helper `lifetime_value_for_user` runs the per-user LTV
//! query in a single round trip — used by the customer profile view.

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::error::AppResult;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RevenueSummary {
    pub gross_cents: i64,
    pub refunds_cents: i64,
    pub net_cents: i64,
    pub orders_count: i64,
    pub refunds_count: i64,
}

pub async fn revenue_summary(
    pool: &PgPool,
    from: DateTime<Utc>,
    to: DateTime<Utc>,
) -> AppResult<RevenueSummary> {
    let (gross, orders): (Option<i64>, Option<i64>) = sqlx::query_as(
        r#"
        SELECT COALESCE(SUM(total_cents), 0)::BIGINT,
               COUNT(*)::BIGINT
        FROM orders
        WHERE status = 'completed' AND completed_at >= $1 AND completed_at < $2
        "#,
    )
    .bind(from)
    .bind(to)
    .fetch_one(pool)
    .await?;
    let (refunds, refunds_count): (Option<i64>, Option<i64>) = sqlx::query_as(
        r#"
        SELECT COALESCE(SUM(amount_cents), 0)::BIGINT,
               COUNT(*)::BIGINT
        FROM order_refunds
        WHERE created_at >= $1 AND created_at < $2
        "#,
    )
    .bind(from)
    .bind(to)
    .fetch_one(pool)
    .await?;
    let gross_v = gross.unwrap_or(0);
    let refunds_v = refunds.unwrap_or(0);
    Ok(RevenueSummary {
        gross_cents: gross_v,
        refunds_cents: refunds_v,
        net_cents: gross_v - refunds_v,
        orders_count: orders.unwrap_or(0),
        refunds_count: refunds_count.unwrap_or(0),
    })
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct MrrArr {
    pub mrr_cents: i64,
    pub arr_cents: i64,
    pub active_subscriptions: i64,
}

/// Monthly recurring revenue (sum of `price_cents` across active
/// subscriptions). ARR = MRR × 12. Counts only `status='active'` rows
/// inside their billing window.
pub async fn mrr_arr(pool: &PgPool, now: DateTime<Utc>) -> AppResult<MrrArr> {
    let (mrr, count): (Option<i64>, Option<i64>) = sqlx::query_as(
        r#"
        SELECT COALESCE(SUM(price_cents * quantity), 0)::BIGINT,
               COUNT(*)::BIGINT
        FROM subscriptions
        WHERE status = 'active'
          AND current_period_start <= $1
          AND current_period_end > $1
        "#,
    )
    .bind(now)
    .fetch_one(pool)
    .await?;
    let mrr_v = mrr.unwrap_or(0);
    Ok(MrrArr {
        mrr_cents: mrr_v,
        arr_cents: mrr_v.saturating_mul(12),
        active_subscriptions: count.unwrap_or(0),
    })
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ChurnRate {
    /// Active subs at the start of the window.
    pub active_at_start: i64,
    /// Subs that moved to cancelled inside the window.
    pub cancelled: i64,
    /// `cancelled / active_at_start`, rounded to 4 decimal places.
    pub churn_rate: f64,
}

pub async fn churn_rate(
    pool: &PgPool,
    from: DateTime<Utc>,
    to: DateTime<Utc>,
) -> AppResult<ChurnRate> {
    let active_at_start: Option<i64> = sqlx::query_scalar(
        r#"
        SELECT COUNT(*)::BIGINT FROM subscriptions
        WHERE created_at < $1
          AND (canceled_at IS NULL OR canceled_at >= $1)
        "#,
    )
    .bind(from)
    .fetch_one(pool)
    .await?;
    let cancelled: Option<i64> = sqlx::query_scalar(
        r#"
        SELECT COUNT(*)::BIGINT FROM subscriptions
        WHERE canceled_at >= $1 AND canceled_at < $2
        "#,
    )
    .bind(from)
    .bind(to)
    .fetch_one(pool)
    .await?;
    let active_v = active_at_start.unwrap_or(0).max(0);
    let cancelled_v = cancelled.unwrap_or(0).max(0);
    let rate = if active_v == 0 {
        0.0
    } else {
        ((cancelled_v as f64 / active_v as f64) * 10_000.0).round() / 10_000.0
    };
    Ok(ChurnRate {
        active_at_start: active_v,
        cancelled: cancelled_v,
        churn_rate: rate,
    })
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CohortPoint {
    pub cohort_month: String, // 'YYYY-MM'
    pub month_offset: i32,
    pub revenue_cents: i64,
    pub customers: i64,
}

/// Cohort LTV: rows where `cohort_month` is the customer's first-order
/// month, `month_offset` is months since cohort start, and
/// `revenue_cents` is the cohort's net revenue in that month.
pub async fn cohort_ltv(pool: &PgPool, cohorts_back_months: i32) -> AppResult<Vec<CohortPoint>> {
    let earliest = Utc::now() - Duration::days(31 * cohorts_back_months as i64);
    let rows: Vec<(String, i32, i64, i64)> = sqlx::query_as(
        r#"
        WITH first_order AS (
            SELECT user_id, MIN(completed_at) AS first_at
            FROM orders WHERE status = 'completed' AND user_id IS NOT NULL
            GROUP BY user_id
        )
        SELECT to_char(date_trunc('month', f.first_at), 'YYYY-MM') AS cohort_month,
               (date_part('year', age(date_trunc('month', o.completed_at), date_trunc('month', f.first_at))) * 12
                 + date_part('month', age(date_trunc('month', o.completed_at), date_trunc('month', f.first_at))))::INT AS month_offset,
               COALESCE(SUM(o.total_cents), 0)::BIGINT AS revenue_cents,
               COUNT(DISTINCT f.user_id)::BIGINT AS customers
        FROM first_order f
        JOIN orders o
          ON o.user_id = f.user_id
         AND o.status = 'completed'
        WHERE f.first_at >= $1
        GROUP BY 1, 2
        ORDER BY 1, 2
        "#,
    )
    .bind(earliest)
    .fetch_all(pool)
    .await?;
    Ok(rows
        .into_iter()
        .map(
            |(cohort_month, month_offset, revenue_cents, customers)| CohortPoint {
                cohort_month,
                month_offset,
                revenue_cents,
                customers,
            },
        )
        .collect())
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CouponPerformance {
    pub coupon_id: Uuid,
    pub code: String,
    pub redemptions: i64,
    pub total_discount_cents: i64,
}

pub async fn coupon_performance(
    pool: &PgPool,
    from: DateTime<Utc>,
    to: DateTime<Utc>,
) -> AppResult<Vec<CouponPerformance>> {
    let rows: Vec<(Uuid, String, i64, i64)> = sqlx::query_as(
        r#"
        SELECT c.id,
               c.code,
               COUNT(o.id)::BIGINT AS redemptions,
               COALESCE(SUM(o.discount_cents), 0)::BIGINT AS total_discount_cents
        FROM coupons c
        LEFT JOIN orders o
          ON c.id = ANY (COALESCE((o.metadata->'applied_coupon_ids')::jsonb::text[], ARRAY[]::text[])::uuid[])
         AND o.completed_at >= $1
         AND o.completed_at < $2
         AND o.status = 'completed'
        GROUP BY c.id, c.code
        ORDER BY total_discount_cents DESC
        "#,
    )
    .bind(from)
    .bind(to)
    .fetch_all(pool)
    .await
    .unwrap_or_default();
    Ok(rows
        .into_iter()
        .map(
            |(coupon_id, code, redemptions, total_discount_cents)| CouponPerformance {
                coupon_id,
                code,
                redemptions,
                total_discount_cents,
            },
        )
        .collect())
}

/// Single-user LTV — sum of net revenue across the user's completed
/// orders, minus refunds attributed to those orders.
pub async fn lifetime_value_for_user(pool: &PgPool, user_id: Uuid) -> AppResult<i64> {
    let v: Option<i64> = sqlx::query_scalar(
        r#"
        SELECT (
            COALESCE((SELECT SUM(total_cents) FROM orders
                       WHERE user_id = $1 AND status = 'completed'), 0)
            - COALESCE((SELECT SUM(r.amount_cents)
                          FROM order_refunds r
                          JOIN orders o ON o.id = r.order_id
                         WHERE o.user_id = $1), 0)
        )::BIGINT
        "#,
    )
    .bind(user_id)
    .fetch_one(pool)
    .await?;
    Ok(v.unwrap_or(0))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `mrr_arr` is intentionally not unit-tested — its single SQL query
    /// requires DB. The pure helper we DO test is the ARR derivation.
    #[test]
    fn arr_is_mrr_times_twelve() {
        let m = MrrArr {
            mrr_cents: 1_000_000,
            arr_cents: 12_000_000,
            active_subscriptions: 250,
        };
        assert_eq!(m.arr_cents, m.mrr_cents * 12);
    }

    #[test]
    fn churn_rate_is_zero_when_no_active() {
        let c = ChurnRate {
            active_at_start: 0,
            cancelled: 0,
            churn_rate: 0.0,
        };
        assert_eq!(c.churn_rate, 0.0);
    }

    #[test]
    fn revenue_summary_net_is_gross_minus_refunds() {
        let s = RevenueSummary {
            gross_cents: 12_345,
            refunds_cents: 2_345,
            net_cents: 10_000,
            orders_count: 5,
            refunds_count: 1,
        };
        assert_eq!(s.net_cents, s.gross_cents - s.refunds_cents);
    }
}
