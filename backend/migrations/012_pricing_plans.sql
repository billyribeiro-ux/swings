-- Migration 012: Admin-configurable pricing plans with change audit log

CREATE TABLE pricing_plans (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name TEXT NOT NULL,
    slug TEXT NOT NULL UNIQUE,
    description TEXT DEFAULT '',
    stripe_price_id TEXT,
    stripe_product_id TEXT,
    amount_cents INTEGER NOT NULL,
    currency TEXT NOT NULL DEFAULT 'usd',
    interval TEXT NOT NULL DEFAULT 'month' CHECK (interval IN ('month', 'year', 'one_time')),
    interval_count INTEGER NOT NULL DEFAULT 1,
    trial_days INTEGER NOT NULL DEFAULT 0,
    features JSONB NOT NULL DEFAULT '[]',
    highlight_text TEXT DEFAULT '',
    is_popular BOOLEAN NOT NULL DEFAULT FALSE,
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    sort_order INTEGER NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_pricing_plans_active ON pricing_plans (is_active, sort_order);
CREATE INDEX idx_pricing_plans_slug ON pricing_plans (slug);

CREATE TABLE pricing_change_log (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    plan_id UUID NOT NULL REFERENCES pricing_plans(id) ON DELETE CASCADE,
    field_changed TEXT NOT NULL,
    old_value TEXT,
    new_value TEXT,
    changed_by UUID NOT NULL REFERENCES users(id),
    changed_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_pricing_change_log_plan ON pricing_change_log (plan_id, changed_at DESC);

-- Seed default plans
INSERT INTO pricing_plans (id, name, slug, amount_cents, currency, interval, interval_count, features, is_popular, sort_order)
VALUES
    (gen_random_uuid(), 'Monthly', 'monthly', 4900, 'usd', 'month', 1, '["Weekly watchlists & trade alerts", "Full course library access", "Members-only community", "Mobile app access"]'::jsonb, false, 1),
    (gen_random_uuid(), 'Annual', 'annual', 39900, 'usd', 'year', 1, '["Everything in Monthly", "Save $189/year vs monthly", "Priority support", "Exclusive annual member content"]'::jsonb, true, 2);
