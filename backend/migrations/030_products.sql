-- EC-01: Digital-goods product model.
--
-- Scope trim (§0 D1): simple + subscription + downloadable + bundle only.
-- No shipping, no physical inventory, no B2B sub-users, no multi-location stock.
-- US Stripe Tax + EU VAT MOSS only.
--
-- Money is stored as integer minor units (`BIGINT` cents); the Rust service
-- layer lifts `BIGINT` into the `Money` newtype (see `common::money`). All
-- timestamps are `timestamptz`.

CREATE TABLE products (
    id            UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    slug          TEXT NOT NULL UNIQUE,
    name          TEXT NOT NULL,
    description   TEXT,
    product_type  TEXT NOT NULL CHECK (product_type IN
                    ('simple','subscription','downloadable','bundle')),
    status        TEXT NOT NULL DEFAULT 'draft'
                    CHECK (status IN ('draft','published','archived')),
    -- Money is stored as integer cents; use the Money newtype in the repo layer.
    price_cents           BIGINT,
    compare_at_cents      BIGINT,
    currency              TEXT NOT NULL DEFAULT 'USD',
    tax_class             TEXT NOT NULL DEFAULT 'standard',
    stripe_product_id     TEXT,
    stripe_price_id       TEXT,
    gallery_media_ids     UUID[] NOT NULL DEFAULT '{}',
    featured_media_id     UUID,
    seo_title             TEXT,
    seo_description       TEXT,
    metadata              JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_by            UUID REFERENCES users(id) ON DELETE SET NULL,
    created_at            TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at            TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_products_status ON products (status, updated_at DESC);
CREATE INDEX idx_products_type ON products (product_type) WHERE status = 'published';

CREATE TABLE product_variants (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    product_id      UUID NOT NULL REFERENCES products(id) ON DELETE CASCADE,
    sku             TEXT UNIQUE,
    name            TEXT,
    price_cents     BIGINT,         -- override product price if set
    currency        TEXT,
    attributes      JSONB NOT NULL DEFAULT '{}'::jsonb,  -- { color: 'blue', size: 'M' } etc.
    stripe_price_id TEXT,
    position        INT NOT NULL DEFAULT 0,
    is_active       BOOLEAN NOT NULL DEFAULT TRUE,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_variants_product ON product_variants (product_id) WHERE is_active = TRUE;

CREATE TABLE product_attributes (
    -- Global attribute catalogue (optional, FluentForms-style shared attr pool).
    key         TEXT PRIMARY KEY,
    label       TEXT NOT NULL,
    input_type  TEXT NOT NULL CHECK (input_type IN ('select','multi_select','text')),
    values      JSONB NOT NULL DEFAULT '[]'::jsonb
);

CREATE TABLE downloadable_assets (
    id                  UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    product_id          UUID NOT NULL REFERENCES products(id) ON DELETE CASCADE,
    variant_id          UUID REFERENCES product_variants(id) ON DELETE CASCADE,
    storage_key         TEXT NOT NULL,         -- R2 object key (EC-07 wires signed URL issuance)
    filename            TEXT NOT NULL,
    mime_type           TEXT NOT NULL,
    size_bytes          BIGINT NOT NULL,
    sha256              TEXT NOT NULL,
    access_policy       TEXT NOT NULL DEFAULT 'purchase_required'
                          CHECK (access_policy IN ('purchase_required','member_tier','public')),
    required_tier       TEXT,           -- when access_policy='member_tier', e.g. 'pro'
    download_limit      INT,            -- per-purchase, null = unlimited
    expires_after_hours INT,            -- signed URL TTL, null = site default (24h)
    created_at          TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_assets_product ON downloadable_assets (product_id);

CREATE TABLE bundle_items (
    id                UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    bundle_product_id UUID NOT NULL REFERENCES products(id) ON DELETE CASCADE,
    child_product_id  UUID NOT NULL REFERENCES products(id),
    child_variant_id  UUID REFERENCES product_variants(id),
    quantity          INT NOT NULL DEFAULT 1 CHECK (quantity > 0),
    position          INT NOT NULL DEFAULT 0
);

-- The plan-draft PK used COALESCE to flatten NULL variants to a zero-UUID
-- sentinel so the triple stayed unique. Postgres rejects expressions in
-- PRIMARY KEY columns, so we enforce the same constraint via a pair of
-- partial UNIQUE indexes — one for the NULL-variant case and one for the
-- concrete-variant case. This is the idiomatic Postgres replacement.
CREATE UNIQUE INDEX idx_bundle_items_unique_with_variant
    ON bundle_items (bundle_product_id, child_product_id, child_variant_id)
    WHERE child_variant_id IS NOT NULL;
CREATE UNIQUE INDEX idx_bundle_items_unique_no_variant
    ON bundle_items (bundle_product_id, child_product_id)
    WHERE child_variant_id IS NULL;
CREATE INDEX idx_bundle_items_bundle ON bundle_items (bundle_product_id);
