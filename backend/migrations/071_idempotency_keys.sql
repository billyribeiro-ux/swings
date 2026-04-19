-- ADM-15: Idempotency-Key persistence for admin POST mutations.
--
-- Stores the cached response for a given (actor_user_id, key) pair so
-- a retried request returns the original response byte-for-byte instead
-- of re-running the side effect (e.g. double-creating an order, double
-- refunding, double-comping a membership).
--
-- The table is keyed on (user_id, key) because Idempotency-Key is
-- defined per-actor: two different operators using the same key value
-- must not collide. Within an actor, the same key replays the cached
-- response. A request_hash column protects against the
-- "same key, different body" footgun — that case 422s.
--
-- TTL is enforced by `expires_at`; a separate prune job (ADM-16) sweeps
-- expired rows. Default TTL is 24h, matching Stripe's idempotency
-- semantics.

CREATE TABLE IF NOT EXISTS idempotency_keys (
    user_id        UUID         NOT NULL,
    key            TEXT         NOT NULL,
    method         TEXT         NOT NULL,
    path           TEXT         NOT NULL,
    request_hash   BYTEA        NOT NULL,
    status_code    INTEGER,
    response_body  BYTEA,
    response_headers JSONB,
    in_flight      BOOLEAN      NOT NULL DEFAULT TRUE,
    created_at     TIMESTAMPTZ  NOT NULL DEFAULT now(),
    completed_at   TIMESTAMPTZ,
    expires_at     TIMESTAMPTZ  NOT NULL DEFAULT now() + INTERVAL '24 hours',
    PRIMARY KEY (user_id, key),
    CONSTRAINT idempotency_keys_key_len CHECK (char_length(key) BETWEEN 1 AND 255)
);

CREATE INDEX IF NOT EXISTS idempotency_keys_expires_at_idx
    ON idempotency_keys (expires_at);

CREATE INDEX IF NOT EXISTS idempotency_keys_in_flight_idx
    ON idempotency_keys (in_flight)
    WHERE in_flight = TRUE;

COMMENT ON TABLE idempotency_keys IS
    'ADM-15: per-actor Idempotency-Key replay cache for admin POST mutations.';
