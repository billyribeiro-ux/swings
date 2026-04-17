-- EC-03: Persistent cart.
--
-- A cart is keyed by either a `user_id` (authed) OR an `anonymous_id`
-- (guest). The `anonymous_id` is a UUID the frontend mints on first
-- visit and keeps in `localStorage` + an `X-Anonymous-Id` header so
-- refresh loops don't lose state. Exactly one cart per identity is
-- enforced by a pair of partial unique indexes — `UNIQUE (user_id)` /
-- `UNIQUE (anonymous_id)` as declared at the table level would reject
-- NULLs inconsistently across PG versions, hence the partial shape.
--
-- The running totals columns (`subtotal_cents`, `discount_cents`,
-- `tax_cents`, `total_cents`) are a materialised view of `cart_items`
-- + applied coupons. They are kept up to date by the service layer on
-- every mutation so the read path is O(1); the values are authoritative
-- only within the same transaction that wrote them — any consumer that
-- needs a fully reconciled total re-computes via `cart::compute_totals`.

CREATE TABLE IF NOT EXISTS carts (
    id                  UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id             UUID REFERENCES users(id) ON DELETE SET NULL,
    anonymous_id        UUID,
    currency            TEXT NOT NULL DEFAULT 'usd',
    subtotal_cents      BIGINT NOT NULL DEFAULT 0,
    discount_cents      BIGINT NOT NULL DEFAULT 0,
    tax_cents           BIGINT NOT NULL DEFAULT 0,
    total_cents         BIGINT NOT NULL DEFAULT 0,
    applied_coupon_ids  UUID[] NOT NULL DEFAULT '{}',
    metadata            JSONB NOT NULL DEFAULT '{}',
    expires_at          TIMESTAMPTZ,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at          TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CHECK (user_id IS NOT NULL OR anonymous_id IS NOT NULL)
);

CREATE UNIQUE INDEX IF NOT EXISTS carts_user_id_key
    ON carts (user_id) WHERE user_id IS NOT NULL;

CREATE UNIQUE INDEX IF NOT EXISTS carts_anonymous_id_key
    ON carts (anonymous_id) WHERE anonymous_id IS NOT NULL;

CREATE INDEX IF NOT EXISTS carts_expires_at_idx
    ON carts (expires_at) WHERE expires_at IS NOT NULL;

CREATE TABLE IF NOT EXISTS cart_items (
    id                  UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    cart_id             UUID NOT NULL REFERENCES carts(id) ON DELETE CASCADE,
    product_id          UUID NOT NULL REFERENCES products(id),
    variant_id          UUID REFERENCES product_variants(id),
    quantity            INT NOT NULL DEFAULT 1 CHECK (quantity > 0),
    unit_price_cents    BIGINT NOT NULL CHECK (unit_price_cents >= 0),
    line_total_cents    BIGINT NOT NULL CHECK (line_total_cents >= 0),
    metadata            JSONB NOT NULL DEFAULT '{}',
    created_at          TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at          TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Collapse duplicate (product, variant) rows into a single line by
-- summing quantities — the service layer relies on this uniqueness to
-- do quantity deltas without LOCK FOR UPDATE scans.
CREATE UNIQUE INDEX IF NOT EXISTS cart_items_unique_line
    ON cart_items (cart_id, product_id, COALESCE(variant_id, '00000000-0000-0000-0000-000000000000'::uuid));

CREATE INDEX IF NOT EXISTS cart_items_cart_id_idx ON cart_items (cart_id);
