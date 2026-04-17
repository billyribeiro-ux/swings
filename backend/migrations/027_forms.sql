-- FORM-01: Form builder schema model + field library.
--
-- Seven tables capture the form-builder domain ahead of the admin UI (FORM-09)
-- and public renderer (FORM-10). This migration is the data primitive the
-- downstream subsystems (FORM-02 validation, FORM-03 submissions + audit,
-- FORM-04 multi-step + save-and-resume, FORM-05 file uploads, FORM-06
-- anti-spam, FORM-07 integrations, FORM-08 payment fields) layer on top of.
--
--   * `forms`            — top-level form record; `settings` carries anti-spam
--                          toggles, honeypot field name, rate-limit overrides.
--   * `form_versions`    — immutable snapshots of the field tree + logic rules.
--                          Exactly one row per (form_id) is marked
--                          `is_published`; that row is what the public
--                          submit endpoint validates against.
--   * `form_submissions` — one row per submit attempt, with enriched audit
--                          fields (ip_hash, UA, referrer, UTM) and the
--                          validated `data_json` payload keyed by field id.
--   * `form_files`       — file + image uploads captured as part of a
--                          submission. `storage_key` is the R2 object key;
--                          FORM-05 wires the actual upload path.
--   * `form_partials`    — save-and-resume drafts; `resume_token_hash` stores
--                          SHA-256 of a random 32-byte token delivered to
--                          the user via email. `expires_at` bounds retention.
--
-- All JSONB columns are typed as `serde_json::Value` on the Rust side; the
-- canonical `FieldSchema` + `LogicRule` shapes are documented in
-- `backend/src/forms/schema.rs` and `backend/src/forms/logic.rs` and surfaced
-- to clients via the generated OpenAPI.

-- ── Forms ───────────────────────────────────────────────────────────────

CREATE TABLE forms (
    id           UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    slug         TEXT NOT NULL UNIQUE,
    name         TEXT NOT NULL,
    description  TEXT,
    is_active    BOOLEAN NOT NULL DEFAULT TRUE,
    settings     JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_by   UUID REFERENCES users(id) ON DELETE SET NULL,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at   TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_forms_slug ON forms (slug);
CREATE INDEX idx_forms_active ON forms (is_active) WHERE is_active = TRUE;

-- ── Form versions ──────────────────────────────────────────────────────

CREATE TABLE form_versions (
    id           UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    form_id      UUID NOT NULL REFERENCES forms(id) ON DELETE CASCADE,
    version      INT NOT NULL,
    schema_json  JSONB NOT NULL,
    logic_json   JSONB NOT NULL DEFAULT '[]'::jsonb,
    is_published BOOLEAN NOT NULL DEFAULT FALSE,
    published_at TIMESTAMPTZ,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (form_id, version)
);

-- Exactly one active version per form (partial unique index matches the
-- access pattern in `repo::get_active_version`).
CREATE INDEX idx_form_versions_active ON form_versions (form_id) WHERE is_published = TRUE;

-- ── Submissions ────────────────────────────────────────────────────────

CREATE TABLE form_submissions (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    form_id         UUID NOT NULL REFERENCES forms(id) ON DELETE CASCADE,
    form_version_id UUID NOT NULL REFERENCES form_versions(id),
    subject_id      UUID REFERENCES users(id) ON DELETE SET NULL,
    anonymous_id    UUID,
    status          TEXT NOT NULL DEFAULT 'complete'
                    CHECK (status IN ('complete','partial','spam','deleted')),
    data_json       JSONB NOT NULL,
    files_json      JSONB NOT NULL DEFAULT '[]'::jsonb,
    ip_hash         TEXT NOT NULL,
    user_agent      TEXT NOT NULL,
    referrer        TEXT,
    utm             JSONB NOT NULL DEFAULT '{}'::jsonb,
    validation_errors JSONB,
    submitted_at    TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_submissions_form ON form_submissions (form_id, submitted_at DESC);
CREATE INDEX idx_submissions_subject ON form_submissions (subject_id, submitted_at DESC) WHERE subject_id IS NOT NULL;
CREATE INDEX idx_submissions_status ON form_submissions (form_id, status, submitted_at DESC);

-- ── Files ──────────────────────────────────────────────────────────────

CREATE TABLE form_files (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    submission_id   UUID REFERENCES form_submissions(id) ON DELETE CASCADE,
    form_id         UUID NOT NULL REFERENCES forms(id) ON DELETE CASCADE,
    field_key       TEXT NOT NULL,
    storage_key     TEXT NOT NULL,
    filename        TEXT NOT NULL,
    mime_type       TEXT NOT NULL,
    size_bytes      BIGINT NOT NULL,
    sha256          TEXT NOT NULL,
    uploaded_at     TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_form_files_submission ON form_files (submission_id);

-- ── Partials (save-and-resume) ─────────────────────────────────────────

CREATE TABLE form_partials (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    form_id         UUID NOT NULL REFERENCES forms(id) ON DELETE CASCADE,
    resume_token_hash TEXT NOT NULL,
    data_json       JSONB NOT NULL,
    subject_id      UUID REFERENCES users(id) ON DELETE SET NULL,
    expires_at      TIMESTAMPTZ NOT NULL,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_form_partials_expires ON form_partials (expires_at);
CREATE INDEX idx_form_partials_token ON form_partials (resume_token_hash);
