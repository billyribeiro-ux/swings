# AUDIT — PHASE 3: IMPLEMENTATION PLAN

**Date:** 2026-04-17
**Scope:** Dependency-ordered plan to close every gap enumerated in Phase 2 while respecting all PE7 constraints (pnpm, Svelte 5 runes, OKLCH tokens, `@layer` cascade, 9 breakpoints, TypeScript strict, Rust `deny(warnings)` + `forbid(unsafe_code)`, money in integer minor units, timestamps `timestamptz`, Idempotency-Key on payments, webhook signature verification).

---

## 0. DEFAULT DECISIONS (assumed; flag any to override)

I need six architectural calls before the plan is actionable. You did not supply them, so I am proceeding on the following defaults. Each has a concrete rationale; any can be reversed with limited rework before Phase 4 begins on its dependent subsystem.

| #   | Decision                   | Default                                                                                                                                                                                                       | Rationale                                                                                                                                                                                                                |
| --- | -------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| D1  | **E-commerce scope**       | **Digital-goods trim** (products: simple + subscription + downloadable + bundle of digital). No shipping, no physical inventory, no B2B sub-users, no multi-location stock. US Stripe Tax + EU VAT MOSS only. | The business is a trader-signals + LMS SaaS; shipping/inventory parity would be gold-plating. All targets (subs, courses, downloadables, memberships) are digital. If I'm wrong, raise scope **before FDN phase exits**. |
| D2  | **Type contract**          | **`utoipa` (Rust annotation-first) → OpenAPI 3.1 → `openapi-typescript` codegen for frontend**.                                                                                                               | Keeps Rust as single source of truth; no hand-written `src/lib/api/types.ts` drift. `utoipa` integrates with axum via `utoipa-axum`. Frontend regenerates types in `pnpm check` / prebuild.                              |
| D3  | **Queue / event bus**      | **Postgres-backed transactional outbox** (`outbox_events` table + background worker pool in the API binary; room to swap for `pgmq` extension or Redis Streams later).                                        | We are already 100% on Postgres. Redis adds ops surface (and $) for something a transactional outbox + `LISTEN`/`NOTIFY` covers. Semantics: at-least-once with idempotency-keyed consumers.                              |
| D4  | **Email provider**         | **Resend** (API-based) for transactional + marketing. Lettre SMTP retained as a dev-only fallback behind `EMAIL_PROVIDER=smtp`.                                                                               | Delivery webhooks (bounce / complaint / open / click), built-in suppression, React-Email / MJML compile support. Aligns with notifications DLQ.                                                                          |
| D5  | **Geo detection**          | **Cloudflare `CF-IPCountry` + Vercel `X-Vercel-IP-Country` edge headers primary**, **MaxMind GeoLite2 (`maxminddb` crate) embedded fallback** when the API is hit directly (Railway origin).                  | Zero-latency in the common path (edge-forwarded); accurate fallback for direct traffic and for city-level detail when needed.                                                                                            |
| D6  | **CSS rebuild sequencing** | **Phase 3.5 prerequisite** — PE7 token migration lands before any new admin/public UI ships in Phase 4.                                                                                                       | New Svelte components must not inherit hex tokens. Existing surfaces keep hex tokens behind an `@layer overrides` compatibility shim until each surface is migrated.                                                     |

I will also assume:

- **Multi-tenancy:** single tenant (this one site). No saleable "for other orgs" multi-tenancy in scope.
- **i18n surface:** consent banner + consent log + DSAR flows are i18n-ready (required by EU regs). Commerce/UI i18n is out of scope for Phase 4 v1.
- **Feature flags:** simple env-var gating (`ENABLE_CONSENT_BANNER=1` etc.). No LaunchDarkly/ConfigCat.
- **Observability stack:** `tracing` → JSON formatter → stdout (consumed by Railway / Vercel logs). Metrics via `metrics` crate → Prometheus `/metrics` endpoint (admin-auth-gated). OpenTelemetry integration is a Phase 5 follow-up.

---

## 1. DEPENDENCY ORDER (subsystem DAG)

```
FDN (foundations)
 ├─ FDN-01  Crate hygiene + error model
 ├─ FDN-02  utoipa + OpenAPI emission
 ├─ FDN-03  PE7 CSS migration            ◄── all new UI depends on this
 ├─ FDN-04  Transactional outbox + worker pool
 ├─ FDN-05  Notifications core (channels, preferences, retry, unsubscribe)
 ├─ FDN-06  Geo + UA + HTML sanitizer + money types
 ├─ FDN-07  Authz policy engine
 ├─ FDN-08  Security headers (CSP) + rate-limit expansion
 └─ FDN-09  Email provider cutover (Resend)

CONSENT (depends: FDN-03, FDN-05, FDN-06, FDN-08)
 ├─ CONSENT-01  Banner + category model
 ├─ CONSENT-02  Script blocker + consent gate
 ├─ CONSENT-03  Consent log + DSAR workflow
 ├─ CONSENT-04  Consent Mode v2 + TCF v2.2 + GPC
 ├─ CONSENT-05  Geo + region variants
 ├─ CONSENT-06  i18n framework + translations
 └─ CONSENT-07  Admin UI + policy versioning

FORM (depends: FDN-04, FDN-05, FDN-06)
 ├─ FORM-01  Schema model + field library
 ├─ FORM-02  Validation engine (server + client)
 ├─ FORM-03  Submission store + audit
 ├─ FORM-04  Multi-step + conditional + save/resume + repeater
 ├─ FORM-05  File uploads (chunked → R2)
 ├─ FORM-06  Anti-spam (honeypot + Turnstile + rate-limit + dedupe)
 ├─ FORM-07  Integrations (webhook-out, Mailchimp, ActiveCampaign, HubSpot, Zapier, Google Sheets, Notion, Airtable)
 ├─ FORM-08  Payment field (Stripe)
 ├─ FORM-09  Admin builder (drag/drop + live preview)
 └─ FORM-10  Public renderer

POP (depends: FORM-01..FORM-10, FDN-06 geo/UA)
 ├─ POP-01  Expanded triggers + targeting (geo, UTM, URL regex, cart, tier, time/day, returning visitor)
 ├─ POP-02  A/B testing engine + winner promotion
 ├─ POP-03  Template library + cloneable presets
 ├─ POP-04  Additional types (content locker, countdown bar, gamified spin/scratch)
 ├─ POP-05  Server-side frequency capping + per-user rules
 ├─ POP-06  Revenue attribution + form embedding

EC (depends: FDN, CONSENT, FORM where forms drive EC events)
 ├─ EC-01  Product model (simple + sub + downloadable + bundle, attributes, variations)
 ├─ EC-02  Catalog (facets, search, filter, sort, pagination)
 ├─ EC-03  Cart (persistent, guest+authed, merge, fees, coupons, totals)
 ├─ EC-04  Checkout (custom Stripe Elements, saved PMs, VAT capture)
 ├─ EC-05  Order state machine + refunds + notes + emails
 ├─ EC-06  Invoice + receipt PDFs
 ├─ EC-07  Digital delivery (expiring signed URLs, download quotas)
 ├─ EC-08  Tax (Stripe Tax primary; manual rate fallback)
 ├─ EC-09  Subscriptions 2.0 (pause, upgrade/downgrade, proration, dunning, switching, early renewal)
 ├─ EC-10  Memberships (plans, restriction engine, drip, member pricing)
 ├─ EC-11  Coupon engine refactor (Decimal, cart-integrated, BOGO, category-scoped)
 └─ EC-12  Reports (event-sourced MRR/ARR/LTV/churn/cohorts)
```

Arrows are hard preconditions; each subsystem inside a group can be built in the listed order.

---

## 2. FOUNDATIONS (FDN)

### FDN-01 — Rust crate hygiene + error-model refactor

- `backend/src/main.rs` top of file: `#![deny(warnings)] #![forbid(unsafe_code)]`.
- Delete every `#[allow(dead_code)]` — either wire the type up or remove it.
- Replace every `.unwrap()` / `.expect()` in prod paths (enumerated in Phase 2 §B) with typed errors. Keep `expect()` only at startup assertions (`main.rs`) with explicit safety comments.
- Extend `AppError` with: `Unprocessable(String)`, `TooManyRequests`, `Conflict(String)` (exists), `ServiceUnavailable(String)`, `PayloadTooLarge(String)`, `NotImplemented`.
- Introduce `Problem` JSON body (RFC 7807): `{ "type": "...", "title": "...", "status": 0, "detail": "...", "instance": "..." }`. `IntoResponse` emits Problem+JSON with `Content-Type: application/problem+json`.
- **Migrations:** none.
- **Tests:** `cargo test -p swings-api --lib error::` — round-trip each variant → Problem response and back.

