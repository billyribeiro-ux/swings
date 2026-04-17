-- Migration 050 (POP-01): expanded triggers + targeting rules
--
-- Replaces the 6-type CHECK constraint on popups.popup_type with a lookup
-- table so operators can extend the catalogue without a schema migration.
-- Seeds the ten concrete types referenced across POP-01..POP-04.
--
-- targeting_rules stays JSONB — the schema is enforced at the application
-- boundary (see backend/src/popups/targeting.rs). The documented key set
-- lives in the comment block below so readers do not have to grep Rust.

-- ── popup_types lookup ───────────────────────────────────────────────────

CREATE TABLE IF NOT EXISTS popup_types (
    key         TEXT PRIMARY KEY,
    label       TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    is_active   BOOLEAN NOT NULL DEFAULT TRUE
);

INSERT INTO popup_types (key, label, description) VALUES
    ('modal',          'Modal',           'Centered dialog with optional backdrop'),
    ('slide_in',       'Slide-in',        'Corner slide-in toast'),
    ('floating_bar',   'Floating bar',    'Sticky top/bottom announcement bar'),
    ('fullscreen',     'Fullscreen',      'Full-viewport interstitial'),
    ('inline',         'Inline',          'Inline-embedded block'),
    ('content_locker', 'Content locker',  'Blurred content until form submit'),
    ('countdown',      'Countdown',       'Urgency countdown bar'),
    ('notification',   'Notification',    'Passive browser-style notification'),
    ('spin_to_win',    'Spin-to-win',     'Gamified prize wheel'),
    ('scratch_card',   'Scratch card',    'Scratch-to-reveal prize card')
ON CONFLICT (key) DO UPDATE
    SET label = EXCLUDED.label,
        description = EXCLUDED.description;

-- ── popups.popup_type: drop old CHECK, add FK ────────────────────────────
-- The 015 migration created the CHECK inline with no explicit name, so PG
-- assigned one like `popups_popup_type_check`. We drop IF EXISTS by that
-- canonical name; in environments where the constraint was renamed we fall
-- back to scanning pg_constraint.

DO $$
DECLARE
    conname TEXT;
BEGIN
    FOR conname IN
        SELECT c.conname
          FROM pg_constraint c
          JOIN pg_class t ON t.oid = c.conrelid
         WHERE t.relname = 'popups'
           AND c.contype = 'c'
           AND pg_get_constraintdef(c.oid) ILIKE '%popup_type%'
    LOOP
        EXECUTE format('ALTER TABLE popups DROP CONSTRAINT IF EXISTS %I', conname);
    END LOOP;
END $$;

-- Add the FK as NOT VALID first so concurrent writes are not blocked; then
-- validate. `IF NOT EXISTS` on the constraint itself is not supported by
-- Postgres, so we gate on pg_constraint instead.
DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_constraint WHERE conname = 'popups_popup_type_fkey'
    ) THEN
        ALTER TABLE popups
            ADD CONSTRAINT popups_popup_type_fkey
            FOREIGN KEY (popup_type) REFERENCES popup_types(key)
            NOT VALID;
    END IF;
END $$;

ALTER TABLE popups VALIDATE CONSTRAINT popups_popup_type_fkey;

-- ── targeting_rules schema (documentation only) ──────────────────────────
COMMENT ON COLUMN popups.targeting_rules IS $doc$
JSONB object. Supported keys (all optional; missing = no constraint):

  pages                     TEXT[]  URL patterns, legacy: "*", "/path", "/prefix/*"
  devices                   TEXT[]  subset of {mobile, desktop, tablet}
  userStatus                TEXT[]  {all, anonymous, authenticated, member}

  # POP-01 expansion
  geo                       TEXT[]  ISO-3166-1 alpha-2 codes, uppercase
  utm_source                TEXT[]  case-insensitive match on UTM source
  utm_medium                TEXT[]  case-insensitive match on UTM medium
  utm_campaign              TEXT[]  case-insensitive match on UTM campaign
  url_include_regex         TEXT    RE2-style regex; path MUST match
  url_exclude_regex         TEXT    RE2-style regex; path MUST NOT match
  cart_value_cents_min      INT     lower bound (inclusive), integer cents
  cart_value_cents_max      INT     upper bound (inclusive), integer cents
  cart_contains_sku         TEXT[]  match if cart contains ANY listed SKU
  membership_tier           TEXT[]  visitor membership tier must be in set
  time_of_day_start         TEXT    "HH:MM" (24h, visitor local)
  time_of_day_end           TEXT    "HH:MM" (24h, visitor local)
  day_of_week               INT[]   subset of {0..6}, 0=Sunday
  returning_visitor         BOOL    true = show only to returning visitors
  device                    TEXT    alias for singleton `devices`
  browser_family            TEXT[]  e.g. ["Chrome","Firefox"]
  new_visitor_max_pageviews INT     show only while pageview_count <= N
$doc$;
