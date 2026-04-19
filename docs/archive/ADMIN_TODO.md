# Admin Backoffice — Outstanding Work  [ARCHIVED]

> **Last revised:** 2026-04-19
> **Status:** ARCHIVED — every line item below has shipped. This file is
> kept solely for traceability of the admin-hardening track. Do not
> add new items here — open a GitHub issue, and update
> [`../RUNBOOK.md`](../RUNBOOK.md) if it introduces a new alert path.
>
> **Completion summary (verified 2026-04-19):**
> * P0/#1 DSAR artefact TTL sweep — `backend/src/services/dsar_artifact_sweep.rs`
> * P0/#2 Idempotency-Key cache GC — `backend/src/services/idempotency_gc.rs`
> * P1/#3 DSAR async UI + artefact streamer — `src/routes/admin/dsar/+page.svelte`
> * P1/#4 Frontend SDK regen — `src/lib/api/schema.d.ts` (auto-generated; CI guards drift)
> * P2/#5 DSAR worker R2 path coverage — `backend/tests/dsar_r2_artifact.rs`
> * P2/#6 Idempotency race-condition test — `concurrent_same_key_creates_exactly_one_resource`
> * P2/#7 Playwright admin specs — `e2e/admin/admin-uis.spec.ts`
> * P2/#8 Grafana + Prometheus — `ops/grafana/admin-overview.dashboard.json` + `ops/prometheus/admin-alerts.rules.yml`
> * P2/#9 Operator runbook — [`../RUNBOOK.md`](../RUNBOOK.md)
> * P3/#10 Admin nav for observability — partial; tracked in GH issues if extended
> * P3/#11 CHANGELOG — superseded by Conventional Commits + `git log`
>
> Original tracker text follows for historical reference.

---

**Status as of 2026-04-19** — the enterprise-hardening backend track is shipped (commit `717a7ae`). The items below are residual frontend, ops glue, and coverage gaps. Each entry is sized so it can be picked up independently.

Priority legend:
- **P0** — silent prod problem (data loss, leaks, growing-forever tables).
- **P1** — operator can't actually use the new feature without it.
- **P2** — quality / coverage / observability that a Google L7 review would flag.
- **P3** — nice-to-have polish.

---

## P0 — silent prod problems

### 1. DSAR artefact TTL sweep
**Owner area:** `backend/src/services/`
**Why:** `dsar_jobs.artifact_expires_at` and the `dsar_jobs_artifact_expiry_idx` partial index were added in migration `073`, but no worker actually deletes the underlying R2 object or local file when the TTL elapses. Today a presigned URL stops working but the **bytes linger forever** — that's an ROI of zero on the TTL design.

**To do:**
- New module `backend/src/services/dsar_artifact_sweep.rs`:
  - `prune_once(pool, media_backend)` — selects rows where `artifact_expires_at < now()` AND `artifact_storage_key IS NOT NULL`, calls `media_backend.delete(key)` for R2 / `tokio::fs::remove_file` for local, then `UPDATE dsar_jobs SET artifact_url = NULL, artifact_storage_key = NULL, artifact_kind = NULL, artifact_expires_at = NULL WHERE id = $1`.
  - `run_loop(pool, media_backend, shutdown, interval)` — same shape as `audit_retention::run_loop`. Default interval `3600s`, env override `DSAR_SWEEP_INTERVAL_SECS`.
  - Metrics: `dsar_artifacts_swept_total`, `dsar_artifacts_sweep_failed_total`, `dsar_artifacts_sweep_duration_seconds`.
- Add `delete_object` to `MediaBackend` (already exists on `R2Storage`; add the `Local` arm with `tokio::fs::remove_file` ignoring `NotFound`).
- Wire `tokio::spawn(dsar_artifact_sweep::run_loop(...))` and graceful shutdown into `main.rs` next to the other workers.
- Integration test `tests/dsar_artifact_sweep.rs`: backdate `artifact_expires_at`, write a fixture file, run `prune_once`, assert file gone + columns nulled.