### FDN-02 — `utoipa` + OpenAPI 3.1 emission

- Add to `Cargo.toml`: `utoipa = { version = "5", features = ["axum_extras", "chrono", "uuid", "preserve_order"] }`, `utoipa-axum`, `utoipa-swagger-ui`.
- Annotate every handler + DTO with `#[utoipa::path]` + `#[derive(ToSchema)]`.
- `main.rs`: mount `/api/openapi.json` (admin-gated in prod, public in dev) and `/api/docs` (Swagger UI, admin-gated).
- Frontend: add `scripts/openapi-to-ts.mjs` that runs `openapi-typescript http://localhost:3001/api/openapi.json -o src/lib/api/schema.d.ts` into a generated file; delete hand-written `src/lib/api/types.ts` once every consumer uses the generated types.
- Add `pnpm gen:types` to `package.json` and prepend to `pnpm check` and `ci:frontend`.
- **Migrations:** none.
- **Tests:** snapshot of `openapi.json` committed to `backend/tests/snapshots/openapi.json`; diff assertion in CI.

### FDN-03 — PE7 CSS migration

Replace `src/styles/tokens.css`, `global.css`, `reset.css`, delete unused `src/routes/layout.css`.

New files:

- `src/styles/layers.css` — `@layer reset, base, tokens, layout, components, utilities, overrides;`
- `src/styles/tokens.css` — OKLCH color scales for navy/teal/gold/neutral/status, typographic scale using `clamp()` with the 9-tier breakpoint system, spacing, radii, shadows, z-index, motion.
- `src/styles/breakpoints.css` — the 9-tier system:
  ```
  xs  320  (default)
  sm  480
  md  640
  lg  768
  xl  1024
  2xl 1280
  3xl 1536
  4xl 1920
  5xl 3840
  ```
  Exposed as custom-property-driven container queries on root `:where(html)`.
- `src/styles/reset.css` — PE7 reset, `@layer reset`.
- `src/styles/global.css` — base element styles, `@layer base`.
- `src/styles/utilities.css` — `.stack-*`, `.cluster`, `.visually-hidden`, motion/animation utilities, `@layer utilities`.
- `src/styles/overrides.css` — legacy hex-token compatibility shim that maps old `--color-navy` etc. to the new OKLCH scale; marked `@layer overrides` so migrated components can ignore it.
- Logical properties everywhere (`margin-block`, `padding-inline`, `inset-inline-start`, etc.).
- **Migrations:** none.
- **Tests:** Playwright visual-regression pass on `/`, `/pricing`, `/blog`, `/admin` before/after.

### FDN-04 — Transactional outbox + worker pool

- **Migration `019_outbox.sql`:**

  ```sql
  CREATE TABLE outbox_events (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    aggregate_type  TEXT NOT NULL,           -- 'order' | 'subscription' | 'form_submission' | …
    aggregate_id    TEXT NOT NULL,
    event_type      TEXT NOT NULL,           -- 'order.created' | 'subscription.renewed' | …
    payload         JSONB NOT NULL,
    headers         JSONB NOT NULL DEFAULT '{}',  -- idempotency key, correlation id, tenant, etc.
    status          TEXT NOT NULL DEFAULT 'pending'
                    CHECK (status IN ('pending','in_flight','delivered','failed','dead_letter')),
    attempts        INT  NOT NULL DEFAULT 0,
    max_attempts    INT  NOT NULL DEFAULT 8,
    next_attempt_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_error      TEXT,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
  );
  CREATE INDEX idx_outbox_pending ON outbox_events (status, next_attempt_at) WHERE status IN ('pending','in_flight');
  CREATE INDEX idx_outbox_aggregate ON outbox_events (aggregate_type, aggregate_id);

  CREATE TABLE outbox_subscribers (
    id             UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name           TEXT NOT NULL UNIQUE,       -- 'email.notify', 'webhook.mailchimp', …
    event_pattern  TEXT NOT NULL,              -- 'order.*', 'subscription.renewed', …
    is_active      BOOLEAN NOT NULL DEFAULT TRUE
  );
  ```

- **Module tree:**
  ```
  backend/src/events/
    mod.rs            // Event + EventBus + publish() (writes to outbox in same tx as domain mutation)
    outbox.rs         // insert_event, claim_batch (SKIP LOCKED), mark_delivered, mark_failed, to_dlq
    worker.rs         // long-running tokio task; claims N events, dispatches via Dispatcher, exponential backoff
    dispatcher.rs     // Subscriber registry; routes each event_type to one or more handlers
    handlers/
      notify.rs       // sends to notifications subsystem (FDN-05)
      webhook_out.rs  // signed HMAC-SHA256 outbound POST to registered webhook URL
      integration_*.rs // per-third-party adapters (Mailchimp, HubSpot, …) for FORM-07
  ```
- Worker concurrency: `OUTBOX_WORKERS=4` (default), per-event `SELECT … FOR UPDATE SKIP LOCKED LIMIT N`.
- Retry: `next_attempt_at = now() + (2^attempts * 1s jitter ±20%)`. Move to `dead_letter` after `max_attempts`.
- `POST /api/admin/outbox/{id}/retry`, `GET /api/admin/outbox?status=dead_letter` for ops.
- **Tests:** unit (`outbox::claim_batch` race under N concurrent workers — 2 workers must not double-claim); integration (publish → worker delivers → subscriber invoked exactly-once with idempotency key).

### FDN-05 — Notifications core

- **Migration `020_notifications.sql`:**

  ```sql
  CREATE TABLE notification_templates (
    id            UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    key           TEXT NOT NULL UNIQUE,       -- 'user.welcome', 'order.confirmed'
    channel       TEXT NOT NULL CHECK (channel IN ('email','sms','push','in_app','slack','discord','webhook')),
    locale        TEXT NOT NULL DEFAULT 'en',
    subject       TEXT,
    body_source   TEXT NOT NULL,               -- MJML / Markdown / Mustache (per-channel)
    body_compiled TEXT NOT NULL,               -- pre-rendered HTML/text
    variables     JSONB NOT NULL DEFAULT '[]', -- documented variable list
    version       INT  NOT NULL DEFAULT 1,
    is_active     BOOLEAN NOT NULL DEFAULT TRUE,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at    TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (key, channel, locale, version)
  );

  CREATE TABLE notification_preferences (
    user_id       UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    category      TEXT NOT NULL,               -- 'transactional' | 'marketing' | 'product_updates' | …
    channel       TEXT NOT NULL,
    enabled       BOOLEAN NOT NULL DEFAULT TRUE,
    quiet_hours_start TIME,
    quiet_hours_end   TIME,
    timezone      TEXT NOT NULL DEFAULT 'UTC',
    updated_at    TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (user_id, category, channel)
  );

  CREATE TABLE notification_deliveries (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id         UUID REFERENCES users(id) ON DELETE SET NULL,
    anonymous_email TEXT,
    template_key    TEXT NOT NULL,
    channel         TEXT NOT NULL,
    provider_id     TEXT,
    status          TEXT NOT NULL DEFAULT 'queued'
                    CHECK (status IN ('queued','sent','delivered','bounced','complained','opened','clicked','failed','suppressed')),
    subject         TEXT,
    rendered_body   TEXT,
    metadata        JSONB NOT NULL DEFAULT '{}',
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
  );
  CREATE INDEX idx_deliveries_user ON notification_deliveries (user_id, created_at DESC);
  CREATE INDEX idx_deliveries_status ON notification_deliveries (status, created_at DESC);

  CREATE TABLE notification_suppression (
    email        TEXT PRIMARY KEY,
    reason       TEXT NOT NULL,   -- 'bounce_hard', 'complaint', 'user_unsubscribe_all'
    suppressed_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
  );

  CREATE TABLE unsubscribe_tokens (
    token_hash   TEXT PRIMARY KEY,
    user_id      UUID REFERENCES users(id) ON DELETE CASCADE,
    email        TEXT NOT NULL,
    category     TEXT,                        -- NULL = all marketing
    expires_at   TIMESTAMPTZ NOT NULL,
    used         BOOLEAN NOT NULL DEFAULT FALSE
  );
  ```

