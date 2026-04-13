-- Remember status before moving to trash (WordPress-style restore)
ALTER TABLE blog_posts
    ADD COLUMN pre_trash_status post_status NULL,
    ADD COLUMN trashed_at TIMESTAMPTZ NULL;

CREATE INDEX idx_blog_posts_trashed ON blog_posts (trashed_at DESC) WHERE status = 'trash';
