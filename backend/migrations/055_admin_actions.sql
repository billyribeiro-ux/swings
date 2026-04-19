-- ADM-01: append-only admin / support audit log.
--
-- Every privileged action that mutates a member, subscription, order, role,
-- coupon, course enrollment, consent record, or impersonation token writes
-- exactly one row to `admin_actions`. The table is intentionally generic so
-- new domains can adopt it without further schema work — the `target_kind`
-- column carries the resource type (`user`, `subscription`, `order`, etc.)
-- and `metadata` carries any structured payload the handler considers
-- relevant (e.g. `{"reason": "...", "duration_days": 30}` for a suspension).
--
-- Append-only contract:
--   * No `UPDATE` / `DELETE` permission is granted to the application role.
--   * Retention is enforced by a future scheduled job, not by handlers.
--
-- Pairs with `services::audit::record_admin_action`, which is the only
-- supported writer in application code.

CREATE TABLE IF NOT EXISTS admin_actions (
    id           UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    actor_id     UUID        NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
    actor_role   user_role   NOT NULL,
    action       TEXT        NOT NULL CHECK (length(action) BETWEEN 1 AND 128),
    target_kind  TEXT        NOT NULL CHECK (length(target_kind) BETWEEN 1 AND 64),
    target_id    TEXT,
    ip_address   INET,
    user_agent   TEXT,
    metadata     JSONB       NOT NULL DEFAULT '{}'::jsonb,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_admin_actions_actor      ON admin_actions (actor_id, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_admin_actions_target     ON admin_actions (target_kind, target_id, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_admin_actions_action     ON admin_actions (action, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_admin_actions_created_at ON admin_actions (created_at DESC);

COMMENT ON TABLE admin_actions IS
    'Append-only audit log for privileged admin / support actions. Written via services::audit::record_admin_action.';
