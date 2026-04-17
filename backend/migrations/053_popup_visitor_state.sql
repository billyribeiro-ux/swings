-- Migration 053 (POP-05): per-visitor popup state.
--
-- Server-side frequency capping works off this table rather than client
-- cookies alone because cookies can be cleared by the visitor (or by a
-- privacy extension) to farm the same popup repeatedly. The composite PK
-- doubles as the lookup index — every read is `WHERE anonymous_id=$1 AND
-- popup_id=$2` so we do not need a second index.

CREATE TABLE IF NOT EXISTS popup_visitor_state (
    anonymous_id     UUID NOT NULL,
    popup_id         UUID NOT NULL REFERENCES popups(id) ON DELETE CASCADE,
    first_shown_at   TIMESTAMPTZ,
    last_shown_at    TIMESTAMPTZ,
    times_shown      INT  NOT NULL DEFAULT 0,
    times_dismissed  INT  NOT NULL DEFAULT 0,
    converted        BOOLEAN NOT NULL DEFAULT FALSE,
    updated_at       TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (anonymous_id, popup_id)
);

-- Reverse index so "which popups has this visitor converted on?" queries
-- don't have to scan the primary index.
CREATE INDEX IF NOT EXISTS popup_visitor_state_converted_idx
    ON popup_visitor_state (anonymous_id) WHERE converted = TRUE;
