-- EC-04: Address book.
--
-- Each user keeps an arbitrary list of saved addresses. The checkout
-- handler uses `kind` to disambiguate billing vs shipping, and the UI
-- treats `is_default = TRUE` as the pre-selected entry on the form. The
-- partial unique index enforces "at most one default per (user, kind)"
-- without rejecting NULL `is_default` rows.

CREATE TABLE IF NOT EXISTS addresses (
    id            UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id       UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    kind          TEXT NOT NULL CHECK (kind IN ('billing','shipping')),
    full_name     TEXT NOT NULL,
    company       TEXT,
    line1         TEXT NOT NULL,
    line2         TEXT,
    city          TEXT NOT NULL,
    state         TEXT,
    postal_code   TEXT NOT NULL,
    country       TEXT NOT NULL,    -- ISO 3166-1 alpha-2
    phone         TEXT,
    is_default    BOOLEAN NOT NULL DEFAULT FALSE,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at    TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS addresses_user_id_idx ON addresses (user_id);
CREATE UNIQUE INDEX IF NOT EXISTS addresses_default_per_kind
    ON addresses (user_id, kind) WHERE is_default;

-- Link orders to billing/shipping addresses captured at checkout time.
-- We snapshot the address fields onto the order itself in `metadata` so
-- later edits to the user's address book don't rewrite history; the FK
-- here is just a convenience pointer for "view the user's saved address
-- this came from".
ALTER TABLE orders
    ADD COLUMN IF NOT EXISTS billing_address_id  UUID REFERENCES addresses(id) ON DELETE SET NULL,
    ADD COLUMN IF NOT EXISTS shipping_address_id UUID REFERENCES addresses(id) ON DELETE SET NULL;
