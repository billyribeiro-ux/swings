-- FDN-04: Transactional outbox + worker pool
--
-- `outbox_events` holds domain events enqueued inside the same transaction as
-- the domain mutation. A background worker (backend/src/events/worker.rs)
-- leases rows with `SELECT ... FOR UPDATE SKIP LOCKED`, dispatches them, and
-- advances their status. Exponential backoff + max-attempts caps feed the
-- `dead_letter` state; `attempts`/`next_attempt_at` are the retry knobs.

CREATE TABLE outbox_events (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    aggregate_type  TEXT NOT NULL,
    aggregate_id    TEXT NOT NULL,
    event_type      TEXT NOT NULL,
    payload         JSONB NOT NULL,
    headers         JSONB NOT NULL DEFAULT '{}'::jsonb,
    status          TEXT NOT NULL DEFAULT 'pending'
                    CHECK (status IN ('pending', 'in_flight', 'delivered', 'failed', 'dead_letter')),
    attempts        INT  NOT NULL DEFAULT 0,
    max_attempts    INT  NOT NULL DEFAULT 8,
    next_attempt_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_error      TEXT,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Partial index supports the worker's claim query: cheap scan of ready rows.
CREATE INDEX idx_outbox_pending
    ON outbox_events (status, next_attempt_at)
    WHERE status IN ('pending', 'in_flight');

-- Lookup-by-aggregate (admin UI filtering, consumer-side deduplication).
CREATE INDEX idx_outbox_aggregate
    ON outbox_events (aggregate_type, aggregate_id);

CREATE TABLE outbox_subscribers (
    id             UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name           TEXT NOT NULL UNIQUE,
    event_pattern  TEXT NOT NULL,
    is_active      BOOLEAN NOT NULL DEFAULT TRUE
);
