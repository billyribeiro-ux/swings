# AUDIT — PHASE 1: REPOSITORY RECONNAISSANCE

**Project:** `swings` (Precision Options Signals)
**Date:** 2026-04-17
**Stack:** SvelteKit 2.x (Svelte 5.55) + Rust/Axum 0.8 + PostgreSQL 16 + pnpm 10
**Scope:** Read-only forensic audit. No mutations performed.

---

## 1. REPOSITORY TREE (depth 4)

```
swings/
├── .github/
│   └── workflows/
│       └── ci.yml                       # frontend + backend CI (pnpm, cargo)
├── .vscode/ .cursor/ .windsurf/         # editor configs
├── .svelte-kit/                         # generated (ignored)
├── .vercel/                             # generated
├── backend/                             # Rust/Axum API (workspace root = repo)
│   ├── Cargo.toml
│   ├── Dockerfile
│   ├── railway.toml
│   ├── nixpacks.toml
│   ├── migrations/
│   │   ├── 001_initial.sql              # users, refresh_tokens, subs, watchlists, alerts, enrollments
│   │   ├── 002_blog.sql                 # media, categories, tags, posts, junctions, revisions
│   │   ├── 003_password_resets.sql
│   │   ├── 004_media_title.sql
│   │   ├── 005_user_author_profile.sql  # bio/social URLs on users
│   │   ├── 006_post_format.sql
│   │   ├── 007_post_meta.sql
│   │   ├── 008_media_focal.sql
│   │   ├── 009_analytics.sql            # analytics_sessions, analytics_events
│   │   ├── 010_normalize_user_emails.sql
│   │   ├── 011_courses.sql              # courses, modules, lessons, lesson_progress
│   │   ├── 012_pricing_plans.sql        # pricing_plans + pricing_change_log
│   │   ├── 013_coupons.sql              # coupons + coupon_usages
│   │   ├── 014_analytics_enhanced.sql   # sales_events, monthly_revenue_snapshots
│   │   ├── 015_popups.sql               # popups, popup_submissions, popup_events
│   │   ├── 016_blog_trash_meta.sql
│   │   ├── 017_webhook_idempotency.sql  # processed_webhook_events
│   │   └── 018_refresh_token_families.sql
│   └── src/
│       ├── main.rs                      # Axum bootstrap, router wiring, CORS, ServeDir
│       ├── config.rs                    # env-backed Config + prod readiness check
│       ├── db.rs                        # ~1800 lines of sqlx query functions (all domains)
│       ├── email.rs                     # Lettre SMTP + Tera inline HTML templates
│       ├── error.rs                     # thiserror AppError + IntoResponse
│       ├── extractors.rs                # AuthUser / AdminUser / OptionalAuthUser JWT extractors
│       ├── middleware.rs                # (re-exports rate_limit module)
│       ├── middleware/
│       │   └── rate_limit.rs            # tower_governor per-route layers
│       ├── models.rs                    # ~1280 lines — all domain types + DTOs
│       ├── stripe_api.rs                # Stripe helpers (not read — flagged)
│       ├── services/
│       │   ├── mod.rs
│       │   └── storage.rs               # R2 (S3-compat) via aws-sdk-s3 + MediaBackend enum
│       └── handlers/
│           ├── mod.rs
│           ├── admin.rs                 # dashboard, analytics, members, watchlists, alerts
│           ├── analytics.rs             # ingest events
│           ├── auth.rs                  # register/login/refresh/me/logout/forgot/reset
│           ├── blog.rs                  # posts/categories/tags/media/revisions/meta
│           ├── coupons.rs               # full CRUD + validate + apply + bulk + stats
│           ├── courses.rs               # admin CRUD, modules, lessons, enroll, progress
│           ├── member.rs                # profile, subscription, watchlists, enrollments
│           ├── popups.rs                # admin CRUD + public active/event/submit
│           ├── pricing.rs               # plans CRUD + change log
│           ├── subscriptions.rs         # admin list + stats (NEW, untracked file)
│           └── webhooks.rs              # Stripe webhook (HMAC verify + idempotency)
├── src/                                 # SvelteKit app
│   ├── app.css, app.d.ts, app.html
│   ├── hooks.client.ts
│   ├── hooks.server.ts                  # security headers, immutable cache, error id
│   ├── service-worker.ts
│   ├── styles/
│   │   ├── tokens.css                   # HEX tokens (NOT OKLCH — constraint violation)
│   │   ├── reset.css
│   │   └── global.css
│   ├── lib/
│   │   ├── analytics/                   # AnalyticsBeacon + cta tracking
│   │   ├── api/                         # ApiClient (fetch + refresh), types.ts (hand-written)
│   │   ├── assets/ client/ data/ seo/ stores/ utils/
│   │   └── components/
│   │       ├── admin/ (analytics/)
│   │       ├── charts/ editor/ landing/ popups/ traders/ ui/
│   └── routes/
│       ├── +layout.svelte, +layout.ts, +page.svelte, layout.css
│       ├── about/ blog/ (category, tag, [slug]) courses/ ([slug])
│       ├── pricing/ (monthly, annual) privacy/ terms/ success/
│       ├── register/ login/
│       ├── dashboard/
│       │   ├── +layout.svelte, +layout.ts, +page.svelte
│       │   ├── account/ courses/[slug]/ watchlists/[id]/
│       ├── admin/
│       │   ├── +layout.svelte, +layout.ts, +page.svelte
│       │   ├── analytics/ author/ blog/ (categories, tags, media, new, [id])
│       │   ├── coupons/ (new, [id])
│       │   ├── courses/ (new, [id])
│       │   ├── forgot-password/ reset-password/
│       │   ├── members/[id]/
│       │   ├── popups/ (new, [id])
│       │   ├── settings/
│       │   ├── subscriptions/plans/
│       │   └── watchlists/ (new, [id])
│       ├── sitemap.xml/
│       └── api/
│           ├── create-checkout-session/ # SvelteKit server endpoint (Stripe)
│           └── greeks-pdf/
├── scripts/                             # seo-audit etc.
├── static/                              # public assets
├── e2e/                                 # Playwright
├── docs/                                # project docs
├── package.json                         # SvelteKit root (no workspace)
├── pnpm-workspace.yaml                  # only `onlyBuiltDependencies: [esbuild]`
├── svelte.config.js                     # adapter-vercel nodejs22.x
├── vite.config.ts                       # proxies /api → 127.0.0.1:3001
├── tsconfig.json                        # strict, bundler resolution
├── eslint.config.js
├── playwright.config.ts
├── vitest.browser.config.ts
├── docker-compose.yml                   # postgres:16-alpine + backend image
├── Dockerfile                           # frontend (unused? root-level)
├── vercel.json                          # rewrites /api/* → Railway prod URL
├── render.yaml                          # legacy Render config (docker)
├── .env.example                         # Stripe + SW toggle only
└── (many markdown audit/docs files)
```

