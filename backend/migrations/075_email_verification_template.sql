-- AUTH-01: user e-mail verification template.
--
-- Adds the transactional template consumed by `handlers/auth.rs` when
-- registration/resend verification mints a token.

INSERT INTO notification_templates
    (key, channel, locale, subject, body_source, body_compiled, variables, version, is_active)
SELECT
    'user.email_verification',
    'email',
    'en',
    'Verify your email — Precision Options Signals',
    '<!DOCTYPE html><html lang="en"><head><meta charset="UTF-8"><title>Verify your email</title></head><body style="margin:0;padding:0;background-color:#0a0f1c;font-family:-apple-system,BlinkMacSystemFont,Segoe UI,Roboto,sans-serif;color:#e2e8f0;"><div style="width:100%;background-color:#0a0f1c;padding:40px 0;"><div style="max-width:600px;margin:0 auto;background-color:#111827;border-radius:12px;overflow:hidden;border:1px solid #1e293b;"><div style="background:linear-gradient(135deg,#0a0f1c 0%,#1a1f3c 100%);padding:32px 40px;text-align:center;border-bottom:2px solid #0fa4af;"><a href="{{ app_url }}" style="font-size:28px;font-weight:800;color:#0fa4af;text-decoration:none;">Precision Options Signals</a></div><div style="padding:40px;"><h1 style="font-size:24px;font-weight:700;color:#f1f5f9;margin:0 0 16px 0;">Verify your email</h1><p style="font-size:16px;line-height:1.6;color:#94a3b8;margin:0 0 16px 0;">Hi {{ name }},</p><p style="font-size:16px;line-height:1.6;color:#94a3b8;margin:0 0 16px 0;">Please confirm your email address to secure your account and receive important alerts.</p><div style="text-align:center;margin:32px 0;"><a href="{{ verify_url }}" style="display:inline-block;background-color:#0fa4af;color:#ffffff;text-decoration:none;padding:14px 40px;border-radius:8px;font-size:16px;font-weight:600;">Verify email</a></div><p style="font-size:13px;color:#0fa4af;word-break:break-all;">{{ verify_url }}</p><p style="font-size:14px;color:#94a3b8;">This link expires in 24 hours.</p></div><div style="background-color:#0a0f1c;padding:24px 40px;text-align:center;border-top:1px solid #1e293b;"><p style="font-size:12px;color:#475569;margin:0;">&copy; {{ year }} Precision Options Signals. All rights reserved.</p></div></div></div></body></html>',
    '',
    '["name","verify_url","app_url","year"]'::jsonb,
    1,
    TRUE
WHERE NOT EXISTS (
    SELECT 1
      FROM notification_templates
     WHERE key = 'user.email_verification'
       AND channel = 'email'
       AND locale = 'en'
);
