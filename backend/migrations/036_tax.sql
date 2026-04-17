-- EC-08: Tax rates + tax-exempt customers.
--
-- Primary tax engine for jurisdictions we handle manually (US state sales tax,
-- EU VAT MOSS countries we've registered for). Stripe Tax remains the path of
-- first resort at checkout (set `automatic_tax: enabled` on the PaymentIntent)
-- but we own a fallback so Stripe outages or unsupported regions don't block
-- the basket.
--
-- Money is basis-points of the tax rate (1234 = 12.34%). A single rate is
-- keyed by `(region, class)` so we can model reduced-rate classes such as
-- books or digital services alongside the standard rate in the same region.
-- Compound rates multiply against the already-taxed subtotal — rare in the
-- USA but common in Quebec / India GST+CGST / pre-2018 Brazil. The cart /
-- checkout layer computes compound tax last.
--
-- Regions are ISO 3166-1 alpha-2 or ISO 3166-2 subdivisions: `US-CA`, `GB`,
-- `DE`, `QC-CA`. We store them as TEXT rather than a FK to a regions table
-- because the ISO tree is large and stable; collisions are impossible and
-- the dimension table buys us nothing.

CREATE TABLE IF NOT EXISTS tax_rates (
    id            UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    region        TEXT NOT NULL,
    rate_bps      INT NOT NULL CHECK (rate_bps >= 0 AND rate_bps <= 100000),
    compound      BOOLEAN NOT NULL DEFAULT FALSE,
    class         TEXT NOT NULL DEFAULT 'standard',
    label         TEXT NOT NULL,
    is_active     BOOLEAN NOT NULL DEFAULT TRUE,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (region, class)
);

CREATE INDEX IF NOT EXISTS tax_rates_region_idx ON tax_rates (region) WHERE is_active;

-- Per-user exemption. `vat_id` holds the customer's VAT / GST / HST number;
-- `reason` is a free-form audit field ("reseller permit CA-12345"). When
-- `expires_at` is in the past the row is ignored — the engine treats the
-- customer as normal. `ON DELETE CASCADE` so GDPR user-deletion sweeps it.
CREATE TABLE IF NOT EXISTS tax_exempt_customers (
    user_id       UUID PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    vat_id        TEXT,
    reason        TEXT,
    expires_at    TIMESTAMPTZ,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
