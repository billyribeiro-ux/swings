-- Forensic Wave-1 PR-2: seed permissions for legacy admin mutators that the
-- per-domain split never re-classified.
--
-- Until now, the routes in `backend/src/handlers/admin.rs` (member billing
-- portal, sub cancel/resume, role update, account delete, watchlists,
-- watchlist alerts) ran on the bare `AdminUser` extractor with no
-- `policy.require()` call. The `admin` role implicitly held them by being
-- the only role allowed past the extractor — but `support` could not
-- reach them at all, and the FDN-07 matrix had no key to pin them to.
--
-- This migration introduces the missing keys so handlers can call
-- `admin.require(&state.policy, "admin.<resource>.<verb>")` at route entry.
-- The `admin` role inherits every key automatically (021_rbac.sql:250-252).
-- `support` is granted the read-only watchlist + member-detail keys and the
-- billing-portal key (a read-shaped action that issues a customer-self
-- portal URL, not a write to the subscription itself).

INSERT INTO permissions (key, description) VALUES
    ('admin.member.billing_portal', 'Issue a Stripe billing portal URL on a member''s behalf'),
    ('admin.member.subscription.manage', 'Cancel or resume a member''s subscription from the admin surface'),
    ('admin.member.role.update',  'Change a member''s role / privilege level'),
    ('admin.watchlist.read',      'Read watchlist configuration (admin / support triage)'),
    ('admin.watchlist.manage',    'Create, update, or delete watchlists'),
    ('admin.watchlist.alert.read',   'Read watchlist alert configuration'),
    ('admin.watchlist.alert.manage', 'Create, update, or delete watchlist alerts')
ON CONFLICT (key) DO NOTHING;

-- admin: superset by virtue of the catch-all in 021_rbac.sql, but spell it
-- out for clarity + so a future role-revocation cannot brick the surface
-- without an explicit `admin.role.revoke` call.
INSERT INTO role_permissions (role, permission)
SELECT 'admin'::user_role, key FROM permissions
WHERE key IN (
    'admin.member.billing_portal',
    'admin.member.subscription.manage',
    'admin.member.role.update',
    'admin.watchlist.read',
    'admin.watchlist.manage',
    'admin.watchlist.alert.read',
    'admin.watchlist.alert.manage'
)
ON CONFLICT (role, permission) DO NOTHING;

-- support: read-only on watchlists/alerts + the billing-portal short-cut
-- so customer-service agents can hand a member a self-service link
-- without escalating to admin. Mutations stay admin-only.
INSERT INTO role_permissions (role, permission) VALUES
    ('support'::user_role, 'admin.member.billing_portal'),
    ('support'::user_role, 'admin.watchlist.read'),
    ('support'::user_role, 'admin.watchlist.alert.read')
ON CONFLICT (role, permission) DO NOTHING;

-- Forensic C-7: author role can author posts but never had access to the
-- admin shell. The PrivilegedUser extractor in `extractors.rs` gates
-- `/api/admin/*` on `admin.dashboard.read`, so authors couldn't reach the
-- handlers their `blog.post.*_own` permissions are designed for. Granting
-- `admin.dashboard.read` opens the door; the per-handler permission check
-- (now resolved against `_own` vs `_any` based on `author_id` ownership)
-- still constrains what they can actually do.
INSERT INTO role_permissions (role, permission) VALUES
    ('author'::user_role, 'admin.dashboard.read')
ON CONFLICT (role, permission) DO NOTHING;
