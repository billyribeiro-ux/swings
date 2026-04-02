-- Blog post status enum
CREATE TYPE post_status AS ENUM ('draft', 'pending_review', 'published', 'private', 'scheduled', 'trash');

-- Media table (used by blog posts and general uploads)
CREATE TABLE media (
    id UUID PRIMARY KEY,
    uploader_id UUID NOT NULL REFERENCES users(id) ON DELETE SET NULL,
    filename TEXT NOT NULL,
    original_filename TEXT NOT NULL,
    mime_type TEXT NOT NULL,
    file_size BIGINT NOT NULL,
    width INT,
    height INT,
    alt_text TEXT DEFAULT '',
    caption TEXT DEFAULT '',
    storage_path TEXT NOT NULL,
    url TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_media_uploader ON media (uploader_id);
CREATE INDEX idx_media_mime ON media (mime_type);
CREATE INDEX idx_media_created ON media (created_at DESC);

-- Blog categories (hierarchical via parent_id)
CREATE TABLE blog_categories (
    id UUID PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    slug TEXT NOT NULL UNIQUE,
    description TEXT DEFAULT '',
    parent_id UUID REFERENCES blog_categories(id) ON DELETE SET NULL,
    sort_order INT NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_blog_categories_slug ON blog_categories (slug);
CREATE INDEX idx_blog_categories_parent ON blog_categories (parent_id);

-- Blog tags (flat)
CREATE TABLE blog_tags (
    id UUID PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    slug TEXT NOT NULL UNIQUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_blog_tags_slug ON blog_tags (slug);

-- Blog posts
CREATE TABLE blog_posts (
    id UUID PRIMARY KEY,
    author_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    title TEXT NOT NULL,
    slug TEXT NOT NULL UNIQUE,
    content TEXT NOT NULL DEFAULT '',
    content_json JSONB,
    excerpt TEXT DEFAULT '',
    featured_image_id UUID REFERENCES media(id) ON DELETE SET NULL,
    status post_status NOT NULL DEFAULT 'draft',
    visibility TEXT NOT NULL DEFAULT 'public',
    password_hash TEXT,
    is_sticky BOOLEAN NOT NULL DEFAULT FALSE,
    allow_comments BOOLEAN NOT NULL DEFAULT TRUE,
    meta_title TEXT DEFAULT '',
    meta_description TEXT DEFAULT '',
    canonical_url TEXT DEFAULT '',
    og_image_url TEXT DEFAULT '',
    reading_time_minutes INT NOT NULL DEFAULT 0,
    word_count INT NOT NULL DEFAULT 0,
    scheduled_at TIMESTAMPTZ,
    published_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_blog_posts_slug ON blog_posts (slug);
CREATE INDEX idx_blog_posts_author ON blog_posts (author_id);
CREATE INDEX idx_blog_posts_status ON blog_posts (status);
CREATE INDEX idx_blog_posts_published ON blog_posts (published_at DESC);
CREATE INDEX idx_blog_posts_created ON blog_posts (created_at DESC);

-- Blog post ↔ category junction
CREATE TABLE blog_post_categories (
    post_id UUID NOT NULL REFERENCES blog_posts(id) ON DELETE CASCADE,
    category_id UUID NOT NULL REFERENCES blog_categories(id) ON DELETE CASCADE,
    PRIMARY KEY (post_id, category_id)
);

-- Blog post ↔ tag junction
CREATE TABLE blog_post_tags (
    post_id UUID NOT NULL REFERENCES blog_posts(id) ON DELETE CASCADE,
    tag_id UUID NOT NULL REFERENCES blog_tags(id) ON DELETE CASCADE,
    PRIMARY KEY (post_id, tag_id)
);

-- Blog revisions (full content snapshots)
CREATE TABLE blog_revisions (
    id UUID PRIMARY KEY,
    post_id UUID NOT NULL REFERENCES blog_posts(id) ON DELETE CASCADE,
    author_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    title TEXT NOT NULL,
    content TEXT NOT NULL,
    content_json JSONB,
    revision_number INT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_blog_revisions_post ON blog_revisions (post_id, revision_number DESC);

-- Seed a default "Uncategorized" category
INSERT INTO blog_categories (id, name, slug, description, sort_order)
VALUES ('00000000-0000-0000-0000-000000000001', 'Uncategorized', 'uncategorized', 'Default category', 0);
