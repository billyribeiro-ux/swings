-- ADM-16: Audit-log retention configuration.
--
-- Purpose
-- -------
-- The `admin_actions` table is append-only by design (no UPDATE / DELETE
-- grant in handlers) and grows monotonically. Without a sweeper it will
-- eventually outgrow whatever box hosts Postgres. SOX / ISO 27001 / GDPR
-- audit log retention guidance lands around 365 days for normal operator
-- activity; high-stakes events (impersonation, DSAR erase) typically
-- demand 2555 days (7 years). The default below is the conservative
-- generic value — operators tune via the settings UI per environment.
--
-- The actual sweep runs out-of-band in `services::audit_retention`
-- driven by a tokio task spawned from `main.rs`. This migration only
-- guarantees the configuration row exists at boot so the task does not
-- have to special-case "key missing → ?".
--
-- Row-level retention is enforced by `created_at < NOW() - retention`,
-- pruned in batches of `audit.prune_batch_size` rows so the worker does
-- not lock the table for long. The existing
-- `idx_admin_actions_created_at` index from migration `055_admin_actions`
-- already supports the WHERE clause; no new index is needed.

INSERT INTO app_settings (key, value, value_type, is_secret, description, category)
VALUES
    ('audit.retention_days',
     '365'::jsonb,
     'int',
     FALSE,
     'How many days of admin_actions audit history to keep. Rows older than this are pruned by the audit-retention worker. Set to 0 to disable pruning entirely.',
     'audit'),
    ('audit.prune_batch_size',
     '5000'::jsonb,
     'int',
     FALSE,
     'Maximum number of audit rows the retention worker deletes per iteration. Keep small enough that the lock window stays under a second on the production database.',
     'audit')
ON CONFLICT (key) DO NOTHING;

COMMENT ON COLUMN app_settings.value IS
    'Typed value envelope. For audit retention settings: int days / int batch size.';
