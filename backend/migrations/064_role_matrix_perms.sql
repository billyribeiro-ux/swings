-- ADM-09: permission catalogue extension for the role/permission matrix
-- admin surface introduced in this round.
--
-- The `user_role` enum is fixed at migration time (Member/Author/Support/Admin).
-- The catalogue exposed via /api/admin/security/roles therefore lets admins
-- mutate the *mapping* (role_permissions rows) and inspect the
-- canonical permission catalogue, but not invent new role labels — that
-- is a deliberate design constraint, not a TODO.

INSERT INTO permissions (key, description) VALUES
    ('admin.role.read',   'Read the role/permission matrix and the permission catalogue'),
    ('admin.role.manage', 'Mutate the role/permission matrix (grant + revoke + reload)')
ON CONFLICT (key) DO NOTHING;

-- Admin gets both keys; nothing seeded for support — the matrix
-- is a security-sensitive surface and helpdesk staff should never
-- be able to escalate their own permissions.
INSERT INTO role_permissions (role, permission)
SELECT 'admin'::user_role, p.key
  FROM permissions p
 WHERE p.key IN ('admin.role.read', 'admin.role.manage')
ON CONFLICT (role, permission) DO NOTHING;
