CREATE TABLE IF NOT EXISTS post_meta (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    post_id     UUID NOT NULL REFERENCES blog_posts(id) ON DELETE CASCADE,
    meta_key    TEXT NOT NULL,
    meta_value  TEXT NOT NULL DEFAULT '',
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_post_meta_post_id ON post_meta(post_id);
CREATE UNIQUE INDEX IF NOT EXISTS idx_post_meta_post_key ON post_meta(post_id, meta_key);
