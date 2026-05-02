# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

The canonical agent guide is **[`AGENTS.md`](./AGENTS.md)**. Read it for the full picture.
This file is a structured summary of the most critical points.

---

## ⛔ NON-NEGOTIABLE — MCP-FIRST WORKFLOW

These rules apply on **every** session and **every** edit. The CI gate
catches some of this; the rules below catch the rest. Skipping them is
the most common way bad code lands in this repo. There are NO exceptions
for "small" changes — the bugs that ship are the ones that look small.

### Before writing or editing any `.svelte` file

1. `mcp__svelte__list-sections` — get the table of contents.
2. `mcp__svelte__get-documentation` — fetch every section whose
   `use_cases` matches the change (runes, lifecycle, snippets, forms,
   transitions, accessibility, etc.). **Read them first, write code second.**
3. Write the code.
4. `mcp__svelte__svelte-autofixer` on every modified `.svelte` file.
   Loop until the autofixer reports clean. The autofixer is the LAST
   gate, not the only one.

If you skip steps 1–2 and let the autofixer "find it later," you will
ship Svelte 4 idioms in a Svelte 5 codebase. That is exactly how the
"Publish doesn't clear the editor", "members delete flashes the
skeleton", and "watchlists kebab does nothing on tablet" classes of bug
land. You have been warned.

### Before writing or editing any `.rs` file

1. `mcp__rust-analyzer__rust_analyzer_set_workspace` — pin to
   `/Users/billyribeiro/Desktop/my-websites/swings/backend`. Once per
   session is enough; don't re-pin per file.
2. `mcp__rust-analyzer__rust_analyzer_workspace_diagnostics` — runs
   real rust-analyzer instead of `cargo check`. Catches clippy/rustfmt
   drift seconds faster.
3. `mcp__rust-analyzer__rust_analyzer_hover` /
   `definition` / `references` / `completion` — type-aware navigation.
   Use these instead of `grep` when you need to know "what does this
   trait return" or "who calls this fn".
4. `mcp__rust-analyzer__rust_analyzer_format` on every edited `.rs`
   file before commit.
