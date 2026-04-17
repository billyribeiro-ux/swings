-- no-transaction
--
-- FDN-07: Role-based access control infrastructure.
--
-- Extends the existing `user_role` enum with two new values (`author`, `support`)
-- and introduces a permissions catalogue + role→permission mapping that the
-- runtime `authz::Policy` engine loads at startup.
--
-- The §12 authz matrix in AUDIT_PHASE3_PLAN.md is the source of truth; the seed
-- data below implements that matrix verbatim, plus the natural extensions across
-- all core domains (blog, course, coupon, order, subscription, popup, form,
-- consent, notification, admin).
--
-- This migration is intentionally non-transactional: Postgres forbids using a
-- freshly-added enum value in the same transaction as the `ALTER TYPE ... ADD
-- VALUE` statement that introduced it, and the seed data references 'author' /
-- 'support'.
--
-- Handler-level enforcement (policy.require(...) calls at each route) is wired
-- in Round 2b; FDN-07 ships the infrastructure only.

-- ── Enum extension ──────────────────────────────────────────────────────
ALTER TYPE user_role ADD VALUE IF NOT EXISTS 'author';
ALTER TYPE user_role ADD VALUE IF NOT EXISTS 'support';

-- ── Catalogue tables ────────────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS permissions (
    key         TEXT PRIMARY KEY,
    description TEXT NOT NULL DEFAULT ''
);

CREATE TABLE IF NOT EXISTS role_permissions (
    role       user_role NOT NULL,
    permission TEXT NOT NULL REFERENCES permissions(key) ON DELETE CASCADE,
    PRIMARY KEY (role, permission)
);

CREATE INDEX IF NOT EXISTS idx_role_permissions_role ON role_permissions (role);

-- ── Permission catalogue seed ───────────────────────────────────────────
-- Keys are dot-delimited and verb-suffixed: `<domain>.<resource>.<action>`.
-- `ON CONFLICT DO NOTHING` keeps the migration idempotent.

-- User self-service + administration
INSERT INTO permissions (key, description) VALUES
    ('user.self.read',     'Read own user profile'),
    ('user.self.update',   'Update own user profile'),
    ('user.other.read',    'Read another user''s profile'),
    ('user.other.update',  'Update another user''s profile'),
    ('user.other.delete',  'Delete another user account')
ON CONFLICT (key) DO NOTHING;

-- Blog posts, media, categories
INSERT INTO permissions (key, description) VALUES
    ('blog.post.read_any',     'Read unpublished / draft blog posts across all authors'),
    ('blog.post.create',       'Create a new blog post'),
    ('blog.post.update_own',   'Update a blog post the caller authored'),
    ('blog.post.update_any',   'Update any blog post regardless of author'),
    ('blog.post.delete_own',   'Delete a blog post the caller authored'),
    ('blog.post.delete_any',   'Delete any blog post regardless of author'),
    ('blog.post.publish',      'Publish / unpublish a blog post'),
    ('blog.media.upload',      'Upload a media asset'),
    ('blog.media.delete_own',  'Delete a media asset the caller uploaded'),
    ('blog.media.delete_any',  'Delete any media asset regardless of uploader'),
    ('blog.category.manage',   'Create, update, or delete blog categories')
ON CONFLICT (key) DO NOTHING;

-- Courses
INSERT INTO permissions (key, description) VALUES
    ('course.read_enrolled', 'Read courses the caller is enrolled in'),
    ('course.read_any',      'Read any course regardless of enrollment'),
    ('course.manage',        'Create, update, or delete courses and lessons'),
    ('course.enroll.self',   'Enroll self into a course'),
    ('course.enroll.other',  'Enroll or revoke enrollment for another user'),
    ('course.progress.read_self', 'Read own course progress'),
    ('course.progress.read_any',  'Read any user''s course progress')
ON CONFLICT (key) DO NOTHING;

-- Coupons
INSERT INTO permissions (key, description) VALUES
    ('coupon.apply',         'Apply a coupon during checkout'),
    ('coupon.read_any',      'Read coupon configuration (admin / support triage)'),
    ('coupon.manage',        'Create, update, or delete coupons')
