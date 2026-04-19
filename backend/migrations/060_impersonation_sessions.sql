-- ADM-07: server-side state for admin impersonation tokens.
--
-- Pairs with:
--   * `auth/impersonation.rs`                 (mint / lookup / revoke repo)
--   * `extractors::AuthUser`                  (validates `imp_session` claim
--                                              against this table on every
--                                              request — revoking a row
--                                              immediately invalidates the
--                                              JWT, side-stepping the
--                                              symmetric-secret stateless
--                                              limitation)
--   * `handlers::admin_impersonation::*`      (CRUD endpoints under
--                                              /api/admin/security/impersonation)
--   * `middleware::impersonation_banner`      (response header injection so
--                                              the SPA renders a banner)
--
-- Every row is a single end-to-end impersonation episode:
--   * `actor_user_id`  — the real admin who minted the token
--   * `target_user_id` — the user whose context the admin assumed
--   * `reason`         — required free-text justification (audit hygiene)
--   * `expires_at`     — server-side TTL; defended in BOTH the JWT `exp`
--                        AND the row check, so even if the secret leaks
--                        and an attacker forges a long `exp`, the row
--                        check refuses past-`expires_at`.
--   * `revoked_at` / `revoked_by` / `revoke_reason` — explicit kill switch.
--
-- Read-side queries always filter on `revoked_at IS NULL AND expires_at > NOW()`
-- so a partial index on those covers the hot path used by AuthUser.

CREATE TABLE IF NOT EXISTS impersonation_sessions (
    id              UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    actor_user_id   UUID        NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
    actor_role      user_role   NOT NULL,
    target_user_id  UUID        NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
    reason          TEXT        NOT NULL CHECK (length(reason) BETWEEN 1 AND 500),
    issued_at       TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expires_at      TIMESTAMPTZ NOT NULL,
    revoked_at      TIMESTAMPTZ,
    revoked_by      UUID        REFERENCES users(id) ON DELETE SET NULL,
    revoke_reason   TEXT        CHECK (revoke_reason IS NULL OR length(revoke_reason) BETWEEN 1 AND 500),
    ip_address      INET,
    user_agent      TEXT,
    CHECK (expires_at > issued_at),
    CHECK (
        (revoked_at IS NULL AND revoked_by IS NULL AND revoke_reason IS NULL)
        OR
        (revoked_at IS NOT NULL AND revoked_by IS NOT NULL)
    )
);

-- Hot path: per-request lookup by id, filtered to active rows.
CREATE INDEX IF NOT EXISTS idx_impersonation_sessions_active
    ON impersonation_sessions (id)
    WHERE revoked_at IS NULL;

-- Admin UI: list active sessions, newest first.
CREATE INDEX IF NOT EXISTS idx_impersonation_sessions_actor
    ON impersonation_sessions (actor_user_id, issued_at DESC);

CREATE INDEX IF NOT EXISTS idx_impersonation_sessions_target
    ON impersonation_sessions (target_user_id, issued_at DESC);
