-- ADM-07-α: notify the impersonated user when an admin starts a session.
--
-- GDPR Art. 32 requires "appropriate technical measures" to protect
-- personal data. Logging in as another user is by definition a
-- privileged read of every personal-data field; the affected data
-- subject MUST receive a contemporaneous notification with the actor,
-- justification, and TTL. The send is best-effort (failures must not
-- block the admin), and the audit row in `admin_actions` is the
-- compliance-grade source of truth.
--
-- The template lives alongside the other transactional templates
-- introduced in migration 020. We use the same hand-rolled HTML scaffold
-- so the look-and-feel matches the existing welcome / reset emails.

INSERT INTO notification_templates
    (key, channel, locale, subject, body_source, body_compiled, variables, version, is_active)
VALUES (
    'admin.impersonation_started',
    'email',
    'en',
    'Admin support session started on your account — Precision Options Signals',
    '<!DOCTYPE html><html lang="en"><head><meta charset="UTF-8"><title>Admin support session started</title></head><body style="margin:0;padding:0;background-color:#0a0f1c;font-family:-apple-system,BlinkMacSystemFont,Segoe UI,Roboto,sans-serif;color:#e2e8f0;"><div style="width:100%;background-color:#0a0f1c;padding:40px 0;"><div style="max-width:600px;margin:0 auto;background-color:#111827;border-radius:12px;overflow:hidden;border:1px solid #1e293b;"><div style="background:linear-gradient(135deg,#0a0f1c 0%,#1a1f3c 100%);padding:32px 40px;text-align:center;border-bottom:2px solid #0fa4af;"><a href="{{ app_url }}" style="font-size:28px;font-weight:800;color:#0fa4af;letter-spacing:-0.5px;text-decoration:none;">Precision Options Signals</a></div><div style="padding:40px;"><h1 style="font-size:24px;font-weight:700;color:#f1f5f9;margin:0 0 16px 0;">A support agent accessed your account</h1><p style="font-size:16px;line-height:1.6;color:#94a3b8;margin:0 0 16px 0;">Hi {{ name }},</p><p style="font-size:16px;line-height:1.6;color:#94a3b8;margin:0 0 16px 0;">An authorised administrator started a temporary support session on your account. They can act as you for up to <strong style="color:#0fa4af;">{{ ttl_minutes }} minutes</strong>. The session ends automatically — no action is required from you.</p><table role="presentation" style="width:100%;margin:24px 0;border:1px solid #1e293b;border-radius:8px;border-collapse:separate;border-spacing:0;"><tr><td style="padding:12px 16px;color:#94a3b8;font-size:14px;">Reason provided</td><td style="padding:12px 16px;color:#e2e8f0;font-size:14px;">{{ reason }}</td></tr><tr><td style="padding:12px 16px;color:#94a3b8;font-size:14px;border-top:1px solid #1e293b;">Started at</td><td style="padding:12px 16px;color:#e2e8f0;font-size:14px;border-top:1px solid #1e293b;">{{ issued_at }}</td></tr><tr><td style="padding:12px 16px;color:#94a3b8;font-size:14px;border-top:1px solid #1e293b;">Expires at</td><td style="padding:12px 16px;color:#e2e8f0;font-size:14px;border-top:1px solid #1e293b;">{{ expires_at }}</td></tr></table><p style="font-size:14px;line-height:1.6;color:#94a3b8;margin:0 0 16px 0;">If you did not request support and believe this access was unauthorised, please reply to this email immediately and we will investigate.</p></div><div style="background-color:#0a0f1c;padding:24px 40px;text-align:center;border-top:1px solid #1e293b;"><p style="font-size:12px;color:#475569;margin:0;">&copy; {{ year }} Precision Options Signals. All rights reserved.</p></div></div></div></body></html>',
    '',
    '["name","reason","issued_at","expires_at","ttl_minutes","app_url","year"]'::jsonb,
    1,
    TRUE
);
