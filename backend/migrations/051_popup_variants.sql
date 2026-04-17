-- Migration 051 (POP-02): A/B variant infrastructure.
--
-- popup_variants stores the per-variant copy/style + traffic weight;
-- popup_events and popup_submissions gain a nullable variant_id so the
-- admin analytics view can slice by variant. Assignments themselves are
-- stable-hash computed (backend/src/popups/variants.rs); the cookie is
-- the single source of truth on the client.

CREATE TABLE IF NOT EXISTS popup_variants (
    id             UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    popup_id       UUID NOT NULL REFERENCES popups(id) ON DELETE CASCADE,
    name           TEXT NOT NULL,
    content_json   JSONB NOT NULL DEFAULT '{"elements": []}',
    style_json     JSONB NOT NULL DEFAULT '{}',
    traffic_weight INT  NOT NULL DEFAULT 50 CHECK (traffic_weight BETWEEN 0 AND 100),
    is_winner      BOOLEAN NOT NULL DEFAULT FALSE,
    created_at     TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS popup_variants_popup_idx ON popup_variants (popup_id);

ALTER TABLE popup_events
    ADD COLUMN IF NOT EXISTS variant_id UUID REFERENCES popup_variants(id) ON DELETE SET NULL;
CREATE INDEX IF NOT EXISTS popup_events_variant_idx ON popup_events (variant_id);

ALTER TABLE popup_submissions
    ADD COLUMN IF NOT EXISTS variant_id UUID REFERENCES popup_variants(id) ON DELETE SET NULL;
CREATE INDEX IF NOT EXISTS popup_submissions_variant_idx ON popup_submissions (variant_id);
