-- Link Stripe-backed subscriptions to a catalog `pricing_plans` row so
-- admin price rollouts can target the correct members (and stay precise
-- when multiple catalog plans share the same billing cadence).

ALTER TABLE subscriptions
    ADD COLUMN IF NOT EXISTS pricing_plan_id UUID REFERENCES pricing_plans (id) ON DELETE SET NULL;

CREATE INDEX IF NOT EXISTS idx_subscriptions_pricing_plan_id
    ON subscriptions (pricing_plan_id)
    WHERE pricing_plan_id IS NOT NULL;
