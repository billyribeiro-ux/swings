-- ADM-10: permission catalogue extension for the admin members surface
-- (search + manual create) introduced in this round.
--
-- Granular permissions are introduced rather than reusing the legacy
-- `admin.dashboard.read` umbrella because the membership table is the
-- highest-PII surface in the product (email, lifecycle, role) and we
-- want operators to be able to grant read-only access (e.g. to
-- finance / legal) without unlocking creation or mutation.

INSERT INTO permissions (key, description) VALUES
    ('admin.member.read',   'List + search + read individual member records'),
    ('admin.member.create', 'Manually create a member account (with optional invite email)'),
    ('admin.member.update', 'Edit lifecycle / profile fields on a member record'),
    ('admin.member.delete', 'Hard-delete a member account')
ON CONFLICT (key) DO NOTHING;

-- Admin gets the full set.
INSERT INTO role_permissions (role, permission)
SELECT 'admin'::user_role, p.key
  FROM permissions p
 WHERE p.key IN (
     'admin.member.read',
     'admin.member.create',
     'admin.member.update',
     'admin.member.delete'
 )
ON CONFLICT (role, permission) DO NOTHING;

-- Support gets read-only — they already triage tickets that reference
-- member identity.
INSERT INTO role_permissions (role, permission) VALUES
    ('support'::user_role, 'admin.member.read')
ON CONFLICT (role, permission) DO NOTHING;