---

## 2. SUBSYSTEM INVENTORY

### 2.1 Rust Crate (`backend/swings-api v0.1.0`, edition 2021)

**Direct dependencies (from `backend/Cargo.toml`):**

| Crate                            | Pinned                                               | Latest (Apr 2026) | Notes                                               |
| -------------------------------- | ---------------------------------------------------- | ----------------- | --------------------------------------------------- |
| `axum`                           | 0.8 (+macros,multipart)                              | 0.8.x             | Current.                                            |
| `axum-extra`                     | 0.12 (+typed-header, cookie)                         | 0.12.x            | Current.                                            |
| `tokio`                          | 1 (full)                                             | 1.48+             | Unpinned minor.                                     |
| `tower`                          | 0.5                                                  | 0.5.x             | Current.                                            |
| `tower-http`                     | 0.6 (cors, trace, fs)                                | 0.6.x             | Current.                                            |
| `sqlx`                           | 0.8 (postgres+chrono+uuid+json+rust_decimal+migrate) | 0.8.x             | Current.                                            |
| `jsonwebtoken`                   | 10                                                   | 10.x              | Current.                                            |
| `argon2`                         | 0.5                                                  | 0.5.x             | Current.                                            |
| `serde`/`serde_json`             | 1                                                    | 1.x               | Current.                                            |
| `uuid`                           | 1 (v4, serde)                                        | 1.x               | Current.                                            |
| `chrono`                         | 0.4 (serde)                                          | 0.4.x             | Current.                                            |
| `dotenvy`                        | 0.15                                                 | 0.15.x            | Current.                                            |
| `tracing` / `tracing-subscriber` | 0.1 / 0.3                                            | Current.          |
| `thiserror`                      | 2                                                    | 2.x               | Current.                                            |
| `anyhow`                         | 1                                                    | 1.x               | Current.                                            |
| `sha2` / `hmac`                  | 0.10 / 0.12                                          | Current.          |
| `validator`                      | 0.20 (derive)                                        | 0.20+             | Current.                                            |
| `stripe-rust` (`async-stripe`)   | 0.39 (tokio-hyper)                                   | —                 | Package `async-stripe`; check for newer if needed.  |
| `axum_typed_multipart`           | 0.16                                                 | 0.16+             | Current.                                            |
| `tempfile`                       | 3                                                    | —                 | Current.                                            |
| `sanitize-filename`              | 0.6                                                  | —                 | Current.                                            |
| `lettre`                         | 0.11 (tokio1-rustls, smtp, builder)                  | 0.11.x            | Current.                                            |
| `tera`                           | 1                                                    | 1.x               | Templates inlined as `const` strings (not on-disk). |
| `rand`                           | 0.8                                                  | 0.9.x exists      | **Slightly outdated** (rand 0.9). Minor.            |
| `rust_decimal`                   | 1 (serde-with-str)                                   | 1.x               | Current.                                            |
| `aws-sdk-s3`                     | 1 (rt-tokio, rustls)                                 | 1.x               | Current.                                            |
| `bytes`                          | 1                                                    | 1.x               | Current.                                            |
| `governor` / `tower_governor`    | 0.10 / 0.8                                           | Current.          |