- **Module tree:**
  ```
  backend/src/notifications/
    mod.rs
    templates.rs       // compile MJML → HTML via `mrml`; Markdown/Mustache for non-email channels
    channels/
      email.rs         // trait Channel; Resend + Lettre implementations
      sms.rs           // Twilio (behind feature flag)
      push.rs          // WebPush VAPID
      in_app.rs        // writes to in_app_notifications table
      slack.rs discord.rs webhook.rs
    preferences.rs     // quiet hours, category toggles, unsubscribe
    send.rs            // Notification::send(user, template_key, ctx) → publishes 'notification.queued' event
    worker.rs          // outbox consumer for 'notification.queued' events
    webhooks/          // provider delivery webhooks (Resend bounce/open/click, Twilio DLR, etc.)
      resend.rs twilio.rs
  ```
- **Handlers:** `POST /api/webhooks/email/resend` (HMAC verify), `GET /u/unsubscribe?token=...` (public GET, consumes token, flips preference), `GET/PUT /api/member/notification-preferences`.
- **Tests:** unit (template render with locale fallback), integration (send → worker → mock Resend → delivery row transitions `queued→sent→delivered`).

### FDN-06 — Geo + UA + HTML sanitizer + money types

- Add crates: `maxminddb = "0.25"`, `woothee = "0.13"`, `ammonia = "4"`, `rust_decimal = "1"` (already), plus a shared `Money` newtype in a new `backend/src/common/money.rs`:
  ```rust
  #[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
  #[serde(transparent)]
  pub struct Money(i64);           // minor units (cents)
  impl Money { pub fn cents(v: i64) -> Self …; pub fn as_cents(self) -> i64 …; /* add/sub/mul/percent */ }
  ```
  All new DTOs use `Money` (deserialized as integer); existing coupon `f64` path gets migrated under EC-11.
- `backend/src/common/geo.rs` — `fn country(req_headers, remote_ip) -> Option<CountryCode>` — consults edge headers first, falls back to MaxMind DB loaded once at startup.
- `backend/src/common/ua.rs` — wrap `woothee` with cached parse per unique UA string (`moka` cache, 10k entries).
- `backend/src/common/html.rs` — `pub fn sanitize_user_html(&str) -> String` using an `ammonia::Builder` with a conservative allowlist for blog / form-richtext / popup HTML blocks.
- **Migrations:** none (utility-only).
- **Tests:** unit for each util; golden-file tests for sanitizer round-trips.

### FDN-07 — Authz policy engine

- **Migration `021_rbac.sql`:**

  ```sql
  -- Roles: keep existing enum `user_role` (member, admin); add 'author' and 'support' via ALTER TYPE.
  ALTER TYPE user_role ADD VALUE IF NOT EXISTS 'author';
  ALTER TYPE user_role ADD VALUE IF NOT EXISTS 'support';

  CREATE TABLE permissions (
    key         TEXT PRIMARY KEY,            -- 'blog.post.update', 'order.refund'
    description TEXT NOT NULL DEFAULT ''
  );
  CREATE TABLE role_permissions (
    role        user_role NOT NULL,
    permission  TEXT NOT NULL REFERENCES permissions(key) ON DELETE CASCADE,
    PRIMARY KEY (role, permission)
  );
  ```

- Seed permission catalogue in same migration (`blog.*`, `course.*`, `coupon.*`, `order.*`, `subscription.*`, `popup.*`, `form.*`, `consent.*`, `notification.*`, `admin.*`).
- **Module:** `backend/src/authz.rs` with `Policy` + `require(AuthUser, "blog.post.update", &post)` helper; row-level checks inline (author_id match etc.).
- Extract from `extractors.rs`: `AdminUser` stays but gains `has_permission(&str) -> bool` (checks cached role→permission join loaded at startup + reloaded on admin mutation).
- **Tests:** permission matrix asserted in a table-driven test per handler.

### FDN-08 — CSP + rate-limit expansion

- `src/hooks.server.ts` adds full CSP:
  ```
  default-src 'self';
  script-src 'self' 'nonce-{perReq}' https://js.stripe.com https://challenges.cloudflare.com;
  style-src 'self' 'unsafe-inline';     // temporary; remove once all CSS is in layers
  img-src 'self' data: https://*.r2.cloudflarestorage.com https://*.r2.dev;
  connect-src 'self' https://api.stripe.com https://api.resend.com;
  font-src 'self' data:;
  frame-src https://js.stripe.com https://challenges.cloudflare.com;
  frame-ancestors 'none';
  base-uri 'self';
  form-action 'self';
  report-uri /api/csp-report;
  ```
- `Strict-Transport-Security`, `Cross-Origin-Opener-Policy: same-origin`, `Cross-Origin-Resource-Policy: same-site`.
- Per-request nonce injected via Svelte's `%sveltekit.nonce%` placeholder — requires app.html update.
- Rust rate-limit: add `governor` layers for `/api/webhooks/*` (low burst), `/api/popups/submit`, `/api/popups/event`, `/api/coupons/apply`, `/api/forms/submit`, `/api/consent/record`, `/api/member/**` (authed-user-keyed).
- Distributed quota: introduce `rate_limit_buckets` table for multi-instance enforcement (falls back to in-process governor when a single-instance deployment is detected via `INSTANCE_COUNT=1`).
- **Migrations:** `022_rate_limits.sql` — `rate_limit_buckets(key TEXT, window_start TIMESTAMPTZ, count INT, PRIMARY KEY(key, window_start))`.
- **Tests:** CSP report endpoint accepts `application/csp-report` + `application/reports+json`.

### FDN-09 — Email provider cutover (Resend)

- Add `resend-rs = "0.17"` (or use plain `reqwest` if the crate is stale); env: `RESEND_API_KEY`, `RESEND_FROM`, `RESEND_WEBHOOK_SECRET`.
- Port every Tera inline template in `backend/src/email.rs` to a proper MJML template file under `backend/templates/email/*.mjml` loaded at startup; compiled via `mrml`.
- Remove inline `const` templates.
- Provider abstraction: `trait EmailProvider { async fn send(…) → Result<ProviderId>; }` with `Resend`, `Smtp` (dev), `Noop` (test) implementations.
- Ingest Resend delivery webhook at `/api/webhooks/email/resend` (HMAC + timestamp tolerance) — updates `notification_deliveries.status`.

---

## 3. CONSENT MANAGEMENT

### CONSENT-01 Banner + category model

- **Migration `023_consent.sql`:**
  ```sql
  CREATE TABLE consent_policies (
    id           UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    version      INT NOT NULL,
    markdown     TEXT NOT NULL,
    effective_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    locale       TEXT NOT NULL DEFAULT 'en',
    UNIQUE (version, locale)
  );
  CREATE TABLE consent_categories (
    key         TEXT PRIMARY KEY,           -- 'necessary','functional','analytics','marketing','personalization'
    label       TEXT NOT NULL,
    description TEXT NOT NULL,
    is_required BOOLEAN NOT NULL DEFAULT FALSE,  -- 'necessary' is true
    sort_order  INT NOT NULL DEFAULT 0
  );
  CREATE TABLE consent_services (
    id           UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name         TEXT NOT NULL,             -- 'Google Analytics 4', 'Meta Pixel', …
    vendor       TEXT NOT NULL,
    category     TEXT NOT NULL REFERENCES consent_categories(key),
    domains      TEXT[] NOT NULL DEFAULT '{}',
    cookies      JSONB NOT NULL DEFAULT '[]',
    privacy_url  TEXT,
    is_active    BOOLEAN NOT NULL DEFAULT TRUE
  );
  CREATE TABLE consent_banner_configs (
    id           UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    region       TEXT NOT NULL DEFAULT 'default',  -- ISO country or 'EU','US-CA', etc.
    layout       TEXT NOT NULL DEFAULT 'bar' CHECK (layout IN ('bar','box','popup','fullscreen')),
    position     TEXT NOT NULL DEFAULT 'bottom' CHECK (position IN ('top','bottom','center','bottom-left','bottom-right')),
    theme_json   JSONB NOT NULL DEFAULT '{}',
    copy_json    JSONB NOT NULL DEFAULT '{}',
    locale       TEXT NOT NULL DEFAULT 'en',
    is_active    BOOLEAN NOT NULL DEFAULT TRUE,
    UNIQUE (region, locale)
  );
  ```
- **Module:** `backend/src/consent/`.
- **Frontend:** `src/lib/components/consent/ConsentBanner.svelte`, `.../ConsentPreferences.svelte` (in-app "Manage Cookies") using Svelte 5 runes, PE7 tokens, `{@attach}` for focus trap, dialog ARIA, ESC-to-close.

### CONSENT-02 Script blocker + consent gate

