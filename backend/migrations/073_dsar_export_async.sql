-- ADM-17: async DSAR export pipeline.
--
-- Background
-- ----------
-- The synchronous `create_export` handler composes the JSON envelope
-- inline and embeds it as a `data:` URI on `dsar_jobs.artifact_url`.
-- That worked while exports were tiny but creates two scaling problems:
--
--   1. Composing a multi-MiB envelope while holding an HTTP connection
--      will time the request out at the load balancer.
--   2. `data:` URIs balloon `dsar_jobs` table size and fall outside any
--      object-storage retention policy.
--
-- The fix is a queued worker that picks up pending export jobs, runs
-- the composer asynchronously, uploads to object storage (R2 in prod,
-- local disk in dev), and stores the storage key plus a TTL on the row.
-- Operators (and the admin UI) download via a privileged streamer that
-- re-presigns on demand so a stolen URL can never outlive its TTL.
--
-- This migration only widens the schema. Worker logic lives in
-- `src/services/dsar_worker.rs`. The synchronous mode remains the
-- default to keep existing call sites + tests behaviourally stable;
-- callers opt in via `async=true` in the request body.

ALTER TABLE dsar_jobs
    ADD COLUMN IF NOT EXISTS artifact_kind TEXT
        CHECK (artifact_kind IS NULL
               OR artifact_kind IN ('inline', 'r2', 'local')),
    ADD COLUMN IF NOT EXISTS artifact_storage_key TEXT,
    ADD COLUMN IF NOT EXISTS artifact_expires_at TIMESTAMPTZ;

-- Backfill: every existing artefact is the inline `data:` URI variant.
UPDATE dsar_jobs
SET artifact_kind = 'inline'
WHERE artifact_url IS NOT NULL
  AND artifact_kind IS NULL;

-- Allow the existing status enum to carry the intermediate `composing`
-- state so the worker can mark a row "claimed" without conflicting with
-- the existing `'pending' | 'approved' | 'completed' | 'rejected' |
-- 'cancelled' | 'failed'` set. The CHECK constraint must be replaced
-- in one shot.
ALTER TABLE dsar_jobs DROP CONSTRAINT IF EXISTS dsar_jobs_status_check;
ALTER TABLE dsar_jobs ADD CONSTRAINT dsar_jobs_status_check
    CHECK (status IN
        ('pending','approved','composing','completed','rejected','cancelled','failed'));

-- Index supports the worker's claim query
-- (`WHERE kind='export' AND status='pending'`).
CREATE INDEX IF NOT EXISTS dsar_jobs_export_pending_idx
    ON dsar_jobs (created_at)
    WHERE kind = 'export' AND status = 'pending';

-- Index supports the periodic TTL sweep (`WHERE artifact_expires_at <
-- NOW()`).
CREATE INDEX IF NOT EXISTS dsar_jobs_artifact_expiry_idx
    ON dsar_jobs (artifact_expires_at)
    WHERE artifact_expires_at IS NOT NULL;

COMMENT ON COLUMN dsar_jobs.artifact_kind IS
    'How the artefact was persisted: inline data: URI, R2 object, or local file.';
COMMENT ON COLUMN dsar_jobs.artifact_storage_key IS
    'Stable object key (R2) or filename (local) used by the streamer to (re-)serve the artefact.';
COMMENT ON COLUMN dsar_jobs.artifact_expires_at IS
    'Wall-clock expiry of the artefact. After this time the streamer 410s and a TTL sweep deletes the underlying object.';
