-- EC-02: Product catalog search + faceting + nested categories.
--
-- * `products.search_tsv` — tsvector pre-computed by a trigger from the
--   combined name + description + short_description text. GIN index on
--   the tsvector column keeps `@@ to_tsquery(…)` matches sub-ms.
-- * `product_categories` — nested catalogue using parent_id + a
--   materialised `path` text column so we can query descendants with a
--   cheap `LIKE prefix` scan. ltree would be cleaner but requires the
--   `ltree` extension; a text path + prefix match works on vanilla PG.
-- * `product_category_links` — many-to-many join (products can belong to
--   multiple categories; categories list multiple products).
-- * `product_facets` materialized view — per-category + per-tag product
--   counts used to build the filter sidebar without a full table scan.
--
-- "Tags" are read out of `products.metadata->'tags'` — a JSONB string
-- array. Keeping tags inside the row avoids a third join and lets admins
-- edit them from the existing metadata-editor UI.

-- ── tsvector column + trigger ─────────────────────────────────────────

ALTER TABLE products
    ADD COLUMN IF NOT EXISTS search_tsv tsvector;

CREATE OR REPLACE FUNCTION products_search_tsv_update() RETURNS trigger AS $$
BEGIN
    NEW.search_tsv :=
        setweight(to_tsvector('simple', coalesce(NEW.name, '')), 'A') ||
        setweight(to_tsvector('simple', coalesce(NEW.description, '')), 'B') ||
        setweight(to_tsvector('simple', coalesce(NEW.seo_description, '')), 'C');
    RETURN NEW;
END
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS products_search_tsv_trg ON products;
CREATE TRIGGER products_search_tsv_trg
BEFORE INSERT OR UPDATE OF name, description, seo_description ON products
FOR EACH ROW EXECUTE FUNCTION products_search_tsv_update();

-- Backfill every existing row so the index is complete.
UPDATE products SET search_tsv
    = setweight(to_tsvector('simple', coalesce(name, '')), 'A')
        || setweight(to_tsvector('simple', coalesce(description, '')), 'B')
        || setweight(to_tsvector('simple', coalesce(seo_description, '')), 'C');

CREATE INDEX IF NOT EXISTS products_search_tsv_idx ON products USING gin (search_tsv);

-- ── Categories ────────────────────────────────────────────────────────

CREATE TABLE IF NOT EXISTS product_categories (
    id            UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    slug          TEXT NOT NULL UNIQUE,
    name          TEXT NOT NULL,
    description   TEXT,
    parent_id     UUID REFERENCES product_categories(id) ON DELETE SET NULL,
    path          TEXT NOT NULL DEFAULT '',
    sort_order    INT NOT NULL DEFAULT 0,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at    TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS product_categories_parent_idx ON product_categories (parent_id);
CREATE INDEX IF NOT EXISTS product_categories_path_idx ON product_categories (path text_pattern_ops);

-- Maintain the `path` materialised column. Path format is `/parent-slug/child-slug/`
-- so a descendant query is `path LIKE '/root-slug/%'`.
CREATE OR REPLACE FUNCTION product_categories_path_update() RETURNS trigger AS $$
DECLARE
    parent_path TEXT;
BEGIN
    IF NEW.parent_id IS NULL THEN
        NEW.path := '/' || NEW.slug || '/';
    ELSE
        SELECT path INTO parent_path FROM product_categories WHERE id = NEW.parent_id;
        NEW.path := COALESCE(parent_path, '/') || NEW.slug || '/';
    END IF;
    NEW.updated_at := NOW();
    RETURN NEW;
END
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS product_categories_path_trg ON product_categories;
CREATE TRIGGER product_categories_path_trg
BEFORE INSERT OR UPDATE OF slug, parent_id ON product_categories
FOR EACH ROW EXECUTE FUNCTION product_categories_path_update();

-- ── Product ↔ category links ──────────────────────────────────────────

CREATE TABLE IF NOT EXISTS product_category_links (
    product_id   UUID NOT NULL REFERENCES products(id) ON DELETE CASCADE,
    category_id  UUID NOT NULL REFERENCES product_categories(id) ON DELETE CASCADE,
    PRIMARY KEY (product_id, category_id)
);

CREATE INDEX IF NOT EXISTS product_category_links_category_idx
    ON product_category_links (category_id);

-- ── Facets materialized view ──────────────────────────────────────────
--
-- Refreshed via a periodic worker (REFRESH MATERIALIZED VIEW CONCURRENTLY)
-- so a heavy search query doesn't block on a live aggregate. First cut is
-- a non-concurrent view because the view is small; we can switch to
-- CONCURRENTLY once the catalogue grows past ~10k products by adding a
-- UNIQUE index on the facet row key.

CREATE MATERIALIZED VIEW IF NOT EXISTS product_facets AS
SELECT
    'category'::text         AS facet_kind,
    c.id::text               AS facet_value,
    c.slug                   AS facet_label,
    COUNT(DISTINCT p.id)::bigint AS product_count
FROM product_categories c
LEFT JOIN product_category_links l ON c.id = l.category_id
LEFT JOIN products p ON l.product_id = p.id AND p.status = 'published'
GROUP BY c.id, c.slug
UNION ALL
SELECT
    'tag'::text              AS facet_kind,
    tag                      AS facet_value,
    tag                      AS facet_label,
    COUNT(*)::bigint         AS product_count
FROM products p,
     LATERAL jsonb_array_elements_text(COALESCE(p.metadata->'tags', '[]'::jsonb)) AS tag
WHERE p.status = 'published'
GROUP BY tag;

CREATE INDEX IF NOT EXISTS product_facets_kind_idx ON product_facets (facet_kind);
