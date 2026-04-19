-- ADM-02: user lifecycle columns + auth observability tables.
--
-- Adds the missing fields the audit (Phase 1) flagged as gaps in the
-- members admin domain:
--
--   * `users.suspended_at`     — soft suspension (login blocked, account intact)
--   * `users.suspension_reason`— operator-supplied note (free text, ≤ 512 chars)
--   * `users.banned_at`        — hard ban (login blocked, content hidden)
--   * `users.ban_reason`       — operator-supplied note (free text, ≤ 512 chars)
--   * `users.email_verified_at`— set by self-serve verify or admin override
--
-- Adds:
--   * `failed_login_attempts`  — append-only ledger; populated by /api/auth/login
--                                on every credential failure. Drives the
--                                "Failed login attempts viewer" admin page.
--   * `email_verification_tokens` — single-use tokens for the verify-email flow.
--
-- All new columns are nullable to keep existing rows valid; semantically
-- `NULL` means "not in that state".

-- ── users.* lifecycle columns ───────────────────────────────────────────
ALTER TABLE users ADD COLUMN IF NOT EXISTS suspended_at      TIMESTAMPTZ;
ALTER TABLE users ADD COLUMN IF NOT EXISTS suspension_reason TEXT
    CHECK (suspension_reason IS NULL OR length(suspension_reason) <= 512);
ALTER TABLE users ADD COLUMN IF NOT EXISTS banned_at         TIMESTAMPTZ;
ALTER TABLE users ADD COLUMN IF NOT EXISTS ban_reason        TEXT
    CHECK (ban_reason IS NULL OR length(ban_reason) <= 512);
ALTER TABLE users ADD COLUMN IF NOT EXISTS email_verified_at TIMESTAMPTZ;

CREATE INDEX IF NOT EXISTS idx_users_suspended_at ON users (suspended_at) WHERE suspended_at IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_users_banned_at    ON users (banned_at)    WHERE banned_at    IS NOT NULL;

-- ── failed_login_attempts ──────────────────────────────────────────────
-- One row per credential failure on POST /api/auth/login.
-- `email` is whatever the client supplied — NOT a FK to users(id), since
-- enumeration attacks fire against non-existent emails too.
CREATE TABLE IF NOT EXISTS failed_login_attempts (
    id          UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    email       TEXT        NOT NULL CHECK (length(email) BETWEEN 1 AND 320),
    ip_address  INET,
    user_agent  TEXT,
    reason      TEXT        NOT NULL CHECK (reason IN ('unknown_email','bad_password','suspended','banned','rate_limited')),
    occurred_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_failed_login_email   ON failed_login_attempts (lower(email), occurred_at DESC);
CREATE INDEX IF NOT EXISTS idx_failed_login_ip      ON failed_login_attempts (ip_address, occurred_at DESC);
CREATE INDEX IF NOT EXISTS idx_failed_login_recent  ON failed_login_attempts (occurred_at DESC);

-- ── email_verification_tokens ──────────────────────────────────────────
CREATE TABLE IF NOT EXISTS email_verification_tokens (
    id         UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id    UUID        NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    token_hash TEXT        NOT NULL UNIQUE,
    expires_at TIMESTAMPTZ NOT NULL,
    used_at    TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_email_verify_user ON email_verification_tokens (user_id);
CREATE INDEX IF NOT EXISTS idx_email_verify_hash ON email_verification_tokens (token_hash);

COMMENT ON COLUMN users.suspended_at      IS 'Soft suspension timestamp; login blocked while NOT NULL.';
COMMENT ON COLUMN users.banned_at         IS 'Hard ban timestamp; login blocked + content hidden while NOT NULL.';
COMMENT ON COLUMN users.email_verified_at IS 'Set when the user (or an admin) confirms ownership of the email.';