**Missing crates that Phase 3/4 will require** (not present today): `reqwest` (carrier APIs, SaaS webhooks), `html-escape`/`ammonia` (HTML sanitisation for user content & forms), `ulid` or strong order-number generator, `jiff` or equivalent for complex TZ logic in subscriptions, `opentelemetry` for spans, `base64`, `urlencoding`, `woothee`/`uap-core` (UA parsing for consent + analytics), `maxminddb` (geo for consent), `lazy_static`/`once_cell` for regex caches, `quick-xml`/`printpdf` or `wkhtmltopdf` bridge (invoices/packing slips), `mjml-rust` or external MJML renderer, `webpush`/`web-push` (WebPush VAPID), `rusqlite` N/A, `async-channel`/`crossbeam` (event bus), `redis`/`deadpool-redis` (queue / cache / rate limiter for distributed multi-instance).

### 2.2 npm / pnpm (root `package.json`)

Engines: `node >=24.14.1`. Lockfile: `pnpm-lock.yaml`. Workspace file exists but declares only `onlyBuiltDependencies: [esbuild]` — **not a multi-package workspace**; Rust API lives in `backend/` out-of-band.

**Runtime deps (excerpt):**

| Package                                                | Pinned        | Notes                                                     |
| ------------------------------------------------------ | ------------- | --------------------------------------------------------- |
| `svelte`                                               | ^5.55.3       | Current (5.x). Runes available.                           |
| `@sveltejs/kit`                                        | ^2.57.1       | Current.                                                  |
| `@sveltejs/vite-plugin-svelte`                         | ^7            | Current.                                                  |
| `@sveltejs/adapter-vercel`                             | ^6.3.3        | Active adapter.                                           |
| `@sveltejs/adapter-netlify`                            | ^6.0.4        | Unused — prune candidate.                                 |
| `@sveltejs/adapter-auto`                               | ^7.0.1        | Unused — prune candidate.                                 |
| `vite`                                                 | ^8.0.8        | Current.                                                  |
| `vitest`                                               | ^4.1.4        | Current.                                                  |
| `@vitest/browser-playwright` / `vitest-browser-svelte` | 4.1.x / 2.1.x | Current.                                                  |
| `typescript`                                           | ^6.0.2        | **TS 6.x — current**. `tsconfig` is strict.               |
| `typescript-eslint`                                    | ^8.58.1       | Current.                                                  |
| `eslint`                                               | ^10.2.0       | Current.                                                  |
| `@playwright/test` / `playwright`                      | ^1.59.1       | Current.                                                  |
| `prettier`                                             | ^3.8.2        | Current.                                                  |
| `stripe`                                               | ^22.0.1       | Client-side SDK.                                          |
| `phosphor-svelte`                                      | ^3.1.0        | **Phosphor present — compliant**. No `lucide-*` detected. |
| `@tiptap/*`                                            | ^3.22.3       | Full TipTap 3 stack for `PostEditor`.                     |
| `@threlte/core` / `@threlte/extras`                    | 8.5 / 9.14    | 3D (unused by audit target domains).                      |
| `apexcharts`, `d3`, `gsap`, `three`                    | —             | Presentation-only.                                        |
| `date-fns` + `@date-fns/tz`                            | 4.x / 1.4     | Current.                                                  |
| `@vercel/speed-insights`                               | 2.0.0         | Current.                                                  |
| `@types/three`, `@types/d3`                            | —             | fine.                                                     |