**Definition of done:** test passes; production logs show `dsar-artifact-sweep worker started` on boot; metric `dsar_artifacts_swept_total` is non-zero after 24h.

---

### 2. Idempotency-Key cache GC
**Owner area:** `backend/src/services/`
**Why:** `idempotency_keys.expires_at` is set to `now() + 24h` on every claim, and the `idempotency_keys_expires_at_idx` index exists, but nothing prunes. Every admin POST writes a row → table grows unbounded.

**To do:**
- New module `backend/src/services/idempotency_gc.rs`:
  - `prune_once(pool, batch_size)` — `DELETE FROM idempotency_keys WHERE expires_at < now() AND ctid = ANY (ARRAY(SELECT ctid FROM idempotency_keys WHERE expires_at < now() LIMIT $1 FOR UPDATE SKIP LOCKED))`. Loop until fewer than `batch_size` deleted.
  - `run_loop(pool, shutdown, interval)` — default `900s`, env `IDEMPOTENCY_GC_INTERVAL_SECS`. Reuse `app_settings` keys `idempotency.gc_batch_size` (default 1000) so operators can tune without a redeploy.
- Migration `074_idempotency_gc_settings.sql` to seed `idempotency.gc_batch_size`.
- Metrics: `idempotency_keys_pruned_total`, `idempotency_keys_prune_duration_seconds`, `idempotency_keys_table_rows` (gauge, sampled per cycle).
- Wire into `main.rs` and graceful shutdown.
- Integration test: insert an expired row + a fresh row, `prune_once`, assert only the expired one is gone.

**Definition of done:** integration test passes; `idempotency_keys` row count plateaus in staging.

---

## P1 — operator can't use the new feature

### 3. DSAR UI — async export + artefact streamer
**File:** `src/routes/admin/dsar/+page.svelte`, `src/lib/api/admin-security.ts`

**Why:** backend supports `POST /jobs/export {async: true}` returning 202, the `pending`/`composing` statuses, and `GET /jobs/{id}/artifact` for local-mode streaming. None of that is wired in the UI, so operators can't trigger or consume it.

**To do — API client (`admin-security.ts`):**
- Extend `DsarExportRequest` with `async?: boolean`.
- Make `DsarExportResponse.export: unknown | null` (worker leaves it null on the 202 path).
- Add status union members: `'pending' | 'composing' | 'completed' | 'cancelled' | 'failed'`.
- New helper `streamArtifact(id: string)` returning a Blob (uses `api.getRaw(...)` if it exists, otherwise raw `fetch` with the bearer token).

**To do — page (`dsar/+page.svelte`):**
- Add an `<input type="checkbox" bind:checked={asyncMode} />` next to the compose form. Default false, sticky in `localStorage`.
- When `asyncMode` is true, change submit copy to `Queue export (async)` and on 202 show `Queued — refresh in a few seconds.` instead of the inline preview.
- Status filter `<select>`: add `composing` option.
- Status badge: add `badge--info` for `pending` / `composing`.
- Row action: when `job.status === 'completed' && job.kind === 'export'`, render a `Download artefact` button. If `artifact_url` starts with `/api/admin/dsar/jobs/`, treat it as the streamer route and call `streamArtifact()`; otherwise (R2 presigned URL) keep the existing `<a download>` flow.
- Show TTL remaining when `artifact_expires_at` is present (e.g. `Expires in 4h`).
- Inspector drawer: surface `artifact_kind` and `artifact_expires_at`.

**Definition of done:** operator can submit async, see the row land as `pending`, watch it transition to `completed`, and download the artefact — all without opening DevTools.

---

### 4. Regenerate the frontend TS SDK
**File:** `src/lib/api/schema.d.ts` (auto-generated)

**Why:** backend OpenAPI now includes `admin_dsar_stream_artifact` and the `async` field on `ExportRequest`, but `schema.d.ts` hasn't been regenerated, so any code path consuming the typed SDK is stale.

