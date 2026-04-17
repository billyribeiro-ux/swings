-- FORM-05: chunked / multipart file-upload staging.
--
-- One row per successfully-staged upload, regardless of whether the form
-- has been submitted yet. Two FK columns (both nullable) link back to
-- either a draft (`partial_id`) or completed submission (`submission_id`):
--
--   * While save-and-resume is active (FORM-04) the upload is attached to
--     the draft so a reloaded form keeps its CV/photo reachable.
--   * On submit, the row is re-homed: `submission_id` is backfilled and
--     `partial_id` is cleared. Later draft GC (`gc_expired_partials`)
--     won't blow the upload away because of `ON DELETE SET NULL`.
--
-- Storage key layout (plan §FORM-05):
--   `forms/{form_id}/{partial_or_submission_id}/{upload_id}-{sanitised_name}`
--
-- `sha256` is the raw 32-byte digest BYTEA (same shape as the partials
-- resume token). `mime_type` is the SERVER-sniffed MIME (see `infer::get`
-- on the first 512 B); the handler rejects mismatches before insert.

CREATE TABLE IF NOT EXISTS form_uploads (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    form_id         UUID NOT NULL REFERENCES forms(id) ON DELETE CASCADE,
    partial_id      UUID NULL REFERENCES form_partials(id) ON DELETE SET NULL,
    submission_id   UUID NULL REFERENCES form_submissions(id) ON DELETE SET NULL,
    field_key       TEXT NOT NULL,
    storage_key     TEXT NOT NULL,
    mime_type       TEXT NOT NULL,
    size_bytes      BIGINT NOT NULL CHECK (size_bytes >= 0),
    sha256          BYTEA NOT NULL,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_form_uploads_storage_key
    ON form_uploads (storage_key);

CREATE INDEX IF NOT EXISTS idx_form_uploads_form_field
    ON form_uploads (form_id, field_key);

CREATE INDEX IF NOT EXISTS idx_form_uploads_partial
    ON form_uploads (partial_id) WHERE partial_id IS NOT NULL;

CREATE INDEX IF NOT EXISTS idx_form_uploads_submission
    ON form_uploads (submission_id) WHERE submission_id IS NOT NULL;