**Legacy/unexpected:** `adapter-auto` + `adapter-netlify` shipped alongside `adapter-vercel`; two unused.

**Build-dep flags:** `pnpm.peerDependencyRules.allowedVersions['vite-plugin-devtools-json>vite']: '*'` — intentional Vite peer pin override.

### 2.3 SvelteKit app structure

- **Router:** file-based under `src/routes`. Public, `dashboard/` (member), `admin/` tree enumerated in Section 1.
- **Server endpoints:** only `src/routes/api/create-checkout-session/+server.ts` and `src/routes/api/greeks-pdf/+server.ts` + `sitemap.xml/+server.ts`. All real API surface lives in Rust.
- **Hooks:**
  - `hooks.server.ts` — adds `X-Frame-Options`, `X-Content-Type-Options`, `Referrer-Policy`, `Permissions-Policy`, immutable cache for `/_app/immutable/`. Has `handleError` with correlation id logging. **No CSP header today.**
  - `hooks.client.ts` — exists (service-worker policy). Not read in full.
- **API client (`src/lib/api/client.ts`):** fetch wrapper with bearer auth injection, single-flight refresh token rotation on 401. No request-id propagation, no telemetry hooks.
- **Types (`src/lib/api/types.ts`):** ~788 lines, hand-maintained, mirrors Rust DTOs across all domains. Drift risk — no codegen.
- **CSS / design tokens:** `src/styles/tokens.css` is a **HEX-based** token set (`--color-navy: #0b1d3a` etc.). **NOT OKLCH. Missing `@layer` cascade. No nine-tier breakpoint system (xs→xl5).** Constraint violation vs. PE7 mandate — migration required.

### 2.4 Database (migrations 001–018; schema version `018`)

Applied at runtime via `sqlx::migrate!("./migrations")` in `main.rs`. Enum types (`user_role`, `subscription_plan`, `subscription_status`, `trade_direction`, `post_status`, `discount_type`) are created in-place.

**Tables today (by migration origin):**

| Table                                                                                                         | Migration                          | Purpose                                                                         |
| ------------------------------------------------------------------------------------------------------------- | ---------------------------------- | ------------------------------------------------------------------------------- |
| `users`                                                                                                       | 001 (+005 profile, +010 normalize) | auth identity, role, author profile                                             |
| `refresh_tokens`                                                                                              | 001 (+018 families/used)           | JWT refresh rotation with family-based reuse detection                          |
| `subscriptions`                                                                                               | 001                                | Stripe-backed member subscriptions, enum plan/status                            |
| `watchlists`, `watchlist_alerts`                                                                              | 001                                | weekly trade setups                                                             |
| `course_enrollments`                                                                                          | 001 (+011 last_lesson)             | high-level enrollment tracker                                                   |
| `media`                                                                                                       | 002 (+004 title, +008 focal)       | image/file library, R2 or local                                                 |
| `blog_categories` / `blog_tags` / `blog_posts` / `blog_post_categories` / `blog_post_tags` / `blog_revisions` | 002 (+006 format, +016 trash)      | full blog CMS                                                                   |
| `password_reset_tokens`                                                                                       | 003                                | 1-hour reset tokens                                                             |
| `post_meta`                                                                                                   | 007                                | WP-style key/value meta                                                         |
| `analytics_sessions`, `analytics_events`                                                                      | 009                                | page_view / impression / click events                                           |
| `courses`, `course_modules`, `course_lessons`, `lesson_progress`                                              | 011                                | LMS                                                                             |
| `pricing_plans`, `pricing_change_log`                                                                         | 012                                | admin-editable plans + audit log                                                |
| `coupons`, `coupon_usages`                                                                                    | 013                                | discount codes + per-user usage                                                 |
| `sales_events`, `monthly_revenue_snapshots`                                                                   | 014                                | revenue analytics (model present, **unused** — `#[allow(dead_code)]` on struct) |
| `popups`, `popup_submissions`, `popup_events`                                                                 | 015                                | basic popup builder                                                             |
| `processed_webhook_events`                                                                                    | 017                                | Stripe webhook idempotency (event_id unique)                                    |