**To do:**
- Find the regen command (likely `npm run gen:openapi` or `npx openapi-typescript`). Check `package.json` scripts.
- Run it against `backend/tests/snapshots/openapi.json` (the snapshot is the source of truth; the live spec drifts during dev).
- Commit the regenerated `schema.d.ts`.
- If the codebase uses the typed client elsewhere (search for `paths['/api/admin/dsar`), audit those call sites.

**Definition of done:** `npm run check` passes with no missing-property errors on the new endpoints.

---

## P2 — quality, coverage, observability

### 5. DSAR worker R2 path coverage
**File:** new `backend/tests/dsar_async_export_r2.rs`

**Why:** the 6 existing async-export tests only exercise `MediaBackend::Local`. The R2 branch (`upload` + `presign_get`) has only the `is_r2()` smoke check — a regression in the S3 SDK upgrade or in our presign wrapper would slip through CI.

**Approach (pick one):**
- **LocalStack via testcontainers-rs** — spin up an `s3` container in a `#[tokio::test(flavor = "multi_thread")]` fixture, point an `R2Storage` at it, run the worker, verify the object lands and the presigned URL is reachable.
- **Mock at the `R2Storage` boundary** — extract a `trait ObjectStore` and inject a fake. Lower fidelity but no Docker dependency in CI.

Recommendation: testcontainers — the rest of the repo already touches Postgres via testcontainers patterns.

**Definition of done:** `cargo test --test dsar_async_export_r2 -- --ignored` passes locally with Docker; gated `--ignored` so plain CI still runs without Docker.

---

### 6. Idempotency in-flight race-condition test
**File:** extend `backend/tests/admin_idempotency.rs`

**Why:** `admin_idempotency.rs` covers replay, mismatch, and missing-header — but not the **concurrent** branch where two requests with the same key hit the middleware in parallel. The `ON CONFLICT DO NOTHING` claim makes this safe in theory; the test would prove it.

**Approach:** `tokio::join!(post(...), post(...))` with the same `Idempotency-Key`, assert one returns 2xx + one returns `409 Conflict` (in-flight) or both return identical 2xx (depending on which finishes first). Repeat 10x to flake-shake.

**Definition of done:** test passes 100/100 runs locally.

---

### 7. Playwright E2E for the new admin UIs
**Folder:** `e2e/admin/`

**Why:** Security console has an E2E spec. Audit, DSAR, Orders, Subscriptions, Members (modernised), Settings, Subscriptions-manual do not. A future refactor that breaks the dashboard nav will go undetected.

**To do — one spec per surface, each ~5 assertions:**
- `audit.spec.ts` — login as admin, hit `/admin/audit`, type into the search box, assert at least one row, assert CSV export link.
- `dsar.spec.ts` — open page, fill the export form, assert toast.
- `orders.spec.ts` — list loads, filter by status, open detail drawer, assert refund button gated by status.
- `subscriptions-manual.spec.ts` — search by email, open a sub, assert comp/extend buttons render.
- `members-manage.spec.ts` — search returns rows, open create form, assert validation on empty submit.
- `settings.spec.ts` — list loads, edit one string setting, save, assert toast.

Reuse the `seedAdmin` fixture from `e2e/admin/security.spec.ts`.

**Definition of done:** `npx playwright test e2e/admin` is green; CI runs them.

---

### 8. Grafana / alerting rules
**Folder:** new `ops/grafana/` and `ops/prometheus/alerts/`

**Why:** the new metrics (`admin_mutation_rate_limited_total`, `audit_pruned_total`, `dsar_export_failed_total`, `dsar_export_duration_seconds`, `idempotency_*`, `dsar_artifacts_swept_total`) are emitted but nothing alerts on them. SRE has no dashboard for the admin plane.