- `src/lib/consent/gate.ts` — `loadScript(url, { categories: ['analytics'] })` resolves only after the user has granted matching categories, otherwise defers until grant event.
- Replace unconditional `AnalyticsBeacon.svelte` load with a consent-gated mount.
- `data-consent-category="analytics"` HTML attribute pattern for inline `<script>` blocks in `app.html` / MDX; a tiny runtime scans them after consent and hydrates.

### CONSENT-03 Consent log + DSAR

- **Migration `024_consent_log.sql`:**

  ```sql
  CREATE TABLE consent_records (
    id             UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    subject_id     UUID,                        -- user_id if authed
    anonymous_id   UUID,                        -- browser-generated UUID cookie
    ip_hash        TEXT NOT NULL,               -- SHA256(ip + daily_salt) for GDPR-safe logging
    user_agent     TEXT NOT NULL,
    country        TEXT,
    banner_version INT NOT NULL,
    policy_version INT NOT NULL,
    categories     JSONB NOT NULL,              -- { necessary:true, analytics:false, ... }
    services       JSONB NOT NULL DEFAULT '{}', -- granular per-service if user used advanced mode
    action         TEXT NOT NULL CHECK (action IN ('granted','denied','updated','revoked','expired','prefill')),
    tcf_string     TEXT,
    gpc_signal     BOOLEAN,
    created_at     TIMESTAMPTZ NOT NULL DEFAULT NOW()
  );
  CREATE INDEX idx_consent_subject ON consent_records (subject_id, created_at DESC);
  CREATE INDEX idx_consent_anon ON consent_records (anonymous_id, created_at DESC);

  CREATE TABLE dsar_requests (
    id            UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id       UUID REFERENCES users(id) ON DELETE SET NULL,
    email         TEXT NOT NULL,
    kind          TEXT NOT NULL CHECK (kind IN ('access','delete','portability','rectification','opt_out_sale')),
    status        TEXT NOT NULL DEFAULT 'pending'
                   CHECK (status IN ('pending','verifying','in_progress','fulfilled','denied','cancelled')),
    verification_token_hash TEXT,
    payload       JSONB NOT NULL DEFAULT '{}',
    fulfilled_at  TIMESTAMPTZ,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT NOW()
  );
  ```

- `POST /api/consent/record`, `GET /api/consent/me`, `POST /api/dsar`, `GET /api/admin/dsar`, `POST /api/admin/dsar/{id}/fulfill`.
- DSAR fulfillment worker: emits ZIP with JSON of `users`, `orders`, `subscriptions`, `notification_deliveries`, `form_submissions`, `consent_records` for the subject.

### CONSENT-04 Consent Mode v2 + TCF v2.2 + GPC

- `src/lib/consent/gcm.ts` — writes `dataLayer.push(['consent','default'|'update', {...}])` with `ad_storage`, `analytics_storage`, `ad_user_data`, `ad_personalization` signals.
- Client reads `navigator.globalPrivacyControl`; if true, default to `denied` for marketing/personalization and record a `gpc_signal=true` consent row.
- TCF v2.2: a dependency (`@iabtcf/core` npm) constructs the TC string from category selections; emitted in `__tcfapi` postmessage protocol shim.

### CONSENT-05 Geo variants

- `backend/src/consent/geo.rs` — resolves region → banner config.
- Public endpoint `/api/consent/banner` returns the resolved config (single round-trip; cacheable per region).

### CONSENT-06 i18n

- Add `@inlang/paraglide-js-adapter-sveltekit` (or raw JSON message store) — low-blast scope: consent flows + DSAR + unsubscribe only.
- `backend/src/i18n/` holds 40+ translation catalogs (`en, es, fr, de, it, pt-BR, nl, sv, da, nb, fi, pl, cs, el, ro, hu, …, ja, ko, zh-Hans, zh-Hant, ar, he, tr, id, th, vi, hi`).
- Translation sources stored in Postgres (`notification_templates` table already supports `locale`) and/or JSON files served from R2.

### CONSENT-07 Admin UI + policy versioning

- Routes: `/admin/consent/banner`, `/admin/consent/categories`, `/admin/consent/services`, `/admin/consent/policies`, `/admin/consent/log`, `/admin/dsar`.
- Live preview of banner at all 9 breakpoints.
- Audit log immutable (INSERT-only table + periodic hash-chain snapshot into an `integrity_anchors` table for tamper evidence).

---

## 4. FORMS

### FORM-01 Schema model

- **Migration `025_forms.sql`:**

  ```sql
  CREATE TABLE forms (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    slug        TEXT NOT NULL UNIQUE,
    name        TEXT NOT NULL,
    description TEXT DEFAULT '',
    status      TEXT NOT NULL DEFAULT 'draft' CHECK (status IN ('draft','published','archived')),
    schema      JSONB NOT NULL,    -- canonical field tree (see below)
    settings    JSONB NOT NULL DEFAULT '{}',  -- confirmations, redirects, anti-spam, integrations
    locale      TEXT NOT NULL DEFAULT 'en',
    created_by  UUID NOT NULL REFERENCES users(id),
    published_at TIMESTAMPTZ,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at   TIMESTAMPTZ NOT NULL DEFAULT NOW()
  );
  CREATE TABLE form_versions (
    id         UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    form_id    UUID NOT NULL REFERENCES forms(id) ON DELETE CASCADE,
    version    INT NOT NULL,
    schema     JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (form_id, version)
  );
  CREATE TABLE form_submissions (
    id            UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    form_id       UUID NOT NULL REFERENCES forms(id) ON DELETE CASCADE,
    form_version  INT NOT NULL,
    user_id       UUID REFERENCES users(id) ON DELETE SET NULL,
    session_id    UUID,
    anonymous_id  UUID,
    data          JSONB NOT NULL,   -- validated normalized values keyed by field.id
    raw_data      JSONB,            -- original payload (if different)
    ip_hash       TEXT,
    user_agent    TEXT,
    referrer      TEXT,
    utm           JSONB NOT NULL DEFAULT '{}',
    status        TEXT NOT NULL DEFAULT 'complete' CHECK (status IN ('partial','complete','spam','deleted')),
    score         NUMERIC(6,2),                -- quizzes
    submitted_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
  );
  CREATE INDEX idx_fsub_form ON form_submissions (form_id, submitted_at DESC);
  CREATE INDEX idx_fsub_status ON form_submissions (status, submitted_at DESC);

  CREATE TABLE form_files (
    id            UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    submission_id UUID NOT NULL REFERENCES form_submissions(id) ON DELETE CASCADE,
    field_id      TEXT NOT NULL,
    r2_key        TEXT NOT NULL,
    mime_type     TEXT NOT NULL,
    size_bytes    BIGINT NOT NULL,
    original_name TEXT NOT NULL,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT NOW()
  );

  CREATE TABLE form_partials (   -- save-and-resume
    id            UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    form_id       UUID NOT NULL REFERENCES forms(id) ON DELETE CASCADE,
    resume_token_hash TEXT NOT NULL UNIQUE,
    email         TEXT NOT NULL,
    data          JSONB NOT NULL,
    step          INT NOT NULL DEFAULT 0,
    expires_at    TIMESTAMPTZ NOT NULL,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT NOW()
  );
  ```

- **Canonical schema JSON shape** (documented in OpenAPI):
  ```json
  {
  	"id": "form-root",
  	"version": 1,
  	"steps": [
  		{
  			"id": "step-1",
  			"title": "Contact",
  			"fields": [
  				{
  					"id": "email",
  					"type": "email",
  					"label": "Email",
  					"required": true,
  					"validation": { "asyncUnique": "users.email" }
  				},
  				{ "id": "phone", "type": "phone", "country": "US", "required": false }
  			]
  		}
  	],
  	"logic": [
  		{
  			"when": { "field": "email", "op": "matches", "value": ".*@corp\\.com" },
  			"then": { "show": ["step-2"] }
  		}
  	]
  }
  ```
- Field types catalogue (all 33 from Phase 2 §2.2): `text, email, phone, url, textarea, number, slider, rating, date, time, datetime, select, multi_select, radio, checkbox, file, image, signature, richtext, hidden, html, section_break, page_break, address, consent, terms, html_block, payment, subscription, nps, likert, matrix, ranking, calculation, api_dropdown, country, state, post_ref, product_ref`.

### FORM-02 Validation engine

- Rust `backend/src/forms/validation.rs` — interprets the schema, validates per-field + cross-field + async (DB check e.g. unique). Matches equivalent client-side TS engine for optimistic UX.
- Shared JSON of validators so client & server agree byte-for-byte.

### FORM-03 Submission store + audit

- `POST /api/forms/{slug}/submit` (optional auth), records IP-hashed + UA + referrer + UTM (parsed from `Referer` + payload body), runs anti-spam pipeline, persists.

