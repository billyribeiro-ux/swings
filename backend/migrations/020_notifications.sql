-- FDN-05: Notifications core.
--
-- Five tables:
--   * `notification_templates`    — versioned template registry keyed by
--                                   (key, channel, locale).
--   * `notification_preferences`  — per-user / per-(category, channel) opt-in
--                                   plus quiet-hours window.
--   * `notification_deliveries`   — immutable (append-only conceptually) record
--                                   of every send attempt and its eventual
--                                   provider-reported status.
--   * `notification_suppression`  — provider-bounce / user-unsubscribe-all deny
--                                   list keyed by e-mail.
--   * `unsubscribe_tokens`        — one-shot HMAC-SHA256 tokens consumed by the
--                                   public `/u/unsubscribe` route.
--
-- Seed rows at the bottom migrate the four currently-inline templates from
-- `backend/src/email.rs` (welcome, password_reset, subscription_confirmation,
-- subscription_cancelled) into the DB. Subsequent mutations go through the
-- admin API (`/api/admin/notifications/templates`).

-- ── Templates ───────────────────────────────────────────────────────────

CREATE TABLE notification_templates (
    id            UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    key           TEXT NOT NULL,
    channel       TEXT NOT NULL CHECK (channel IN
                    ('email','sms','push','in_app','slack','discord','webhook')),
    locale        TEXT NOT NULL DEFAULT 'en',
    subject       TEXT,
    body_source   TEXT NOT NULL,
    body_compiled TEXT NOT NULL,
    variables     JSONB NOT NULL DEFAULT '[]'::jsonb,
    version       INT  NOT NULL DEFAULT 1,
    is_active     BOOLEAN NOT NULL DEFAULT TRUE,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at    TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (key, channel, locale, version)
);

CREATE INDEX idx_notification_templates_active
    ON notification_templates (key, channel, locale)
    WHERE is_active = TRUE;

-- ── Preferences ─────────────────────────────────────────────────────────

CREATE TABLE notification_preferences (
    user_id           UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    category          TEXT NOT NULL,
    channel           TEXT NOT NULL,
    enabled           BOOLEAN NOT NULL DEFAULT TRUE,
    quiet_hours_start TIME,
    quiet_hours_end   TIME,
    timezone          TEXT NOT NULL DEFAULT 'UTC',
    updated_at        TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (user_id, category, channel)
);

-- ── Deliveries ──────────────────────────────────────────────────────────

