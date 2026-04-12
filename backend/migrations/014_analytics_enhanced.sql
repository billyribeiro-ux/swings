-- Migration 014: Enhanced analytics for revenue, sales, and funnel tracking

CREATE TABLE sales_events (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    event_type TEXT NOT NULL CHECK (event_type IN ('new_subscription', 'renewal', 'upgrade', 'downgrade', 'cancellation', 'refund', 'course_purchase')),
    amount_cents INTEGER NOT NULL DEFAULT 0,
    currency TEXT NOT NULL DEFAULT 'usd',
    plan_id UUID REFERENCES pricing_plans(id) ON DELETE SET NULL,
    coupon_id UUID REFERENCES coupons(id) ON DELETE SET NULL,
    stripe_payment_intent_id TEXT,
    stripe_invoice_id TEXT,
    metadata JSONB NOT NULL DEFAULT '{}',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_sales_events_user ON sales_events (user_id);
CREATE INDEX idx_sales_events_type ON sales_events (event_type, created_at DESC);
CREATE INDEX idx_sales_events_created ON sales_events (created_at DESC);
CREATE INDEX idx_sales_events_plan ON sales_events (plan_id);

CREATE TABLE monthly_revenue_snapshots (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    year INTEGER NOT NULL,
    month INTEGER NOT NULL,
    mrr_cents BIGINT NOT NULL DEFAULT 0,
    arr_cents BIGINT NOT NULL DEFAULT 0,
    total_revenue_cents BIGINT NOT NULL DEFAULT 0,
    new_subscribers INTEGER NOT NULL DEFAULT 0,
    churned_subscribers INTEGER NOT NULL DEFAULT 0,
    net_subscriber_change INTEGER NOT NULL DEFAULT 0,
    avg_revenue_per_user_cents INTEGER NOT NULL DEFAULT 0,
    computed_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(year, month)
);

CREATE INDEX idx_monthly_snapshots_date ON monthly_revenue_snapshots (year DESC, month DESC);