### FORM-04 Multi-step / conditional / save-and-resume / repeaters

- Server emits resume tokens (random 32-byte, SHA-256 stored); client can resume via `/forms/{slug}?resume={token}`.
- Repeater fields: array under `data` keyed by field id; validation walks per-item.

### FORM-05 File uploads

- `POST /api/forms/{slug}/upload` — multipart; chunked uploads via `Content-Range`; R2 put with per-submission prefix key `forms/{form_id}/{submission_draft_id}/…`.
- MIME sniff on server (first 512 B) to reject spoofed extensions.

### FORM-06 Anti-spam

- Honeypot field `form_hp` always emitted; rejects if filled.
- Cloudflare Turnstile (`TURNSTILE_SITE_KEY` / `TURNSTILE_SECRET`); verify via `https://challenges.cloudflare.com/turnstile/v0/siteverify`.
- Akismet adapter (optional) for text fields.
- Dedupe: hash (form_id + email + normalized payload) and reject within 60s window.

### FORM-07 Integrations

- Event emitted on successful submit: `form.submission.created` → outbox fan-out.
- Adapters under `backend/src/events/handlers/integration_*.rs`: Mailchimp, ActiveCampaign, ConvertKit, HubSpot, Salesforce, Zapier (catch-all webhook), Make, Google Sheets (service account), Notion, Airtable, Zoho.
- Per-form selection stored in `forms.settings.integrations[]` with encrypted credentials (sealed box via `age`-style or KMS; v1 envelope-encrypt with `APP_DATA_KEY`).

### FORM-08 Payment field

- Field type `payment` renders Stripe Elements on client; server creates a PaymentIntent with `automatic_payment_methods: true`, returns client secret; submission row is linked to the resulting `orders` row (built in EC-05).
- Donation sub-variant with suggested amounts; subscription sub-variant creates a Stripe subscription.

### FORM-09 Admin builder

- Route: `/admin/forms/**` with drag/drop palette → `schema` JSON edit.
- Live preview iframe at all 9 breakpoints.
- Version diff viewer (`form_versions`).

### FORM-10 Public renderer

- `src/lib/components/forms/FormRenderer.svelte` + `FormField.svelte` per field type (33 sub-components under `.../forms/fields/`).
- WCAG 2.2 AA: `aria-describedby`, `aria-invalid`, `aria-live="polite"` error summary, correct `fieldset`/`legend`, keyboard focus ring via `:focus-visible`.

---

## 5. POPUPS (gap-fill)

### POP-01 Expanded triggers & targeting

- **Migration `026_popups_ext.sql`:** add `variant_group_id UUID`, `targeting_rules` already JSONB — extend schema to cover `geo`, `utm`, `url_regex`, `cart_value_cents`, `cart_contains_sku[]`, `membership_tier`, `time_of_day`, `day_of_week`, `returning_visitor` bools/ranges. Drop the 6-popup-type CHECK constraint and use a lookup table `popup_types(key TEXT PK, label TEXT, description TEXT, is_active BOOL)` to permit `content_locker`, `countdown`, `notification`, `spin_to_win`, `scratch_card`.
- Server-side filter logic in `handlers/popups.rs::matches_targeting_rules` expanded; geo resolved via FDN-06.

### POP-02 A/B testing

- **Migration `027_popup_variants.sql`:**
  ```sql
  CREATE TABLE popup_variants (
    id         UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    popup_id   UUID NOT NULL REFERENCES popups(id) ON DELETE CASCADE,
    name       TEXT NOT NULL,
    content_json JSONB NOT NULL,
    style_json JSONB NOT NULL,
    traffic_weight INT NOT NULL DEFAULT 50 CHECK (traffic_weight BETWEEN 0 AND 100),
    is_winner  BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
  );
  ALTER TABLE popup_events ADD COLUMN variant_id UUID REFERENCES popup_variants(id);
  ALTER TABLE popup_submissions ADD COLUMN variant_id UUID REFERENCES popup_variants(id);
  ```
- Variant assignment: sticky per anonymous/user id via hash; writes a cookie `swings_popup_variant:{popup_id}={variant_id}`.
- Significance: two-proportion z-test on (impressions, submissions) reported in admin; auto-promote the winner when `p < 0.05` AND `min_samples >= N`.

### POP-03 Template library

- Ship 12+ starter templates (newsletter, exit-intent discount, content locker, countdown urgency bar, spin-to-win, scratch card, NPS, survey, lead magnet download, announcement bar, cookie pre-banner, feedback). Stored as seed rows in `popups` with `is_active=FALSE` + a `is_template BOOL` flag (`ALTER TABLE popups ADD COLUMN is_template BOOLEAN NOT NULL DEFAULT FALSE`).

### POP-04 Additional popup types

- `content_locker`: wraps page content in a blurred panel until form submit or category grant.
- `countdown`: supports fixed end date + per-visitor rolling countdown (persisted in localStorage).
- `gamified`: `spin_to_win` with prize weight array and a `coupon_generator` hook that creates a one-time coupon on win.

### POP-05 Server-side frequency capping

- **Migration `028_popup_impressions.sql`:**
  ```sql
  CREATE TABLE popup_visitor_state (
    anonymous_id UUID NOT NULL,
    popup_id     UUID NOT NULL REFERENCES popups(id) ON DELETE CASCADE,
    last_shown_at TIMESTAMPTZ,
    times_shown  INT NOT NULL DEFAULT 0,
    times_dismissed INT NOT NULL DEFAULT 0,
    converted    BOOLEAN NOT NULL DEFAULT FALSE,
    PRIMARY KEY (anonymous_id, popup_id)
  );
  ```
- `frequency_config` interpreter honors `every_n_days`, `until_converted`, `max_dismissals`.

### POP-06 Revenue attribution + form embedding

- When an order/subscription is completed within `ATTRIBUTION_WINDOW_HOURS` (default 24) of a popup submission on the same session, write a `popup_attributions(popup_id, variant_id, session_id, order_id, amount_cents, attributed_at)` row.
- Popups can embed a Domain-2 form via `content_json.elements[].type="form_ref"` with `form_id`.

---

## 6. E-COMMERCE (digital-goods trim)

### EC-01 Product model

- **Migration `030_products.sql`:**
  ```sql
  CREATE TYPE product_kind AS ENUM ('simple','subscription','downloadable','bundle');
  CREATE TABLE products (
    id           UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    kind         product_kind NOT NULL,
    sku          TEXT UNIQUE,
    slug         TEXT NOT NULL UNIQUE,
    name         TEXT NOT NULL,
    description  TEXT NOT NULL DEFAULT '',
    short_description TEXT DEFAULT '',
    status       TEXT NOT NULL DEFAULT 'draft' CHECK (status IN ('draft','published','archived')),
    price_cents  BIGINT NOT NULL DEFAULT 0,
    currency     TEXT NOT NULL DEFAULT 'usd',
    tax_class    TEXT,
    attributes   JSONB NOT NULL DEFAULT '{}',
    images       JSONB NOT NULL DEFAULT '[]',
    metadata     JSONB NOT NULL DEFAULT '{}',
    published_at TIMESTAMPTZ,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at   TIMESTAMPTZ NOT NULL DEFAULT NOW()
  );
  CREATE TABLE product_variants (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    product_id  UUID NOT NULL REFERENCES products(id) ON DELETE CASCADE,
    sku         TEXT UNIQUE,
    attributes  JSONB NOT NULL DEFAULT '{}',  -- { color:'blue', size:'L' }
    price_cents BIGINT NOT NULL,
    stripe_price_id TEXT,
    is_default  BOOLEAN NOT NULL DEFAULT FALSE
  );
  CREATE TABLE downloadable_assets (
    id           UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    product_id   UUID NOT NULL REFERENCES products(id) ON DELETE CASCADE,
    r2_key       TEXT NOT NULL,
    file_name    TEXT NOT NULL,
    size_bytes   BIGINT NOT NULL,
    max_downloads INT,
    expires_after INTERVAL
  );
  CREATE TABLE bundle_items (
    bundle_id   UUID NOT NULL REFERENCES products(id) ON DELETE CASCADE,
    child_id    UUID NOT NULL REFERENCES products(id) ON DELETE CASCADE,
    quantity    INT NOT NULL DEFAULT 1,
    PRIMARY KEY (bundle_id, child_id)
  );
  CREATE TABLE product_categories (…); CREATE TABLE product_tags (…);  -- mirrors blog_*
  ```
