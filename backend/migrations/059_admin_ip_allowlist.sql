-- ADM-06: optional IP allowlist for the `/api/admin/*` surface.
--
-- The middleware in `src/middleware/admin_ip_allowlist.rs` consults this
-- table on every admin request. Behaviour matrix:
--
--   * Zero active rows  → middleware passes through unchanged. This keeps
--     fresh installs from locking the operator out before they have a
--     chance to add their own IP.
--   * One+ active rows  → request IP MUST fall inside one of the active
--     `cidr` ranges. Failures return `403 Forbidden` and are logged.
--
-- Rows are mutated only by handlers in `src/handlers/admin_ip_allowlist.rs`,
-- which require the `admin.ip_allowlist.manage` permission. Reads use
-- `admin.ip_allowlist.read`. Both permissions are seeded for the `admin`
-- role only — support staff intentionally cannot view or change the list.
--
-- The `cidr` column is the native Postgres CIDR type so the middleware can
-- run a server-side `cidr >>= inet` containment check, which already
-- handles IPv4-mapped IPv6 correctly. `text` casts are used at the SQL
-- boundary to avoid pulling the optional `sqlx/ipnetwork` feature flag.

CREATE TABLE IF NOT EXISTS admin_ip_allowlist (
    id           UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    cidr         CIDR        NOT NULL,
    label        TEXT        NOT NULL CHECK (length(label) BETWEEN 1 AND 200),
    is_active    BOOLEAN     NOT NULL DEFAULT TRUE,
    created_by   UUID        NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (cidr)
);

CREATE INDEX IF NOT EXISTS idx_admin_ip_allowlist_active
    ON admin_ip_allowlist (is_active)
    WHERE is_active;

COMMENT ON TABLE admin_ip_allowlist IS
    'Optional CIDR allowlist gating /api/admin/* routes. Empty = open; non-empty = strict.';

INSERT INTO permissions (key, description) VALUES
    ('admin.ip_allowlist.read',   'View the admin IP allowlist'),
    ('admin.ip_allowlist.manage', 'Create / update / delete admin IP allowlist entries')
ON CONFLICT (key) DO NOTHING;

INSERT INTO role_permissions (role, permission) VALUES
    ('admin'::user_role, 'admin.ip_allowlist.read'),
    ('admin'::user_role, 'admin.ip_allowlist.manage')
ON CONFLICT (role, permission) DO NOTHING;
