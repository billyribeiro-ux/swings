-- CONSENT-03: Consent event log + DSAR workflow.
--
-- Two tables, both append-conceptually (DSAR rows update their status column
-- only; `consent_records` is strictly append-only):
--
--   * `consent_records` — immutable audit log of every consent-decision event
--                         (grant / deny / update / revoke / expire / prefill).
--                         GDPR Art. 7(1) requires the controller to demonstrate
--                         the subject consented; this is the artefact that
--                         evidence gets pulled from. IP is stored hashed with a
--                         daily-rotated salt so we can still detect
--                         fraud/abuse patterns without retaining raw IPs.
--
--   * `dsar_requests`   — GDPR Art. 15–22 + CCPA §1798.100–130 workflow state.
--                         Subjects (authed or anonymous-by-email) file access,
--                         deletion, portability, rectification, or opt-out
--                         requests; admins fulfil them from the admin UI.
--
-- Template seed at the bottom adds `dsar.verify` so the public DSAR endpoint
-- has a notification template to dispatch without a follow-up migration.
-- Pattern mirrors the four transactional templates seeded in
-- `020_notifications.sql`.

-- ── Consent event log ───────────────────────────────────────────────────

CREATE TABLE consent_records (
    id             UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    -- One of subject_id / anonymous_id must always be present (see CHECK).
    subject_id     UUID REFERENCES users(id) ON DELETE SET NULL,
    anonymous_id   UUID,
    -- SHA256(ip_bytes || daily_salt). Daily-rotated so point-in-time linkage
    -- is possible for fraud review within the rotation window, but no
    -- long-lived identifier is stored.
    ip_hash        TEXT NOT NULL,
    user_agent     TEXT NOT NULL,
    country        TEXT,
    banner_version INT  NOT NULL,
    policy_version INT  NOT NULL,
    categories     JSONB NOT NULL,
    services       JSONB NOT NULL DEFAULT '{}'::jsonb,
    action         TEXT NOT NULL CHECK (action IN
                    ('granted','denied','updated','revoked','expired','prefill')),
    tcf_string     TEXT,
    gpc_signal     BOOLEAN,
    created_at     TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT consent_records_subject_or_anon
        CHECK (subject_id IS NOT NULL OR anonymous_id IS NOT NULL)
);

CREATE INDEX idx_consent_records_subject
    ON consent_records (subject_id, created_at DESC)
    WHERE subject_id IS NOT NULL;

CREATE INDEX idx_consent_records_anon
    ON consent_records (anonymous_id, created_at DESC)
    WHERE anonymous_id IS NOT NULL;

CREATE INDEX idx_consent_records_created
    ON consent_records (created_at DESC);

-- ── DSAR workflow ───────────────────────────────────────────────────────

CREATE TABLE dsar_requests (
    id                      UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    -- Authed subject, if the request came from a logged-in session. Anonymous
    -- requests (CCPA "opt out of sale" most commonly) still carry an email.
    user_id                 UUID REFERENCES users(id) ON DELETE SET NULL,
    email                   TEXT NOT NULL,
    kind                    TEXT NOT NULL CHECK (kind IN
                              ('access','delete','portability','rectification','opt_out_sale')),
    status                  TEXT NOT NULL DEFAULT 'pending'
                              CHECK (status IN
                                ('pending','verifying','in_progress','fulfilled','denied','cancelled')),
    -- SHA256 hash of the one-shot verification token we e-mail to the subject.
    -- Never store the raw token; redeeming compares hashes.
    verification_token_hash TEXT,
    payload                 JSONB NOT NULL DEFAULT '{}'::jsonb,
    fulfilled_at            TIMESTAMPTZ,
    fulfilled_by            UUID REFERENCES users(id) ON DELETE SET NULL,
    -- Where the fulfilment artefact lives. For small JSON access/portability
    -- responses we currently inline a `data:` URI placeholder; R2 uploads
    -- land in a later subsystem (tracked under CONSENT-07).
    fulfillment_url         TEXT,
    admin_notes             TEXT,
    created_at              TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at              TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_dsar_status_created
    ON dsar_requests (status, created_at DESC);

CREATE INDEX idx_dsar_user_created
    ON dsar_requests (user_id, created_at DESC)
    WHERE user_id IS NOT NULL;

-- Case-insensitive lookup: operators search by `lower(email)`.
CREATE INDEX idx_dsar_email_created
    ON dsar_requests (LOWER(email), created_at DESC);

-- ── Template seed: dsar.verify ──────────────────────────────────────────
--
-- The public `POST /api/dsar` endpoint dispatches this template with a
-- `{{ verify_url }}` pointing to the subject-facing confirmation page.
-- A minimal inline HTML body matches the visual tone of `020_notifications.sql`
-- seeds — full MJML + copywriter polish is deferred to CONSENT-06 / FDN-09.

INSERT INTO notification_templates
    (key, channel, locale, subject, body_source, body_compiled, variables, version, is_active)
VALUES (
    'dsar.verify',
    'email',
    'en',
    'Confirm your data request — Precision Options Signals',
    '<!DOCTYPE html><html lang="en"><head><meta charset="UTF-8"><title>Confirm your data request</title></head><body style="margin:0;padding:0;background-color:#0a0f1c;font-family:-apple-system,BlinkMacSystemFont,Segoe UI,Roboto,sans-serif;color:#e2e8f0;"><div style="width:100%;background-color:#0a0f1c;padding:40px 0;"><div style="max-width:600px;margin:0 auto;background-color:#111827;border-radius:12px;overflow:hidden;border:1px solid #1e293b;"><div style="background:linear-gradient(135deg,#0a0f1c 0%,#1a1f3c 100%);padding:32px 40px;text-align:center;border-bottom:2px solid #0fa4af;"><a href="{{ app_url }}" style="font-size:28px;font-weight:800;color:#0fa4af;text-decoration:none;">Precision Options Signals</a></div><div style="padding:40px;"><h1 style="font-size:24px;font-weight:700;color:#f1f5f9;margin:0 0 16px 0;">Confirm your data request</h1><p style="font-size:16px;line-height:1.6;color:#94a3b8;margin:0 0 16px 0;">We received a <strong style="color:#e2e8f0;">{{ kind }}</strong> request linked to this e-mail address. To prevent abuse, please confirm you made the request by clicking the button below.</p><div style="text-align:center;margin:32px 0;"><a href="{{ verify_url }}" style="display:inline-block;background-color:#0fa4af;color:#ffffff;text-decoration:none;padding:14px 40px;border-radius:8px;font-size:16px;font-weight:600;">Confirm request</a></div><p style="font-size:14px;line-height:1.6;color:#94a3b8;margin:0 0 16px 0;">If you did not make this request, you can ignore this e-mail — no action will be taken.</p><p style="font-size:13px;color:#0fa4af;word-break:break-all;">{{ verify_url }}</p></div><div style="background-color:#0a0f1c;padding:24px 40px;text-align:center;border-top:1px solid #1e293b;"><p style="font-size:12px;color:#475569;margin:0;">&copy; {{ year }} Precision Options Signals. All rights reserved.</p></div></div></div></body></html>',
    '',
    '["kind","verify_url","app_url","year"]'::jsonb,
    1,
    TRUE
);
