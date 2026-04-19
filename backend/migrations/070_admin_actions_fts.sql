-- ADM-14: full-text + JSON-path search over `admin_actions`.
--
-- The audit log grows linearly with operator activity (~10⁴ rows per
-- month at current scale). For investigations we need:
--
--   * Substring match on actor name / target id / action / metadata.
--   * Targeted lookup by jsonb key (e.g. find every action with
--     `metadata->>'reason' ILIKE '%fraud%'`).
--   * Cursor-paginated time-range scrolling.
--
-- Strategy:
--
--   1. A persisted `tsvector` column populated by trigger from the
--      tuple `(action, target_kind, target_id, jsonb_pretty(metadata))`.
--      `tsvector` is the only Postgres index type that handles
--      multi-word ranking; trigram covers single-substring needs but
--      not phrase queries.
--   2. A GIN index on the `tsvector` (FTS path) and a separate GIN on
--      the raw `metadata` jsonb (JSON-path path).
--   3. A trigram GIN on `target_id` because that field is queried
--      most often as a substring (UUIDs / order numbers).
--
-- The trigger is `BEFORE INSERT` only — `admin_actions` is append-
-- only by contract, so we never need to maintain the tsvector on
-- update.

CREATE EXTENSION IF NOT EXISTS pg_trgm;

ALTER TABLE admin_actions
    ADD COLUMN IF NOT EXISTS search_tsv tsvector;

CREATE OR REPLACE FUNCTION admin_actions_search_tsv_update()
RETURNS trigger AS $$
BEGIN
    NEW.search_tsv :=
        setweight(to_tsvector('simple', coalesce(NEW.action,        '')), 'A') ||
        setweight(to_tsvector('simple', coalesce(NEW.target_kind,   '')), 'B') ||
        setweight(to_tsvector('simple', coalesce(NEW.target_id,     '')), 'B') ||
        setweight(to_tsvector('simple', coalesce(jsonb_pretty(NEW.metadata), '')), 'C');
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS admin_actions_search_tsv_trg ON admin_actions;
CREATE TRIGGER admin_actions_search_tsv_trg
    BEFORE INSERT ON admin_actions
    FOR EACH ROW EXECUTE FUNCTION admin_actions_search_tsv_update();

-- Backfill existing rows so historic data is searchable too.
UPDATE admin_actions
SET search_tsv
    = setweight(to_tsvector('simple', coalesce(action, '')), 'A')
    || setweight(to_tsvector('simple', coalesce(target_kind, '')), 'B')
    || setweight(to_tsvector('simple', coalesce(target_id, '')), 'B')
    || setweight(to_tsvector('simple', coalesce(jsonb_pretty(metadata), '')), 'C')
WHERE search_tsv IS NULL;

CREATE INDEX IF NOT EXISTS admin_actions_search_tsv_idx
    ON admin_actions USING gin (search_tsv);

CREATE INDEX IF NOT EXISTS admin_actions_metadata_gin
    ON admin_actions USING gin (metadata jsonb_path_ops);

CREATE INDEX IF NOT EXISTS admin_actions_target_id_trgm
    ON admin_actions USING gin (target_id gin_trgm_ops);

-- ── permissions ────────────────────────────────────────────────────────

INSERT INTO permissions (key, description) VALUES
    ('admin.audit.read',   'List + drill-down admin audit log entries'),
    ('admin.audit.export', 'Export filtered admin audit log as CSV')
ON CONFLICT (key) DO NOTHING;

INSERT INTO role_permissions (role, permission)
SELECT 'admin'::user_role, k FROM (VALUES
    ('admin.audit.read'),
    ('admin.audit.export')
) AS p(k)
ON CONFLICT DO NOTHING;

INSERT INTO role_permissions (role, permission)
VALUES ('support'::user_role, 'admin.audit.read')
ON CONFLICT DO NOTHING;
