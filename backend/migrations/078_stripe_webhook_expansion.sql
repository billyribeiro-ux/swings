-- EC-13: Stripe webhook expansion — refunds, dunning, disputes, invoices, audit.
--
-- Today the Stripe webhook only handles 4 events
-- (`customer.subscription.{created,updated,deleted}` +
--  `checkout.session.completed`); refunds, dunning, disputes, trials, and
-- pause/resume silently diverge between Stripe and the local DB mirror.
--
-- This migration introduces the storage layer for the missing events:
--
--   * `subscription_invoices`   — one row per Stripe invoice we acknowledge.
--                                 `paid` invoices are marked terminal; the
--                                 webhook may also flip a row from `paid` to
--                                 `open` when Stripe re-invoices after a
--                                 charge dispute.
--   * `payment_failures`        — per-attempt failure log (dunning events).
--                                 Drives operator dashboards independent
--                                 from `dunning_attempts`, which is still
--                                 the *scheduler* (set in 041_subscriptions_v2.sql).
--   * `payment_refunds`         — Stripe-driven refund mirror. Distinct from
--                                 `order_refunds` (admin-driven, EC-05)
--                                 because charges may be refunded for
--                                 subscription invoices that never produced
--                                 an `orders` row. The two tables
--                                 cross-reference via `order_id` when the
--                                 charge does correspond to an order.
--   * `payment_disputes`        — chargeback ledger. `status` follows
--                                 Stripe's `Dispute.status` taxonomy.
--   * `subscription_trial_events` — `trial_will_end` reminders we have
--                                 emitted to the customer; primary key on
--                                 (subscription_id, trial_end) prevents
--                                 double-sending after Stripe retries.
--   * `stripe_webhook_audit`    — immutable audit log for webhook event
--                                 handling. Distinct from `admin_actions`
--                                 because webhooks have no `users(id)`
--                                 actor; the FK in `admin_actions.actor_id`
--                                 prevents reusing that table for system
--                                 actors.
--
-- Forward-only. Use IF NOT EXISTS / ON CONFLICT DO NOTHING per AGENTS.md §5
-- so a partial-failure replay is a no-op.

-- ── subscription_invoices ──────────────────────────────────────────────
--
-- Mirror of Stripe invoice rows we have acknowledged via webhook. Used to
-- compute reportable revenue without round-tripping to the Stripe API.

