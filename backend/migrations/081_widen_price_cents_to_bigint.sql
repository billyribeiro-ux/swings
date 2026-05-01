-- Widen every "money in cents" column from INTEGER (max ~$21.4M) to BIGINT
-- (max ~$92 quintillion) so the platform can quote any plausible enterprise
-- price without overflow.
--
-- Why now: most price columns were already BIGINT (see `030_products.sql`,
-- `031_cart.sql`, `034_form_payments.sql`, `035_orders.sql`,
-- `041_subscriptions_v2.sql`, `042_memberships.sql`, `054_popup_attributions.sql`,
-- `078_stripe_webhook_expansion.sql`). The four columns below were the
-- last INTEGER stragglers — annual/lifetime SKUs at the high end of the
-- catalogue (or aggregate sums in `analytics`) sit close enough to the
-- INT4 ceiling ($21,474,836.47) that we want headroom *before* a real
-- customer hits it.
--
-- Compatibility: Postgres performs the type change in-place via
-- `pg_attribute` rewrite — INTEGER → BIGINT is one of the lossless
-- promotions that does NOT require a table rewrite for narrow types
-- in PG 16, but the planner WILL take an `ACCESS EXCLUSIVE` lock for
-- the duration. All four tables here are small (< 100k rows in any
-- realistic deployment), so the lock is sub-second.
--
-- Rust models in `backend/src/models.rs` are widened from `i32` →
-- `i64` in the same change so sqlx FromRow keeps decoding cleanly.
-- Tests + clippy enforce the cross-language consistency.

-- ── pricing_plans.amount_cents ──────────────────────────────────────────
ALTER TABLE pricing_plans
    ALTER COLUMN amount_cents TYPE BIGINT USING amount_cents::BIGINT;

-- ── courses.price_cents ────────────────────────────────────────────────
ALTER TABLE courses
    ALTER COLUMN price_cents TYPE BIGINT USING price_cents::BIGINT;

-- ── subscriptions.grandfathered_price_cents ────────────────────────────
ALTER TABLE subscriptions
    ALTER COLUMN grandfathered_price_cents TYPE BIGINT USING grandfathered_price_cents::BIGINT;

-- ── sales_events.amount_cents (analytics) ──────────────────────────────
-- This is the aggregate cents value attached to each tracked sales event.
-- A single event will never approach $21M, but the column is summed in
-- analytics rollups (`SELECT SUM(amount_cents) ...`) and PG's INTEGER
-- aggregation overflows once the running total crosses INT4's ceiling —
-- which is plausible across a year of revenue. BIGINT removes that risk.
ALTER TABLE sales_events
    ALTER COLUMN amount_cents TYPE BIGINT USING amount_cents::BIGINT;

-- ── coupons.max_discount_cents + coupon_usage.discount_applied_cents ───
-- Coupon caps and applied discounts are also money values — keep them on
-- the same i64-cents SSOT as everything else so the type story is uniform
-- end-to-end (Postgres BIGINT ↔ Rust i64 ↔ JSON number, no narrowing
-- conversions, no overflow surface).
ALTER TABLE coupons
    ALTER COLUMN min_purchase_cents TYPE BIGINT USING min_purchase_cents::BIGINT,
    ALTER COLUMN max_discount_cents TYPE BIGINT USING max_discount_cents::BIGINT;
ALTER TABLE coupon_usages
    ALTER COLUMN discount_applied_cents TYPE BIGINT USING discount_applied_cents::BIGINT;