5. `cargo clippy --manifest-path backend/Cargo.toml --all-targets -- -D warnings`
   as the final gate (catches what rust-analyzer doesn't).

### Cite the rule in your work

When you make a change covered by a hard rule below (Migrations,
RBAC, Audit, Idempotency, etc.), cite the rule number in your commit
message or PR description. This is how the user can audit at-a-glance
that you actually followed the rule instead of pattern-matched a
similar edit.

### Behavioural verification, not just gates

`pnpm check` + `pnpm lint` + `pnpm test:unit` and `cargo test --lib`
prove the code COMPILES. They do NOT prove the feature works. For any
UI-visible change (CLS, button state, modal flow, optimistic updates,
loading skeletons, toast feedback) you MUST either:

- write a Playwright spec in `e2e/admin/*.spec.ts` that exercises the
  flow and captures CLS via `PerformanceObserver({ type: 'layout-shift' })`,
  OR
- explicitly tell the user "I can't see this — please open the browser
  and verify X, Y, Z".

Do not claim "this is done" based on CI gates alone. CI gates are a
necessary but not sufficient proof.

---

## What this repo is

`swings` is a production full-stack membership and content platform.

| Surface  | Stack                                                 | Host       |
| -------- | ----------------------------------------------------- | ---------- |
| Frontend | SvelteKit · Svelte 5 runes · scoped CSS (no Tailwind) | Vercel     |
| Backend  | Rust 2021 · Axum · `sqlx` · Tokio                     | Railway    |
| Database | PostgreSQL 16 · forward-only `sqlx` migrations        | Railway    |
| Storage  | Cloudflare R2 (S3-compatible)                         | Cloudflare |

Single pnpm workspace at root; Cargo crate at `backend/`. No separate frontend package.

---

## Commands

| Goal                           | Command                                                                        |
| ------------------------------ | ------------------------------------------------------------------------------ |
| Frontend dev                   | `pnpm dev`                                                                     |
| Backend dev                    | `pnpm dev:api`                                                                 |
| Both together                  | `pnpm dev:all`                                                                 |
| Type-check frontend            | `pnpm check`                                                                   |
| Lint frontend                  | `pnpm lint`                                                                    |
| Frontend unit tests            | `pnpm test:unit`                                                               |
| Frontend E2E                   | `pnpm test:e2e`                                                                |
| Backend fmt check (CI parity)  | `cargo fmt --manifest-path backend/Cargo.toml --all -- --check`                |
| Backend lint (CI parity)       | `cargo clippy --manifest-path backend/Cargo.toml --all-targets -- -D warnings` |
| Backend tests (needs Postgres) | `cd backend && cargo test`                                                     |
| Whole-repo CI parity           | `pnpm ci:all`                                                                  |
| Regenerate OpenAPI TS types    | `pnpm gen:types`                                                               |
| Stripe webhook forwarding      | `pnpm stripe:listen`                                                           |

Git hooks (managed by **`simple-git-hooks`**, not Husky — `node_modules/.husky/` will not exist):

- **pre-commit**: `pnpm lint && pnpm test:unit -- --run` — fast gate, runs every commit.
- **pre-push**: `pnpm ci:all` — full frontend + backend CI parity, including prettier, type-check, lint, SEO audit, unit tests, build, cargo fmt, clippy, and `cargo test`. Slower (~3-5 min) but it's the same gate CI runs.

Do not bypass with `--no-verify` without documenting why in the commit message. If a hook fails, fix the root cause; don't re-stage and `--amend` past it.

---

## Hard rules

1. **Migrations are forward-only.** Never edit a `0NN_*.sql` file after it's been applied. Add a new one instead. Highest version in tree: `091_perf_indexes.sql` (some gaps are fine; every prefix must be unique).
2. **Audit every admin mutation.** Use one of the high-level helpers in `backend/src/services/audit.rs`: `audit_admin`, `audit_admin_priv`, `audit_admin_priv_no_target`, `audit_admin_under_impersonation`. They all wrap `record_admin_action_best_effort` (the underlying writer) with the right ergonomics. See `admin_create_post` (uses `audit_admin`) vs `admin_update_post` (uses `audit_admin_priv`) in `backend/src/handlers/blog.rs` for the reference shapes. Bare `services::audit::record_admin_action` exists but is rarely the right call site.
3. **RBAC every admin mutation.** Call `state.policy.require(...)` (or one of the project's action helpers like `require_blog_post_action` in `handlers/blog.rs`) **before** the mutation runs. The permission matrix is seeded by `021_rbac.sql`. Naming convention: `<domain>.<resource>.<verb>` (e.g. `blog.post.update`, `blog.post.update_own`). Ownership-aware mutations split into `_own` / `_any` variants — see `require_blog_post_action` for the canonical pattern.
4. **`Idempotency-Key` on all admin POSTs.** Enforced by `backend/src/middleware/idempotency.rs`. Race coverage: copy the `Barrier`-based pattern from `concurrent_same_key_creates_exactly_one_resource` in `tests/admin_idempotency.rs`.
5. **No `unwrap()` / `expect()` / `panic!()` in non-test code** for **runtime-fallible** operations. Use `?` with `AppError`. Provably-infallible `expect()` (compile-time-known constants, hardcoded regex, `NonZeroU32::new(LITERAL)`) is acceptable when paired with a SAFETY comment explaining why; the right _long-term_ fix is to push the invariant into the type system (use non-zero types so the construction is total). Not enforced by clippy today; intent is "no surprise panics on user-reachable paths."
6. **No `any` in TypeScript.** Use `unknown` and narrow. ESLint enforces this.
7. **Svelte 5 runes only.** `$state`, `$derived`, `$effect`, `{#snippet}` / `{@render}` — no Svelte 4 patterns (`export let`, reactive `$:` blocks, `<svelte:component>`-style dispatch).
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
  migrations/         Forward-only sqlx migrations (highest: 091)
  tests/              Integration tests (real Postgres on :5433)
e2e/                  Playwright specs
ops/                  Prometheus rules + Grafana dashboard
docs/                 All long-form docs (start at docs/README.md)
scripts/              Repo automation
```

### Auth / session model

- **Sessions**: JWT access tokens + refresh-token rotation (`001_initial.sql`, `018_refresh_token_families.sql`). Access tokens have a short TTL; refresh tokens are family-tracked so a stolen refresh token can be invalidated en bloc.
- **BFF cookie path is primary**: the SvelteKit frontend hits `/api/auth/login` and the backend issues `Set-Cookie: swings_access=...; HttpOnly` + `swings_refresh=...; HttpOnly`. The SPA uses `credentials: 'include'` and never sees a raw bearer token. Direct `Authorization: Bearer` calls still work (used by the integration test harness) but the cookie is the production path.
- **Admin extractor**: `backend/src/extractors.rs` exposes `AdminUser` (any admin role) and `PrivilegedUser` (admin OR resource-author for ownership-aware mutations). New admin handlers should default to `PrivilegedUser` + an explicit `policy.require()` so RBAC is type-system-visible at the function signature.
- **Impersonation**: `backend/src/security/impersonation.rs` — always emits a paired `admin_actions` row via `audit_admin_under_impersonation`.
- **IP allowlist**: `backend/src/security/ip_allowlist.rs` — fails open when the table is empty (so a fresh deploy can be configured), fails closed otherwise.
- **Bootstrap admin**: seeded from `ADMIN_EMAIL` / `ADMIN_PASSWORD` env vars at startup. Required in production (`bail!()`); skipped with a warning in dev. Test fixture defaults: `admin@swings.test` / `admin-password-1234`.

### Background workers

Spawned from `backend/src/main.rs`. Each emits a `*_last_success_unixtime` Prometheus gauge so the runbook can detect a stalled loop.

| Worker                  | File                              | Default interval |
| ----------------------- | --------------------------------- | ---------------- |
| Outbox event dispatcher | `events/worker.rs`                | continuous       |
| Audit log retention     | `services/audit_retention.rs`     | `3600s`          |
| DSAR export             | `services/dsar_worker.rs`         | `30s`            |
| DSAR artefact TTL sweep | `services/dsar_artifact_sweep.rs` | `3600s`          |
| Idempotency-Key GC      | `services/idempotency_gc.rs`      | `300s`           |
| Blog post scheduler     | `services/blog_scheduler.rs`      | `60s`            |

When adding a worker: add the metric, the Prometheus alert in `ops/`, and the runbook section in `docs/RUNBOOK.md` in the same PR. AGENTS.md §7 must also list it.

---

## Testing

- **Rust unit tests** — colocated under `#[cfg(test)]`. Run with `cargo test --lib`. **Must print `0 ignored`** (current count: 538 passed). Failure to keep this at zero means a colocated test was skipped without justification.
- **Rust integration tests** — `backend/tests/` (49 files). Require Postgres on `:5433` via `backend/docker-compose.yml`. Bring-up:
  ```bash
  docker compose -f backend/docker-compose.yml up -d db
  DATABASE_URL_TEST=postgres://postgres:postgres@localhost:5433/swings \
    cargo test --manifest-path backend/Cargo.toml
  ```
  Test DB credentials are deliberately the boring defaults (`postgres/postgres@swings`) so the env-var line above is the canonical incantation; do not hardcode a different one.
- **Frontend unit tests** — `pnpm test:unit -- --run` (Vitest, currently 103 passing). Browser-mode tests via `pnpm test:browser`.
- **E2E** — Playwright specs in `e2e/`. CLS-sensitive flows use `PerformanceObserver({ type: 'layout-shift' })`; see `e2e/admin/members-filter-cls.spec.ts` for the pattern.
- **Never call `std::env::set_var` in tests** — not thread-safe. Inject config as function arguments instead (see `backend/src/forms/integration_config.rs` for the pattern).
- **`#[ignore]` is forbidden as a way to skip Postgres setup** — move DB-dependent tests to `backend/tests/` (which is where Postgres is available) instead. `#[ignore]` IS acceptable for tests gated on a documented integrator-wiring step (e.g. `tests/observability.rs`'s end-to-end test that needs the harness's `build_router` to add the observability layers); in that case the attribute MUST carry a reason string explaining the gate (`#[ignore = "TestApp omits observability layers — see module docs"]`).
- **Idempotency race coverage** — copy the `concurrent_same_key_creates_exactly_one_resource` `Barrier` pattern from `tests/admin_idempotency.rs`.
- **Audit-row coverage for new admin mutations** — copy the pattern in `tests/admin_blog_post_autosave.rs`: hit the endpoint, then assert an `admin_actions` row exists with the right `action`, `target_id`, `actor_user_id`, and metadata. Compile-time evidence of an `audit_admin*` call in the handler is necessary but not sufficient.

---

## Svelte MCP (mandatory for .svelte edits)

**Always use the Svelte MCP server when writing or modifying any `.svelte` file.** No exceptions.

1. `list-sections` → find relevant docs.
2. `get-documentation` for any section whose `use_cases` matches the change.
3. `svelte-autofixer` on every modified `.svelte` file — loop until clean.
4. `playground-link` only if the user asks and no files were written.

---

## Rust MCP (use when surfaced)

The `mcp__rust-analyzer__*` tools (`rust_analyzer_set_workspace`,
`rust_analyzer_workspace_diagnostics`, `rust_analyzer_format`,
`rust_analyzer_hover` / `definition` / `references` / `completion`) are
**not always loaded** in every Claude Code session — they appear as
deferred tools that need to be fetched via `ToolSearch` before they can
be called. If they ARE available, prefer them over re-running `cargo
check` per file: they're type-aware and faster.

**Canonical Rust gate (always works, with or without MCP)** — run this
before every push:

```bash
cargo fmt --manifest-path backend/Cargo.toml --all -- --check
cargo clippy --manifest-path backend/Cargo.toml --all-targets -- -D warnings
cargo test --manifest-path backend/Cargo.toml --lib    # 538 passing, 0 ignored
```

The pre-push hook (`pnpm ci:all`) wraps all of the above. If you fall
back to the `cargo` commands directly, this is also what CI runs — same
output, same gating.

---

## Deployment

| Surface  | Platform | Config                                  |
| -------- | -------- | --------------------------------------- |
| Frontend | Vercel   | `vercel.json` + `svelte.config.js`      |
| Backend  | Railway  | `Dockerfile` at repo root (single file) |

One Dockerfile serves both Railway and local `docker-compose.yml`. Do not create a second one.

Full guide: [`docs/DEPLOYMENT.md`](./docs/DEPLOYMENT.md). Runbook: [`docs/RUNBOOK.md`](./docs/RUNBOOK.md).

---

## Known open items (non-blockers)

These are tracked gaps that the team is aware of — they are NOT new findings.
A new agent should not "discover" and refile them; instead, propose a fix
when working in the area, citing the O-number.

| #   | Item                                                                                                | Notes                                                                                                               |
| --- | --------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------- |
| O-1 | `consent.rs` admin handlers use `AdminUser` extractor only — no fine-grained `policy.require()` yet | DSAR endpoints (`admin_list_dsar`, `admin_fulfill_dsar`) and adjacent consent admin handlers. GDPR/CCPA territory.  |
| O-2 | TOTP / MFA for admin roles — not yet implemented                                                    | Multi-day feature; needs migration for TOTP secret storage, recovery codes, and login flow changes.                 |
| O-3 | `/api/greeks-pdf` returns `success: true` without a real PDF                                        | Lives at `src/routes/api/greeks-pdf/+server.ts`. Either return 501 or wire to Resend/SendGrid.                      |
| O-4 | `anchor_recent` in `consent/integrity.rs` has no worker invoking it                                 | Function exists at `consent/integrity.rs:99` with a `// TODO: schedule hourly` comment. Needs a `services/` worker. |

### Deferred items from PR #45 audit batch (2026-05-02)

A forensic audit raised these; they were verified as real but deliberately
not patched in that batch. Recorded so they don't get re-flagged:

| Item                                                                                                            | Why deferred                                                                                                                                                                                                                                                  |
| --------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Provably-infallible `expect()` in `rate_limit.rs`, `validation.rs`, `preferences.rs`, `impersonation_banner.rs` | Right fix is type-system enforcement (push the invariant into the construction so the call is total), not mechanical conversion to `unwrap_or` with a fake fallback for an impossible case. Track and pair with a future `clippy::expect_used = "deny"` flip. |
| Mutex-poisoning `expect()` in `antispam.rs:321`, `uploads.rs:141/147/185`                                       | Industry-standard pattern (a poisoned mutex is unrecoverable; panicking is the right behaviour). The real smell is `uploads.rs:206` (`expect("session just touched")` — should be a structural invariant).                                                    |
| CSP `unsafe-inline` in `src/hooks.server.ts:46`                                                                 | Multi-day project. Migrating to nonce-based CSP requires every `<style>`, `<script>`, and inline event handler to accept a per-request nonce — including third-party widgets (Google Fonts, etc.).                                                            |
| `{@html}` without `safeHtml()` in `SectionHeader.svelte`, `WhoItsFor.svelte`                                    | Defense-in-depth gap. Backend already sanitizes at the write boundary, but a layered approach should also wrap `{@html}` consumers in `$lib/utils/safeHtml.ts`. Quick win when next touching those components.                                                |
| Webhook channel stub at `notifications/channels/webhook.rs`                                                     | Returns `Permanent("not implemented")` — correct stub pattern. Implementing requires product spec for retry policy, signing scheme, payload schema.                                                                                                           |
| R2 deletion path drops DB row but leaves R2 object (`commerce/repo.rs:472`)                                     | Needs a retention policy decision ("keep deleted assets for N days?") before a cleanup worker can be designed. Tracked as EC-07.                                                                                                                              |
