# AGENTS.md — guidance for AI coding agents working on `swings`

> **Last revised:** 2026-04-19
> **Audience:** any AI coding tool reading this file (Cursor, Codex,
> Claude Code, Copilot, Aider, Continue, Windsurf, …) and the human
> reviewers who pair with them.
> **Status:** active — single source of truth for agent behaviour in
> this repository. `CLAUDE.md` is a thin alias to this file; do not
> add agent guidance anywhere else.

---

## 1. What this repository is

`swings` is a production full-stack membership and content platform.

| Surface  | Stack                                                          | Hosted on  |
| -------- | -------------------------------------------------------------- | ---------- |
| Frontend | SvelteKit · Svelte 5 (runes) · scoped CSS (no Tailwind)        | Vercel     |
| Backend  | Rust (2021 edition) · Axum · `sqlx` · Tokio                    | Railway    |
| Database | PostgreSQL 16 · forward-only `sqlx` migrations                 | Railway    |
| Storage  | Cloudflare R2 (S3-compatible) + local fallback                 | Cloudflare |
| CI       | GitHub Actions (`ci`, `sql-lint`, `openapi-drift`, `security`) | GitHub     |

Production endpoints:

- Frontend — `https://swings-gamma.vercel.app`
- Backend — `https://swings-production.up.railway.app`

The repository is a single pnpm workspace (`package.json` at root)
with a Cargo crate at `backend/`. There is **no** separate frontend
package — the SvelteKit app lives at the repo root.

---

## 2. Layout you must learn before editing

```
src/                  SvelteKit app (routes, lib, components, stores)
static/               Static frontend assets
messages/             Inlang i18n catalogues
e2e/                  Playwright specs
backend/
  src/
    handlers/         HTTP handlers — admin, member, public, webhooks
    services/         Background workers + cross-cutting services
    middleware/       Tower layers (idempotency, rate-limit, IP allowlist)
    security/         Impersonation + IP allowlist primitives
    observability/    Tracing + metrics scaffolding
    commerce/ consent/ popups/ forms/ notifications/ pdf/   Domain modules
  migrations/         sqlx forward-only migrations (versions 001–074)
  tests/              Integration tests against a real Postgres
ops/                  Prometheus rules + Grafana dashboard
docs/                 All long-form documentation (see docs/README.md)
scripts/              Repo automation (audit dump, OpenAPI→TS, SEO check)
.github/workflows/    CI definitions
```

The complete documentation index lives in [`docs/README.md`](./docs/README.md).
Read it before adding a new doc — there is a place for everything.

---

## 3. Hard rules (do not violate)

These are the rules that have caused production incidents in this
repository. Each one is enforced by tests, CI, or runtime guards;
violating them will fail review or break prod.

1. **Migrations are forward-only.** Once a `0NN_*.sql` file has been
   applied to _any_ environment, **never** edit its contents. sqlx
   stores a SHA-384 of the file in `_sqlx_migrations.checksum` and
   refuses to boot if it changes. Add a new migration instead.
2. **No duplicate migration version prefixes.** `_sqlx_migrations.version`
   is a primary key. Two files with the same `0NN_` prefix means the
   second one fails to insert at boot. Pick the next free integer.
3. **Migration ordering matters.** A migration that does
   `REFERENCES other_table(id)` must come _after_ the one that creates
   `other_table`. CI runs the full migration set against an empty DB,
   but check ordering by hand when adding cross-domain DDL.
4. **Audit every admin mutation.** Admin handlers must call
   `services::audit::record(...)` so the row lands in `admin_actions`.
   No exceptions for "trivial" actions.
5. **Permission-check every admin mutation.** Admin handlers must call
   `policy.require(ctx, "<perm>")` _before_ doing the work. The full
   permission matrix is seeded by migration `021_rbac.sql`. Add new
   permissions via a new migration that inserts into `permissions` and
   `role_permissions`.
6. **`Idempotency-Key` is required on admin POSTs.** The middleware in
   `backend/src/middleware/idempotency.rs` enforces this; do not bypass
   it for new admin handlers.
7. **No `unwrap()`, no `expect()`, no `panic!()` in non-test code.**
   Use `?` with `AppError`. Tests may use `unwrap()` for clarity.
8. **No `any` in TypeScript.** `tsconfig.json` enables strict mode and
   `noImplicitAny`. Use `unknown` and narrow.
9. **Svelte 5 runes only.** `$state`, `$derived`, `$effect`. Do **not**
   write `let foo = $state(...)` and then mutate `foo.bar` if `foo`
   isn't a reactive primitive — use `$state` on the bag.
10. **Never commit secrets.** `.env*` is gitignored; use Railway / Vercel
    env vars in production. See [`SECURITY.md`](./SECURITY.md).

---

## 4. Common commands