- Map existing `courses` and `pricing_plans` to `products` via application-level façade (no destructive migration in phase 4; instead create a `v_products_all` view that unions the three if needed, but the cleaner path is to migrate courses→products with a `kind='downloadable'` plus a `course_contents` link table. Discuss at Phase 4 kickoff.).

### EC-02 Catalog / facets / search

- Postgres tsvector full-text index + faceting via `materialized_view product_facets`; pagination + sort via keyset.
- Optional Phase 5 swap to Meilisearch.

### EC-03 Cart

- **Migration `031_cart.sql`:**
  ```sql
  CREATE TABLE carts (
    id           UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id      UUID REFERENCES users(id) ON DELETE SET NULL,
    anonymous_id UUID,
    currency     TEXT NOT NULL DEFAULT 'usd',
    subtotal_cents BIGINT NOT NULL DEFAULT 0,
    discount_cents BIGINT NOT NULL DEFAULT 0,
    tax_cents    BIGINT NOT NULL DEFAULT 0,
    total_cents  BIGINT NOT NULL DEFAULT 0,
    applied_coupon_ids UUID[] NOT NULL DEFAULT '{}',
    metadata     JSONB NOT NULL DEFAULT '{}',
    expires_at   TIMESTAMPTZ,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (user_id), UNIQUE (anonymous_id)
  );
  CREATE TABLE cart_items (
    id           UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    cart_id      UUID NOT NULL REFERENCES carts(id) ON DELETE CASCADE,
    product_id   UUID NOT NULL REFERENCES products(id),
    variant_id   UUID REFERENCES product_variants(id),
    quantity     INT NOT NULL DEFAULT 1 CHECK (quantity > 0),
    unit_price_cents BIGINT NOT NULL,
    line_total_cents BIGINT NOT NULL,
    metadata     JSONB NOT NULL DEFAULT '{}'
  );
  ```
- Merge-on-login: move `anonymous_id` cart's items into the user's cart, summing duplicates.
- Abandoned-cart: scheduled worker emits `cart.abandoned` event after 24h of inactivity.

### EC-04 Checkout

- New route group `/checkout/**` with Stripe Elements embed.
- `POST /api/checkout/sessions` — creates `orders` row in `pending`, PaymentIntent with metadata, returns client secret + idempotency key flow.
- Address book: `addresses(id, user_id, kind, country, state, postal_code, city, line1, line2, is_default)` under migration `032_addresses.sql`.

### EC-05 Orders

- **Migration `033_orders.sql`:**
  ```sql
  CREATE TYPE order_status AS ENUM ('pending','processing','on_hold','completed','refunded','cancelled','failed');
  CREATE TABLE orders (
    id            UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    number        TEXT NOT NULL UNIQUE,     -- 'ORD-2026-000123'
    user_id       UUID REFERENCES users(id),
    cart_id       UUID REFERENCES carts(id),
    status        order_status NOT NULL DEFAULT 'pending',
    currency      TEXT NOT NULL DEFAULT 'usd',
    subtotal_cents BIGINT NOT NULL,
    discount_cents BIGINT NOT NULL DEFAULT 0,
    tax_cents     BIGINT NOT NULL DEFAULT 0,
    total_cents   BIGINT NOT NULL,
    email         TEXT NOT NULL,
    billing_address_id UUID REFERENCES addresses(id),
    stripe_payment_intent_id TEXT,
    idempotency_key TEXT UNIQUE,
    metadata      JSONB NOT NULL DEFAULT '{}',
    placed_at     TIMESTAMPTZ,
    completed_at  TIMESTAMPTZ,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at    TIMESTAMPTZ NOT NULL DEFAULT NOW()
  );
  CREATE TABLE order_items (…);       -- mirrors cart_items
  CREATE TABLE order_notes (id, order_id, author_id, kind ('customer'|'internal'), body, created_at);
  CREATE TABLE order_refunds (id, order_id, amount_cents, reason, stripe_refund_id, created_at);
  CREATE TABLE order_state_transitions (id, order_id, from_status, to_status, actor_id, reason, created_at);
  ```
- State machine enforced in `backend/src/orders/state.rs` with typed transitions.

### EC-06 Invoice + receipt PDFs

- `backend/src/pdf/` — uses `printpdf` or `weasyprint` sidecar (Dockerfile hint). Generates invoice + receipt + packing slip (blank for digital orders).
- Delivered via `GET /api/orders/{id}/invoice.pdf` (user-scoped) or `GET /api/admin/orders/{id}/invoice.pdf`.

### EC-07 Digital delivery

- On `order.completed`, emit `order.downloadable.granted` events → worker creates rows in `user_downloads(user_id, product_id, asset_id, expires_at, downloads_remaining)`.
- `GET /api/downloads/{token}` — issues an R2 presigned URL with short TTL (5 min), decrements quota.

### EC-08 Tax

- Primary: Stripe Tax (set `automatic_tax: enabled` on PaymentIntent + collect VAT ID field on checkout).
- Fallback: `tax_rates(region, rate, compound, class)` table for jurisdictions we handle manually.

### EC-09 Subscriptions 2.0

- Extend `subscriptions`:
  ```sql
  ALTER TABLE subscriptions
    ADD COLUMN paused_at TIMESTAMPTZ,
    ADD COLUMN pause_resumes_at TIMESTAMPTZ,
    ADD COLUMN trial_end TIMESTAMPTZ,
    ADD COLUMN cancel_at TIMESTAMPTZ,
    ADD COLUMN canceled_at TIMESTAMPTZ,
    ADD COLUMN billing_cycle_anchor TIMESTAMPTZ,
    ADD COLUMN quantity INT NOT NULL DEFAULT 1,
    ADD COLUMN price_cents BIGINT;
  CREATE TABLE subscription_changes (
    id             UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    subscription_id UUID NOT NULL REFERENCES subscriptions(id) ON DELETE CASCADE,
    kind           TEXT NOT NULL,    -- 'upgrade','downgrade','pause','resume','renew_early','switch_plan'
    from_plan_id   UUID, to_plan_id   UUID,
    proration_cents BIGINT,
    actor_id       UUID REFERENCES users(id),
    created_at     TIMESTAMPTZ NOT NULL DEFAULT NOW()
  );
  CREATE TABLE dunning_attempts (
    subscription_id UUID NOT NULL REFERENCES subscriptions(id) ON DELETE CASCADE,
    attempt        INT NOT NULL,
    scheduled_at   TIMESTAMPTZ NOT NULL,
    executed_at    TIMESTAMPTZ,
    result         TEXT,
    PRIMARY KEY (subscription_id, attempt)
  );
  ```
- Dunning worker: retry schedule `{+1d, +3d, +7d, +14d}`; cancel after last attempt.

### EC-10 Memberships

- **Migration `034_memberships.sql`:**
  ```sql
  CREATE TABLE membership_plans (
    id           UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    slug         TEXT NOT NULL UNIQUE,
    name         TEXT NOT NULL,
    description  TEXT DEFAULT '',
    grants_access_to JSONB NOT NULL DEFAULT '{}',  -- { categories:[], products:[], urls:[] }
    drip_rules   JSONB NOT NULL DEFAULT '[]',      -- [{after_days:7, resource:'course:abc'}]
    default_duration_days INT,
    is_active    BOOLEAN NOT NULL DEFAULT TRUE
  );
  CREATE TABLE memberships (
    id            UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id       UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    plan_id       UUID NOT NULL REFERENCES membership_plans(id),
    granted_by    TEXT NOT NULL,  -- 'product:{id}','manual','promotion:{coupon}'
    status        TEXT NOT NULL DEFAULT 'active' CHECK (status IN ('active','paused','expired','cancelled')),
    starts_at     TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    ends_at       TIMESTAMPTZ,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT NOW()
  );
  CREATE TABLE member_discounts (
    plan_id     UUID NOT NULL REFERENCES membership_plans(id) ON DELETE CASCADE,
    scope       TEXT NOT NULL, -- 'all','product:{id}','category:{id}'
    discount_type discount_type NOT NULL,
    discount_value NUMERIC(10,2) NOT NULL,
    PRIMARY KEY (plan_id, scope)
  );
  ```
- Restriction engine: `fn can_access(user_id, resource_type, resource_id) -> bool` consulted by blog / courses / products / downloadable handlers.

### EC-11 Coupon engine refactor

- **Migration `035_coupons_money.sql`:** change `coupons.discount_value` to split columns `percentage_bps INT` (basis points) + `fixed_cents BIGINT`, or keep single column but migrate API + models off `f64`. Add `excludes_plan_ids`, `excludes_product_ids`, `excludes_category_ids` UUID[] columns.
- Introduce BOGO engine: `bogo_rules JSONB` (`buy:{product_id or category}`, `get:{product_id}`, `quantity`, `discount_value`).
- Cart integration: apply during cart-total recalculation, not only at Stripe Checkout.