**To do — alerts (`ops/prometheus/alerts/admin.yaml`):**
- `AdminMutationRateLimitSurge` — `rate(admin_mutation_rate_limited_total[5m]) > 0.5` for 10m → warn (one operator hammering or a script gone wild).
- `DsarExportFailing` — `increase(dsar_export_failed_total[15m]) > 3` → page (artefact build is broken).
- `DsarExportLatencyP95` — `histogram_quantile(0.95, rate(dsar_export_duration_seconds_bucket[10m])) > 60` → warn.
- `AuditRetentionStalled` — `time() - audit_prune_last_success_unixtime > 86400` → warn (worker died).
- `IdempotencyTableUnbounded` — `idempotency_keys_table_rows > 1_000_000` → warn (GC #2 not running).

**To do — dashboard (`ops/grafana/admin-plane.json`):**
- Panel: mutation rate-limit hits per actor (top 10).
- Panel: DSAR job state distribution (pending / composing / completed / failed) over 24h.
- Panel: DSAR export latency histogram.
- Panel: Audit-log pruning throughput.
- Panel: Idempotency cache rows + GC throughput.
- Panel: Impersonation active session count + mints/revokes.

**Definition of done:** dashboard renders in Grafana with our Prometheus datasource; alerts fire in staging when triggered manually.

---

### 9. Operator runbook
**File:** new `docs/ADMIN_RUNBOOK.md`

**Why:** the new failure modes are non-obvious to anyone who didn't write them. A 3am page about `dsar_export_failed_total` should resolve in <10 minutes via the runbook.

**To do — sections:**
- **DSAR export stuck in `composing`** — likely worker died mid-flight. SQL to reset to `pending`. How to identify the offending row by `requested_by` + `created_at`.
- **DSAR export failing** — check `failure_reason` column. Common causes: target user vanished (race), R2 credentials rotated, disk full on local backend.
- **Audit retention not running** — check `audit_prune_last_success_unixtime`. SQL to manually trigger one prune. How to bump `audit.retention_days` for a temporary legal-hold.
- **Mutation rate-limit storm** — how to identify the actor (`admin_mutation_rate_limited_total` is unlabeled by actor for cardinality reasons; check tracing logs for `actor_key`). How to temporarily widen the policy.
- **Idempotency table runaway growth** — confirm GC worker is alive; manual prune SQL; raise `idempotency.gc_batch_size`.
- **IP allowlist locked everyone out** — emergency superuser bypass via `ADMIN_IP_ALLOWLIST_BYPASS=1` env (verify this exists; if not, add it and document).
- **Impersonation session won't end** — manual revoke SQL; how to scrub the JWT from the user's browser.

**Definition of done:** an SRE who has never seen the codebase can resolve each scenario from the runbook alone.

---

## P3 — polish

### 10. Admin nav surface for observability
**File:** `src/lib/components/admin/admin-nav.ts`, new `/admin/system/*` pages

**Why:** all the new infra (idempotency cache, rate-limit metrics, DSAR worker queue depth, audit-retention status) is invisible from the UI. SRE has Prometheus; operators don't.

**To do (small):**
- `/admin/system/workers` — shows last-success time for each background worker (audit retention, DSAR worker, DSAR sweep, idempotency GC) by reading their `*_last_success_unixtime` gauges via a new `GET /api/admin/system/workers` endpoint.
- `/admin/system/idempotency` — shows recent rows with method/path/status, lets you inspect a key's cached response. Read-only.
- `/admin/system/queue` — shows DSAR jobs in `pending` / `composing` with `Retry` action that resets a stuck row.

Lower priority because Grafana covers the SRE side; this is purely operator UX.

---

### 11. CHANGELOG
**File:** `CHANGELOG.md` (create if missing)

Entry for the enterprise-hardening release covering ADM-15 through ADM-18 and all the UI work, with links to the relevant migrations. Following Keep-a-Changelog format.

---

## Ordering recommendation

1. **#1 + #2** (P0) — they're leaking storage / growing tables right now.
2. **#3** (P1) — without it the async DSAR work is shelfware.
3. **#4** (P1) — unblocks #3 if we want it strongly typed.
4. **#7** (P2) — protects the work going forward.
5. **#9** (P2) — lets ops actually own the system.
6. **#5, #6, #8** (P2) in any order.
7. **#10, #11** (P3) when there's slack.

Total realistic cost: **~3-4 engineer days** for P0+P1, another **2-3 days** for P2.
