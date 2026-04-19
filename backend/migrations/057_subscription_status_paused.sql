-- no-transaction
--
-- ADM-03: align the `subscription_status` enum with EC-09's pause/resume work.
--
-- `commerce::subscriptions::pause` (Phase 1 audit hit) writes
-- `status = 'paused'`, but the enum from `001_initial.sql` only listed
-- ('active','canceled','past_due','trialing','unpaid'). The mismatch meant
-- any caller of `pause()` would crash on an enum-cast error in production.
--
-- Postgres forbids using a freshly-added enum value in the same transaction
-- as the `ALTER TYPE … ADD VALUE` that introduced it; that's why this
-- migration is non-transactional.

ALTER TYPE subscription_status ADD VALUE IF NOT EXISTS 'paused';