| Goal                              | Command                                                                        |
| --------------------------------- | ------------------------------------------------------------------------------ |
| Install everything                | `pnpm install`                                                                 |
| Frontend dev server               | `pnpm dev`                                                                     |
| Backend dev server                | `pnpm dev:api` _(runs `cargo run` inside `backend/`)_                          |
| Both, side by side                | `pnpm dev:all`                                                                 |
| Frontend type-check               | `pnpm check`                                                                   |
| Frontend lint                     | `pnpm lint`                                                                    |
| Frontend unit tests               | `pnpm test:unit`                                                               |
| Frontend browser tests            | `pnpm test:browser`                                                            |
| Frontend E2E                      | `pnpm test:e2e`                                                                |
| Backend format (CI parity)        | `cargo fmt --manifest-path backend/Cargo.toml --all -- --check`                |
| Backend lint (CI parity)          | `cargo clippy --manifest-path backend/Cargo.toml --all-targets -- -D warnings` |
| Backend tests (needs Postgres)    | `cd backend && cargo test`                                                     |
| Whole-repo CI parity              | `pnpm ci:all`                                                                  |
| Regenerate frontend OpenAPI types | `pnpm gen:types`                                                               |
| Lint SQL migrations               | `sqlfluff lint backend/migrations --dialect postgres` (pin `3.2.5`)            |

The pre-commit hook runs `pnpm lint && pnpm test:unit -- --run`. Do
not bypass it without a documented reason in the commit message.

---

## 5. Migrations

- Files live in `backend/migrations/` named `0NN_short_description.sql`.
- Versions in use today: `001–028, 030–039, 041–043, 050–080` (gaps are fine
  but every prefix must be unique).
- Two safe defaults when authoring a new migration:
  - Use `IF NOT EXISTS` / `IF EXISTS` and `ON CONFLICT DO NOTHING` so a
    re-run after a partial failure is idempotent.
  - If you need to `ALTER TYPE ... ADD VALUE` and then immediately use
    the new variant, end the section with `COMMIT;` — Postgres
    requires it before the new enum label is visible to subsequent
    statements in the same simple-query transaction. See
    `021_rbac.sql` for the canonical example.
- Verify a fresh DB before pushing:
  ```bash
  psql -d postgres -c 'CREATE DATABASE swings_check;'
  for f in $(ls backend/migrations/*.sql | sort); do
    psql -d swings_check -v ON_ERROR_STOP=1 -f "$f"
  done
  psql -d postgres -c 'DROP DATABASE swings_check;'
  ```

---

## 6. Authentication & authorisation model

- Sessions are JWT bearer tokens; refresh-token rotation is in
  `001_initial.sql` + `018_refresh_token_families.sql`.
- Admin extractor: `backend/src/extractors/admin.rs` (`AdminUser`).
- Permission check: `policy.require(ctx, "admin.<resource>.<verb>")`.
- The matrix is seeded by `021_rbac.sql` and extended by the
  `0NN_*_perms.sql` family of migrations.
- Impersonation: `backend/src/security/impersonation.rs` + `060_*` and
  `061_*` migrations. Always emits a paired `admin_actions` row.
- IP allowlist: `backend/src/security/ip_allowlist.rs` + `059_*`. The
  middleware fails open if the table is empty and fails closed
  otherwise.

---

## 7. Background workers

Spawned from `backend/src/main.rs` and tracked by Prometheus gauges.
All of them have a graceful-shutdown path bound to the `tokio::signal`
ctrl-c handler.

| Worker                  | File                              | Default interval |
| ----------------------- | --------------------------------- | ---------------- |
| Outbox event dispatcher | `events/worker.rs`                | continuous       |
| Audit log retention     | `services/audit_retention.rs`     | `3600s`          |
| DSAR export             | `services/dsar_worker.rs`         | `30s`            |
| DSAR artefact TTL sweep | `services/dsar_artifact_sweep.rs` | `3600s`          |
| Idempotency-Key GC      | `services/idempotency_gc.rs`      | `300s`           |

Every worker emits `*_last_success_unixtime` so the runbook can
detect a stalled loop. Add the same shape when introducing a new one.

---

## 8. Observability contract

- Metrics endpoint — `/metrics` (admin-gated in production, public in
  dev). Wired in `observability/handler.rs`.
- Prometheus rules — `ops/prometheus/admin-alerts.rules.yml`. Every
  alert carries `severity`, `team`, `domain`, and `runbook_url`
  pointing at a section in [`docs/RUNBOOK.md`](./docs/RUNBOOK.md).
- Grafana dashboard — `ops/grafana/admin-overview.dashboard.json`.
- When you add a new worker or middleware, add the metric _and_ the
  alert _and_ the runbook section _in the same change_. CI does not
  enforce this yet but reviewers will reject otherwise.

---

## 9. Testing expectations

- **Unit tests (Rust)** — colocated with the module under `#[cfg(test)]`.
- **Integration tests (Rust)** — in `backend/tests/`, use the harness in
  `backend/tests/support/`. They bring up an isolated Postgres on `:5433`
  via `backend/docker-compose.yml` (see
  [`docs/wiring/FDN-TESTHARNESS-WIRING.md`](./docs/wiring/FDN-TESTHARNESS-WIRING.md)).
- **Frontend unit tests** — Vitest, in `*.test.ts` next to the unit.
- **E2E** — Playwright, in `e2e/`. Run with `pnpm test:e2e`.
- **Snapshot tests** — `backend/tests/snapshots/openapi.json` is the
  source of truth for the frontend SDK. Regenerate with
  `pnpm gen:types`.
