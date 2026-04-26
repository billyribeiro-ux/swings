# swings

[![CI](https://github.com/billyribeiro-ux/swings/actions/workflows/ci.yml/badge.svg)](https://github.com/billyribeiro-ux/swings/actions/workflows/ci.yml)
[![SQL Lint](https://github.com/billyribeiro-ux/swings/actions/workflows/sql-lint.yml/badge.svg)](https://github.com/billyribeiro-ux/swings/actions/workflows/sql-lint.yml)
[![OpenAPI Drift](https://github.com/billyribeiro-ux/swings/actions/workflows/openapi-drift.yml/badge.svg)](https://github.com/billyribeiro-ux/swings/actions/workflows/openapi-drift.yml)
[![Security](https://github.com/billyribeiro-ux/swings/actions/workflows/security.yml/badge.svg)](https://github.com/billyribeiro-ux/swings/actions/workflows/security.yml)

> **Last revised:** 2026-04-19
> **Status:** active
> **Frontend (Vercel):** SvelteKit · Svelte 5 · TailwindCSS v4
> **Backend (Railway):** Rust · Axum · SQLx · PostgreSQL 16
> **Deployment target:** swings-gamma.vercel.app + swings-production.up.railway.app

`swings` is a full-stack membership and content platform: a SvelteKit
marketing/member experience on the front, a hardened Rust admin/back-office
on the back, and a Postgres data plane in between. The repository is a single
pnpm workspace plus a Cargo crate at `backend/`.

---

## Table of contents

1. [Architecture at a glance](#architecture-at-a-glance)
2. [Repository layout](#repository-layout)
3. [Quick start](#quick-start)
4. [Common workflows](#common-workflows)
5. [Testing](#testing)
6. [Deployment](#deployment)
7. [Operations](#operations)
8. [Documentation index](#documentation-index)
9. [Conventions](#conventions)
10. [Contributing & support](#contributing--support)

---

## Architecture at a glance

```
                    ┌─────────────────────────┐
                    │    Cloudflare R2        │  (media + DSAR artefacts)
                    └─────────────┬───────────┘
                                  │ S3 API
┌──────────────┐  HTTPS  ┌────────┴──────────┐  Postgres TLS  ┌─────────────┐
│  SvelteKit   ├────────▶│   Rust / Axum     ├───────────────▶│ Postgres 16 │
│   (Vercel)   │ JSON    │  swings-api       │                │  (Railway)  │
└──────┬───────┘         │  (Railway)        │                └─────────────┘
       │                 └──────┬─────────┬──┘
       │ SSR fetch              │         │
       ▼                        ▼         ▼
   browser            Prometheus    Stripe / Postmark
                       /metrics
```

- **Frontend** — SvelteKit (Svelte 5 runes), TailwindCSS v4, GSAP, Three.js,
  Tiptap. Deployed to Vercel via `@sveltejs/adapter-vercel`.
- **Backend** — Axum-on-Tokio with SQLx and an in-process worker pool for
  background tasks (DSAR export, audit-log retention, idempotency-cache GC,
  artefact TTL sweep). Built and shipped from `backend/`.
- **Database** — PostgreSQL 16 with 67 forward-only `sqlx` migrations
  in `backend/migrations/` (versions 001–075, gap-tolerant).
- **Object storage** — Cloudflare R2 (S3-compatible) for media and DSAR
  exports, with a `Local` filesystem fallback for development.
- **Observability** — Prometheus metrics on `/metrics`; provisioning-ready
  rules and Grafana dashboard live in [`ops/`](./ops/).

For the full RBAC/audit/security model, see
[`docs/archive/AUDIT_PHASE3_PLAN.md`](./docs/archive/AUDIT_PHASE3_PLAN.md) §12
(authz matrix) and [`docs/INFRASTRUCTURE.md`](./docs/INFRASTRUCTURE.md).

---

## Repository layout

```
.
├── src/                  # SvelteKit app (routes, lib, components, stores)
├── static/               # Static frontend assets
├── messages/             # Inlang (i18n) message catalogues
├── e2e/                  # Playwright end-to-end specs
├── backend/              # Rust crate — Axum API, workers, migrations, tests
│   ├── src/
│   │   ├── handlers/     # HTTP handlers (admin, member, public, webhooks)
│   │   ├── services/     # Background workers + cross-cutting services
│   │   ├── middleware/   # Tower layers (idempotency, rate limit, IP allowlist…)
│   │   ├── security/     # Impersonation + IP allowlist primitives
│   │   ├── observability/# Tracing + metrics scaffolding
│   │   └── …             # commerce, consent, popups, forms, notifications, pdf
│   ├── migrations/       # sqlx forward-only migrations (67 files, versions 001–075)
│   └── tests/            # Integration tests against a real Postgres
├── ops/                  # Prometheus rules + Grafana dashboard + provisioning README
├── docs/                 # All long-form documentation (see index below)
├── scripts/              # Repo automation (audit dump, OpenAPI → TS, SEO check)
├── .github/workflows/    # CI: ci.yml, sql-lint.yml, openapi-drift.yml, security.yml
├── docker-compose.yml    # Full local stack (api + db; Postgres on host :5434)
├── Dockerfile            # Single source of truth — used by Railway, Render, compose
├── .dockerignore         # Trims build context for the consolidated Dockerfile
├── render.yaml           # Render blueprint — references ./Dockerfile
├── vercel.json           # Vercel routing/env hints
└── package.json          # pnpm workspace root
```

---

## Quick start

### Prerequisites

| Tool       | Version            | Notes                                                              |
| ---------- | ------------------ | ------------------------------------------------------------------ |
| Node.js    | `>=24.14.1`        | Pinned in `package.json#engines`                                   |
| pnpm       | `10.33.0`          | `corepack enable` then `corepack prepare`                          |
| Rust       | `1.83+` stable     | `rustup default stable`                                            |
| PostgreSQL | `16.x`             | Local install or via `docker compose up -d db`                     |
| sqlx-cli   | `0.8.x`            | `cargo install sqlx-cli --no-default-features --features postgres` |
| Docker     | `24+` _(optional)_ | Only needed for compose / R2 emulator (MinIO)                      |

### Clone & install

```bash
git clone https://github.com/billyribeiro-ux/swings.git
cd swings
pnpm install
```

### Spin up Postgres

```bash
docker compose up -d db        # Postgres on host :5434→5432, user/pass swings/swings_secret
```

Or use a local `psql` install — see `.env.example` for the expected
connection string format.

### Run the backend

```bash
cp backend/.env.example backend/.env   # then fill in JWT_SECRET, ADMIN_*, …
pnpm dev:api                            # cargo run inside backend/
```

The API listens on `http://localhost:3001`. On first boot it runs
migrations, seeds the admin user, and starts every background worker.

### Run the frontend

```bash
pnpm dev                       # http://localhost:5173
```

Or run both simultaneously with live reload:

```bash
pnpm dev:all
```

**Stripe (test mode) — hosted checkout + webhooks:** add `STRIPE_SECRET_KEY` and
`PUBLIC_APP_URL` to the **repo root** `.env` (and the same `sk_test_` in
`backend/.env`); then run `pnpm stripe:listen` in a third terminal and copy
the `whsec_` into `backend/.env` as `STRIPE_WEBHOOK_SECRET`. See
[`docs/stripe-local-testing.md`](./docs/stripe-local-testing.md).

---

## Common workflows

| Goal                                | Command                                                                                       |
| ----------------------------------- | --------------------------------------------------------------------------------------------- |
| Regenerate frontend OpenAPI types   | `pnpm gen:types`                                                                              |
| Type-check + lint frontend          | `pnpm check && pnpm lint`                                                                     |
| Format everything                   | `pnpm format` · `cargo fmt --manifest-path backend/Cargo.toml`                                |
| Backend lint (CI parity)            | `cargo clippy --manifest-path backend/Cargo.toml --all-targets -- -D warnings`                |
| Add a new SQL migration             | Drop a `0NN_description.sql` into `backend/migrations/` (forward-only, no edits after deploy) |
| Tail prod logs                      | `railway logs --service swings`                                                               |
| Forward Stripe webhooks → local API | `pnpm stripe:listen` (requires [Stripe CLI](https://docs.stripe.com/stripe-cli))              |

---

## Testing

```bash
# ── Frontend ──────────────────────────────────────────────────────────────
pnpm test:unit          # vitest, ~67 specs, runs under the pre-commit hook
pnpm test:browser       # vitest + playwright browser env
pnpm test:e2e           # full Playwright suite (e2e/)

# ── Backend ───────────────────────────────────────────────────────────────
cd backend
cargo test              # unit + integration tests (requires DATABASE_URL_TEST)
cargo test --test admin_idempotency concurrent_same_key_creates_exactly_one_resource

# ── Whole-repo CI parity ──────────────────────────────────────────────────
pnpm ci:all             # ci:frontend + ci:backend
```

The integration test harness brings up its own isolated Postgres on
`:5433` via `backend/docker-compose.yml` — see
[`docs/wiring/FDN-TESTHARNESS-WIRING.md`](./docs/wiring/FDN-TESTHARNESS-WIRING.md).

R2-dependent tests are gated on `R2_TEST_*` env vars and skip cleanly when
no S3-compatible emulator (MinIO/LocalStack) is running.

---

## Deployment

| Surface  | Platform | Source of truth                                       | Runtime              |
| -------- | -------- | ----------------------------------------------------- | -------------------- |
| Frontend | Vercel   | `vercel.json` + `svelte.config.js` (`adapter-vercel`) | Edge / Node (auto)   |
| Backend  | Railway  | root `Dockerfile` (build context = repo root)         | `swings-api` service |
| Backend  | Render   | `render.yaml` → root `Dockerfile`                     | mirror deploy target |
| Database | Railway  | provisioned Postgres 16 add-on                        | persistent volume    |

A healthy Railway boot prints:

```
authz policy loaded from role_permissions, pairs: 182
Admin user seeded (...)
outbox worker pool started, workers: 4
audit-retention worker spawned, interval_secs: 3600
dsar-export worker spawned, interval_secs: 30
dsar-artifact-sweep worker spawned, interval_secs: 3600
idempotency-gc worker spawned, interval_secs: 300
Swings API listening on port 3001
```

Full guide: [`docs/DEPLOYMENT.md`](./docs/DEPLOYMENT.md). For
infra topology, see [`docs/INFRASTRUCTURE.md`](./docs/INFRASTRUCTURE.md).

---

## Operations

- **Metrics & alerts** — Prometheus rules in
  [`ops/prometheus/admin-alerts.rules.yml`](./ops/prometheus/admin-alerts.rules.yml),
  Grafana dashboard in
  [`ops/grafana/admin-overview.dashboard.json`](./ops/grafana/admin-overview.dashboard.json),
  provisioning instructions in [`ops/README.md`](./ops/README.md).
- **On-call runbook** — [`docs/RUNBOOK.md`](./docs/RUNBOOK.md). Every alert in
  the rules file links here via `runbook_url`.
- **Security policy** — [`SECURITY.md`](./SECURITY.md).
- **CI policy** — [`docs/ci.md`](./docs/ci.md).

---

## Documentation index

The canonical list lives in [`docs/README.md`](./docs/README.md). Quick
shortcuts:

- [`docs/RUNBOOK.md`](./docs/RUNBOOK.md) — operator runbook for new alerts.
- [`docs/DEPLOYMENT.md`](./docs/DEPLOYMENT.md) — Vercel + Railway go-live.
- [`docs/stripe-local-testing.md`](./docs/stripe-local-testing.md) — test keys, webhooks, dynamic plans + `price_data`, official test cards.
- [`docs/INFRASTRUCTURE.md`](./docs/INFRASTRUCTURE.md) — full stack topology.
- [`docs/SEO_RUNBOOK.md`](./docs/SEO_RUNBOOK.md) — SEO operating standard.
- [`docs/wiring/`](./docs/wiring/) — integrator-facing wiring docs
  (test harness, observability, common utilities).
- [`docs/archive/`](./docs/archive/) — historical audit phases and reports
  kept for traceability; **not** the source of truth for current behaviour.

---

## Conventions

- **Languages** — Rust 2021 edition, TypeScript strict, Svelte 5 runes only.
- **No `any`, no `unwrap()`** in checked-in code paths (tests excluded).
- **Migrations are forward-only.** Edit a migration after it has been
  applied to _any_ environment and the next boot will fail the sqlx
  checksum guard. Add a new migration instead.
- **Audit everything** — admin mutations route through
  `services::audit::record(...)` so the action lands in `admin_actions`.
- **No mutation without a permission check** — every admin handler calls
  `policy.require(ctx, "<perm>")`. The matrix is seeded by migration 21.
- **Idempotency-Key** — required on admin POSTs; middleware + GC are
  documented in [`docs/RUNBOOK.md`](./docs/RUNBOOK.md).

For agent-tool-specific guidance (Cursor, Codex, Claude Code, Copilot),
see [`AGENTS.md`](./AGENTS.md).

---

## Contributing & support

- File issues / PRs on GitHub.
- Use [Conventional Commits](https://www.conventionalcommits.org/) for
  commit subjects (`feat:`, `fix:`, `docs:`, `chore:`, …).
- Pre-commit hook runs `pnpm lint` and `pnpm test:unit -- --run`. Don't
  bypass it without a documented reason.
- Security-sensitive issues: see [`SECURITY.md`](./SECURITY.md).

— Maintained by the swings core team.
