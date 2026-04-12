-- Migration 013: Full coupon system with usage tracking

CREATE TYPE discount_type AS ENUM ('percentage', 'fixed_amount', 'free_trial');

CREATE TABLE coupons (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    code TEXT NOT NULL UNIQUE,
    description TEXT DEFAULT '',
    discount_type discount_type NOT NULL DEFAULT 'percentage',
    discount_value NUMERIC(10, 2) NOT NULL DEFAULT 0,
    min_purchase_cents INTEGER DEFAULT 0,
    max_discount_cents INTEGER,
    applies_to TEXT NOT NULL DEFAULT 'all' CHECK (applies_to IN ('all', 'monthly', 'annual', 'course', 'specific_plans')),
    applicable_plan_ids UUID[] DEFAULT '{}',
    applicable_course_ids UUID[] DEFAULT '{}',
    usage_limit INTEGER,
    usage_count INTEGER NOT NULL DEFAULT 0,
    per_user_limit INTEGER NOT NULL DEFAULT 1,
    starts_at TIMESTAMPTZ,
    expires_at TIMESTAMPTZ,
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    stackable BOOLEAN NOT NULL DEFAULT FALSE,
    first_purchase_only BOOLEAN NOT NULL DEFAULT FALSE,
    stripe_coupon_id TEXT,
    stripe_promotion_code_id TEXT,
    created_by UUID NOT NULL REFERENCES users(id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_coupons_code ON coupons (code);
CREATE INDEX idx_coupons_active ON coupons (is_active, starts_at, expires_at);

CREATE TABLE coupon_usages (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    coupon_id UUID NOT NULL REFERENCES coupons(id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    subscription_id UUID REFERENCES subscriptions(id) ON DELETE SET NULL,
    discount_applied_cents INTEGER NOT NULL DEFAULT 0,
    used_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_coupon_usages_coupon ON coupon_usages (coupon_id);
CREATE INDEX idx_coupon_usages_user ON coupon_usages (user_id);
CREATE INDEX idx_coupon_usages_coupon_user ON coupon_usages (coupon_id, user_id);