- **Race-condition coverage** — when shipping anything that uses
  `Idempotency-Key`, copy the `concurrent_same_key_creates_exactly_one_resource`
  pattern from `tests/admin_idempotency.rs` (Barrier + parallel join).
- **Never mutate `std::env` in tests.** `cargo test` runs in parallel,
  `setenv(3)` / `getenv(3)` are not thread-safe on POSIX, and Rust 2024
  marks `std::env::set_var` as `unsafe fn` for exactly this reason.
  Instead, refactor the module under test to take its secret or config
  value as an argument. The canonical example is
  `backend/src/forms/integration_config.rs`: production calls
  `SealedCredential::seal` / `.unseal()` which resolve a
  `OnceLock<Cipher>` from env exactly once at startup; tests construct
  an explicit `Cipher` with deterministic bytes and call
  `seal_with` / `unseal_with`. No test ever touches env, no `Mutex`, no
  `serial_test` dependency. Apply the same pattern to any new module
  that needs config-injection for tests.
- **Ignored tests are forbidden.** `#[ignore]` hides dormant assertions
  and lets regressions land silently — if a test needs Postgres or an
  external service, move it to `backend/tests/` where the harness
  provides it (see the Postgres-backed rate-limit coverage in
  `tests/rate_limit_postgres.rs` for the pattern). `cargo test --lib`
  should print `0 ignored` on every run.

---

## 10. Deployment

| Surface  | Platform | Build config                                          |
| -------- | -------- | ----------------------------------------------------- |
| Frontend | Vercel   | `vercel.json` + `svelte.config.js` (`adapter-vercel`) |
| Backend  | Railway  | root `Dockerfile` (build context = repo root)         |
| Backend  | Render   | `render.yaml` → root `Dockerfile`                     |

There is exactly **one** Dockerfile (at the repo root). All three
deploy targets — Railway, Render, local `docker-compose.yml` — build
from the repo root using that single file. Harden in one place. The
root `.dockerignore` keeps the build context small.

Full deployment guide: [`docs/DEPLOYMENT.md`](./docs/DEPLOYMENT.md).

---

## 11. Tooling-specific notes

### Svelte MCP server (Cursor / Codex / Claude Code)

When writing or modifying Svelte files, the Svelte MCP server is
mandatory:

1. `list-sections` to discover relevant docs.
2. `get-documentation` for any section whose `use_cases` matches the
   change.
3. `svelte-autofixer` on every modified Svelte file before handing
   back. Loop until clean.
4. `playground-link` only if the user asks and only when no files
   were written.

### rust-analyzer MCP server

Installed via `cargo install rust-analyzer-mcp` and registered in
`~/.codeium/windsurf/mcp_config.json` as `rust-analyzer`. When
modifying Rust files, prefer these tools over re-running `cargo check`
on every iteration:

- `rust_analyzer_set_workspace` — pin the LSP to `backend/` at the
  start of a Rust session.
- `rust_analyzer_workspace_diagnostics` — real rust-analyzer errors
  across the crate; catches clippy/rustfmt drift before `cargo`.
- `rust_analyzer_format` — rustfmt a single file in-place. Run this
  on every edited `.rs` file before handing back; `cargo fmt --check`
  in CI will otherwise reject a formatter-drift-only PR.
- `rust_analyzer_hover` / `definition` / `references` / `completion` —
  cheaper than grep for type-aware navigation.

Fallbacks if the MCP server is unavailable: the CI parity commands
in §4 (`cargo fmt -- --check`, `cargo clippy -- -D warnings`).

### Phosphor icon migration codemod

`phosphor-svelte@3+` deprecated every unsuffixed default export
(`ShieldCheck`) in favour of an `Icon`-suffixed twin
(`ShieldCheckIcon`). If the IDE shows strikethroughs on phosphor
imports, run the codemod instead of editing files by hand:

```bash
node scripts/migrate-phosphor.mjs
```

Idempotent, word-boundary safe (will not collide on identifiers like
`Users` inside `UsersList`), and skips already-migrated imports. The
script lives alongside other repo automation in `scripts/`.

### Cursor rules

Workspace-level rules live in `.cursor/rules/`. They are loaded
automatically by Cursor; there is no need to read them directly unless
debugging rule application. Currently:

- `.cursor/rules/svelte-mcp.mdc` — restates the Svelte MCP discipline
  above with auto-attach metadata.

### Sub-agents

When a task has multiple independent investigation streams (e.g.
"audit module X" + "audit module Y"), prefer launching them in
parallel. Do not parallelise dependent steps.

---

## 12. When you are stuck

In order:

1. Check [`docs/RUNBOOK.md`](./docs/RUNBOOK.md) — every known failure
   mode is documented there with a remediation.
2. Read the relevant wiring doc in [`docs/wiring/`](./docs/wiring/).
3. Inspect production logs: `railway logs --service swings`.
4. Replay the migration set against a throwaway DB (see §5).
5. Open an issue with the diagnostic output — never silently work
   around a guard.