### 2.5 Auth / Session / Middleware

- **Password hashing:** Argon2 (default params). OsRng salt.
- **Tokens:** JWT (HS256 default header) with custom `Claims { sub: Uuid, role, iat, exp }`.
- **Refresh flow:** SHA-256-hashed refresh tokens persisted; single-use; reuse detection drops entire `family_id` (good). Refresh endpoint issues new access + new refresh token.
- **Extractors:** `AuthUser`, `AdminUser` (403 if role ≠ admin), `OptionalAuthUser` (Infallible). Hardcoded `role == "admin"` string compare.
- **Rate limiting:** `tower_governor` + `SmartIpKeyExtractor` (reads `X-Forwarded-For`). Layers: login 5/min, register 10/hr, forgot-password 3/hr, analytics 120/s. **Only applied to a handful of auth routes** — not applied to checkout, webhooks, coupon-apply, popup-submit, media-upload.
- **CORS:** permissive headers via `Any`, origins from env (`CORS_ALLOWED_ORIGINS` or `FRONTEND_URL`). Panics on empty.
- **Frontend auth store:** `src/lib/stores/auth.svelte.ts` (runes). Tokens held client-side; `access_token` sent as `Authorization: Bearer`.

### 2.6 Payments / Stripe

- **Crate:** `async-stripe` 0.39 (runtime tokio-hyper).
- **Config:** `STRIPE_SECRET_KEY`, `STRIPE_WEBHOOK_SECRET`; both required in prod (panic if empty).
- **Public:** `src/routes/api/create-checkout-session/+server.ts` (SvelteKit endpoint — separate from Rust API).
- **Webhook:** `POST /api/webhooks/stripe` — manual HMAC-SHA256 signature verification with 5-minute tolerance, idempotency via `processed_webhook_events`, 1%-probabilistic cleanup. Handled event types: `customer.subscription.created/updated`, `customer.subscription.deleted`, `checkout.session.completed`. **Note:** `handle_checkout_completed` seeds plan as `Monthly` blindly and assumes 30-day period — corrected later by `subscription.updated`. Unhandled (acknowledged) events include `invoice.*`, `payment_intent.*`, `charge.*`, dispute events, dunning.
- **Products/orders model:** none. Product catalog absent. No cart, checkout, order, invoice, refund, tax, shipping, inventory, or payment-method-vault logic.
- **Rust `stripe_api.rs`:** present but not inspected in this pass.

### 2.7 Email / Notifications

- **Transport:** `lettre` 0.11 with STARTTLS relay + SMTP credentials; optional dangerous-no-auth for local dev.
- **Templates:** 4 inline Tera templates hard-coded as `const` strings in `src/email.rs` — `base`, `password_reset`, `welcome`, `subscription_confirmation`, `subscription_cancelled`. **No DB storage, no admin UI, no preview, no i18n, no MJML, no versioning, no test-send, no broadcast.**
- **Senders wired:** `send_password_reset` only. `send_welcome`, `send_subscription_confirmation`, `send_subscription_cancelled` exist but are **not called from any handler or webhook** (verified by reading `handlers/auth.rs` and `handlers/webhooks.rs`).
- **Bounce/complaint/unsubscribe:** none.
- **Delivery log / open / click tracking:** none.
- **Channels:** email only. **No SMS, push, Slack, Discord, webhook-out, in-app inbox.**
- **Orchestration:** direct-call from request path; no queue, no retry, no DLQ, no quiet hours, no preference centre.

### 2.8 Existing forms / popups / consent / e-commerce partials

