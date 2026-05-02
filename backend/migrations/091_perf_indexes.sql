-- Forensic Wave-3 PR-1: performance indexes for hot query paths.
--
-- Each index here was identified by the wave-3 audit as a missing
-- covering index for a query that runs on every request to a
-- common surface (coupon claim, blog list, admin orders list,
-- refresh-token rotation).
--
-- Idempotent (`IF NOT EXISTS`). Transactional — sqlx wraps each
-- migration; this file does NOT use `CREATE INDEX CONCURRENTLY`
-- because it would require `-- no-transaction` and the
-- single-file constraint outweighs the lock window for tables
-- of the size this app currently has. Future migrations against
-- a multi-million-row table SHOULD switch to CONCURRENTLY.

-- ── coupons ─────────────────────────────────────────────────────────────
-- Every redemption + validation path runs `WHERE UPPER(code) = UPPER($1)`
-- (handlers/coupons.rs:363, :629). The base index `idx_coupons_code`
-- can't be used because the predicate calls a function on the column;
-- a functional index on UPPER(code) makes the lookup an index probe
-- instead of a sequential scan + per-row UPPER().
CREATE INDEX IF NOT EXISTS idx_coupons_code_upper
    ON coupons (UPPER(code));

-- ── blog_posts ──────────────────────────────────────────────────────────
-- Public list path: `WHERE status = 'published' ORDER BY is_sticky DESC,
-- published_at DESC` (db.rs:1926). Existing single-column indexes on
-- `status` and `published_at` work for one or the other but force a
-- merge / extra sort. The composite avoids the sort step entirely for
-- the most common public query.
CREATE INDEX IF NOT EXISTS idx_blog_posts_status_pub_at
    ON blog_posts (status, published_at DESC);

-- Scheduler path (services/blog_scheduler.rs): `WHERE status = 'scheduled'
-- AND scheduled_at <= NOW()`. A partial index on the scheduled subset
-- keeps the index tiny (only rows still in flight) and the worker tick
-- becomes an O(rows-due) scan rather than O(all-posts).
CREATE INDEX IF NOT EXISTS idx_blog_posts_scheduled_due
    ON blog_posts (scheduled_at)
    WHERE status = 'scheduled';

-- ── orders ──────────────────────────────────────────────────────────────
-- Admin orders list filters by status and orders by created_at DESC
-- (handlers/admin_orders.rs:218–234). The two single-column indexes
-- can't satisfy this together; PG uses one and re-sorts. The composite
-- supports the common query directly.
CREATE INDEX IF NOT EXISTS idx_orders_status_created_at
    ON orders (status, created_at DESC);

-- ── refresh_tokens ──────────────────────────────────────────────────────
-- Family reuse-detection: `WHERE family_id = $1 AND used = FALSE`
-- (auth.rs::refresh_*). Existing index is family_id only; adding the
-- partial index on the live (used = FALSE) subset matches the
-- predicate exactly and stays small (used rows are rotated quickly).
CREATE INDEX IF NOT EXISTS idx_refresh_tokens_family_active
    ON refresh_tokens (family_id)
    WHERE used = FALSE;
