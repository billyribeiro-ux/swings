-- FORM-04: save-and-resume upgrade for `form_partials`.
--
-- Migration 027_forms.sql stood up a minimal `form_partials` table keyed by
-- a hex SHA-256 string. FORM-04 extends the shape so the renderer (FORM-10)
-- can jump a resumer straight back to the page-break they stopped on, and
-- so the handler can extend the TTL on every partial save instead of
-- creating a new row per draft write.
--
--   * `resume_token_hash` becomes BYTEA (32-byte SHA-256 digest) — the raw
--     bytes are 2× cheaper to bind than the 64-char hex representation and
--     remove the case-normalisation trap. The TRUNCATE clears any legacy
--     rows that couldn't be losslessly transcoded; production has never
--     stored a partial under the old shape (the handlers were not wired in
--     main.rs prior to FORM-03) so this is safe.
--   * `current_step` tracks which page-break the draft stopped on, zero-based
--     into the flat field array after schema-splitting on `page_break`.
--   * `updated_at` rolls forward on every save; drives the GC sweep.
--   * A unique index on `resume_token_hash` replaces the legacy non-unique
--     one — the plaintext token is globally unique by construction so the
--     index is what the resolver hits first, ahead of the form_id filter.

TRUNCATE TABLE form_partials;

DROP INDEX IF EXISTS idx_form_partials_token;

ALTER TABLE form_partials
    ALTER COLUMN resume_token_hash TYPE BYTEA
        USING decode(resume_token_hash, 'hex');

ALTER TABLE form_partials
    ADD COLUMN IF NOT EXISTS current_step INT NOT NULL DEFAULT 0;

ALTER TABLE form_partials
    ADD COLUMN IF NOT EXISTS updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW();

CREATE UNIQUE INDEX IF NOT EXISTS idx_form_partials_token_unique
    ON form_partials (resume_token_hash);

CREATE INDEX IF NOT EXISTS idx_form_partials_expires_at
    ON form_partials (expires_at);