- **Forms:** **No general form builder.** Only per-popup form captured as free-form `form_data JSONB` in `popup_submissions`. No field typing, validation metadata, server-side schema, integrations, conditional logic, multi-step, file uploads, or anti-spam.
- **Popups:** `popups` / `popup_submissions` / `popup_events` tables; handlers CRUD + active list + event track + submit. Supported enums per-column CHECK constraints: 6 popup types, 7 trigger types, 4 display frequencies. **No A/B testing, no template library, no scroll/exit-intent admin UI affordances, no revenue attribution, no focus trap or a11y primitives verified.** Frontend components exist under `src/lib/components/popups/` (not inspected in full).
- **Consent management:** **none.** No cookie banner, no consent log, no DSAR workflow, no geo detection, no Consent Mode v2, no TCF v2.2, no GPC honoring, no i18n. `src/routes/privacy/` and `terms/` are static pages only.
- **E-commerce:** **none beyond subscriptions + courses.** No products, catalog, cart, order, tax, shipping, inventory, memberships, wishlist, B2B, or marketplace. `course_enrollments.course_id TEXT` is loosely typed (course rows exist but enrollments reference by text, not UUID FK).
- **Coupons:** Feature-rich DB schema (percentage / fixed / free_trial, per-user limits, plan/course scoping, date ranges, Stripe coupon mirroring fields). `handlers/coupons.rs` implements CRUD, validate, apply, bulk, stats. However: **not integrated with any cart/order flow** (no cart exists); can only be applied at Stripe Checkout session creation from SvelteKit endpoint.
- **Memberships (WC Memberships parity):** none. Members = Stripe subscribers only. No content drip, URL locking, tiered access, member pricing, drip emails, forum/course hooks.

### 2.9 Root configs actually read

`package.json`, `pnpm-workspace.yaml`, `svelte.config.js`, `vite.config.ts`, `tsconfig.json`, `.env.example`, `docker-compose.yml`, `vercel.json`, `render.yaml`, `backend/Cargo.toml`, `.github/workflows/ci.yml`.

**Notable findings:**

- `svelte.config.js` adapter: Vercel `nodejs22.x`. Prerender entries: `/`, `/about`, `/courses`, `/blog`, `/pricing`, `/pricing/monthly`, `/pricing/annual`.
- `vite.config.ts` dev proxy `/api` → `http://127.0.0.1:3001`.
- `vercel.json` production rewrite `/api/*` → `https://swings-production.up.railway.app/api/*` (Railway is the prod API host; Render config is stale).
- `.env.example` only documents Stripe + `PUBLIC_APP_URL` + SW dev flag — missing: `DATABASE_URL`, `JWT_SECRET`, SMTP, R2, `CORS_ALLOWED_ORIGINS`, `ADMIN_EMAIL/PASSWORD`, `APP_ENV`. `backend/.env` expected but not in repo.
- `docker-compose.yml`: postgres:16-alpine + backend image; env passthrough.
- CI (`.github/workflows/ci.yml`): one workflow, two jobs (`frontend` runs `pnpm ci:frontend`, `backend` runs `cargo fmt/clippy/test/build`). No lint of SQL, no Playwright in CI, no security scans (cargo-audit, pnpm audit), no coverage, no migration-forward check.

---

## 3. DEPENDENCY & CONSTRAINT FLAGS (vs. PE7 mandate)

- **CSS constraint violations (design system):**
  1. `src/styles/tokens.css` uses hex, not OKLCH.
  2. No `@layer reset, base, tokens, layout, components, utilities, overrides` cascade declared.
  3. No nine-tier breakpoints (xs 320 → xl5 3840) — tokens file ends with container sizes only (visible to line 120; full file not scanned).
  4. `clamp()` fluid type not evidenced in tokens.
  5. Logical properties not enforced (not grep-verified; flagged for Phase 2).
  6. Comment in tokens explicitly says _"1:1 mapping from Tailwind theme"_ — suggests prior Tailwind origin; verify no `tailwind.config.*` or `@tailwind` directives remain (Glob pass showed none in top level; deep scan in Phase 2).
