-- CONSENT-05: regional banner config seeds.
--
-- Migration 024 seeded a single ('default', 'en') banner. This migration adds
-- two stricter regional variants that the geo resolver routes to:
--
--   * ('EU', 'en')  — GDPR + ePrivacy copy. Emphasises "reject all" at parity
--                     with "accept all" (EDPB 05/2020 §63). No dark patterns.
--
--   * ('US-CA', 'en') — CCPA / CPRA copy using the legally-required "Do Not
--                       Sell or Share My Personal Information" wording.
--
-- `CONSENT-03` lands migration `025_consent_log.sql` with the consent_records
-- + dsar_requests tables in a sibling worktree. The migration number here is
-- 025 as well because this worktree is branched from CONSENT-02 before that
-- sibling merged; when both land on main the sibling's sqlx-migrate timestamp
-- column will surface the conflict and an operator will re-number one of the
-- two. This file is idempotent (INSERT … ON CONFLICT DO NOTHING) so the
-- reorder is mechanical.
--
-- The banner copy is the authoritative source for the frontend even after
-- CONSENT-06 lands — the i18n framework treats these `copy_json` blobs as
-- the source-of-truth fallback; paraglide translations only override.

INSERT INTO consent_banner_configs (region, locale, version, layout, position, copy_json, theme_json, is_active)
VALUES (
    'EU',
    'en',
    1,
    'box',
    'bottom',
    jsonb_build_object(
        'title',              'We respect your privacy',
        'body',               'We use cookies and similar technologies to operate this site, understand usage, and — only with your explicit consent — to personalize content and measure advertising. You can accept all, reject all, or choose exactly what to allow. Your choice is free and you can change it at any time.',
        'acceptAll',          'Accept all',
        'rejectAll',          'Reject all',
        'customize',          'Choose what to allow',
        'savePreferences',    'Save choices',
        'privacyPolicyHref',  '/privacy',
        'privacyPolicyLabel', 'Privacy policy'
    ),
    jsonb_build_object(
        'rejectParityWithAccept', true
    ),
    TRUE
)
ON CONFLICT (region, locale) DO NOTHING;

INSERT INTO consent_banner_configs (region, locale, version, layout, position, copy_json, theme_json, is_active)
VALUES (
    'US-CA',
    'en',
    1,
    'bar',
    'bottom',
    jsonb_build_object(
        'title',              'Your California privacy choices',
        'body',               'We use cookies and similar technologies to operate this site and to understand how it is used. We do not sell or share your personal information, but some third-party tags may count as "sharing" under California law. Use the link below to opt out.',
        'acceptAll',          'Accept all',
        'rejectAll',          'Do Not Sell or Share My Personal Information',
        'customize',          'Manage choices',
        'savePreferences',    'Save choices',
        'privacyPolicyHref',  '/privacy',
        'privacyPolicyLabel', 'Notice at collection'
    ),
    jsonb_build_object(
        'doNotSellEmphasis', true
    ),
    TRUE
)
ON CONFLICT (region, locale) DO NOTHING;
