# swings-api

> **Last revised:** 2026-04-19
> **Crate:** `swings-api` (Rust 2021 edition; toolchain-pinned to stable 1.93+ in CI)
> **Runtime:** Axum 0.8 + Tokio + SQLx 0.8 + PostgreSQL 16
> **Status:** active — production deploys to Railway from this directory
> **Live:** `https://swings-production.up.railway.app`

The Rust backend that powers the swings membership platform: public
APIs (catalog, blog, courses, popups, forms, consent), member APIs
(profile, subscriptions, downloads), and a hardened admin/back-office
surface (audit log, RBAC, DSAR, impersonation, IP allowlist,
maintenance mode, idempotency, retention/GC workers).

The web/frontend layer lives in the parent SvelteKit project — see
[`../README.md`](../README.md) for the system overview.

---

## Table of contents

1. [Module map](#module-map)
2. [Setup](#setup)
3. [Environment variables](#environment-variables)
4. [Database & migrations](#database--migrations)
5. [Background workers](#background-workers)
6. [HTTP surface (high-level)](#http-surface-high-level)
7. [Testing](#testing)
8. [Local Docker workflows](#local-docker-workflows)
9. [Production checklist](#production-checklist)
10. [Further reading](#further-reading)

---

## Module map

```
src/
├── main.rs                # boot: config → migrations → workers → server
├── lib.rs                 # crate facade for tests
├── config.rs              # typed env loader; panics in prod on missing vars
├── db.rs                  # PgPool factory (env-tuned via PGPOOL_*)
├── error.rs               # AppError + RFC 7807 application/problem+json
├── extractors.rs          # AuthUser / AdminUser / PrivilegedUser
├── openapi.rs             # utoipa registry; snapshot-tested
├── authz.rs               # Policy engine (loads role_permissions on boot)
├── handlers/              # HTTP handlers
│   ├── auth.rs                        # register/login/refresh/me/logout
│   ├── member.rs                      # /api/member/*
│   ├── admin.rs + admin_*.rs          # /api/admin/* (audit, dsar, members, orders,
│   │                                   subscriptions, roles, security, settings,
│   │                                   ip_allowlist, impersonation, consent)
│   ├── catalog.rs / products.rs / pricing.rs / cart.rs / coupons.rs
│   ├── blog.rs / courses.rs / popups.rs / forms.rs / notifications.rs
│   ├── consent.rs / csp_report.rs / outbox.rs / analytics.rs / webhooks.rs
├── services/              # Background workers + cross-cutting services
│   ├── audit.rs                       # admin action recorder
│   ├── audit_retention.rs             # purge per app_settings retention
│   ├── dsar_admin.rs                  # DSAR export builder + tombstone summary
│   ├── dsar_worker.rs                 # async DSAR job processor
│   ├── dsar_artifact_sweep.rs         # delete expired R2/local artefacts
│   ├── idempotency_gc.rs              # prune expired idempotency_keys
│   └── storage.rs                     # MediaBackend::{Local, R2} trait
├── middleware/
│   ├── idempotency.rs                 # Idempotency-Key (claim/replay/in-flight)
│   ├── rate_limit.rs                  # per-actor admin mutation limiter
│   ├── admin_ip_allowlist.rs          # CIDR allowlist for /api/admin/*
│   ├── impersonation_banner.rs        # adds X-Impersonator-* headers
│   └── maintenance_mode.rs            # 503 with retry-after when toggled
├── security/
│   ├── impersonation.rs               # session create / lookup / revoke
│   └── ip_allowlist.rs                # CIDR parser + match
├── observability/                     # tracing + Prometheus exporter
├── commerce/                          # money, tax, refund math
├── consent/                           # consent log + integrity hash
├── popups/ forms/ notifications/ events/ pdf/ settings/ common/
└── stripe_api.rs                      # Stripe SDK wrappers
```

---

## Setup

```bash
# 1. Toolchain
rustup default stable             # 1.93+ (CI uses 1.95)
cargo install sqlx-cli --no-default-features --features postgres

# 2. Database (one of):
#    a) docker compose -f ../docker-compose.yml up -d db    # full stack
#    b) docker compose -f docker-compose.yml      up -d db  # test-only DB on :5433
#    c) bring your own local Postgres 16

# 3. Configure env
cp .env.example .env              # then fill in JWT_SECRET, ADMIN_*, etc.

# 4. Run
cargo run                         # http://localhost:3001
```

On boot the binary will: load config, connect Postgres, apply pending
migrations, seed the admin user, register the OpenAPI schema, spawn
every background worker, and begin serving on `$PORT` (default `3001`).

### Where admin credentials live

| Environment    | Where `ADMIN_EMAIL` / `ADMIN_PASSWORD` go                                                                                                                                                                                                                                   |
| -------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Local**      | `backend/.env` only — file is **gitignored** (`backend/.gitignore`). Copy from `.env.example`, put your real password there. That is the normal way to work: secrets on disk for dev, never in git.                                                                         |
| **Production** | Same variable names in Railway **Variables** UI — not a committed file.                                                                                                                                                                                     |
| **Database**   | After first successful seed, only `users.password_hash` exists (Argon2). Plaintext is not recoverable. Changing `.env` later does not rotate an existing admin’s password (see `db::seed_admin` `ON CONFLICT`); delete the user row or use password reset if you forgot it. |

---

## Environment variables

The contract is enforced by `src/config.rs`. In `APP_ENV=production` the
process **panics on startup** if any _required-in-prod_ variable is
missing or empty.

### Required (always)

| Variable       | Purpose                                   |
| -------------- | ----------------------------------------- |
| `DATABASE_URL` | Postgres DSN (`sslmode=require` in production) |
| `JWT_SECRET`   | HS256 signing key for access tokens       |

### Required in production

| Variable                | Purpose                                                         |
| ----------------------- | --------------------------------------------------------------- |
| `ADMIN_EMAIL`           | First-boot admin seed — login email                             |
| `ADMIN_PASSWORD`        | First-boot admin seed — password (argon2id-hashed at rest)      |
| `API_URL`               | Public base URL of this API                                     |
| `FRONTEND_URL`          | Browser origin of the SvelteKit app (used for CORS + redirects) |
| `STRIPE_SECRET_KEY`     | Stripe live secret                                              |
| `STRIPE_WEBHOOK_SECRET` | Stripe webhook signature secret                                 |
| `R2_ACCOUNT_ID`         | Cloudflare account ID                                           |
| `R2_ACCESS_KEY_ID`      | R2 API token access key                                         |
| `R2_SECRET_ACCESS_KEY`  | R2 API token secret                                             |
| `R2_BUCKET_NAME`        | R2 bucket name                                                  |
| `R2_PUBLIC_URL`         | Public CDN base for R2 objects (no trailing `/`)                |

### Optional (with defaults)

| Variable                        | Default        | Notes                                       |
| ------------------------------- | -------------- | ------------------------------------------- |
| `JWT_EXPIRATION_HOURS`          | `24`           | Access token lifetime                       |
| `REFRESH_TOKEN_EXPIRATION_DAYS` | `30`           | Refresh token family lifetime               |
| `PORT`                          | `3001`         | TCP port to bind                            |
| `CORS_ALLOWED_ORIGINS`          | `FRONTEND_URL` | Comma-separated **exact** origins           |
| `UPLOAD_DIR`                    | `./uploads`    | Local media path when R2 isn't configured   |
| `APP_ENV`                       | `development`  | `production` flips the strict-config guards |
| `ADMIN_NAME`                    | `Admin`        | Display name on the seeded admin            |
| `AUDIT_RETENTION_INTERVAL_SECS` | `86400` (24h)  | Audit-log retention worker tick             |
| `DSAR_SWEEP_INTERVAL_SECS`      | `3600` (1h)    | DSAR artefact TTL sweep worker tick         |
| `IDEMPOTENCY_GC_INTERVAL_SECS`  | `900` (15m)    | Idempotency cache GC worker tick            |

`R2_TEST_*` variables (separate from the runtime `R2_*` set) gate the R2
integration tests against a local MinIO/LocalStack emulator.

---

## Database & migrations

- **Engine:** PostgreSQL 16+
- **Driver:** sqlx 0.8 (compile-time-checked queries via `sqlx::query!`)
- **Strategy:** forward-only — a migration committed to `main` is
  immutable. Editing a checksummed migration after deploy will fail
  boot with `migration N was previously applied but has been modified`.

Current count: **72 migration files** spanning version prefixes
`001–028, 030–039, 041–043, 050–080`. Gaps are intentional (renumbering
fallout from forensic ordering fixes — see commit `620ad09`); every
prefix is unique and validated by CI.

Notable schema regions:

| Theme               | Migrations                               |
| ------------------- | ---------------------------------------- |
| Auth & users        | `001`, `010`, `018`                      |
| Blog & media        | `002`, `004`, `006`–`008`, `016`         |
| Analytics           | `009`, `014`                             |
| Products & catalog  | `030` (products + variants + bundles)    |
| Commerce (cart→ord) | `031`, `035`, `036`, `037`, `038`, `039` |
| Subscriptions       | `041`, `042`, `057`, `067`               |
| Coupons             | `013`, `043`                             |
| Forms               | `027`, `032`–`034`                       |
| Popups              | `015`, `050`–`054`                       |
| Consent / DSAR      | `024`–`028`, `069`, `073`                |
| RBAC                | `021`, `058`, `063`–`068`                |
| Admin observability | `055`, `070`, `072`                      |
| Idempotency         | `017`, `071`, `074`                      |
| Impersonation / IP  | `059`–`061`                              |

Run them locally with `sqlx migrate run`, or just start the binary —
`main.rs` runs `sqlx::migrate!()` before serving.

---

## Background workers

`main.rs` spawns these on a `tokio::broadcast::Receiver` shutdown
channel; each emits Prometheus metrics consumed by
[`ops/prometheus/admin-alerts.rules.yml`](../ops/prometheus/admin-alerts.rules.yml).

| Worker                   | Module                            | Default tick | Metric prefix            |
| ------------------------ | --------------------------------- | ------------ | ------------------------ |
| Audit-log retention      | `services/audit_retention.rs`     | 24h          | `audit_retention_*`      |
| DSAR async job processor | `services/dsar_worker.rs`         | 30s          | `dsar_jobs_*`            |
| DSAR artefact TTL sweep  | `services/dsar_artifact_sweep.rs` | 1h           | `dsar_artifacts_swept_*` |
| Idempotency cache GC     | `services/idempotency_gc.rs`      | 15m          | `idempotency_gc_*`       |

See [`docs/RUNBOOK.md`](../docs/RUNBOOK.md) for diagnosis and
remediation of every alert these emit.

---

## HTTP surface (high-level)

The full machine-readable contract is at `GET /api/openapi.json`. In
production the route requires an admin JWT (`AdminUser` extractor); in
dev it is served by `utoipa-swagger-ui` and SwaggerUI is mounted at
`GET /api/docs`. See `src/openapi.rs` for the gating rationale.

Top-level route prefixes:

| Prefix              | Purpose                                             | Auth                               |
| ------------------- | --------------------------------------------------- | ---------------------------------- |
| `/api/auth/*`       | Register / login / refresh / me / logout            | mixed                              |
| `/api/member/*`     | Self-service (profile, subscription, downloads)     | `AuthUser`                         |
| `/api/admin/*`      | Back-office surface (members, orders, audit, DSAR…) | `AdminUser`                        |
| `/api/catalog/*`    | Public catalog                                      | public                             |
| `/api/blog/*`       | Blog reads                                          | public                             |
| `/api/courses/*`    | Course reads + enrolment progress                   | mixed                              |
| `/api/popups/*`     | Popup serve + impression tracking                   | public                             |
| `/api/forms/*`      | Public form submissions                             | public                             |
| `/api/consent/*`    | Consent record + DSAR submit                        | mixed                              |
| `/api/webhooks/*`   | Stripe (HMAC-verified)                              | webhook                            |
| `/metrics`          | Prometheus scrape endpoint                          | admin-gated in prod, public in dev |
| `/api/openapi.json` | OpenAPI 3.1 spec                                    | admin-gated in prod, public in dev |

`PUT`/`POST`/`DELETE` calls under `/api/admin/*` require an
`Idempotency-Key` header and are subject to per-actor mutation rate
limiting. See [`docs/RUNBOOK.md`](../docs/RUNBOOK.md) §Idempotency.

---

## Testing

```bash
# bring up the test DB (port 5433, separate from the dev DB on host 5434)
docker compose -f docker-compose.yml up -d db

# run the suite
DATABASE_URL_TEST=postgres://postgres:postgres@localhost:5433/swings \
  cargo test
```

Highlights:

- `tests/admin_idempotency.rs::concurrent_same_key_creates_exactly_one_resource`
  — 16-way concurrent race against the idempotency middleware.
- `tests/dsar_r2_artifact.rs` — DSAR worker round-trip against an
  S3-compatible emulator (skips if `R2_TEST_*` not set).
- `tests/authz_matrix.rs` — golden snapshot of the
  `role_permissions` seeded by migration 21.
- `tests/openapi_snapshot.rs` — fails CI if the generated OpenAPI
  schema drifts from `tests/snapshots/openapi.json`.

CI parity:

```bash
cargo fmt --all --check
cargo clippy --all-targets -- -D warnings
cargo test
cargo build --release
```

---

## Local Docker workflows

| Need                                  | Command                                          |
| ------------------------------------- | ------------------------------------------------ |
| Full local stack (api + db + uploads) | `docker compose -f ../docker-compose.yml up`     |
| Test-only Postgres on `:5433`         | `docker compose -f docker-compose.yml up -d db`  |
| Build the production image            | `docker build -f ../Dockerfile -t swings-api ..` |

There is only one Dockerfile, at the repo root (`../Dockerfile`). It
is consumed by Railway and the local `docker-compose.yml` — both from
the repo root as build context. A root `.dockerignore` keeps the
context small.

---

## Production checklist

- All env vars in [Required in production](#required-in-production) are
  set and non-empty.
- `APP_ENV=production` is set.
- Postgres is reachable from the container (Railway uses
  `postgres.railway.internal`).
- The R2 bucket exists and the IAM policy permits `Get/Put/Delete`.
- The Prometheus scraper has access to `/metrics`.
- Alert rules in [`../ops/prometheus/admin-alerts.rules.yml`](../ops/prometheus/admin-alerts.rules.yml)
  are loaded; on-call has access to [`../docs/RUNBOOK.md`](../docs/RUNBOOK.md).

---

## Further reading

- [`../docs/INFRASTRUCTURE.md`](../docs/INFRASTRUCTURE.md) — full topology.
- [`../docs/DEPLOYMENT.md`](../docs/DEPLOYMENT.md) — Vercel + Railway go-live.
- [`../docs/RUNBOOK.md`](../docs/RUNBOOK.md) — operator runbook for every alert.
- [`../docs/wiring/FDN-TESTHARNESS-WIRING.md`](../docs/wiring/FDN-TESTHARNESS-WIRING.md)
  — how the integration test harness is wired.
- [`../docs/wiring/OBSERVABILITY-WIRING.md`](../docs/wiring/OBSERVABILITY-WIRING.md)
  — tracing + metrics scaffolding.
- [`migrations/021_rbac.sql`](./migrations/021_rbac.sql) — live source of
  truth for the role × permission matrix.