- **Icons:** `phosphor-svelte` present. No `lucide-*` package in `package.json` — **compliant**.
- **pnpm / TS strict / Svelte 5:** compliant.
- **Rust hygiene:** **NOT compliant with mandate** — `#![deny(warnings)]` and `#![forbid(unsafe_code)]` are **not** declared in `main.rs`. `#[allow(dead_code)]` present on `SalesEvent`, `MonthlyRevenueSnapshot`, `RevenueAnalytics`, etc. (unused revenue types). Several `.unwrap()` / `.expect()` in prod paths (pool config expects, `panic!` in config assert, `expect("non-zero quota")` in rate-limit layer builders).
- **Money modeling:** generally `amount_cents: i32` (compliant), but coupons use `discount_value NUMERIC(10,2)` + `rust_decimal::Decimal` (mixed) and public `CreateCouponRequest.discount_value: f64` (violates "never float for money"). Needs migration strategy.
- **Timestamp modeling:** `timestamptz` everywhere; RFC3339 serialisation via chrono — compliant.
- **Idempotency:** Stripe webhook has it (`processed_webhook_events`). **No Idempotency-Key header support on any mutating payment endpoint** (there are no payment endpoints other than Stripe Checkout via SvelteKit server endpoint; this gap becomes significant in Phase 3 when order/checkout APIs are added).
- **Webhook signature verification:** only Stripe today. No SendGrid/Resend/Twilio/etc inbound.

---

## 4. RISK / ATTENTION ITEMS SURFACED IN PHASE 1

1. **Uncommitted changes.** `git status` shows modifications to `config.rs`, `db.rs`, `error.rs`, `extractors.rs`, 7 handlers, `main.rs`, `rate_limit.rs`, `models.rs`, `storage.rs`, 6 Svelte files, and a new untracked `handlers/subscriptions.rs`. The audit was performed on the working-tree state (which includes these). Before Phase 4, these must either land on the branch or be stashed so migrations/handlers are not written against an unstable base.
2. **`handlers/subscriptions.rs` is untracked** but is wired by `main.rs` (`handlers::subscriptions::admin_router()`). Build will fail on a clean checkout. Confirmed file exists and is non-trivial (154 lines).
3. **`stripe_api.rs` not inspected** in this pass; may contain active Stripe client usage relevant to Phase 2 gap analysis. Flagged.
4. **Render config (`render.yaml`) stale.** Production is Railway per `vercel.json` rewrite. Either prune `render.yaml` or re-purpose.
5. **Types drift.** `src/lib/api/types.ts` is hand-maintained and duplicates every Rust DTO. No generator exists. Phase 3 must select OpenAPI or `ts-rs`/`specta`.
6. **Email senders exist but are never invoked** for welcome / subscription lifecycle. Low-effort wiring, but currently dead.
7. **Admin authz is role-string compare.** No policy matrix, no per-resource scopes, no row-level authorisation (e.g. blog author can only edit own drafts). Fine for today's admin-only surface, insufficient for WooCommerce-class authz.
8. **No CSP** in `hooks.server.ts`. Mandatory before we render third-party-consent-gated scripts in Phase 4.
9. **Multi-instance concerns.** `tower_governor` is in-process only — distributed rate-limit will need Redis or DB-backed quota before horizontal scale. Session store (JWT) is stateless (fine). Consent, popup frequency, abandoned-cart, and queue subsystems will push the architecture toward a shared backing store (Redis recommended).
10. **`sales_events` table unused.** Revenue analytics struct family is `#[allow(dead_code)]`. MRR/ARR is computed at read-time from `subscriptions` × `pricing_plans` in `db::admin_estimated_mrr_arr_cents` — event-sourced revenue not yet reconciled.

---

## 5. OUT-OF-SCOPE FOR PHASE 1 (carried to Phase 2)

- Deep-read of `stripe_api.rs`, `handlers/coupons.rs`, `handlers/popups.rs`, `handlers/courses.rs`, `handlers/pricing.rs`, `handlers/blog.rs` implementations (only router surfaces + signatures enumerated).
- Full read of `src/lib/components/popups/*`, `src/lib/components/editor/PostEditor.svelte`, `src/lib/stores/auth.svelte.ts`, `hooks.client.ts`.
- Grep sweep for Tailwind residue, `lucide`, `any`, `@ts-ignore`, `as unknown as`, `.unwrap()` in prod paths.
- Full `tokens.css` (120 lines read out of unknown total; breakpoint system to verify) and `global.css` / `reset.css`.
- `.env.example` gap vs. actual `Config::from_env` requirements (inventory for env schema).

---

## 6. CHECKPOINT

Phase 1 deliverable complete. **Awaiting your review/approval before beginning Phase 2** (domain gap matrices for e-commerce, forms, popups, consent, notifications).
