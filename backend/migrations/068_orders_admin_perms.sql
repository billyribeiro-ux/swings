-- ADM-12: orders admin surface (manual create, void, partial refund,
-- CSV export).
--
-- Five granular permissions. Admin gets the full set. Support gets
-- `read` (helpdesk triage) + `refund` (capped by the operator UI to
-- partial refunds; the engine itself enforces the no-overrefund
-- invariant). Manual create + void are admin-only because they
-- mutate revenue records that finance reconciles against.

INSERT INTO permissions (key, description) VALUES
    ('admin.order.read',   'List + read orders across users'),
    ('admin.order.create', 'Manually create an order outside the checkout flow'),
    ('admin.order.void',   'Cancel a non-terminal order'),
    ('admin.order.refund', 'Issue a partial or full refund against an order'),
    ('admin.order.export', 'Export the orders table to CSV')
ON CONFLICT (key) DO NOTHING;

INSERT INTO role_permissions (role, permission)
SELECT 'admin'::user_role, p.key
  FROM permissions p
 WHERE p.key IN (
     'admin.order.read',
     'admin.order.create',
     'admin.order.void',
     'admin.order.refund',
     'admin.order.export'
 )
ON CONFLICT (role, permission) DO NOTHING;

INSERT INTO role_permissions (role, permission) VALUES
    ('support'::user_role, 'admin.order.read'),
    ('support'::user_role, 'admin.order.refund')
ON CONFLICT (role, permission) DO NOTHING;