CREATE TABLE IF NOT EXISTS subscription_invoices (
    id                         UUID         PRIMARY KEY DEFAULT gen_random_uuid(),
    stripe_invoice_id          TEXT         NOT NULL UNIQUE,
    stripe_subscription_id     TEXT,
    stripe_customer_id         TEXT,
    subscription_id            UUID         REFERENCES subscriptions(id) ON DELETE SET NULL,
    user_id                    UUID         REFERENCES users(id) ON DELETE SET NULL,
    status                     TEXT         NOT NULL CHECK (status IN
                                              ('draft','open','paid','void',
                                               'uncollectible','past_due')),
    amount_due_cents           BIGINT       NOT NULL CHECK (amount_due_cents >= 0),
    amount_paid_cents          BIGINT       NOT NULL DEFAULT 0 CHECK (amount_paid_cents >= 0),
    currency                   TEXT         NOT NULL,
    attempt_count              INT          NOT NULL DEFAULT 0,
    period_start               TIMESTAMPTZ,
    period_end                 TIMESTAMPTZ,
    paid_at                    TIMESTAMPTZ,
    created_at                 TIMESTAMPTZ  NOT NULL DEFAULT NOW(),
    updated_at                 TIMESTAMPTZ  NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS subscription_invoices_sub_idx
    ON subscription_invoices (subscription_id) WHERE subscription_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS subscription_invoices_user_idx
    ON subscription_invoices (user_id) WHERE user_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS subscription_invoices_status_idx
    ON subscription_invoices (status, created_at DESC);

-- ── payment_failures ───────────────────────────────────────────────────
--
-- Per-attempt failure log. Sister table of `dunning_attempts` (which holds
-- the *future* schedule); this table holds the *historical* record of
-- what actually failed and why. Webhooks are the sole writer.

CREATE TABLE IF NOT EXISTS payment_failures (
    id                         UUID         PRIMARY KEY DEFAULT gen_random_uuid(),
    stripe_event_id            TEXT         NOT NULL UNIQUE,
    stripe_invoice_id          TEXT,
    stripe_payment_intent_id   TEXT,
    stripe_customer_id         TEXT,
    subscription_id            UUID         REFERENCES subscriptions(id) ON DELETE SET NULL,
    user_id                    UUID         REFERENCES users(id) ON DELETE SET NULL,
    amount_cents               BIGINT,
    currency                   TEXT,
    failure_code               TEXT,
    failure_message            TEXT,
    attempt_count              INT          NOT NULL DEFAULT 1,
    next_payment_attempt       TIMESTAMPTZ,
    final                      BOOLEAN      NOT NULL DEFAULT FALSE,
    created_at                 TIMESTAMPTZ  NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS payment_failures_sub_idx
    ON payment_failures (subscription_id, created_at DESC) WHERE subscription_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS payment_failures_user_idx
    ON payment_failures (user_id, created_at DESC) WHERE user_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS payment_failures_invoice_idx
    ON payment_failures (stripe_invoice_id) WHERE stripe_invoice_id IS NOT NULL;

-- ── payment_refunds ────────────────────────────────────────────────────
--
-- Stripe-driven refund mirror. Distinct from `order_refunds` (admin-driven,
-- EC-05) because not every refund corresponds to a row in `orders`
-- (subscription invoices skip the orders table). When the charge is for an
-- order, `order_id` cross-links to that row so revenue reports can union
-- both refund sources.

CREATE TABLE IF NOT EXISTS payment_refunds (
    id                         UUID         PRIMARY KEY DEFAULT gen_random_uuid(),
    stripe_refund_id           TEXT         NOT NULL UNIQUE,
    stripe_charge_id           TEXT,
    stripe_payment_intent_id   TEXT,
    stripe_customer_id         TEXT,
    stripe_invoice_id          TEXT,
    order_id                   UUID         REFERENCES orders(id) ON DELETE SET NULL,
    subscription_id            UUID         REFERENCES subscriptions(id) ON DELETE SET NULL,
    user_id                    UUID         REFERENCES users(id) ON DELETE SET NULL,
    amount_cents               BIGINT       NOT NULL CHECK (amount_cents > 0),
    currency                   TEXT         NOT NULL,
    reason                     TEXT,
    status                     TEXT         NOT NULL DEFAULT 'succeeded'
                                              CHECK (status IN ('succeeded','pending','failed','canceled')),
    created_at                 TIMESTAMPTZ  NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS payment_refunds_charge_idx
    ON payment_refunds (stripe_charge_id) WHERE stripe_charge_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS payment_refunds_order_idx
    ON payment_refunds (order_id) WHERE order_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS payment_refunds_sub_idx
    ON payment_refunds (subscription_id) WHERE subscription_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS payment_refunds_user_idx
    ON payment_refunds (user_id, created_at DESC) WHERE user_id IS NOT NULL;

-- ── payment_disputes ───────────────────────────────────────────────────
--
-- Chargeback ledger. Status follows Stripe's `Dispute.status` taxonomy.
-- A dispute is opened by `charge.dispute.created` and updated by the
-- (not-yet-wired) `charge.dispute.{updated,closed,funds_reinstated,
-- funds_withdrawn}` family — the table is forward-compatible.

CREATE TABLE IF NOT EXISTS payment_disputes (
    id                         UUID         PRIMARY KEY DEFAULT gen_random_uuid(),
    stripe_dispute_id          TEXT         NOT NULL UNIQUE,
    stripe_charge_id           TEXT,
    stripe_payment_intent_id   TEXT,
    stripe_customer_id         TEXT,
    order_id                   UUID         REFERENCES orders(id) ON DELETE SET NULL,
    subscription_id            UUID         REFERENCES subscriptions(id) ON DELETE SET NULL,
    user_id                    UUID         REFERENCES users(id) ON DELETE SET NULL,
    amount_cents               BIGINT       NOT NULL,
    currency                   TEXT         NOT NULL,
    reason                     TEXT,
    status                     TEXT         NOT NULL,
    evidence_due_by            TIMESTAMPTZ,
    is_charge_refundable       BOOLEAN      NOT NULL DEFAULT FALSE,
    metadata                   JSONB        NOT NULL DEFAULT '{}'::jsonb,
    created_at                 TIMESTAMPTZ  NOT NULL DEFAULT NOW(),
    updated_at                 TIMESTAMPTZ  NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS payment_disputes_status_idx
    ON payment_disputes (status, created_at DESC);
CREATE INDEX IF NOT EXISTS payment_disputes_order_idx
    ON payment_disputes (order_id) WHERE order_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS payment_disputes_sub_idx
    ON payment_disputes (subscription_id) WHERE subscription_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS payment_disputes_user_idx
    ON payment_disputes (user_id, created_at DESC) WHERE user_id IS NOT NULL;

-- Optional disputed flag on orders so admin views can surface a badge
-- without joining the disputes table for every read. Defaults to FALSE so
-- existing rows are unaffected.
ALTER TABLE orders
    ADD COLUMN IF NOT EXISTS disputed_at TIMESTAMPTZ;

-- ── subscription_trial_events ──────────────────────────────────────────
--
-- Stripe fires `customer.subscription.trial_will_end` 3 days before the
-- trial ends. Stripe may resend the event after a transient delivery
-- failure; the (subscription_id, trial_end) primary key prevents
-- double-sending the reminder email.

CREATE TABLE IF NOT EXISTS subscription_trial_events (
    subscription_id            UUID         NOT NULL REFERENCES subscriptions(id) ON DELETE CASCADE,
    trial_end                  TIMESTAMPTZ  NOT NULL,
    notified_at                TIMESTAMPTZ  NOT NULL DEFAULT NOW(),
    PRIMARY KEY (subscription_id, trial_end)
);

-- ── stripe_webhook_audit ───────────────────────────────────────────────
--
-- Lightweight audit log for webhook event handling. Distinct from
-- `admin_actions` because that table requires a `users(id)` actor (FK
-- with ON DELETE RESTRICT); webhooks have no human actor and we will not
-- weaken the FK on the admin audit table.

CREATE TABLE IF NOT EXISTS stripe_webhook_audit (
    id              UUID         PRIMARY KEY DEFAULT gen_random_uuid(),
    stripe_event_id TEXT         NOT NULL,
    event_type      TEXT         NOT NULL,
    actor           TEXT         NOT NULL DEFAULT 'system:stripe-webhook',
    target_kind     TEXT,
    target_id       TEXT,
    metadata        JSONB        NOT NULL DEFAULT '{}'::jsonb,
    created_at      TIMESTAMPTZ  NOT NULL DEFAULT NOW()
);

-- One row per (event, target) so re-processing the same event id (which
-- the idempotency layer already prevents at the entry point) cannot bloat
-- this table either.
CREATE UNIQUE INDEX IF NOT EXISTS stripe_webhook_audit_event_target_idx
    ON stripe_webhook_audit (stripe_event_id, target_kind, target_id);
CREATE INDEX IF NOT EXISTS stripe_webhook_audit_event_type_idx
    ON stripe_webhook_audit (event_type, created_at DESC);
CREATE INDEX IF NOT EXISTS stripe_webhook_audit_target_idx
    ON stripe_webhook_audit (target_kind, target_id, created_at DESC)
    WHERE target_id IS NOT NULL;

-- ── Notification template seeds ────────────────────────────────────────
--
-- Three new transactional templates wired by the webhook handlers. Bodies
-- mirror the dark-themed layout used by the existing 020_notifications.sql
-- seeds so the admin notification preview surface stays consistent.

INSERT INTO notification_templates
    (key, channel, locale, subject, body_source, body_compiled, variables, version, is_active)
SELECT
    'subscription.payment_failed',
    'email',
    'en',
    'Payment Failed — Action Required',
    '<!DOCTYPE html><html lang="en"><head><meta charset="UTF-8"><title>Payment Failed</title></head><body style="margin:0;padding:0;background-color:#0a0f1c;font-family:-apple-system,BlinkMacSystemFont,Segoe UI,Roboto,sans-serif;color:#e2e8f0;"><div style="width:100%;background-color:#0a0f1c;padding:40px 0;"><div style="max-width:600px;margin:0 auto;background-color:#111827;border-radius:12px;overflow:hidden;border:1px solid #1e293b;"><div style="background:linear-gradient(135deg,#0a0f1c 0%,#1a1f3c 100%);padding:32px 40px;text-align:center;border-bottom:2px solid #0fa4af;"><a href="{{ app_url }}" style="font-size:28px;font-weight:800;color:#0fa4af;text-decoration:none;">Precision Options Signals</a></div><div style="padding:40px;"><h1 style="font-size:24px;font-weight:700;color:#f1f5f9;margin:0 0 16px 0;">Payment failed</h1><p style="font-size:16px;line-height:1.6;color:#94a3b8;margin:0 0 16px 0;">Hi {{ name }},</p><p style="font-size:16px;line-height:1.6;color:#94a3b8;margin:0 0 16px 0;">We were unable to process your most recent payment. To keep your subscription active, please update your card on file before {{ next_attempt }}.</p><div style="text-align:center;margin:32px 0;"><a href="{{ billing_portal_url }}" style="display:inline-block;background-color:#0fa4af;color:#ffffff;text-decoration:none;padding:14px 40px;border-radius:8px;font-size:16px;font-weight:600;">Update Payment Method</a></div></div><div style="background-color:#0a0f1c;padding:24px 40px;text-align:center;border-top:1px solid #1e293b;"><p style="font-size:12px;color:#475569;margin:0;">&copy; {{ year }} Precision Options Signals. All rights reserved.</p></div></div></div></body></html>',
    '',
    '["name","next_attempt","billing_portal_url","app_url","year"]'::jsonb,
    1,
    TRUE
WHERE NOT EXISTS (
    SELECT 1 FROM notification_templates
     WHERE key = 'subscription.payment_failed' AND channel = 'email' AND locale = 'en'
);

INSERT INTO notification_templates
    (key, channel, locale, subject, body_source, body_compiled, variables, version, is_active)
SELECT
    'subscription.payment_recovered',
    'email',
    'en',
    'Payment Successful — Subscription Restored',
    '<!DOCTYPE html><html lang="en"><head><meta charset="UTF-8"><title>Payment Successful</title></head><body style="margin:0;padding:0;background-color:#0a0f1c;font-family:-apple-system,BlinkMacSystemFont,Segoe UI,Roboto,sans-serif;color:#e2e8f0;"><div style="width:100%;background-color:#0a0f1c;padding:40px 0;"><div style="max-width:600px;margin:0 auto;background-color:#111827;border-radius:12px;overflow:hidden;border:1px solid #1e293b;"><div style="background:linear-gradient(135deg,#0a0f1c 0%,#1a1f3c 100%);padding:32px 40px;text-align:center;border-bottom:2px solid #0fa4af;"><a href="{{ app_url }}" style="font-size:28px;font-weight:800;color:#0fa4af;text-decoration:none;">Precision Options Signals</a></div><div style="padding:40px;"><h1 style="font-size:24px;font-weight:700;color:#f1f5f9;margin:0 0 16px 0;">You''re back!</h1><p style="font-size:16px;line-height:1.6;color:#94a3b8;margin:0 0 16px 0;">Hi {{ name }},</p><p style="font-size:16px;line-height:1.6;color:#94a3b8;margin:0 0 16px 0;">Your most recent payment cleared. Your subscription is active again — thanks for sticking with us.</p><div style="text-align:center;margin:32px 0;"><a href="{{ app_url }}/member" style="display:inline-block;background-color:#0fa4af;color:#ffffff;text-decoration:none;padding:14px 40px;border-radius:8px;font-size:16px;font-weight:600;">Go to Dashboard</a></div></div><div style="background-color:#0a0f1c;padding:24px 40px;text-align:center;border-top:1px solid #1e293b;"><p style="font-size:12px;color:#475569;margin:0;">&copy; {{ year }} Precision Options Signals. All rights reserved.</p></div></div></div></body></html>',
    '',
    '["name","app_url","year"]'::jsonb,
    1,
    TRUE
WHERE NOT EXISTS (
    SELECT 1 FROM notification_templates
     WHERE key = 'subscription.payment_recovered' AND channel = 'email' AND locale = 'en'
);

INSERT INTO notification_templates
    (key, channel, locale, subject, body_source, body_compiled, variables, version, is_active)
SELECT
    'subscription.trial_ending',
    'email',
    'en',
    'Your Trial Ends in 3 Days',
    '<!DOCTYPE html><html lang="en"><head><meta charset="UTF-8"><title>Trial Ending Soon</title></head><body style="margin:0;padding:0;background-color:#0a0f1c;font-family:-apple-system,BlinkMacSystemFont,Segoe UI,Roboto,sans-serif;color:#e2e8f0;"><div style="width:100%;background-color:#0a0f1c;padding:40px 0;"><div style="max-width:600px;margin:0 auto;background-color:#111827;border-radius:12px;overflow:hidden;border:1px solid #1e293b;"><div style="background:linear-gradient(135deg,#0a0f1c 0%,#1a1f3c 100%);padding:32px 40px;text-align:center;border-bottom:2px solid #0fa4af;"><a href="{{ app_url }}" style="font-size:28px;font-weight:800;color:#0fa4af;text-decoration:none;">Precision Options Signals</a></div><div style="padding:40px;"><h1 style="font-size:24px;font-weight:700;color:#f1f5f9;margin:0 0 16px 0;">Your trial ends soon</h1><p style="font-size:16px;line-height:1.6;color:#94a3b8;margin:0 0 16px 0;">Hi {{ name }},</p><p style="font-size:16px;line-height:1.6;color:#94a3b8;margin:0 0 16px 0;">Your free trial ends on <strong style="color:#0fa4af;">{{ trial_end_date }}</strong>. You''ll be charged automatically unless you cancel.</p><div style="text-align:center;margin:32px 0;"><a href="{{ billing_portal_url }}" style="display:inline-block;background-color:#0fa4af;color:#ffffff;text-decoration:none;padding:14px 40px;border-radius:8px;font-size:16px;font-weight:600;">Manage Subscription</a></div></div><div style="background-color:#0a0f1c;padding:24px 40px;text-align:center;border-top:1px solid #1e293b;"><p style="font-size:12px;color:#475569;margin:0;">&copy; {{ year }} Precision Options Signals. All rights reserved.</p></div></div></div></body></html>',
    '',
    '["name","trial_end_date","billing_portal_url","app_url","year"]'::jsonb,
    1,
    TRUE
WHERE NOT EXISTS (
    SELECT 1 FROM notification_templates
     WHERE key = 'subscription.trial_ending' AND channel = 'email' AND locale = 'en'
);
