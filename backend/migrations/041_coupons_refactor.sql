-- EC-11: Coupon engine refactor.
--
-- Adds the Money-native, BOGO-aware, scope-extended columns the new
-- `commerce::coupons::CouponEngine` consumes. The legacy columns
-- (`discount_value`, `discount_type`, `max_discount_cents`) stay in place
-- for one release cycle behind a compatibility shim; a later migration
-- drops them once the admin UI fully migrates.

ALTER TABLE coupons
    ADD COLUMN IF NOT EXISTS discount_value_cents BIGINT,
    ADD COLUMN IF NOT EXISTS discount_percent_bps INT,     -- basis points (e.g. 2500 = 25%)
    ADD COLUMN IF NOT EXISTS scope TEXT NOT NULL DEFAULT 'cart'
        CHECK (scope IN ('cart','product','category','subscription')),
    ADD COLUMN IF NOT EXISTS bogo_config JSONB,  -- { buy_qty, get_qty, applies_to_product_ids }
    ADD COLUMN IF NOT EXISTS includes_product_ids UUID[] NOT NULL DEFAULT '{}',
    ADD COLUMN IF NOT EXISTS excludes_product_ids UUID[] NOT NULL DEFAULT '{}',
    ADD COLUMN IF NOT EXISTS includes_category_ids UUID[] NOT NULL DEFAULT '{}',
    ADD COLUMN IF NOT EXISTS recurring_mode TEXT NOT NULL DEFAULT 'one_time'
        CHECK (recurring_mode IN ('one_time','forever','repeating'));

-- Backfill new money columns from the existing Decimal column.
--   * fixed -> dollars × 100 = cents.
--   * percentage -> percent × 100 = basis points.
-- The legacy `discount_value` / `discount_type` / `max_discount_cents`
-- columns remain; the service layer now prefers the new columns.
UPDATE coupons
   SET discount_value_cents = CASE
           WHEN discount_type = 'fixed_amount' THEN (discount_value * 100)::BIGINT
       END,
       discount_percent_bps = CASE
           WHEN discount_type = 'percentage' THEN (discount_value * 100)::INT
       END
 WHERE discount_value_cents IS NULL
   AND discount_percent_bps IS NULL;

CREATE INDEX IF NOT EXISTS idx_coupons_scope ON coupons (scope)
    WHERE is_active = TRUE;
