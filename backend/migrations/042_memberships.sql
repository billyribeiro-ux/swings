-- EC-10: Memberships.
--
-- A membership grants a user access to a set of resources (categories,
-- products, arbitrary URLs, courses) for a window of time. Plans
-- describe the offer; memberships are the per-user grants.
--
-- `grants_access_to` is a JSONB envelope with the following shape:
--   { "categories": ["uuid","uuid"], "products": ["uuid"], "urls": ["/blog/*"],
--     "courses": ["uuid"] }
-- The access engine reads this once on plan publish + caches per-user.
--
-- `drip_rules` are an array of `{ "after_days": 7, "resource": "course:abc" }`
-- entries — the worker materialises drips into per-user `memberships`
-- rows by extending `grants_access_to` over time.

CREATE TABLE IF NOT EXISTS membership_plans (
    id                    UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    slug                  TEXT NOT NULL UNIQUE,
    name                  TEXT NOT NULL,
    description           TEXT NOT NULL DEFAULT '',
    grants_access_to      JSONB NOT NULL DEFAULT '{}',
    drip_rules            JSONB NOT NULL DEFAULT '[]',
    default_duration_days INT,
    price_cents           BIGINT,
    is_active             BOOLEAN NOT NULL DEFAULT TRUE,
    created_at            TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at            TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS membership_plans_active_idx
    ON membership_plans (slug) WHERE is_active;

CREATE TABLE IF NOT EXISTS memberships (
    id            UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id       UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    plan_id       UUID NOT NULL REFERENCES membership_plans(id),
    granted_by    TEXT NOT NULL,                                  -- 'product:{id}','manual','promotion:{coupon}','order:{id}'
    status        TEXT NOT NULL DEFAULT 'active'
                  CHECK (status IN ('active','paused','expired','cancelled')),
    starts_at     TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    ends_at       TIMESTAMPTZ,
    metadata      JSONB NOT NULL DEFAULT '{}',
    created_at    TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at    TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS memberships_user_idx ON memberships (user_id);
CREATE INDEX IF NOT EXISTS memberships_plan_idx ON memberships (plan_id);
CREATE INDEX IF NOT EXISTS memberships_active_idx
    ON memberships (user_id) WHERE status = 'active';

-- Member-only discounts surfaced by the cart's coupon engine. `scope`
-- is one of: 'all' | 'product:{uuid}' | 'category:{uuid}'.
CREATE TABLE IF NOT EXISTS member_discounts (
    plan_id          UUID NOT NULL REFERENCES membership_plans(id) ON DELETE CASCADE,
    scope            TEXT NOT NULL,
    discount_type    TEXT NOT NULL CHECK (discount_type IN ('percentage','fixed_cents')),
    discount_value   BIGINT NOT NULL CHECK (discount_value > 0),
    PRIMARY KEY (plan_id, scope)
);