ON CONFLICT (key) DO NOTHING;

-- Orders, payments, refunds
INSERT INTO permissions (key, description) VALUES
    ('order.mine.read',       'Read own order history'),
    ('order.any.read',        'Read any user''s order'),
    ('order.any.update',      'Update any order metadata'),
    ('order.refund.create',   'Issue a refund (support is capped)'),
    ('order.invoice.read_self', 'Download own invoice PDF'),
    ('order.invoice.read_any',  'Download any invoice PDF')
ON CONFLICT (key) DO NOTHING;

-- Subscriptions
INSERT INTO permissions (key, description) VALUES
    ('subscription.mine.read',    'Read own subscription state'),
    ('subscription.mine.manage',  'Cancel / update own subscription'),
    ('subscription.any.read',     'Read any subscription'),
    ('subscription.any.manage',   'Administer any subscription (refund, swap plan, etc.)'),
    ('subscription.plan.manage',  'Create, update, or delete subscription plans')
ON CONFLICT (key) DO NOTHING;

-- Popups
INSERT INTO permissions (key, description) VALUES
    ('popup.submit',   'Submit a popup form as an anonymous / authed visitor'),
    ('popup.event',    'Record a popup impression / interaction event'),
    ('popup.manage',   'Create, update, or delete popup configurations'),
    ('popup.read_analytics', 'Read popup conversion analytics')
ON CONFLICT (key) DO NOTHING;

-- Forms
INSERT INTO permissions (key, description) VALUES
    ('form.submit',            'Submit a public form'),
    ('form.manage',            'Create, update, or delete forms'),
    ('form.submission.read_any', 'Read any form submission'),
    ('form.submission.delete_any', 'Delete any form submission (DSAR)')
ON CONFLICT (key) DO NOTHING;

-- Consent (GDPR/CCPA log + DSAR)
INSERT INTO permissions (key, description) VALUES
    ('consent.record',   'Record a consent decision for self or anonymous visitor'),
    ('consent.log.read_self', 'Read own consent audit trail'),
    ('consent.log.read_any',  'Read any user''s consent audit trail'),
    ('dsar.submit',      'Submit a DSAR request'),
    ('dsar.fulfill',     'Fulfill / export / delete data for a DSAR request')
ON CONFLICT (key) DO NOTHING;

-- Notifications
INSERT INTO permissions (key, description) VALUES
    ('notification.mine.read',      'Read own notification inbox'),
    ('notification.mine.mark_read', 'Mark own notifications as read'),
    ('notification.broadcast.create', 'Broadcast a notification to a segment / all users'),
    ('notification.template.manage',  'Create, update, or delete notification templates')
ON CONFLICT (key) DO NOTHING;

-- Admin (cross-cutting admin controls)
INSERT INTO permissions (key, description) VALUES
    ('admin.dashboard.read', 'Access the admin dashboard'),
    ('admin.audit.read',     'Read the admin audit log'),
    ('admin.role.manage',    'Assign or revoke user roles'),
    ('admin.permission.manage', 'Modify the role → permission mapping at runtime'),
    ('admin.outbox.retry',   'Retry failed outbox deliveries'),
    ('admin.outbox.read',    'Read outbox job state')
ON CONFLICT (key) DO NOTHING;

-- ── Role → permission seed ──────────────────────────────────────────────
-- Implements the §12 matrix. We cast string literals to `user_role` for
-- clarity; the implicit cast also works but this documents intent.

-- member: self-only reads + explicit consumer actions.
INSERT INTO role_permissions (role, permission) VALUES
    ('member'::user_role, 'user.self.read'),
    ('member'::user_role, 'user.self.update'),
    ('member'::user_role, 'course.read_enrolled'),
    ('member'::user_role, 'course.enroll.self'),
    ('member'::user_role, 'course.progress.read_self'),
    ('member'::user_role, 'coupon.apply'),
    ('member'::user_role, 'order.mine.read'),
    ('member'::user_role, 'order.invoice.read_self'),
    ('member'::user_role, 'subscription.mine.read'),
    ('member'::user_role, 'subscription.mine.manage'),
    ('member'::user_role, 'popup.submit'),
    ('member'::user_role, 'popup.event'),
    ('member'::user_role, 'form.submit'),
    ('member'::user_role, 'consent.record'),
    ('member'::user_role, 'consent.log.read_self'),
    ('member'::user_role, 'dsar.submit'),
    ('member'::user_role, 'notification.mine.read'),
    ('member'::user_role, 'notification.mine.mark_read')
