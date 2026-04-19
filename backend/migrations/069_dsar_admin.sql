-- ADM-13: admin-initiated DSAR (export) + right-to-erasure (tombstone).
--
-- The pre-existing `dsar_requests` table (025_consent_log.sql) is the
-- *subject*-driven workflow: a user submits a request via the public
-- form, validates ownership of their email, and an admin fulfils the
-- request through `consent::admin_fulfill_dsar`.
--
-- ADM-13 layers an *admin*-driven workflow on top:
--
--   * Operators can mint an export at any time (e.g. legal hold,
--     internal audit, escalated support).
--   * Operators can initiate erasure under GDPR Art. 17 with a strict
--     dual-control gate — one admin requests, a *different* admin
--     approves before the tombstone runs.
--
-- Erasure is implemented as a tombstone, NOT a hard delete:
--
--   * `users` row is preserved so foreign keys in `orders`,
--     `subscriptions`, `memberships`, `consent_records`,
--     `admin_actions` etc. stay valid (financial / regulatory
--     retention obligations override the right to be forgotten on a
--     per-row basis — Art. 17 § 3(b)).
--   * Direct PII columns are overwritten with deterministic
--     placeholders (`erased-{uuid}@deleted.local`, NULL name, NULL
--     bio, etc.) so a join from any retained row does not leak the
--     subject's identity.
--   * `users.erased_at` + `users.erasure_job_id` are stamped so
--     downstream queries can short-circuit (e.g. login refuses
--     tombstoned accounts; UI displays "Deleted user").
--
-- See `services::dsar_admin::tombstone_user` for the canonical column
-- list — adding a new PII column to `users` MUST be paired with an
-- update to that fn (caught by `tests/admin_dsar.rs::tombstone_clears_all_pii`).

-- ── users tombstone marker ─────────────────────────────────────────────

ALTER TABLE users ADD COLUMN IF NOT EXISTS erased_at        TIMESTAMPTZ;
ALTER TABLE users ADD COLUMN IF NOT EXISTS erasure_job_id   UUID;

CREATE INDEX IF NOT EXISTS idx_users_erased_at
    ON users (erased_at)
    WHERE erased_at IS NOT NULL;

-- ── dsar_jobs ──────────────────────────────────────────────────────────

CREATE TABLE IF NOT EXISTS dsar_jobs (
    id                 UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    target_user_id     UUID NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
    kind               TEXT NOT NULL
                         CHECK (kind IN ('export', 'erase')),
    status             TEXT NOT NULL DEFAULT 'pending'
                         CHECK (status IN
                           ('pending','approved','completed','rejected','cancelled','failed')),
    -- Single-control for `export`; first leg of dual-control for `erase`.
    requested_by       UUID NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
    request_reason     TEXT NOT NULL CHECK (length(request_reason) BETWEEN 1 AND 1000),
    -- Required for `erase`. Must differ from requested_by (enforced in
    -- the service layer, not the DB, so the constraint message is
    -- friendlier than `chk_…_ne`).
    approved_by        UUID REFERENCES users(id) ON DELETE RESTRICT,
    approval_reason    TEXT CHECK (approval_reason IS NULL OR length(approval_reason) <= 1000),
    approved_at        TIMESTAMPTZ,
    -- Where the export artefact lives. For now we inline a `data:` URI
    -- (parity with `dsar_requests.fulfillment_url`); R2 uploads land
    -- under a later subsystem.
    artifact_url       TEXT,
    -- Erasure tombstone metadata: the placeholder email we wrote and
    -- the column count we cleared, so audits can verify the operation
    -- did what it said.
    erasure_summary    JSONB,
    completed_at       TIMESTAMPTZ,
    failure_reason     TEXT,
    created_at         TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at         TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS dsar_jobs_target_idx
    ON dsar_jobs (target_user_id, created_at DESC);
CREATE INDEX IF NOT EXISTS dsar_jobs_status_idx
    ON dsar_jobs (status, created_at DESC);
CREATE INDEX IF NOT EXISTS dsar_jobs_kind_idx
    ON dsar_jobs (kind, status);

-- Wire a single pending erasure per subject — operators cannot stack
-- two parallel "please delete this user" requests (that would race
-- the dual-control approval).
CREATE UNIQUE INDEX IF NOT EXISTS dsar_jobs_one_pending_erase
    ON dsar_jobs (target_user_id)
    WHERE kind = 'erase' AND status IN ('pending', 'approved');

-- Back-link the marker on `users` so we can join job → user efficiently.
-- ALTER TABLE … ADD CONSTRAINT … FOREIGN KEY would create a cycle, so
-- we keep `users.erasure_job_id` as a soft pointer the service layer
-- maintains.

-- ── permissions ────────────────────────────────────────────────────────

INSERT INTO permissions (key, description) VALUES
    ('admin.dsar.export',         'Mint an admin-initiated DSAR data export for any user'),
    ('admin.dsar.erase.request',  'Propose a right-to-erasure tombstone (first half of dual control)'),
    ('admin.dsar.erase.approve',  'Approve and execute a pending right-to-erasure tombstone'),
    ('admin.dsar.read',           'List and read admin DSAR jobs')
ON CONFLICT (key) DO NOTHING;

INSERT INTO role_permissions (role, permission_key)
SELECT 'admin'::user_role, k
FROM (VALUES
    ('admin.dsar.export'),
    ('admin.dsar.erase.request'),
    ('admin.dsar.erase.approve'),
    ('admin.dsar.read')
) AS p(k)
ON CONFLICT DO NOTHING;

-- Support agents can read jobs (so they can answer "did we delete
-- this user?" without escalating) but cannot mint or approve.
INSERT INTO role_permissions (role, permission_key)
VALUES ('support'::user_role, 'admin.dsar.read')
ON CONFLICT DO NOTHING;
