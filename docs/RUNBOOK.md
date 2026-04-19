# Operator runbook — swings admin platform

> **Last revised**: 2026-04-19
> **Audience**: on-call SRE / platform engineer holding the pager.
> **Companion files**: alerts in [`ops/prometheus/admin-alerts.rules.yml`](../ops/prometheus/admin-alerts.rules.yml),
> dashboards in [`ops/grafana/admin-overview.dashboard.json`](../ops/grafana/admin-overview.dashboard.json),
> closed admin scope ledger in [`docs/archive/ADMIN_TODO.md`](./archive/ADMIN_TODO.md).

Every alert in `admin-alerts.rules.yml` carries a `runbook_url`
annotation that anchors into this file. Section headers MUST stay in
sync with those anchors — if you rename a section, update the rule.

---

## Conventions

- **Severity ladder**:
  - `critical` → page immediately, fix this hour.
  - `warning` → page during business hours, fix this day.
  - `info` → notify on Slack, no pager.
- **First step on every alert**: check the
  [Admin Platform Overview](https://grafana/d/swings-admin-overview)
  dashboard for the matching panel before SSHing anywhere.
- **kubectl prefixes** assume the prod context; substitute as needed.
  Replace `swings-api` with the actual deployment name in your env.
- Database surgery instructions assume `psql` access via the
  `swings_admin` role to the prod replica. **Do not run write
  queries against prod without a second pair of eyes.**

---

## DSAR — async export pipeline

### `DsarExportFailureSpike`

The async DSAR worker is failing one or more jobs every ~3 minutes.

1. Look at the panel **DSAR · Exports failed (last 1h)** + the
   `dsar_export_failed_total{stage}` series. The `stage` label is
   either:
   - `claim` — the worker can't even SELECT-FOR-UPDATE pending rows.
     Almost always a Postgres connection issue (pool exhausted,
     network blip). Check the pool gauge / DB CPU.
   - `process` — claim succeeded but compose / upload / DB-update
     failed. This is where the real failures live.
2. Inspect the offending rows:
   ```sql
   SELECT id, status, failure_reason, artifact_kind, target_user_id
     FROM dsar_jobs
    WHERE status = 'failed'
      AND finished_at > now() - interval '1 hour'
    ORDER BY finished_at DESC
    LIMIT 50;
   ```
3. Common `failure_reason` patterns:
   - `r2:` prefix → R2 transport issue. Verify
     `R2_ACCESS_KEY_ID/R2_SECRET_ACCESS_KEY` haven't been rotated
     out of band; verify the bucket policy still allows
     `s3:PutObject`.
   - `compose:` prefix → builder threw on a target user with
     malformed data. Spot-fix the row in `dsar_jobs.failure_reason`
     and `target_user_id`, then re-queue with `status='pending'`.
   - `db:` prefix → Postgres rejected the final UPDATE. Almost
     always a serialization conflict; safe to re-queue.
4. Re-queue a failed job:
   ```sql
   UPDATE dsar_jobs
      SET status='pending', failure_reason=NULL, started_at=NULL, finished_at=NULL
    WHERE id = $1;
   ```

### `DsarExportBacklogGrowing`

Claims > completions for 30+ min. Either the worker is stuck on a
single slow job, or the compose path is genuinely overloaded.

1. **Look at slowest in-flight jobs**:
   ```sql
   SELECT id, target_user_id, started_at,
          (now() - started_at) AS age, artifact_kind
     FROM dsar_jobs
    WHERE status = 'composing'
    ORDER BY started_at ASC
    LIMIT 20;
   ```
2. If the oldest is more than ~10 min old → the worker is wedged
   on it. Worker pod restart is the right call:
   ```bash
   kubectl rollout restart deploy/swings-api
   ```
   The next worker tick will requeue any rows whose `started_at` is
   older than the visibility timeout (handled in `dsar_worker.rs`).
3. If it's a recurring pattern with the *same* target user — that
   user has unusually large data; consider sharding the export or
   bumping the worker's per-job timeout.

### `DsarArtifactSweepStuck`

`time() - dsar_artifacts_sweep_last_success_unixtime > 7200`. **The
sweep worker has not completed a tick in 2+ hours.** Bytes are
leaking.

1. Check pod logs for the artifact sweep loop:
   ```bash
   kubectl logs -l app=swings-api --tail=500 | rg dsar_artifact_sweep
   ```
2. If the worker thread panicked, the *whole binary* may have
   restarted but only the sweep loop is wedged. Restart the pod:
   ```bash
   kubectl rollout restart deploy/swings-api
   ```
3. If logs show repeated R2 deletion errors → see
   `DsarArtifactSweepFailureSpike`.
4. Manual one-shot prune (psql, **with care**):
   ```sql
   -- See what would be swept this tick:
   SELECT id, artifact_storage_key, artifact_expires_at
     FROM dsar_jobs
    WHERE artifact_expires_at < now()
      AND artifact_storage_key IS NOT NULL
    LIMIT 50;
   ```
   The actual deletion is performed by the worker — do not delete
   from R2 by hand unless you also NULL out the row, otherwise the
   sweep will keep retrying a missing object (which is benign but
   noisy).

### `DsarArtifactSweepFailureSpike`

R2/local deletes failing > 0.01/sec. Most likely cause: rotated R2
credentials.

1. Verify the secret in the namespace matches the R2 console:
   ```bash
   kubectl get secret swings-r2 -o json | jq '.data | map_values(@base64d)'
   ```
2. If the credentials are right, check the bucket lifecycle policy
   for an external "block deletes" rule.
3. If a single key is failing repeatedly, it's stuck in the queue.
   Find it (`stage=delete` in the failure counter), and clear it
   manually:
   ```sql
   UPDATE dsar_jobs
      SET artifact_storage_key=NULL, artifact_url=NULL, artifact_expires_at=NULL
    WHERE id = $1;
   ```

---

## Idempotency — middleware & GC

### `IdempotencyGcStuck`

`time() - idempotency_keys_prune_last_success_unixtime > 3600`.

1. Worker thread is wedged or panicking. First check logs:
   ```bash
   kubectl logs -l app=swings-api --tail=500 | rg idempotency_gc
   ```
2. Pod restart fixes a wedged loop:
   ```bash
   kubectl rollout restart deploy/swings-api
   ```
3. If the GC is alive but not making progress, batch size may be
   too small for the backlog. Bump it:
   ```sql
   UPDATE app_settings
      SET value = '2500'::jsonb
    WHERE key = 'idempotency.gc_batch_size';
   ```
   The worker reads this on every tick so the change takes effect
   immediately — no restart needed.

### `IdempotencyKeysTableTooLarge`

`max(idempotency_keys_table_rows) > 1_000_000`.

1. Check whether the GC is alive (see `IdempotencyGcStuck`). If it
   is healthy but the table is just genuinely large:
2. Confirm the `expires_at` distribution looks sane:
   ```sql
   SELECT
     date_trunc('hour', expires_at) AS bucket,
     count(*)
     FROM idempotency_keys
    WHERE expires_at < now() + interval '7 days'
    GROUP BY bucket
    ORDER BY bucket;
   ```
3. If a single client is mass-replaying with unique keys, find them:
   ```sql
   SELECT user_id, count(*) AS rows
     FROM idempotency_keys
    GROUP BY user_id
    ORDER BY rows DESC
    LIMIT 20;
   ```
   Coordinate with the integrating team — it's almost always a
   misconfigured retry loop on their side.
4. As a last resort, manually prune older rows (do **not** delete
   in-flight rows, they're load-bearing):
   ```sql
   DELETE FROM idempotency_keys
    WHERE expires_at < now() - interval '24 hours';
   ```

### `IdempotencyInFlightSurge`

`> 1` 409 in-flight responses per second sustained 10+ minutes.

1. Identify the source:
   ```sql
   SELECT user_id, key, count(*) AS conflicts
     FROM idempotency_keys
    WHERE created_at > now() - interval '10 minutes'
    GROUP BY user_id, key
    ORDER BY conflicts DESC
    LIMIT 20;
   ```
2. Most often: a webhook firing the same key from multiple sources,
   or a UI button that doesn't disable on click. Track the
   originating user/integration and ping the owner.
3. **Do not** raise the rate limit or shorten the in-flight TTL as
   a mitigation — the 409 is the *correct* response.

### `IdempotencyMismatchSpike`

`> 0.1/s` 422 mismatch responses. Either a client bug or probing.

1. The middleware logs the body hash; cross-reference logs by
   `user_id` to see whether it's one client or many.
2. If one client → almost certainly a logic bug; reach out to the
   integrating team.
3. If many clients with no obvious correlation → consider a
   targeted security review; this is a typical pattern in someone
   probing your idempotency contract.

---

## Audit retention

### `AuditPruneStuck`

`time() - audit_prune_last_success_unixtime > 86_400`. The pruner
hasn't completed a tick in a day.

1. Check logs:
   ```bash
   kubectl logs -l app=swings-api --tail=500 | rg audit_retention
   ```
2. Pod restart usually fixes it. Note the policy is read from
   `app_settings`:
   ```sql
   SELECT key, value
     FROM app_settings
    WHERE key IN ('audit.retention_days', 'audit.prune_batch_size');
   ```
3. If the table has grown large and the batch size is too small to
   make progress, bump it:
   ```sql
   UPDATE app_settings
      SET value = '20000'::jsonb
    WHERE key = 'audit.prune_batch_size';
   ```

### `AuditPruneErrorRate`

`> 0.05/s` errors from the pruner.

1. Almost always a Postgres lock contention issue — the pruner
   takes row locks during `DELETE`. Check
   `pg_stat_activity` for blockers:
   ```sql
   SELECT pid, state, wait_event, query
     FROM pg_stat_activity
    WHERE state != 'idle' AND query ILIKE '%admin_actions%';
   ```
2. The pruner retries automatically; persistent errors usually
   indicate a long-running report query holding a snapshot. Coordinate
   with the report owner or run pruner outside the report window.

---

## Admin RBAC & rate-limit

### `AdminMutationRateLimitSurge`

> 0.5/s 429s on admin mutation endpoints, sustained 10 min.

1. **First, decide if this is benign or hostile.** Check the audit
   log for the actor:
   ```sql
   SELECT actor_id, action, count(*)
     FROM admin_actions
    WHERE created_at > now() - interval '15 minutes'
    GROUP BY actor_id, action
    ORDER BY count DESC
    LIMIT 20;
   ```
2. **Hostile / compromised account**:
   - Revoke the actor's refresh tokens immediately:
     ```sql
     UPDATE refresh_tokens SET revoked_at = now()
      WHERE user_id = $1 AND revoked_at IS NULL;
     ```
   - Invalidate active impersonation sessions for the actor.
   - File an incident; rotate the user's credentials.
3. **Benign automation**: coordinate with the operator to throttle
   the script or use a service account with explicit higher quota.
   Do **not** raise the per-actor cap as a workaround.

---

## Impersonation

### `ImpersonationMintRateLimited`

Operators are minting impersonation sessions faster than policy.

1. Identify the actor and recent mints:
   ```sql
   SELECT actor_id, target_user_id, reason, created_at
     FROM impersonation_sessions
    WHERE created_at > now() - interval '15 minutes'
    ORDER BY created_at DESC;
   ```
2. If reason text suggests a real support fire (multiple distinct
   target users, plausible reason text) → this is benign; consider
   raising the per-actor cap temporarily.
3. If reason text is generic / repeated / on the same target →
   suspicious. Treat as a potential ATO; revoke sessions and
   credentials, file incident.

### `ImpersonationRevokeStorm`

Informational. Common during shift change or scripted cleanup.
Suppress with `silence` if it correlates with a known operation.

---

## Maintenance kill-switch

### `MaintenanceModeActive`

Maintenance mode is rejecting traffic.

1. **If intentional**: silence the alert for the duration of the
   window. The kill-switch is set in `app_settings`:
   ```sql
   SELECT value FROM app_settings WHERE key = 'maintenance.enabled';
   ```
2. **If unintentional**: turn it off immediately:
   ```sql
   UPDATE app_settings
      SET value = 'false'::jsonb
    WHERE key = 'maintenance.enabled';
   ```
   The middleware reads from the settings cache on every request,
   so the change takes effect within a few seconds (no restart).

---

## Common operations not tied to a specific alert

### Failed migrations

The backend will refuse to start on schema mismatch. Check:

```bash
kubectl logs -l app=swings-api --tail=200 | rg -i 'migrat|sqlx'
```

If a migration partially applied, manually inspect the `_sqlx_migrations`
table and the affected schema. **Never `DROP` migration history without
coordination with the platform team.**

Three failure modes have actually bitten production; each has a
distinct fix path.

#### A. Checksum mismatch (`migration N was previously applied but has been modified`)

Someone edited a migration file that is already in `_sqlx_migrations`
(usually a comment-only change that still flips the SHA-384). The
on-disk file is the new truth; we update the recorded checksum:

```sh
HASH=$(python3 -c "import hashlib,sys;print(hashlib.sha384(open(sys.argv[1],'rb').read()).hexdigest())" \
  backend/migrations/0NN_xxx.sql)

railway ssh --service Postgres -- psql -U swings -d swings <<SQL
BEGIN;
UPDATE _sqlx_migrations SET checksum = decode('${HASH}', 'hex')
 WHERE version = NN;
COMMIT;
SQL
```

#### B. PostgreSQL `unsafe use of new value "<x>" of enum type`

`ALTER TYPE … ADD VALUE` cannot be combined with statements that
*reference* the new value in the same transaction. sqlx's
`-- no-transaction` marker only suppresses sqlx's wrapping
transaction; the simple-query protocol still wraps a multi-statement
Query in an implicit transaction.

Fix in the migration itself: add an explicit `COMMIT;` after the
`ALTER TYPE` block, *before* the statements that reference the new
values. See `backend/migrations/021_rbac.sql` for the canonical
shape.

#### C. `relation "<x>" does not exist` (ordering bug)

A migration references a table that is created in a *later*
migration. sqlx applies in numeric order, so this means the file
numbers are wrong. Fixes:

1. Confirm the broken migration has *never* successfully applied
   anywhere (`SELECT version FROM _sqlx_migrations` on every
   environment). If it never applied, `git mv` the dependency to a
   lower free version number — no checksum drift, no destructive
   change.
2. If the broken migration *did* apply somewhere via manual
   surgery, the dependency table needs a small forward migration in
   the next free slot rather than renumbering.
3. Verify locally with a fresh DB before pushing:
   ```sh
   psql -d postgres -c 'DROP DATABASE IF EXISTS swings_migration_test;
                        CREATE DATABASE swings_migration_test;'
   for f in backend/migrations/*.sql; do
     psql -d swings_migration_test -v ON_ERROR_STOP=1 -X -q -f "$f" || break
   done
   ```

The same single-shot replay is the gate before any migration PR
merges; CI does not yet do this end-to-end (tracked).

### Settings cache appears stale

The settings cache reloads on every-call invalidation. If you've
made a manual `UPDATE app_settings` and the change isn't taking
effect, force a reload:

- For a single instance: restart the pod.
- For all instances: bump any setting (e.g. `idempotency.gc_batch_size`)
  by ±1 and back; this triggers a publish to the cache.

### How to find the right pod

```bash
kubectl get pods -l app=swings-api
kubectl logs -l app=swings-api --since=10m | head -200
```

For traces, check the active OTLP endpoint configured for the env;
the trace IDs appear in every error log line.
