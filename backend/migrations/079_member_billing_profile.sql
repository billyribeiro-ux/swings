-- ADM-15: capture-at-checkout billing profile + temporary suspension window.
--
-- The admin Members surface needs to expose every field actually gathered
-- during the Stripe Checkout flow so an operator can read or rewrite
-- them in one place. Today the data is split between the Stripe Customer
-- (street address, phone, country) and our `users` row (email, name); the
-- two drift whenever the admin or the customer mutates one without the
-- other.
--
-- This migration adds the missing columns to `users` and also threads in a
-- `suspended_until` timestamp so an operator can issue a *timeout*
-- (temporary suspension) instead of an open-ended one. The login handler
-- consults `suspended_until` and lazily reactivates the account on the
-- next attempt.
--
-- Forward-only. Every column is nullable so existing rows remain valid;
-- semantically `NULL` means "we never collected this field".

-- ── billing address (mirrors Stripe Address shape) ─────────────────────
ALTER TABLE users ADD COLUMN IF NOT EXISTS billing_line1       TEXT
    CHECK (billing_line1       IS NULL OR length(billing_line1)       <= 256);
ALTER TABLE users ADD COLUMN IF NOT EXISTS billing_line2       TEXT
    CHECK (billing_line2       IS NULL OR length(billing_line2)       <= 256);
ALTER TABLE users ADD COLUMN IF NOT EXISTS billing_city        TEXT
    CHECK (billing_city        IS NULL OR length(billing_city)        <= 128);
ALTER TABLE users ADD COLUMN IF NOT EXISTS billing_state       TEXT
    CHECK (billing_state       IS NULL OR length(billing_state)       <= 128);
ALTER TABLE users ADD COLUMN IF NOT EXISTS billing_postal_code TEXT
    CHECK (billing_postal_code IS NULL OR length(billing_postal_code) <= 32);
-- ISO 3166-1 alpha-2. Stripe normalises to upper-case; we mirror that.
ALTER TABLE users ADD COLUMN IF NOT EXISTS billing_country     TEXT
    CHECK (billing_country     IS NULL OR length(billing_country)     = 2);

-- ── E.164-ish phone number (validation enforced at the handler layer
--     where we have access to the country, not at the DB tier) ─────────
ALTER TABLE users ADD COLUMN IF NOT EXISTS phone               TEXT
    CHECK (phone               IS NULL OR length(phone)               <= 32);

-- ── temporary suspension window ────────────────────────────────────────
-- When `suspended_at IS NOT NULL` and `suspended_until IS NOT NULL`, the
-- login handler treats `now() >= suspended_until` as auto-reactivation
-- (lazy expiry). Open-ended suspensions leave the column NULL.
ALTER TABLE users ADD COLUMN IF NOT EXISTS suspended_until     TIMESTAMPTZ;

CREATE INDEX IF NOT EXISTS idx_users_suspended_until
    ON users (suspended_until) WHERE suspended_until IS NOT NULL;

COMMENT ON COLUMN users.billing_line1       IS 'Captured at Stripe checkout from customer_details.address.line1.';
COMMENT ON COLUMN users.billing_line2       IS 'Captured at Stripe checkout from customer_details.address.line2.';
COMMENT ON COLUMN users.billing_city        IS 'Captured at Stripe checkout from customer_details.address.city.';
COMMENT ON COLUMN users.billing_state       IS 'Captured at Stripe checkout from customer_details.address.state.';
COMMENT ON COLUMN users.billing_postal_code IS 'Captured at Stripe checkout from customer_details.address.postal_code.';
COMMENT ON COLUMN users.billing_country     IS 'ISO 3166-1 alpha-2 country code from customer_details.address.country.';
COMMENT ON COLUMN users.phone               IS 'Captured at Stripe checkout from customer_details.phone.';
COMMENT ON COLUMN users.suspended_until     IS 'Temporary-suspension expiry. NULL when suspension is open-ended.';
