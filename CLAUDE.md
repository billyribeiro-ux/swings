# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

The canonical agent guide is **[`AGENTS.md`](./AGENTS.md)**. Read it for the full picture.
This file is a structured summary of the most critical points.

---

## What this repo is

`swings` is a production full-stack membership and content platform.

| Surface  | Stack                                                    | Host       |
| -------- | -------------------------------------------------------- | ---------- |
| Frontend | SvelteKit · Svelte 5 runes · scoped CSS (no Tailwind)    | Vercel     |
| Backend  | Rust 2021 · Axum · `sqlx` · Tokio                        | Railway    |
| Database | PostgreSQL 16 · forward-only `sqlx` migrations           | Railway    |
| Storage  | Cloudflare R2 (S3-compatible)                            | Cloudflare |

Single pnpm workspace at root; Cargo crate at `backend/`. No separate frontend package.

---

## Commands

| Goal                           | Command                                                                           |
| ------------------------------ | --------------------------------------------------------------------------------- |
| Frontend dev                   | `pnpm dev`                                                                        |
| Backend dev                    | `pnpm dev:api`                                                                    |
| Both together                  | `pnpm dev:all`                                                                    |
| Type-check frontend            | `pnpm check`                                                                      |
| Lint frontend                  | `pnpm lint`                                                                       |
| Frontend unit tests            | `pnpm test:unit`                                                                  |
| Frontend E2E                   | `pnpm test:e2e`                                                                   |
| Backend fmt check (CI parity)  | `cargo fmt --manifest-path backend/Cargo.toml --all -- --check`                   |
| Backend lint (CI parity)       | `cargo clippy --manifest-path backend/Cargo.toml --all-targets -- -D warnings`    |
| Backend tests (needs Postgres) | `cd backend && cargo test`                                                        |
| Whole-repo CI parity           | `pnpm ci:all`                                                                     |
| Regenerate OpenAPI TS types    | `pnpm gen:types`                                                                  |
| Stripe webhook forwarding      | `pnpm stripe:listen`                                                              |

Pre-commit hook: `pnpm lint && pnpm test:unit -- --run`. Do not bypass without documenting why in the commit message.

---

## Hard rules

1. **Migrations are forward-only.** Never edit a `0NN_*.sql` file after it's been applied. Add a new one instead. Current versions: `001–087` (some gaps are fine; every prefix must be unique).
2. **Audit every admin mutation.** Call `services::audit::record(...)` in every admin handler.
3. **RBAC every admin mutation.** Call `policy.require(ctx, "admin.<resource>.<verb>")` before doing work. Matrix is seeded by `021_rbac.sql`.
4. **`Idempotency-Key` on all admin POSTs.** Enforced by `backend/src/middleware/idempotency.rs`.
5. **No `unwrap()` / `expect()` / `panic!()` in non-test code.** Use `?` with `AppError`.
6. **No `any` in TypeScript.** Use `unknown` and narrow.
7. **Svelte 5 runes only.** `$state`, `$derived`, `$effect` — no Svelte 4 patterns.
8. **Icons: Phosphor only.** Import from `phosphor-svelte`. Use the `Icon`-suffixed exports (`ShieldCheckIcon`, not `ShieldCheck`). Run `node scripts/migrate-phosphor.mjs` if you see deprecated imports.

---

## Architecture

```
src/                  SvelteKit app (routes, lib, components, stores)
backend/
  src/
    handlers/         HTTP handlers — admin, member, public, webhooks
    services/         Background workers + cross-cutting services
    middleware/       Tower layers (idempotency, rate-limit, IP allowlist)
    security/         Impersonation + IP allowlist
    commerce/ consent/ popups/ forms/ notifications/ pdf/  Domain modules
  migrations/         Forward-only sqlx migrations (001–087)
  tests/              Integration tests (real Postgres on :5433)
e2e/                  Playwright specs
ops/                  Prometheus rules + Grafana dashboard
docs/                 All long-form docs (start at docs/README.md)
scripts/              Repo automation
```

### Auth / session model