### EC-12 Reports

- Event-sourced MRR/ARR/LTV/churn/cohorts from `sales_events`, `subscription_changes`, `orders`, `refunds`.
- Admin UI charts: `/admin/analytics/revenue`, `/admin/analytics/cohorts`, `/admin/analytics/funnel`, `/admin/analytics/coupons`.

---

## 7. API CONTRACT (OpenAPI 3.1 highlights)

New endpoint tree (all under `/api`), each annotated with `#[utoipa::path]`:

```
/consent
  GET  /banner         → banner config (geo-resolved)
  POST /record         → write consent row
  GET  /me             → current subject's consent state
  GET  /log/mine
  POST /dsar           → open DSAR
  GET  /dsar/{id}      → status

/forms
  GET  /{slug}                       → published schema
  POST /{slug}/submit                → submit (with Idempotency-Key)
  POST /{slug}/upload                → file (multipart, chunked)
  POST /{slug}/partial               → save partial
  GET  /{slug}/resume?token=         → load partial

/cart
  GET  /                             → current cart
  POST /items                        → add (with Idempotency-Key)
  PATCH /items/{id}                  → update qty
  DELETE /items/{id}
  POST /coupon                       → apply
  DELETE /coupon/{id}                → remove

/checkout
  POST /sessions                     → create order + PaymentIntent
  POST /confirm                      → finalize after Stripe action
  POST /address                      → save address to user's book

/orders
  GET  /mine
  GET  /{id}
  GET  /{id}/invoice.pdf

/downloads/{token}
  GET                                → 302 to R2 presigned URL

/products
  GET  /                             → list (facets, sort, pagination)
  GET  /{slug}

/memberships
  GET  /mine
  GET  /plans

/popups  (extended)
  GET  /active                       → server filters by geo, tier, UTM, URL regex
  POST /event                        → impression/click/close/submit/convert
  POST /submit

/member (extended)
  GET  /notification-preferences
  PUT  /notification-preferences
  GET  /memberships
  POST /memberships/{id}/pause
  POST /memberships/{id}/resume
  POST /subscriptions/{id}/change-plan
  POST /subscriptions/{id}/pause
  POST /subscriptions/{id}/resume

/admin
  …/forms, …/popups, …/consent, …/dsar
  …/products, …/carts, …/orders, …/orders/{id}/refund, …/orders/{id}/notes
  …/memberships, …/memberships/plans
  …/notifications/templates, …/notifications/deliveries, …/notifications/broadcasts
  …/integrations
  …/outbox

/webhooks
  /stripe (existing)
  /email/resend
  /sms/twilio            (if SMS enabled)
  /turnstile             (n/a — server-to-server; listed for completeness)
  /form/{integration}    (inbound webhooks from partner tools)
```

All mutating endpoints that touch money or dispatch a notification require an `Idempotency-Key` header; keys stored in `idempotency_keys(key TEXT PK, user_id UUID, route TEXT, response JSONB, status INT, expires_at TIMESTAMPTZ)` (migration `029_idempotency.sql`), TTL 24h.

---

## 8. SVELTEKIT ADMIN UI ROUTE TREE (additions)

```
src/routes/admin/
  consent/
    +page.svelte                    (dashboard)
    banner/+page.svelte
    categories/+page.svelte
    services/+page.svelte
    policies/ (list, [version])
    log/+page.svelte
  dsar/
    +page.svelte                    (queue)
    [id]/+page.svelte               (fulfillment)
  forms/
    +page.svelte
    new/+page.svelte
    [id]/
      +page.svelte                  (builder)
      submissions/+page.svelte
      versions/+page.svelte
      integrations/+page.svelte
  products/
    +page.svelte
    new/+page.svelte
    [id]/
      +page.svelte
      variants/+page.svelte
      assets/+page.svelte           (downloadable)
      categories-tags/+page.svelte
  orders/
    +page.svelte
    [id]/
      +page.svelte                  (detail, notes, refund)
  memberships/
    plans/ (list, [id])
    members/ (list, [id])
  notifications/
    templates/ (list, [key])
    deliveries/+page.svelte
    broadcasts/ (list, new, [id])
    preferences-defaults/+page.svelte
  integrations/+page.svelte         (per-provider wizards)
  outbox/+page.svelte                (failed events, retries, DLQ)
  popups/
    (existing) + templates/+page.svelte + [id]/variants/+page.svelte
```

All new admin screens use Svelte 5 runes (`$state`, `$derived`, `$effect`, `$props`), PE7 tokens, Phosphor icons via `phosphor-svelte`, no Tailwind classes. Layouts use `{#snippet}/{@render}` not slots; element actions via `{@attach}` not `use:action`.

---

## 9. PUBLIC-FACING COMPONENT INVENTORY (new)

```
src/lib/components/
  consent/
    ConsentBanner.svelte
    ConsentPreferencesModal.svelte
    ConsentGate.svelte           ({#if gated}…{/if} for inline scripts/content)
  forms/
    FormRenderer.svelte
    FormStep.svelte
    FormProgress.svelte
    fields/
      TextField.svelte  EmailField.svelte  PhoneField.svelte  UrlField.svelte
      TextareaField.svelte  NumberField.svelte  SliderField.svelte  RatingField.svelte
      DateField.svelte  TimeField.svelte  DateTimeField.svelte
      SelectField.svelte  MultiSelectField.svelte  RadioField.svelte  CheckboxField.svelte
      FileField.svelte  ImageField.svelte  SignatureField.svelte
      RichTextField.svelte  HiddenField.svelte  HtmlBlock.svelte  SectionBreak.svelte  PageBreak.svelte
      AddressField.svelte  ConsentField.svelte  TermsField.svelte
      PaymentField.svelte  SubscriptionField.svelte
      NpsField.svelte  LikertField.svelte  MatrixField.svelte  RankingField.svelte
      CalculationField.svelte  ApiDropdownField.svelte
      CountryField.svelte  StateField.svelte  PostRefField.svelte  ProductRefField.svelte
  popups/
    (existing) + PopupCountdown.svelte  PopupSpinToWin.svelte  PopupScratchCard.svelte
    PopupContentLocker.svelte  PopupCountdownBar.svelte  PopupNotification.svelte
  commerce/
    ProductCard.svelte  ProductGallery.svelte  ProductDetails.svelte
    CartDrawer.svelte  CartItemRow.svelte
    CheckoutStepper.svelte  AddressForm.svelte  PaymentForm.svelte
    OrderSummary.svelte  CouponInput.svelte
    MembershipBadge.svelte  MembershipLockedSection.svelte
    DownloadButton.svelte
  notifications/
    InAppInbox.svelte  NotificationCenter.svelte  NotificationPreferences.svelte
  shared/
    Stepper.svelte  Dialog.svelte  Drawer.svelte  Toast.svelte  Spinner.svelte
    FormField.svelte  Breadcrumbs.svelte  EmptyState.svelte
```

All components: accessibility audit required (axe-core rule pass via Playwright), reduced-motion honored, focus trap on overlays, ESC-close, ARIA dialog roles.

---

## 10. TEST PLAN

Coverage goal: 85%+ on Rust services/handlers, 80%+ on Svelte component unit tests.

- **Rust unit (`#[cfg(test)]`):** per-module; emphasis on `forms/validation.rs`, `orders/state.rs`, `consent/record.rs`, `coupons/*`, `authz.rs`, `events/outbox.rs`, webhook signature verifiers.
- **Rust integration:** `backend/tests/*.rs` using axum's `Router::into_service` + `tower::ServiceExt::oneshot`, seeded Postgres test DB via `sqlx` test fixtures. Cover: full checkout flow, subscription upgrade, refund, DSAR fulfillment, popup A/B winner auto-promotion, form submission → integration fanout, consent GPC override.
- **Playwright E2E (`e2e/`):**
  - `consent/banner.spec.ts` — GPC auto-deny; granular category toggle; persistence across reload.
  - `forms/*.spec.ts` — multi-step, file upload, save/resume, Turnstile challenge, payment field.
  - `popups/*.spec.ts` — scroll trigger, exit intent (desktop only), A/B variant assignment stickiness.
  - `checkout/*.spec.ts` — happy path, coupon apply, refund, membership grant.
  - `a11y.spec.ts` — `@axe-core/playwright` scan on every public route.
