-- CONSENT-07: append-only integrity anchor table.
--
-- Every consent record is, by itself, a row in `consent_records` (landed by
-- CONSENT-03 in a sibling worktree). The integrity-anchor table is the second
-- leg of the tamper-evidence story: periodically hash-chain a window of
-- consent_records rows and persist the digest here. An auditor can replay the
-- chain from `created_at ASC` bounds, recompute the hash, and assert the
-- published anchor matches — which proves that no row was modified after the
-- anchor was written.
--
-- The anchor is a Merkle-root-style rolling hash (see backend/src/consent/
-- integrity.rs::compute_anchor). We do NOT store the input rows here — they
-- live in `consent_records` and are only referenced via
-- `record_count` + `window_start_at` / `window_end_at` for validation.
--
-- TODO: scheduled hourly anchor writes. Sibling subsystem (CONSENT-08
-- scheduler) will wire a tokio-cron job that calls
-- `consent::integrity::anchor_recent(pool, window_size)` on a fixed cadence.
-- This migration just provisions the table.

CREATE TABLE consent_integrity_anchors (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    anchor_hash     TEXT NOT NULL,
    record_count    INT NOT NULL CHECK (record_count >= 0),
    window_start_at TIMESTAMPTZ,
    window_end_at   TIMESTAMPTZ,
    anchored_at     TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    -- Immutability: block every UPDATE / DELETE at the trigger level.
    CONSTRAINT consent_integrity_anchors_window_order
        CHECK (window_start_at IS NULL OR window_end_at IS NULL OR window_start_at <= window_end_at)
);

CREATE INDEX idx_consent_integrity_anchors_anchored_at
    ON consent_integrity_anchors (anchored_at DESC);

-- Append-only guard. Any UPDATE or DELETE raises — callers who need to fix
-- a row must insert a corrective anchor with a different window rather than
-- mutate the original. This is the operational analogue of a blockchain's
-- "write-once" property.
CREATE OR REPLACE FUNCTION consent_integrity_anchors_block_mutation()
RETURNS TRIGGER AS $$
BEGIN
    RAISE EXCEPTION 'consent_integrity_anchors is append-only; use INSERT'
        USING ERRCODE = '42809';
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER consent_integrity_anchors_no_update
    BEFORE UPDATE ON consent_integrity_anchors
    FOR EACH ROW EXECUTE FUNCTION consent_integrity_anchors_block_mutation();

CREATE TRIGGER consent_integrity_anchors_no_delete
    BEFORE DELETE ON consent_integrity_anchors
    FOR EACH ROW EXECUTE FUNCTION consent_integrity_anchors_block_mutation();