- Sessions: JWT bearer tokens + refresh-token rotation (`001_initial.sql`, `018_refresh_token_families.sql`).
- Admin extractor: `backend/src/extractors/admin.rs` (`AdminUser`).
- BFF cookie: `swings_access` httpOnly cookie — the idempotency middleware reads it alongside `Authorization: Bearer`.
- Impersonation: `backend/src/security/impersonation.rs` — always emits a paired `admin_actions` row.
- IP allowlist: `backend/src/security/ip_allowlist.rs` — fails open when the table is empty, fails closed otherwise.

### Background workers

Spawned from `backend/src/main.rs`. Each emits a `*_last_success_unixtime` Prometheus gauge.

| Worker                  | File                              |
| ----------------------- | --------------------------------- |
| Outbox event dispatcher | `events/worker.rs`                |
| Audit log retention     | `services/audit_retention.rs`     |
| DSAR export             | `services/dsar_worker.rs`         |
| Idempotency-Key GC      | `services/idempotency_gc.rs`      |

When adding a worker: add the metric, the Prometheus alert in `ops/`, and the runbook section in `docs/RUNBOOK.md` in the same PR.

---

## Testing

- **Rust unit tests** — colocated under `#[cfg(test)]`. Run with `cargo test --lib`. Must print `0 ignored`.
- **Rust integration tests** — `backend/tests/`. Require Postgres on `:5433` via `backend/docker-compose.yml`.
- **Never call `std::env::set_var` in tests** — not thread-safe. Inject config as function arguments instead (see `backend/src/forms/integration_config.rs` for the pattern).
- **`#[ignore]` is forbidden** — move Postgres-dependent tests to `backend/tests/`.
- **Idempotency race coverage** — copy the `concurrent_same_key_creates_exactly_one_resource` Barrier pattern from `tests/admin_idempotency.rs`.

---

## Svelte MCP (mandatory for .svelte edits)

**Always use the Svelte MCP server when writing or modifying any `.svelte` file.** No exceptions.

1. `list-sections` → find relevant docs.
2. `get-documentation` for any section whose `use_cases` matches the change.
3. `svelte-autofixer` on every modified `.svelte` file — loop until clean.
4. `playground-link` only if the user asks and no files were written.

---

## Rust MCP (preferred for .rs edits)

**Always use the Rust MCP server when modifying Rust files.** It is faster and more precise than re-running `cargo check` on each iteration.

1. `rust_analyzer_set_workspace` — pin the LSP to `backend/` at the start of every Rust session.
2. `rust_analyzer_workspace_diagnostics` — real rust-analyzer errors across the crate.
3. `rust_analyzer_format` — rustfmt a single file in-place. Run on every edited `.rs` file before handing back.
4. `rust_analyzer_hover` / `definition` / `references` / `completion` — type-aware navigation, cheaper than grep.

**Fallback** (if the MCP server is unavailable):
```bash
cargo fmt --manifest-path backend/Cargo.toml --all -- --check
cargo clippy --manifest-path backend/Cargo.toml --all-targets -- -D warnings
```

---

## Deployment

| Surface  | Platform | Config                                         |
| -------- | -------- | ---------------------------------------------- |
| Frontend | Vercel   | `vercel.json` + `svelte.config.js`             |
| Backend  | Railway  | `Dockerfile` at repo root (single file)        |

One Dockerfile serves both Railway and local `docker-compose.yml`. Do not create a second one.

Full guide: [`docs/DEPLOYMENT.md`](./docs/DEPLOYMENT.md). Runbook: [`docs/RUNBOOK.md`](./docs/RUNBOOK.md).

---

## Known open items (non-blockers)

| # | Item |
| - | ---- |
| O-1 | `consent.rs` admin handlers use `AdminUser` extractor only — no fine-grained `policy.require()` yet |
| O-2 | TOTP / MFA for admin roles — not yet implemented |
| O-3 | `/api/greeks-pdf` returns `success: true` without a real PDF |
| O-4 | `anchor_recent` in `consent/integrity.rs` has no worker invoking it |
