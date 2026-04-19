-- ADM-04: extend the FDN-07 permission catalogue with the keys the new
-- admin lifecycle / security handlers enforce.
--
-- Pairs with:
--   * `services::audit::record_admin_action` (writes to `admin_actions`)
--   * `handlers::admin_security::*`          (sessions, audit-log, failed
--                                              logins viewers)
--   * `handlers::admin::*`                   (suspend / ban / reactivate /
--                                              force-reset / verify-email)
--
-- Each permission is granted to the `admin` role unconditionally and to
-- `support` only when the corresponding action is part of the helpdesk
-- runbook. Hard bans, ban reversals, and impersonation stay admin-only.

INSERT INTO permissions (key, description) VALUES
    ('user.suspend',          'Suspend a user account (login blocked, account intact)'),
    ('user.reactivate',       'Reverse a suspension or ban'),
    ('user.ban',              'Hard-ban a user account (login blocked + content hidden)'),
    ('user.force_password_reset', 'Send a password reset email on behalf of a user'),
    ('user.email.verify',     'Mark a user''s email as verified without OTP confirmation'),
    ('user.session.read',     'List active sessions / refresh-token families for any user'),
    ('user.session.revoke',   'Revoke an active session / refresh-token for any user'),
    ('user.impersonate',      'Mint a signed, time-boxed impersonation token for support work'),
    ('admin.security.read',   'Read failed-login attempts and security telemetry'),
    ('form.submission.export','Export form submissions as CSV')
ON CONFLICT (key) DO NOTHING;

-- Support: helpdesk-grade subset.
INSERT INTO role_permissions (role, permission) VALUES
    ('support'::user_role, 'user.suspend'),
    ('support'::user_role, 'user.reactivate'),
    ('support'::user_role, 'user.force_password_reset'),
    ('support'::user_role, 'user.email.verify'),
    ('support'::user_role, 'user.session.read'),
    ('support'::user_role, 'user.session.revoke'),
    ('support'::user_role, 'admin.security.read'),
    ('support'::user_role, 'form.submission.export')
ON CONFLICT (role, permission) DO NOTHING;

-- Admin gets the superset (mirrors the seed pattern from 021_rbac.sql).
INSERT INTO role_permissions (role, permission)
SELECT 'admin'::user_role, p.key
  FROM permissions p
 WHERE p.key IN (
        'user.suspend', 'user.reactivate', 'user.ban',
        'user.force_password_reset', 'user.email.verify',
        'user.session.read', 'user.session.revoke',
        'user.impersonate', 'admin.security.read',
        'form.submission.export'
    )
ON CONFLICT (role, permission) DO NOTHING;
