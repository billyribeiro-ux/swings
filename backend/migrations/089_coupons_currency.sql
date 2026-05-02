-- Forensic Wave-1 PR-5 (H-6): coupons gain a currency column.
--
-- Migration 085 added a `currency` column to `coupon_usages` so each
-- redemption recorded the currency it was applied in. But the parent
-- `coupons` table never had a currency column, so a USD-denominated
-- "5_000 cents off" coupon could be redeemed against a EUR cart and
-- subtract €50 — a 6× over-discount at the time of writing.
--
-- The fix is an explicit per-coupon currency. NULL is preserved as a
-- "currency-agnostic" mode: percent-discount coupons are universal by
-- nature and can stay NULL; legacy fixed-amount coupons keep their
-- existing semantics until backfilled. The redemption handler checks
-- `coupon.currency` against the cart currency and rejects mismatches
-- when both sides are populated.

ALTER TABLE coupons
    ADD COLUMN IF NOT EXISTS currency TEXT;

-- Length-bound the new column to ISO-4217 alpha-3 codes (lowercase,
-- following the project-wide convention used by `coupon_usages.currency`
-- in migration 085).
ALTER TABLE coupons
    ADD CONSTRAINT coupons_currency_format
    CHECK (currency IS NULL OR (currency = LOWER(currency) AND length(currency) = 3))
    NOT VALID;

-- Mark valid so future inserts are bound; existing NULL rows pass the
-- IS NULL branch.
ALTER TABLE coupons VALIDATE CONSTRAINT coupons_currency_format;
