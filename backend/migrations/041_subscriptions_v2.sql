-- EC-09: Subscriptions 2.0 — pause/resume, prorations, dunning, switching.
--
-- Extends the v1 `subscriptions` table from 001_initial with the columns
-- the dunning + switching engines require, and stands up two log tables:
--
--   * `subscription_changes` — every upgrade / downgrade / pause / resume
--     / renew_early / switch_plan emits a row, capturing the proration
--     amount the engine charged or refunded.
--   * `dunning_attempts` — schedule of retry attempts on a failed
--     renewal; the worker walks `WHERE executed_at IS NULL AND
--     scheduled_at <= NOW()` on each tick.

ALTER TABLE subscriptions
    ADD COLUMN IF NOT EXISTS paused_at            TIMESTAMPTZ,
    ADD COLUMN IF NOT EXISTS pause_resumes_at     TIMESTAMPTZ,
    ADD COLUMN IF NOT EXISTS trial_end            TIMESTAMPTZ,
    ADD COLUMN IF NOT EXISTS cancel_at            TIMESTAMPTZ,
    ADD COLUMN IF NOT EXISTS canceled_at          TIMESTAMPTZ,
    ADD COLUMN IF NOT EXISTS billing_cycle_anchor TIMESTAMPTZ,
    ADD COLUMN IF NOT EXISTS quantity             INT NOT NULL DEFAULT 1,
    ADD COLUMN IF NOT EXISTS price_cents          BIGINT;

CREATE TABLE IF NOT EXISTS subscription_changes (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    subscription_id UUID NOT NULL REFERENCES subscriptions(id) ON DELETE CASCADE,
    kind            TEXT NOT NULL CHECK (kind IN
                                          ('upgrade','downgrade','pause','resume',
                                           'renew_early','switch_plan','cancel')),
    from_plan_id    UUID,
    to_plan_id      UUID,
    proration_cents BIGINT NOT NULL DEFAULT 0,
    actor_id        UUID REFERENCES users(id) ON DELETE SET NULL,
    notes           TEXT,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS subscription_changes_sub_idx
    ON subscription_changes (subscription_id, created_at DESC);

CREATE TABLE IF NOT EXISTS dunning_attempts (
    subscription_id UUID NOT NULL REFERENCES subscriptions(id) ON DELETE CASCADE,
    attempt         INT NOT NULL CHECK (attempt > 0),
    scheduled_at    TIMESTAMPTZ NOT NULL,
    executed_at     TIMESTAMPTZ,
    result          TEXT,
    PRIMARY KEY (subscription_id, attempt)
);

CREATE INDEX IF NOT EXISTS dunning_attempts_due_idx
    ON dunning_attempts (scheduled_at) WHERE executed_at IS NULL;
