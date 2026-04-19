-- ADM-08: typed application settings catalogue.
--
-- Single source of truth for runtime-tunable knobs (feature flags,
-- maintenance mode, integration toggles, secret material) that operators
-- need to change without redeploying. Schema choices:
--
--   * `key` is the canonical TEXT id (`system.maintenance_mode`,
--     `marketing.popups.enabled`, `integrations.stripe.api_key`, ...).
--     Dot-namespaced so the admin UI can group by prefix.
--
--   * `value` is JSONB so each entry can carry a typed scalar
--     (string / int / bool / object) without per-type columns. The
--     application enforces the type via the `value_type` discriminator
--     so a frontend cannot silently coerce one to another.
--
--   * `value_type` carries the discriminator: `'string' | 'int' | 'bool'
--     | 'json' | 'secret'`. The `'secret'` variant is special — `value`
--     stores the AES-GCM ciphertext envelope (see
--     `src/settings/crypto.rs`); the read API returns the ciphertext to
--     non-privileged readers and only decrypts when the caller carries
--     `admin.settings.read_secret`.
--
--   * `is_secret` is denormalised from `value_type` for cheap WHERE
--     filtering and to allow operators to mark a non-secret value as
--     "redact in admin listings" without re-typing.
--
--   * `updated_by` is nullable + `ON DELETE SET NULL` because deleting
--     an admin must not destroy history; the `admin_actions` audit row
--     remains the authoritative attribution.
--
-- Append-once defence: there is no `created_at` column because the row
-- is upserted; the `updated_at` trigger covers the bookkeeping. The
-- canonical mutation log lives in `admin_actions`.

CREATE TABLE IF NOT EXISTS app_settings (
    key          TEXT        PRIMARY KEY CHECK (length(key) BETWEEN 1 AND 128),
    value        JSONB       NOT NULL,
    value_type   TEXT        NOT NULL CHECK (value_type IN ('string','int','bool','json','secret')),
    is_secret    BOOLEAN     NOT NULL DEFAULT FALSE,
    description  TEXT,
    category     TEXT        NOT NULL DEFAULT 'general' CHECK (length(category) BETWEEN 1 AND 64),
    updated_at   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_by   UUID        REFERENCES users(id) ON DELETE SET NULL
);

CREATE INDEX IF NOT EXISTS idx_app_settings_category ON app_settings (category, key);

COMMENT ON TABLE app_settings IS
    'Runtime-tunable typed key/value catalogue. Mutated via /api/admin/settings; cache lives in AppState.';

-- ── Seeds: maintenance mode + admin defaults ───────────────────────────
--
-- The maintenance toggles are queried by `middleware/maintenance.rs` on
-- every request. They MUST exist at boot or the middleware would have
-- to special-case "key absent → off", widening the attack surface (an
-- accidental DELETE would silently disable the maintenance gate).

INSERT INTO app_settings (key, value, value_type, is_secret, description, category)
VALUES
    ('system.maintenance_mode',
     'false'::jsonb,
     'bool',
     FALSE,
     'When true, public + member endpoints return 503 with the maintenance message. Admin routes remain reachable.',
     'system'),
    ('system.maintenance_message',
     '"We are performing scheduled maintenance. We will be back shortly."'::jsonb,
     'string',
     FALSE,
     'User-visible string returned with the 503 maintenance response.',
     'system'),
    ('system.maintenance_admin_only',
     'true'::jsonb,
     'bool',
     FALSE,
     'When true, admins can still reach all routes during maintenance. When false, even admins are blocked except /api/admin/settings/*.',
     'system')
ON CONFLICT (key) DO NOTHING;
