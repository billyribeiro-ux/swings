-- CONSENT-01: Banner + category model.
--
-- Four tables form the configuration side of consent management (the event
-- log side lands in CONSENT-03 / migration 025). These tables are the
-- admin-editable surface; the public banner endpoint reads them and the
-- Svelte banner renders whatever the admin shipped.
--
--   * consent_policies        — versioned privacy-policy bodies per locale.
--                               Bumping `version` forces a re-consent prompt
--                               on all subjects per GDPR Art. 7(3).
--   * consent_categories      — the (typically five) buckets a subject sees
--                               in the preferences modal. Stable `key` column
--                               is the PK — never renamed after production.
--   * consent_services        — third-party services (GA, Meta Pixel, …)
--                               grouped under a category. Used by
--                               CONSENT-02's script blocker.
--   * consent_banner_configs  — per-(region, locale) banner config. A single
--                               `default`/`en` row is seeded; CONSENT-05 adds
--                               region variants on top, CONSENT-06 adds
--                               locale variants.

-- ── Policies ────────────────────────────────────────────────────────────

CREATE TABLE consent_policies (
    id           UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    version      INT NOT NULL,
    markdown     TEXT NOT NULL,
    locale       TEXT NOT NULL DEFAULT 'en',
    effective_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_at   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (version, locale)
);

CREATE INDEX idx_consent_policies_effective
    ON consent_policies (locale, effective_at DESC);

-- ── Categories ──────────────────────────────────────────────────────────

CREATE TABLE consent_categories (
    key         TEXT PRIMARY KEY,
    label       TEXT NOT NULL,
    description TEXT NOT NULL,
    is_required BOOLEAN NOT NULL DEFAULT FALSE,
    sort_order  INT NOT NULL DEFAULT 0,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- ── Services ────────────────────────────────────────────────────────────

CREATE TABLE consent_services (
    id           UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    slug         TEXT NOT NULL UNIQUE,
    name         TEXT NOT NULL,
    vendor       TEXT NOT NULL,
    category     TEXT NOT NULL REFERENCES consent_categories(key) ON UPDATE CASCADE,
    domains      TEXT[] NOT NULL DEFAULT '{}',
    cookies      JSONB NOT NULL DEFAULT '[]'::jsonb,
    privacy_url  TEXT,
    description  TEXT,
    is_active    BOOLEAN NOT NULL DEFAULT TRUE,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at   TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_consent_services_category
    ON consent_services (category) WHERE is_active = TRUE;

-- ── Banner configs ──────────────────────────────────────────────────────

CREATE TABLE consent_banner_configs (
    id           UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    region       TEXT NOT NULL DEFAULT 'default',
    locale       TEXT NOT NULL DEFAULT 'en',
    version      INT NOT NULL DEFAULT 1,
    layout       TEXT NOT NULL DEFAULT 'bar'
                 CHECK (layout IN ('bar','box','popup','fullscreen')),
    position     TEXT NOT NULL DEFAULT 'bottom'
                 CHECK (position IN ('top','bottom','center','bottom-start','bottom-end')),
    theme_json   JSONB NOT NULL DEFAULT '{}'::jsonb,
    copy_json    JSONB NOT NULL DEFAULT '{}'::jsonb,
    is_active    BOOLEAN NOT NULL DEFAULT TRUE,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (region, locale)
);

CREATE INDEX idx_consent_banner_active
    ON consent_banner_configs (region, locale) WHERE is_active = TRUE;

-- ── Seeds ───────────────────────────────────────────────────────────────
--
-- The five default categories match the shape the frontend stub
-- (`src/lib/api/consent.ts::DEFAULT_CATEGORIES`) already ships. Keeping
-- the DB seed in sync with the stub means swapping the stub for real
-- network calls is a no-op for the UI.

INSERT INTO consent_categories (key, label, description, is_required, sort_order)
VALUES
    ('necessary',       'Strictly necessary',
     'Essential for the site to function — authentication, fraud prevention, load balancing, and your preferences. Cannot be turned off.',
     TRUE,  0),
    ('functional',      'Functional',
     'Remembers choices you make (language, region, saved filters) to give you a better experience.',
     FALSE, 10),
    ('analytics',       'Analytics',
     'Helps us understand how visitors use the site so we can improve it. All data is aggregated and never sold.',
     FALSE, 20),
    ('marketing',       'Marketing',
     'Lets us measure campaign performance and show you relevant offers on other sites.',
     FALSE, 30),
    ('personalization', 'Personalization',
     'Tailors what you see — recommendations, homepage layout, trader highlights — to your interests.',
     FALSE, 40);

INSERT INTO consent_policies (version, markdown, locale)
VALUES (
    1,
    '# Privacy Policy\n\nThis site uses cookies and similar technologies to operate, to understand how the site is used, and — only with your permission — to personalize content and measure advertising.\n\nYou can change your preferences at any time via the "Manage cookies" link in the footer.',
    'en'
);

INSERT INTO consent_banner_configs (region, locale, version, layout, position, copy_json, theme_json, is_active)
VALUES (
    'default',
    'en',
    1,
    'bar',
    'bottom',
    jsonb_build_object(
        'title',              'We value your privacy',
        'body',               'We use cookies and similar technologies to power the site, understand usage, and — with your permission — personalize content. You can accept everything, reject non-essential categories, or choose exactly what to allow.',
        'acceptAll',          'Accept all',
        'rejectAll',          'Reject all',
        'customize',          'Customize',
        'savePreferences',    'Save preferences',
        'privacyPolicyHref',  '/privacy',
        'privacyPolicyLabel', 'Privacy policy'
    ),
    '{}'::jsonb,
    TRUE
);