CREATE TABLE notification_deliveries (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id         UUID REFERENCES users(id) ON DELETE SET NULL,
    anonymous_email TEXT,
    template_key    TEXT NOT NULL,
    channel         TEXT NOT NULL,
    provider_id     TEXT,
    status          TEXT NOT NULL DEFAULT 'queued'
                    CHECK (status IN ('queued','sent','delivered','bounced',
                                      'complained','opened','clicked','failed',
                                      'suppressed')),
    subject         TEXT,
    rendered_body   TEXT,
    metadata        JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_deliveries_user
    ON notification_deliveries (user_id, created_at DESC);
CREATE INDEX idx_deliveries_status
    ON notification_deliveries (status, created_at DESC);

-- ── Suppression ─────────────────────────────────────────────────────────

CREATE TABLE notification_suppression (
    email         TEXT PRIMARY KEY,
    reason        TEXT NOT NULL,
    suppressed_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- ── Unsubscribe tokens ──────────────────────────────────────────────────

CREATE TABLE unsubscribe_tokens (
    token_hash TEXT PRIMARY KEY,
    user_id    UUID REFERENCES users(id) ON DELETE CASCADE,
    email      TEXT NOT NULL,
    category   TEXT,
    expires_at TIMESTAMPTZ NOT NULL,
    used       BOOLEAN NOT NULL DEFAULT FALSE
);

CREATE INDEX idx_unsubscribe_tokens_expires
    ON unsubscribe_tokens (expires_at);

-- ── Template seeds ──────────────────────────────────────────────────────
--
-- The four transactional templates previously embedded as `const` literals in
-- `backend/src/email.rs` are re-homed here so the admin API can version them.
-- `body_source` is the Tera-style source with the base layout inlined (rather
-- than extended) so template lookup is self-contained — MJML compilation is
-- deferred to FDN-09.

-- 1. user.welcome
INSERT INTO notification_templates
    (key, channel, locale, subject, body_source, body_compiled, variables, version, is_active)
VALUES (
    'user.welcome',
    'email',
    'en',
    'Welcome to Precision Options Signals!',
    '<!DOCTYPE html><html lang="en"><head><meta charset="UTF-8"><title>Welcome to Precision Options Signals</title></head><body style="margin:0;padding:0;background-color:#0a0f1c;font-family:-apple-system,BlinkMacSystemFont,Segoe UI,Roboto,sans-serif;color:#e2e8f0;"><div style="width:100%;background-color:#0a0f1c;padding:40px 0;"><div style="max-width:600px;margin:0 auto;background-color:#111827;border-radius:12px;overflow:hidden;border:1px solid #1e293b;"><div style="background:linear-gradient(135deg,#0a0f1c 0%,#1a1f3c 100%);padding:32px 40px;text-align:center;border-bottom:2px solid #0fa4af;"><a href="{{ app_url }}" style="font-size:28px;font-weight:800;color:#0fa4af;letter-spacing:-0.5px;text-decoration:none;">Precision Options Signals</a></div><div style="padding:40px;"><h1 style="font-size:24px;font-weight:700;color:#f1f5f9;margin:0 0 16px 0;">Welcome to Precision Options Signals!</h1><p style="font-size:16px;line-height:1.6;color:#94a3b8;margin:0 0 16px 0;">Hi {{ name }},</p><p style="font-size:16px;line-height:1.6;color:#94a3b8;margin:0 0 16px 0;">Thanks for creating your account. We''re excited to have you on board!</p><p style="font-size:16px;line-height:1.6;color:#94a3b8;margin:0 0 16px 0;">Precision Options Signals gives you access to professional swing trading analysis, real-time alerts, and a community of traders dedicated to consistent results.</p><div style="text-align:center;margin:32px 0;"><a href="{{ app_url }}" style="display:inline-block;background-color:#0fa4af;color:#ffffff;text-decoration:none;padding:14px 40px;border-radius:8px;font-size:16px;font-weight:600;">Get Started</a></div><p style="font-size:14px;line-height:1.6;color:#94a3b8;margin:0 0 16px 0;">Questions? Just reply to this email &mdash; we''d love to hear from you.</p></div><div style="background-color:#0a0f1c;padding:24px 40px;text-align:center;border-top:1px solid #1e293b;"><p style="font-size:12px;color:#475569;margin:0 0 8px 0;">&copy; {{ year }} Precision Options Signals. All rights reserved.</p></div></div></div></body></html>',
    '',
    '["name","app_url","year"]'::jsonb,
    1,
    TRUE
);

-- 2. user.password_reset
INSERT INTO notification_templates
    (key, channel, locale, subject, body_source, body_compiled, variables, version, is_active)
VALUES (
    'user.password_reset',
    'email',
    'en',
    'Reset Your Password — Precision Options Signals',
    '<!DOCTYPE html><html lang="en"><head><meta charset="UTF-8"><title>Reset Your Password</title></head><body style="margin:0;padding:0;background-color:#0a0f1c;font-family:-apple-system,BlinkMacSystemFont,Segoe UI,Roboto,sans-serif;color:#e2e8f0;"><div style="width:100%;background-color:#0a0f1c;padding:40px 0;"><div style="max-width:600px;margin:0 auto;background-color:#111827;border-radius:12px;overflow:hidden;border:1px solid #1e293b;"><div style="background:linear-gradient(135deg,#0a0f1c 0%,#1a1f3c 100%);padding:32px 40px;text-align:center;border-bottom:2px solid #0fa4af;"><a href="{{ app_url }}" style="font-size:28px;font-weight:800;color:#0fa4af;text-decoration:none;">Precision Options Signals</a></div><div style="padding:40px;"><h1 style="font-size:24px;font-weight:700;color:#f1f5f9;margin:0 0 16px 0;">Reset Your Password</h1><p style="font-size:16px;line-height:1.6;color:#94a3b8;margin:0 0 16px 0;">Hi {{ name }},</p><p style="font-size:16px;line-height:1.6;color:#94a3b8;margin:0 0 16px 0;">We received a request to reset the password for your Precision Options Signals account. Click the button below to choose a new password.</p><div style="text-align:center;margin:32px 0;"><a href="{{ reset_url }}" style="display:inline-block;background-color:#0fa4af;color:#ffffff;text-decoration:none;padding:14px 40px;border-radius:8px;font-size:16px;font-weight:600;">Reset Password</a></div><p style="font-size:14px;color:#94a3b8;">This link will expire in <strong style="color:#e2e8f0;">1 hour</strong>. If you didn''t request this, ignore the email.</p><p style="font-size:13px;color:#0fa4af;word-break:break-all;">{{ reset_url }}</p></div><div style="background-color:#0a0f1c;padding:24px 40px;text-align:center;border-top:1px solid #1e293b;"><p style="font-size:12px;color:#475569;margin:0;">&copy; {{ year }} Precision Options Signals. All rights reserved.</p></div></div></div></body></html>',
    '',
    '["name","reset_url","app_url","year"]'::jsonb,
    1,
    TRUE
);

-- 3. subscription.confirmed
INSERT INTO notification_templates
    (key, channel, locale, subject, body_source, body_compiled, variables, version, is_active)
VALUES (
    'subscription.confirmed',
    'email',
    'en',
    'Your Subscription is Active — Precision Options Signals',
    '<!DOCTYPE html><html lang="en"><head><meta charset="UTF-8"><title>Subscription Confirmed</title></head><body style="margin:0;padding:0;background-color:#0a0f1c;font-family:-apple-system,BlinkMacSystemFont,Segoe UI,Roboto,sans-serif;color:#e2e8f0;"><div style="width:100%;background-color:#0a0f1c;padding:40px 0;"><div style="max-width:600px;margin:0 auto;background-color:#111827;border-radius:12px;overflow:hidden;border:1px solid #1e293b;"><div style="background:linear-gradient(135deg,#0a0f1c 0%,#1a1f3c 100%);padding:32px 40px;text-align:center;border-bottom:2px solid #0fa4af;"><a href="{{ app_url }}" style="font-size:28px;font-weight:800;color:#0fa4af;text-decoration:none;">Precision Options Signals</a></div><div style="padding:40px;"><h1 style="font-size:24px;font-weight:700;color:#f1f5f9;margin:0 0 16px 0;">Subscription Confirmed!</h1><p style="font-size:16px;line-height:1.6;color:#94a3b8;margin:0 0 16px 0;">Hi {{ name }},</p><p style="font-size:16px;line-height:1.6;color:#94a3b8;margin:0 0 16px 0;">Great news &mdash; your <strong style="color:#0fa4af;">{{ plan_name }}</strong> subscription is now active!</p><div style="text-align:center;margin:32px 0;"><a href="{{ app_url }}/member" style="display:inline-block;background-color:#0fa4af;color:#ffffff;text-decoration:none;padding:14px 40px;border-radius:8px;font-size:16px;font-weight:600;">Go to Dashboard</a></div></div><div style="background-color:#0a0f1c;padding:24px 40px;text-align:center;border-top:1px solid #1e293b;"><p style="font-size:12px;color:#475569;margin:0;">&copy; {{ year }} Precision Options Signals. All rights reserved.</p></div></div></div></body></html>',
    '',
    '["name","plan_name","app_url","year"]'::jsonb,
    1,
    TRUE
);

-- 4. subscription.cancelled
INSERT INTO notification_templates
    (key, channel, locale, subject, body_source, body_compiled, variables, version, is_active)
VALUES (
    'subscription.cancelled',
    'email',
    'en',
    'Your Subscription Has Been Cancelled — Precision Options Signals',
    '<!DOCTYPE html><html lang="en"><head><meta charset="UTF-8"><title>Subscription Cancelled</title></head><body style="margin:0;padding:0;background-color:#0a0f1c;font-family:-apple-system,BlinkMacSystemFont,Segoe UI,Roboto,sans-serif;color:#e2e8f0;"><div style="width:100%;background-color:#0a0f1c;padding:40px 0;"><div style="max-width:600px;margin:0 auto;background-color:#111827;border-radius:12px;overflow:hidden;border:1px solid #1e293b;"><div style="background:linear-gradient(135deg,#0a0f1c 0%,#1a1f3c 100%);padding:32px 40px;text-align:center;border-bottom:2px solid #0fa4af;"><a href="{{ app_url }}" style="font-size:28px;font-weight:800;color:#0fa4af;text-decoration:none;">Precision Options Signals</a></div><div style="padding:40px;"><h1 style="font-size:24px;font-weight:700;color:#f1f5f9;margin:0 0 16px 0;">Subscription Cancelled</h1><p style="font-size:16px;line-height:1.6;color:#94a3b8;margin:0 0 16px 0;">Hi {{ name }},</p><p style="font-size:16px;line-height:1.6;color:#94a3b8;margin:0 0 16px 0;">We''re sorry to see you go. Your subscription has been cancelled; you''ll retain premium access until <strong style="color:#0fa4af;">{{ end_date }}</strong>.</p><div style="text-align:center;margin:32px 0;"><a href="{{ app_url }}/member" style="display:inline-block;background-color:#0fa4af;color:#ffffff;text-decoration:none;padding:14px 40px;border-radius:8px;font-size:16px;font-weight:600;">Resubscribe</a></div></div><div style="background-color:#0a0f1c;padding:24px 40px;text-align:center;border-top:1px solid #1e293b;"><p style="font-size:12px;color:#475569;margin:0;">&copy; {{ year }} Precision Options Signals. All rights reserved.</p></div></div></div></body></html>',
    '',
    '["name","end_date","app_url","year"]'::jsonb,
    1,
    TRUE
);
