-- Phase 1.3 / Phase 3 RBAC closure: add the permission keys the ten legacy
-- admin handlers need so each route can call `policy.require(...)` instead
-- of relying solely on the strict `AdminUser` extractor (which the audit
-- in `docs/REMAINING-WORK.md` flagged as missing defense-in-depth).
--
-- Two domains are net-new because no permission row covered them yet:
--
--   * `product.*` — the EC-01 products handler currently has zero policy
--     coverage. Four granular keys (read / manage / variant.manage /
--     asset.manage / bundle.manage) would over-engineer the matrix; one
--     `product.manage` key matches the shape of the sibling `coupon.manage`
--     and `popup.manage` keys.
--   * `consent.config.manage` — the CONSENT-07 admin CRUD (banners,
--     categories, services, policies) has no perm key today. The existing
--     `consent.log.read_any` covers reads (already granted to `support`),
--     so we only need a writer key and we keep it admin-only.
--
-- Default-deny per AGENTS.md §3 hard rule #5: neither new key is granted
-- to `support`. Operators who need consent-config or product-catalogue
-- access in the helpdesk runbook must opt in via the role-matrix admin
-- (ADM-09) once the operational policy is decided.
--
-- All existing perms referenced by the policy.require() additions in the
-- ten legacy handlers (blog.*, course.manage, coupon.manage, popup.manage,
-- subscription.plan.manage, form.manage, form.submission.*,
-- notification.template.manage, notification.broadcast.create,
-- admin.outbox.read, admin.outbox.retry, consent.log.read_any) are seeded
-- by 021_rbac.sql and stay unchanged here.

INSERT INTO permissions (key, description) VALUES
    ('product.manage',        'Create, update, or delete products, variants, assets, and bundle items'),
    ('product.read_any',      'Read all products including unpublished / draft rows'),
    ('consent.config.manage', 'Mutate consent banners, categories, services, and append a new policy version')
ON CONFLICT (key) DO NOTHING;

-- Admin gets the superset of every key this migration introduces.
INSERT INTO role_permissions (role, permission)
SELECT 'admin'::user_role, p.key
  FROM permissions p
 WHERE p.key IN (
        'product.manage',
        'product.read_any',
        'consent.config.manage'
    )
ON CONFLICT (role, permission) DO NOTHING;

-- Support gets the read key only — they triage merch tickets but cannot
-- mutate the catalogue or the consent banner copy.
INSERT INTO role_permissions (role, permission) VALUES
    ('support'::user_role, 'product.read_any')
ON CONFLICT (role, permission) DO NOTHING;
