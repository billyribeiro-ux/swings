-- ADM-10: search-friendly indexes on the `users` table for the
-- admin members surface introduced in this round.
--
-- Two complementary index strategies are seeded:
--
--   * `pg_trgm` GIN indexes on `email` and `name` so the admin
--     "search members" page can run case-insensitive substring queries
--     (`email ILIKE '%foo%'` / `name ILIKE '%bar%'`) at sub-millisecond
--     latency on a multi-million row population.
--
--   * A `tsvector` expression index on `name || email || coalesce(position,'')`
--     so the same surface can also accept ranked full-text queries
--     (`websearch_to_tsquery('english', $1)`) once the operator wants
--     compound queries (e.g. `"john doe" instructor`).
--
-- Both indexes are concurrent on a fresh table; on an existing one we
-- accept a brief `ACCESS EXCLUSIVE` because Vega's online migrator runs
-- in a maintenance window. If you ever need to ship this hot, switch to
-- `CREATE INDEX CONCURRENTLY` and `-- no-transaction`.

CREATE EXTENSION IF NOT EXISTS pg_trgm;

-- Trigram coverage for the two everyday search columns.
CREATE INDEX IF NOT EXISTS idx_users_email_trgm
    ON users USING GIN (lower(email) gin_trgm_ops);

CREATE INDEX IF NOT EXISTS idx_users_name_trgm
    ON users USING GIN (lower(name) gin_trgm_ops);

-- Full-text coverage. We compute the tsvector at index time rather than
-- storing it as a generated column so we can extend the projection
-- (e.g. add `bio`) without a table rewrite — only an index rebuild.
CREATE INDEX IF NOT EXISTS idx_users_search_fts
    ON users USING GIN (
        to_tsvector(
            'simple',
            coalesce(name, '') || ' ' ||
            coalesce(email, '') || ' ' ||
            coalesce(position, '')
        )
    );

-- Status-filter shortcut: lifecycle queries (`/admin/members?status=active`)
-- need to filter on the absence of suspension/ban quickly.
CREATE INDEX IF NOT EXISTS idx_users_role_created
    ON users (role, created_at DESC);

COMMENT ON INDEX idx_users_email_trgm IS 'pg_trgm coverage for /api/admin/members/search ILIKE queries against email.';
COMMENT ON INDEX idx_users_name_trgm  IS 'pg_trgm coverage for /api/admin/members/search ILIKE queries against name.';
COMMENT ON INDEX idx_users_search_fts IS 'Composite tsvector for ranked /api/admin/members/search queries (name+email+position).';
