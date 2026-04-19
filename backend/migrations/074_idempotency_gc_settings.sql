-- ADM-20: tunable settings for the idempotency-key garbage collector.
--
-- The GC worker (`services::idempotency_gc::run_loop`) reads these
-- keys at the top of every iteration so operators can adjust the
-- batch size without a redeploy. Both keys are int-typed and live in
-- the `system` category alongside the audit-retention knobs.

INSERT INTO app_settings (key, value, value_type, is_secret, description, category, updated_at)
VALUES (
    'idempotency.gc_batch_size',
    to_jsonb(1000),
    'int',
    FALSE,
    'Maximum idempotency_keys rows pruned per GC iteration. Larger values reduce loop overhead but extend the lock window.',
    'system',
    NOW()
)
ON CONFLICT (key) DO NOTHING;

-- Note: the GC interval is a process-level env var
-- (`IDEMPOTENCY_GC_INTERVAL_SECS`) rather than a setting because
-- changing it should require an explicit redeploy — runaway loops
-- would otherwise be one mis-typed UPDATE away.

COMMENT ON COLUMN app_settings.value IS
    'JSONB-encoded value. Type-validated against `value_type` at write time.';