- **Visual regression:** Playwright screenshots at all 9 breakpoints for `/`, `/pricing`, `/blog/*`, `/admin`, consent banner, cart drawer, checkout.
- **Load (k6):** `loadtests/k6/checkout.js`, `consent-record.js`, `form-submit.js`, `popup-active.js`. Targets: 95p < 200ms at 500 rps on public endpoints; checkout 95p < 500ms at 100 rps.
- **Security:** `cargo audit` + `pnpm audit` in CI; `trivy fs` on the Docker image; `sqlfluff` linter on migrations.

---

## 11. OBSERVABILITY

- **Tracing:** `tracing` crate with JSON formatter; spans on every handler (`#[tracing::instrument(skip(state), fields(user_id=?auth.user_id))]`); business spans (`checkout.create_session`, `order.state_transition`, `popup.match_targeting`, `consent.evaluate`).
- **Correlation id:** `X-Request-Id` header propagated; generated when absent; attached to every span and error log.
- **Metrics:** `metrics` crate + `metrics-exporter-prometheus`; `/metrics` endpoint admin-gated.
  - `http_requests_total{route,method,status}`, `http_request_duration_seconds{route,method}`
  - `outbox_pending`, `outbox_attempts_total{subscriber,result}`, `outbox_dead_letter_total`
  - `notifications_sent_total{channel,provider,status}`
  - `stripe_webhook_total{event_type,result}`
  - `consent_records_total{category,action}`, `dsar_requests_total{kind,status}`
  - `cart_abandoned_total`, `orders_total{status}`, `mrr_cents`, `arr_cents`
- **Structured logs:** `{ ts, level, target, message, request_id, user_id?, route?, status?, latency_ms, err? }` JSON.
- **Alerts (Phase 5 follow-up):** PagerDuty via metrics gate on `outbox_dead_letter_total`, webhook 5xx rate, DSAR SLA breach (unfulfilled > 30d).

---

## 12. SECURITY REVIEW

### Authz matrix (excerpt; full matrix lives in `backend/src/authz.rs`)

| Resource                 | Action      | Member | Author | Support       | Admin |
| ------------------------ | ----------- | ------ | ------ | ------------- | ----- |
| `user.self`              | read/update | ✓      | ✓      | ✓             | ✓     |
| `user.other`             | read        | ✗      | ✗      | ✓ (read-only) | ✓     |
| `user.other`             | delete      | ✗      | ✗      | ✗             | ✓     |
| `blog.post`              | create      | ✗      | ✓      | ✗             | ✓     |
| `blog.post`              | update own  | ✗      | ✓      | ✗             | ✓     |
| `blog.post`              | update any  | ✗      | ✗      | ✗             | ✓     |
| `order.mine`             | read        | ✓      | ✓      | ✓             | ✓     |
| `order.any`              | read        | ✗      | ✗      | ✓             | ✓     |
| `order.refund`           | create      | ✗      | ✗      | ✓ ($ cap)     | ✓     |
| `coupon.*`               | manage      | ✗      | ✗      | ✗             | ✓     |
| `consent.log`            | read any    | ✗      | ✗      | ✓             | ✓     |
| `dsar.fulfill`           | execute     | ✗      | ✗      | ✓             | ✓     |
| `notification.broadcast` | create      | ✗      | ✗      | ✗             | ✓     |
| `outbox.retry`           | execute     | ✗      | ✗      | ✓             | ✓     |

### Rate limits (per-IP unless noted)

| Route                            | Limit                                   |
| -------------------------------- | --------------------------------------- |
| `POST /api/auth/login`           | 5/min (existing)                        |
| `POST /api/auth/register`        | 10/hour (existing)                      |
| `POST /api/auth/forgot-password` | 3/hour (existing)                       |
| `POST /api/forms/{slug}/submit`  | 10/min/IP, 30/min/form                  |
| `POST /api/forms/{slug}/upload`  | 20/min/IP, 1GB/day/IP                   |
| `POST /api/popups/submit`        | 20/min/IP                               |
| `POST /api/popups/event`         | 120/min/IP                              |
| `POST /api/consent/record`       | 30/min/IP                               |
| `POST /api/dsar`                 | 5/day/email                             |
| `POST /api/checkout/sessions`    | 20/min/user                             |
| `POST /api/cart/*`               | 120/min/user                            |
| `POST /api/webhooks/*`           | 500/min/source (after signature verify) |

### CSRF

- All browser API calls go through same-origin `/api/*` (Vercel rewrite → Railway). Cross-origin credentialed requests rejected.
- Mutating endpoints require `Authorization: Bearer` **or** a `X-CSRF-Token` header matching a double-submit cookie (set by `hooks.server.ts` on login).

### Input sanitization

- User-generated HTML (blog posts, form richtext, popup `html_block`) passes through `ammonia` with category-specific allowlist before persistence; a second pass on render.
- SVG uploads rejected in media library unless `ALLOW_SVG_UPLOADS=1` and content scanned for `<script>` / `on*=`.

### File upload hardening

- MIME sniff (first 512 B) + whitelist (image/\*, application/pdf, text/csv, video/mp4, application/zip for course assets).
- Size limit per field (default 10MB), per submission (default 50MB), per day per IP.
- Virus scan (ClamAV sidecar container) behind `SCAN_UPLOADS=1` flag for v1 ops.
- Served via R2 with `Content-Disposition: attachment; filename="…"` and `X-Content-Type-Options: nosniff`.

### PCI scope

- No card data ever hits our servers. Stripe Elements + PaymentIntent flow; we store only `stripe_payment_intent_id` / last-4 / brand in `orders.metadata`.
- Admin UI never renders raw card fields.
- PCI-DSS SAQ-A eligibility maintained.

### Webhook signing

- Every inbound webhook verifies provider signature with constant-time compare and ±5-min timestamp tolerance (pattern in place for Stripe; mirror for Resend, Twilio, Turnstile server-verify, Mailchimp et al).
- Outbound webhooks (to customer-configured URLs) sign with `HMAC-SHA256(secret, timestamp + "." + body)` + include `Swings-Webhook-Id`, `Swings-Webhook-Timestamp`, `Swings-Webhook-Signature` headers.

### Secrets

- `.env` local only; production secrets via Railway + Vercel env vars.
- Add to `.env.example`: `DATABASE_URL`, `JWT_SECRET`, `ADMIN_EMAIL`, `ADMIN_PASSWORD`, `APP_ENV`, `CORS_ALLOWED_ORIGINS`, `SMTP_*`, `RESEND_*`, `R2_*`, `TURNSTILE_*`, `TWILIO_*`, `MAXMIND_DB_PATH`.
- `backend/src/config.rs::assert_production_ready` extended to fail-fast on all prod-required vars.

---

## 13. ROUGH ENGINEERING BUDGET

For sizing only — real estimates rebuild per-subsystem during Phase 4 kickoff.

| Block          | Focused senior-eng-weeks |
| -------------- | ------------------------ |
| FDN-01..09     | 5                        |
| CONSENT-01..07 | 4                        |
| FORM-01..10    | 8                        |
| POP-01..06     | 3                        |
| EC-01..12      | 10                       |
| **Total**      | **~30 eng-weeks**        |

---

## 14. PROPOSED PHASE-4 SEQUENCING

Unless you specify otherwise, Phase 4 proceeds in this order; each block is stop-and-review:

1. **FDN-01 → FDN-03** (hygiene, OpenAPI, PE7 CSS migration) — enables every other block.
2. **FDN-04 → FDN-05 → FDN-09** (outbox, notifications core, Resend).
3. **FDN-06 → FDN-07 → FDN-08** (utilities, authz, CSP/rate-limits).
4. **CONSENT-01 → ... → CONSENT-07.**
5. **FORM-01 → ... → FORM-10.**
6. **POP-01 → ... → POP-06.**
7. **EC-01 → ... → EC-12.**

---

## CHECKPOINT

Phase 3 deliverable complete. Awaiting review of:

- **The 6 default decisions (§0).** Flag any to flip; highest-blast-radius flip is D3 (outbox vs. Redis).
- **Scope trim (§0 D1).** If full WooCommerce parity is actually in scope, shipping/inventory/B2B subsystems add ~8 more eng-weeks and touch EC-01 heavily.
- **Sequencing (§14).** Happy to reorder — e.g. if CONSENT is urgent for a regulatory deadline it can jump FDN-06..09 with a deferred-fixup backlog.

On approval, Phase 4 begins with **FDN-01** (crate hygiene + error model). I'll execute one subsystem at a time, commit at each stop, summarize diffs, and wait for your go-ahead before starting the next.
