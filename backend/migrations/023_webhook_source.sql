-- FDN-09: multi-source webhook idempotency.
--
-- `processed_webhook_events` was initially scoped to Stripe only (migration
-- 017), using `event_id` as the primary key. Resend events carry independent
-- id namespaces, so without this migration a Stripe event and a Resend event
-- that happen to share an id would collide (possible though improbable —
-- Stripe's `evt_...` prefix does not overlap with Resend's UUID format today,
-- but the schema shouldn't depend on convention).
--
-- Add a `source` column (default `'stripe'` so pre-existing rows retain their
-- provenance), replace the single-column PK with a composite `(source,
-- event_id)` PK, and add a supporting index on the time-bounded cleanup
-- column that survives the constraint swap.

ALTER TABLE processed_webhook_events
    ADD COLUMN IF NOT EXISTS source TEXT NOT NULL DEFAULT 'stripe';

-- Backfill pre-existing rows so the default flows uniformly after we drop
-- the DEFAULT. Belt-and-suspenders: rows created in a concurrent transaction
-- before the migration landed will also pick up the default automatically.
UPDATE processed_webhook_events SET source = 'stripe' WHERE source IS NULL;

-- Drop the default now that existing rows are backfilled — every insert must
-- specify the source explicitly so we never silently collapse Stripe + Resend.
ALTER TABLE processed_webhook_events
    ALTER COLUMN source DROP DEFAULT;

-- Swap the PK. Postgres requires dropping the old constraint first; the
-- constraint name for the original PK is the canonical table_pkey.
ALTER TABLE processed_webhook_events
    DROP CONSTRAINT IF EXISTS processed_webhook_events_pkey;

ALTER TABLE processed_webhook_events
    ADD CONSTRAINT processed_webhook_events_pkey PRIMARY KEY (source, event_id);

-- Index for the 30-day sweep in [`db::cleanup_old_stripe_webhook_events`].
-- Already exists from migration 017 (idx_processed_webhook_events_date); the
-- CREATE INDEX IF NOT EXISTS is idempotent so running the migration against
-- an environment that already has it is a no-op.
CREATE INDEX IF NOT EXISTS idx_processed_webhook_events_date
    ON processed_webhook_events(processed_at);
