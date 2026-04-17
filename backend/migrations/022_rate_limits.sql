-- FDN-08: Distributed rate-limit buckets.
--
-- Backs the `Postgres` variant of the `RateLimitBackend` abstraction in
-- `backend/src/middleware/rate_limit.rs`. Each row is a 1-second sliding-window
-- counter keyed by `(key, window_start)`. Requests increment the bucket for the
-- current second; effective rate is the sum over the last N seconds.
--
-- `window_start` is the second-truncated request timestamp (UTC). Periodic
-- cleanup can `DELETE ... WHERE window_start < now() - interval '5 minutes'`;
-- the `window_start_idx` index keeps both the sum and the cleanup scans cheap.

CREATE TABLE IF NOT EXISTS rate_limit_buckets (
    key          TEXT NOT NULL,
    window_start TIMESTAMPTZ NOT NULL,
    count        INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (key, window_start)
);

CREATE INDEX IF NOT EXISTS rate_limit_buckets_window_start_idx
    ON rate_limit_buckets (window_start);
