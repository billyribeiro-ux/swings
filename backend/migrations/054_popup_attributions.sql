-- Migration 054 (POP-06): revenue attribution.
--
-- popup_attributions links an order or subscription back to the popup /
-- variant whose submission preceded it on the same session. Populated by
-- the `order.completed` / `subscription.started` event handler with a
-- window check (default 24h; tunable via ATTRIBUTION_WINDOW_HOURS).

CREATE TABLE IF NOT EXISTS popup_attributions (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    popup_id        UUID NOT NULL REFERENCES popups(id) ON DELETE CASCADE,
    variant_id      UUID REFERENCES popup_variants(id) ON DELETE SET NULL,
    session_id      UUID NOT NULL,
    order_id        UUID,
    subscription_id UUID,
    amount_cents    BIGINT NOT NULL,
    currency        TEXT NOT NULL,
    attributed_at   TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS popup_attributions_popup_idx ON popup_attributions (popup_id);
CREATE INDEX IF NOT EXISTS popup_attributions_session_idx ON popup_attributions (session_id);
CREATE INDEX IF NOT EXISTS popup_attributions_variant_idx ON popup_attributions (variant_id);
