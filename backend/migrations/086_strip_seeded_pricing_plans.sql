-- Migration 086: strip the demo Monthly + Annual pricing plans seeded in 012.
--
-- The historical 012_pricing_plans.sql migration seeded two demo rows so a
-- fresh dev DB had a working /api/pricing endpoint out of the box. The
-- operator now wants to start from a truly empty catalog and build their
-- real pricing through the admin UI.
--
-- Migrations are forward-only (see AGENTS.md hard rule #1), so we add this
-- DELETE in a new file rather than editing 012. If the rows are already
-- gone (because the operator deleted them via the admin UI before this
-- migration shipped) the DELETE is a no-op and the migration stays
-- idempotent.
--
-- Safe-by-construction: matches by `slug` (UNIQUE) so we only touch the
-- two specific rows seeded by 012, never anything an admin authored.

DELETE FROM pricing_plans WHERE slug IN ('monthly', 'annual');
