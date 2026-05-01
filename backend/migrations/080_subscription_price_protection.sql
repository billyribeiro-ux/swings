-- migration: 080_subscription_price_protection
-- Adds per-subscription grandfathering columns so operators can protect
-- existing members from a catalog price change while new signups pay the
-- updated amount.
--
-- grandfathered_price_cents  – the price the member was promised when they
--   signed up (captured from pricing_plans.amount_cents at checkout time).
--   NULL means the record pre-dates this migration; treat as unprotected.
--
-- grandfathered_currency     – ISO-4217 currency code that goes with the
--   above amount.  Defaults to 'usd' if absent on old rows.
--
-- price_protection_enabled   – when TRUE the pricing rollout service SKIPS
--   this subscription regardless of audience setting.  Operators can flip
--   this per-member in the admin console.

ALTER TABLE subscriptions
    ADD COLUMN IF NOT EXISTS grandfathered_price_cents   INTEGER,
    ADD COLUMN IF NOT EXISTS grandfathered_currency      TEXT     DEFAULT 'usd',
    ADD COLUMN IF NOT EXISTS price_protection_enabled    BOOLEAN  NOT NULL DEFAULT FALSE;

-- Index: quick lookup of all protected subscriptions for a given plan.
-- Used by the rollout-preview endpoint to show "will skip N grandfathered".
CREATE INDEX IF NOT EXISTS idx_subscriptions_price_protected
    ON subscriptions (pricing_plan_id)
    WHERE price_protection_enabled = TRUE;

-- Permission: let admins toggle price-protection on individual members.
INSERT INTO permissions (key, description) VALUES
    ('subscription.price_protection.manage',
     'Toggle per-subscription grandfather price protection')
ON CONFLICT (key) DO NOTHING;

INSERT INTO role_permissions (role, permission) VALUES
    ('admin', 'subscription.price_protection.manage')
ON CONFLICT DO NOTHING;