ON CONFLICT (role, permission) DO NOTHING;

-- author: member baseline + authoring surface for blog.
INSERT INTO role_permissions (role, permission) VALUES
    ('author'::user_role, 'user.self.read'),
    ('author'::user_role, 'user.self.update'),
    ('author'::user_role, 'course.read_enrolled'),
    ('author'::user_role, 'course.enroll.self'),
    ('author'::user_role, 'course.progress.read_self'),
    ('author'::user_role, 'coupon.apply'),
    ('author'::user_role, 'order.mine.read'),
    ('author'::user_role, 'order.invoice.read_self'),
    ('author'::user_role, 'subscription.mine.read'),
    ('author'::user_role, 'subscription.mine.manage'),
    ('author'::user_role, 'popup.submit'),
    ('author'::user_role, 'popup.event'),
    ('author'::user_role, 'form.submit'),
    ('author'::user_role, 'consent.record'),
    ('author'::user_role, 'consent.log.read_self'),
    ('author'::user_role, 'dsar.submit'),
    ('author'::user_role, 'notification.mine.read'),
    ('author'::user_role, 'notification.mine.mark_read'),
    -- author-specific
    ('author'::user_role, 'blog.post.create'),
    ('author'::user_role, 'blog.post.update_own'),
    ('author'::user_role, 'blog.post.delete_own'),
    ('author'::user_role, 'blog.post.publish'),
    ('author'::user_role, 'blog.media.upload'),
    ('author'::user_role, 'blog.media.delete_own')
ON CONFLICT (role, permission) DO NOTHING;

-- support: member baseline + customer-service / moderator reads + limited writes.
INSERT INTO role_permissions (role, permission) VALUES
    ('support'::user_role, 'user.self.read'),
    ('support'::user_role, 'user.self.update'),
    ('support'::user_role, 'course.read_enrolled'),
    ('support'::user_role, 'course.enroll.self'),
    ('support'::user_role, 'course.progress.read_self'),
    ('support'::user_role, 'coupon.apply'),
    ('support'::user_role, 'order.mine.read'),
    ('support'::user_role, 'order.invoice.read_self'),
    ('support'::user_role, 'subscription.mine.read'),
    ('support'::user_role, 'subscription.mine.manage'),
    ('support'::user_role, 'popup.submit'),
    ('support'::user_role, 'popup.event'),
    ('support'::user_role, 'form.submit'),
    ('support'::user_role, 'consent.record'),
    ('support'::user_role, 'consent.log.read_self'),
    ('support'::user_role, 'dsar.submit'),
    ('support'::user_role, 'notification.mine.read'),
    ('support'::user_role, 'notification.mine.mark_read'),
    -- support-specific reads
    ('support'::user_role, 'user.other.read'),
    ('support'::user_role, 'course.read_any'),
    ('support'::user_role, 'course.progress.read_any'),
    ('support'::user_role, 'coupon.read_any'),
    ('support'::user_role, 'order.any.read'),
    ('support'::user_role, 'order.invoice.read_any'),
    ('support'::user_role, 'subscription.any.read'),
    ('support'::user_role, 'consent.log.read_any'),
    ('support'::user_role, 'form.submission.read_any'),
    ('support'::user_role, 'admin.dashboard.read'),
    ('support'::user_role, 'admin.outbox.read'),
    ('support'::user_role, 'admin.outbox.retry'),
    -- support-specific limited writes
    ('support'::user_role, 'order.refund.create'),
    ('support'::user_role, 'dsar.fulfill')
ON CONFLICT (role, permission) DO NOTHING;

-- admin: superset of every permission in the catalogue.
INSERT INTO role_permissions (role, permission)
SELECT 'admin'::user_role, p.key FROM permissions p
ON CONFLICT (role, permission) DO NOTHING;
</content>
</invoke>