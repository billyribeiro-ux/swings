-- ADM-08: permission catalogue extension for the typed-settings surface.
--
-- Three keys, all admin-only by default. Support staff can be granted
-- `admin.settings.read` later via the role matrix UI but never gets
-- mutate or secret-read by default — the value catalogue includes
-- integration credentials and feature flags that lifecycle-staff
-- should not be able to flip silently.

INSERT INTO permissions (key, description) VALUES
    ('admin.settings.read',         'Read non-secret application settings'),
    ('admin.settings.read_secret',  'Read decrypted secret settings'),
    ('admin.settings.write',        'Mutate any application setting (incl. maintenance mode + secrets)')
ON CONFLICT (key) DO NOTHING;

INSERT INTO role_permissions (role, permission)
SELECT 'admin'::user_role, p.key
  FROM permissions p
 WHERE p.key IN (
        'admin.settings.read',
        'admin.settings.read_secret',
        'admin.settings.write'
    )
ON CONFLICT (role, permission) DO NOTHING;
