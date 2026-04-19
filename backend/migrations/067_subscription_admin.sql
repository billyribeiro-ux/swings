-- ADM-11: manual subscription operations admin surface.
--
-- Three operator workflows are unlocked:
--
--   * **comp / gift** — mint a `memberships` row directly, bypassing
--     Stripe entirely (the membership engine is the supported
--     non-billed access mechanism in this codebase).
--   * **extend** — push out a Stripe-backed subscription's
--     `current_period_end` by N days. This drifts the local mirror
--     ahead of Stripe; the audit row makes the drift discoverable
--     and the operator UI must warn the actor before they
--     confirm.
--   * **billing-cycle override** — override `billing_cycle_anchor`
--     in the local mirror so renewal alignment can be corrected
--     ahead of the next dunning-engine pass.
--
-- We also extend the `subscription_changes.kind` CHECK constraint
-- with three new manual-operation tags so the audit timeline can
-- reuse the existing change-log table.
--
-- All three workflows live behind dedicated permissions
-- (`admin.subscription.{read,comp,extend,cycle}`) so operators can
-- be granted read-only context (e.g. finance) without unlocking
-- the mutation surface.

-- ── Permissions catalogue + admin grants ───────────────────────────────
INSERT INTO permissions (key, description) VALUES
    ('admin.subscription.read',   'Read subscriptions / memberships across users'),
    ('admin.subscription.comp',   'Mint a comp / gift membership outside Stripe'),
    ('admin.subscription.extend', 'Extend a subscription''s current_period_end'),
    ('admin.subscription.cycle',  'Override a subscription''s billing-cycle anchor')
ON CONFLICT (key) DO NOTHING;

INSERT INTO role_permissions (role, permission)
SELECT 'admin'::user_role, p.key
  FROM permissions p
 WHERE p.key IN (
     'admin.subscription.read',
     'admin.subscription.comp',
     'admin.subscription.extend',
     'admin.subscription.cycle'
 )
ON CONFLICT (role, permission) DO NOTHING;

-- Support gets read-only context (helpdesk).
INSERT INTO role_permissions (role, permission) VALUES
    ('support'::user_role, 'admin.subscription.read')
ON CONFLICT (role, permission) DO NOTHING;

-- ── Extend subscription_changes.kind with manual ops ───────────────────
ALTER TABLE subscription_changes
    DROP CONSTRAINT IF EXISTS subscription_changes_kind_check;

ALTER TABLE subscription_changes
    ADD CONSTRAINT subscription_changes_kind_check
    CHECK (kind IN (
        'upgrade','downgrade','pause','resume',
        'renew_early','switch_plan','cancel',
        'manual_comp','manual_extend','cycle_override'
    ));
