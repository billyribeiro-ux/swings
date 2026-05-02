-- Migration 087: strip the demo popup templates seeded in 052.
--
-- The historical 052_popup_templates.sql migration seeded ~12 starter
-- templates (newsletter signup, exit-intent discount, spin-to-win, etc.)
-- so the popup builder UI looked populated on first run. The operator
-- wants the dashboard to start at the empty-state and build templates
-- from scratch.
--
-- Migrations are forward-only (see AGENTS.md hard rule #1), so we add this
-- DELETE in a new file rather than editing 052. Idempotent: if the rows
-- are already gone the DELETE is a no-op.
--
-- Safe-by-construction: only touches rows where `is_template = TRUE`, so
-- live (admin-authored) popups are unaffected. The `is_template` column
-- itself is preserved — admins still need it when they later mark new
-- popups as templates from the UI.

DELETE FROM popups WHERE is_template = TRUE;
