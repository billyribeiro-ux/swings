# AUDIT_FIX_PLAN.md

> Phased remediation plan derived from `AUDIT_REPORT.md`.
> Ordered by impact and dependency. Each item lists exact files, the change,
> the verification command, effort estimate, and dependencies.

---

## Phase 1 — Blockers (anything stopping `pnpm build`, `pnpm check`, or `cargo build --release`)

**Status of build commands today:**
- `pnpm check` — green (0 errors, 0 warnings)
- `cargo build --release` — green
- `pnpm build` — not run by audit; deferred to Phase 4 verification

The strict definition of "stops a build" yields **no current blockers**. However, the items below are *runtime* blockers (production endpoints 404, security advisories, broken auth flows) that should be treated with Phase 1 urgency:

### 1.1 — Fix doubled-prefix bug in 4 routers (CONTRACT-A1)
- **Files:**
  - `/Users/billyribeiro/Desktop/my-websites/swings/backend/src/handlers/coupons.rs:28-57`
  - `/Users/billyribeiro/Desktop/my-websites/swings/backend/src/handlers/courses.rs:20-49`
  - `/Users/billyribeiro/Desktop/my-websites/swings/backend/src/handlers/products.rs:48-90`
- **Change:** drop the inner noun prefix from each `Router::new().route(...)`. Example for coupons:
  ```rust
  // before
  Router::new()
    .route("/coupons", get(admin_list_coupons))
    .route("/coupons/{id}", get(admin_get_coupon))
    ...
  // after
  Router::new()
    .route("/", get(admin_list_coupons))
    .route("/{id}", get(admin_get_coupon))
    ...
  ```
- **Side-effect:** `tests/rbac_legacy_handlers.rs:130,218` and `tests/idempotency_legacy_handlers.rs:130,160,217` will need their URL paths updated (drop the doubled noun).
- **Verification:**
  - `cargo test --manifest-path backend/Cargo.toml` — all green
  - Manual: `curl -i http://localhost:3001/api/admin/coupons` returns 401 (auth) not 404
- **Effort:** small
- **Dependency:** none

### 1.2 — Mount `handlers::forms::{public_router, admin_router}` (CONTRACT-A2)
- **File:** `/Users/billyribeiro/Desktop/my-websites/swings/backend/src/main.rs`
- **Change:** add the two missing nests (mirror the test harness in `tests/support/app.rs:677-678`):
  ```rust
  .nest("/api/forms", handlers::forms::public_router())
  .nest("/api/admin/forms", handlers::forms::admin_router())
  ```
- **Verification:**
  - `cargo test --manifest-path backend/Cargo.toml`
  - `curl -i http://localhost:3001/api/forms/some-slug` returns 404-by-id (route exists), not 404-no-route
- **Effort:** trivial
- **Dependency:** none

### 1.3 — Build the missing `/admin/forms/*` admin builder backend OR remove the frontend pages (CONTRACT-A3)
- **Decision required:** is the forms-builder a P1 product feature, or should the routes be deleted?
- **Path A (build):** add `POST /api/admin/forms`, `PUT /api/admin/forms/{id}`, `GET /api/admin/forms`, `GET /api/admin/forms/{id}/versions`, `POST /api/admin/forms/{id}/versions`, `GET /api/admin/forms/{id}/preview` etc. to `handlers/forms.rs`. Tap into the unused `forms/repo.rs:96,122,168,185,213,538` helpers (DEAD-3 confirms they exist, just unwired).
- **Path B (delete):** remove `routes/admin/forms/+page.svelte`, `forms/new/+page.svelte`, `forms/[id]/+page.svelte`, `forms/[id]/versions/+page.svelte`, `forms/[id]/preview/+page.svelte` and their sidebar entry.
- **Effort:** large (Path A) / small (Path B)
- **Dependency:** none

### 1.4 — Fix idempotency middleware to read BFF cookie (CONTRACT-D2, R1 cluster)
- **File:** `/Users/billyribeiro/Desktop/my-websites/swings/backend/src/middleware/idempotency.rs:186-198`
- **Change:** replace the inline bearer-only `decode_subject` with a call to `crate::extractors::extract_access_token(headers)` (already exists, used by `AuthUser` extractor). Then decode that token with `jwt_validation()`.
  ```rust
  // before
  let token = headers.get(AUTHORIZATION)?
      .to_str().ok()?
      .strip_prefix("Bearer ")?;
  // after
  let token = crate::extractors::extract_access_token(headers)?;
  ```
- **Verification:**
  - Add `tests/idempotency_cookie_carry.rs` that POSTs an admin endpoint twice with identical `Idempotency-Key` and a cookie session, asserts the second response is the cached one
  - Existing `tests/admin_idempotency.rs` still passes
- **Effort:** small
- **Dependency:** none

### 1.5 — Patch the 4 RUSTSEC blockers in the rustls/aws-sdk-s3 chain (DEP-1, DEP-2, DEP-3 cluster R3)
- **File:** `/Users/billyribeiro/Desktop/my-websites/swings/backend/Cargo.toml:71`
- **Change:** bump the `aws-sdk-s3` constraint from `"1"` to `"1.120"` (or current). Then run `cargo update -p rustls-webpki -p rustls`.
- **Verification:**
  - `cargo deny --manifest-path backend/Cargo.toml check advisories` — RUSTSEC-2026-0098, -0099, -0104 (×2) all gone
  - `cargo test --manifest-path backend/Cargo.toml` — green (rustls ABI is binary-compatible in minor bumps; sqlx/lettre/reqwest already on 0.23)
  - Spot-check R2 upload + DSAR artifact flow against staging
- **Effort:** small (bump + test)
- **Dependency:** none

### 1.6 — Remove default `JWT_SECRET` from `docker-compose.yml` (CFG-8)
- **File:** `/Users/billyribeiro/Desktop/my-websites/swings/docker-compose.yml:33`
- **Change:** drop the `:-explosive-swings-jwt-secret-key-2026` default. Either require the variable from the developer's environment, or generate a per-stack secret in `predev:all`.
  ```yaml
  # before
  JWT_SECRET: ${JWT_SECRET:-explosive-swings-jwt-secret-key-2026}
  # after
  JWT_SECRET: ${JWT_SECRET:?JWT_SECRET must be set; copy backend/.env.example to backend/.env}
  ```
- **Verification:** `docker compose up -d db api` without env fails fast with the message; with env set, boots normally
- **Effort:** trivial
- **Dependency:** none

---

## Phase 2 — Type errors & A11y errors (frontend)

### 2.1 — Fix `tsc --noEmit` named-type re-exports (TS-1)
- **File:** `/Users/billyribeiro/Desktop/my-websites/swings/src/lib/components/shared/index.ts:11-37`
- **Change:** move named type definitions (`ButtonProps`, `DialogSize`, `DrawerProps`, `ToastKind`, `StepperStep`, `FormFieldChildContext`, `BreadcrumbItem`, `EmptyStateProps`, etc.) out of `<script module lang="ts">` blocks in `.svelte` files into sibling `*.types.ts` modules. Re-export from `index.ts`. svelte-check stays clean and bare `tsc --noEmit` also passes.
- **Alternative (lighter):** mark `pnpm check` as the canonical type-check in CI and ignore raw `tsc`. Document in AGENTS.md.
- **Verification:**
  - `pnpm exec tsc --noEmit` — exit 0
  - `pnpm check` — 0 errors / 0 warnings
- **Effort:** medium (touch ~10 component files)
- **Dependency:** none

### 2.2 — Refactor 17 admin drawers through `Drawer.svelte` primitive (A11Y-3, R6 cluster)
- **Files (consumer side):** `/admin/+layout.svelte:329`, `/admin/audit/+page.svelte:375`, `/admin/dsar/+page.svelte:675`, `/admin/orders/+page.svelte:503`, `/admin/outbox/+page.svelte:293`, `/admin/notifications/{deliveries,suppression,templates}/+page.svelte`, `/admin/forms/[id]/submissions/+page.svelte:510`, `/admin/consent/{log,categories,integrity,services,banner,policy}/+page.svelte`, `/lib/components/editor/MediaLibrary.svelte:204`, `/lib/components/editor/PostEditor.svelte:948`
- **Change:** replace the inline `<div role="button" tabindex="-1">` backdrop pattern with `<Drawer open={...} onClose={...}>` from `/lib/components/shared/Drawer.svelte` (already implements focus trap + Escape + inert siblings).
- **Verification:**
  - svelte-check stays clean
  - Manual a11y test: Tab into an opened drawer, focus trap holds; Escape closes; SR announces "dialog" not "button"
- **Effort:** medium (each replacement is ~30 lines collapsed to 1-2 lines)
- **Dependency:** none

### 2.3 — Wrap `CommandPalette` and `EditorToolbar` shortcuts modal in `Dialog.svelte` (A11Y-5)
- **Files:** `/Users/billyribeiro/Desktop/my-websites/swings/src/lib/components/admin/CommandPalette.svelte:159-221`, `/Users/billyribeiro/Desktop/my-websites/swings/src/lib/components/editor/EditorToolbar.svelte:893-916`
- **Change:** route through `Dialog.svelte` (focus trap + inert background).
- **Verification:** Tab cannot escape modal; Escape closes
- **Effort:** small
- **Dependency:** none

### 2.4 — Replace `TradersModal` overlay button with proper Dialog (A11Y-4)
- **File:** `/Users/billyribeiro/Desktop/my-websites/swings/src/lib/components/traders/TradersModal.svelte:39-46`
- **Change:** swap `<div tabindex="0" role="button" onkeydown=Enter>` for `<Dialog>`.
- **Effort:** small

### 2.5 — Add labels to placeholder-only inputs (A11Y-6)
- **Files:** `/admin/products/[id]/+page.svelte:473-538`, `/admin/popups/new/+page.svelte:181,222,228,240,264-286`, `/admin/popups/[id]/+page.svelte:202`, `/admin/courses/[id]/+page.svelte:200,213-214`, `/admin/blog/+page.svelte:535`, `/admin/subscriptions/plans/+page.svelte:226-318`
- **Change:** wrap each `<input placeholder="X">` in `<label>X<input ... /></label>` OR add `aria-label="X"`.
- **Verification:** axe-core scan (Phase 8 will install it)
- **Effort:** medium (~30 inputs)
- **Dependency:** none

### 2.6 — Migrate 6 `$effect` fetch-on-mount sites to `onMount` (RUNES-2, R5 cluster)
- **Files:**
  - `/admin/blog/tags/+page.svelte:10`
  - `/admin/blog/categories/+page.svelte:14`
  - `/admin/blog/media/+page.svelte:31`
  - `/admin/blog/[id]/+page.svelte:13`
  - `/admin/products/[id]/+page.svelte:94`
  - `/lib/components/editor/MediaLibrary.svelte:26`
- **Change:** replace `$effect(() => { loadX(); });` with `onMount(loadX);`. For MediaLibrary which gates on `if (open)`, use the prop-watch pattern with `untrack(loadMedia)`.
- **Verification:**
  - svelte-check clean
  - Manual: navigate to each page, no infinite-loop crash
- **Effort:** trivial (6 single-line edits)
- **Dependency:** none

### 2.7 — Add `untrack` guard to `FormRenderer.svelte` self-write `$effect` (RUNES-3)
- **File:** `/Users/billyribeiro/Desktop/my-websites/swings/src/lib/components/forms/FormRenderer.svelte:129-137`
- **Change:** wrap body in `untrack(() => { ... })`.
- **Effort:** trivial

### 2.8 — Add admin and dashboard `+error.svelte` boundaries (ROUTE-1, ROUTE-2)
- **Files (new):**
  - `/Users/billyribeiro/Desktop/my-websites/swings/src/routes/admin/+error.svelte`
  - `/Users/billyribeiro/Desktop/my-websites/swings/src/routes/dashboard/+error.svelte`
- **Change:** render error inside the route group's chrome (sidebar/main intact). Show `page.error.message` + correlation id from `hooks.server.ts:160-163`.
- **Verification:** trigger an admin error (e.g. `/admin/<bad-uuid>` for a member detail) — chrome stays, error renders within
- **Effort:** small
- **Dependency:** none

### 2.9 — Drop duplicate `id="main-content"` from root `+error.svelte` (ROUTE-3)
- **File:** `/Users/billyribeiro/Desktop/my-websites/swings/src/routes/+error.svelte:42`
- **Change:** remove `id="main-content"` (root layout already provides it).
- **Effort:** trivial

### 2.10 — Add `tabindex="0"` for keyboard reachability OR convert to button (A11Y-7, A11Y-8)
- **Files:** `/admin/members/+page.svelte:954-967` (raise `.member-card__btn` to `2.75rem`), `/admin/popups/+page.svelte:781-783` (raise `.icon-btn` mobile-only via media query), `/lib/components/popups/PopupRenderer.svelte:81-87` (drop the `<button tabindex="-1">` backdrop, use `<div aria-hidden>`).
- **Effort:** trivial

---

## Phase 3 — Rust correctness (clippy denials, unsafe without justification, unwrap/panic in production paths)

### 3.1 — `cargo clippy -D warnings` is currently green
- **Verification:** `cargo clippy --manifest-path backend/Cargo.toml --all-targets -- -D warnings` exits 0
- **No action needed.**

### 3.2 — Annotate `unreachable!()` in `notifications/webhooks/resend.rs:241` (RUST-4)
- **File:** `/Users/billyribeiro/Desktop/my-websites/swings/backend/src/notifications/webhooks/resend.rs:241`
- **Change:** add `// SAFETY: chunks(3) remainder ∈ {0,1,2}` comment above the `_ => unreachable!()` arm.
- **Effort:** trivial

### 3.3 — Elide redundant lifetime in `forms/integrations.rs:100` (RUST-5)
- **File:** `/Users/billyribeiro/Desktop/my-websites/swings/backend/src/forms/integrations.rs:100`
- **Change:** `fn require_email<'a>(p: &'a SubmissionPayload<'_>) -> Result<&'a str, _>` → `fn require_email(p: &SubmissionPayload<'_>) -> Result<&str, _>`.
- **Effort:** trivial

### 3.4 — Fix `unrust!` macros / non-fatal `expect` calls (RUST-3)
- 13 `.expect(...)` sites all on documented infallible operations. Already justified in code comments. **No action recommended.**

---

## Phase 4 — API contract mismatches

### 4.1 — Fix universal error-shape mismatch (CONTRACT-G1, R4 cluster)
- **File:** `/Users/billyribeiro/Desktop/my-websites/swings/src/lib/api/client.ts:65-66, 79-80, 205-206`
- **Change:**
  ```ts
  // before
  const err = await response.json().catch(() => ({ error: 'Request failed' }));
  throw new ApiError(response.status, err.error || 'Request failed');
  // after
  const err = await response.json().catch(() => ({}));
  const msg = err.detail || err.title || err.error || 'Request failed';
  throw new ApiError(response.status, msg);
  ```
- **Verification:** trigger a 422 from any admin form; toast now shows the actual `detail` from the backend
- **Effort:** trivial
- **Dependency:** none

### 4.2 — Fix `/api/admin/users` → `/api/admin/members` reference (CONTRACT-A4)
- **File:** `/Users/billyribeiro/Desktop/my-websites/swings/src/routes/admin/blog/+page.svelte:234`
- **Change:** `'/api/admin/users?role=admin&per_page=50'` → `'/api/admin/members?role=admin&per_page=50'`
- **Effort:** trivial

### 4.3 — Add missing admin subscriptions list + stats endpoints (CONTRACT-A5)
- **Files:** `/Users/billyribeiro/Desktop/my-websites/swings/backend/src/handlers/admin_subscriptions.rs:53`
- **Change:** add `GET /api/admin/subscriptions?...` (list with pagination, filters) and `GET /api/admin/subscriptions/stats` (counts by status/plan, MRR/ARR — re-use `db::admin_estimated_mrr_arr_cents`).
- **Verification:** `routes/admin/subscriptions/+page.svelte:76,100` renders without 404
- **Effort:** medium

### 4.4 — Fix `/api/admin/popups/analytics` route conflict (CONTRACT-A6)
- **File:** `/Users/billyribeiro/Desktop/my-websites/swings/backend/src/handlers/popups.rs`
- **Change:** add `route("/analytics", get(admin_list_analytics))` BEFORE `route("/{id}/analytics", ...)` (axum matches in registration order). Implement `admin_list_analytics` returning per-popup analytics summaries.
- **Effort:** small

### 4.5 — Add `/api/admin/coupons/stats` endpoint (CONTRACT-A7)
- **File:** `/Users/billyribeiro/Desktop/my-websites/swings/backend/src/handlers/coupons.rs`
- **Change:** add a `GET /stats` route returning total/active/expired/redemption counts.
- **Effort:** small

### 4.6 — Implement member self-service endpoints (CONTRACT-A8)
- **File:** `/Users/billyribeiro/Desktop/my-websites/swings/backend/src/handlers/member.rs`
- **Change:** add `POST /api/member/password` (verify current password + hash new), `DELETE /api/member/account` (soft-delete + Stripe sub cancel), `POST /api/member/coupons/apply`.
- **Effort:** medium

### 4.7 — Wire `Idempotency-Key` on every admin POST in the frontend (CONTRACT-D1)
- **File:** `/Users/billyribeiro/Desktop/my-websites/swings/src/lib/api/client.ts`
- **Change:** in the `request<T>()` method, when `method !== 'GET'` and the URL starts with `/api/admin/`, auto-generate `crypto.randomUUID()` and inject as `Idempotency-Key` if the caller didn't supply one.
- **Verification:** post-fix Phase 1.4, a double-clicked refund now hits the cache; only one DB row created
- **Effort:** small
- **Dependency:** Phase 1.4 (idempotency middleware reading cookies)

### 4.8 — Fix logout BFF cookie carry (CONTRACT-C2)
- **File:** `/Users/billyribeiro/Desktop/my-websites/swings/src/lib/stores/auth.svelte.ts:111`
- **Change:** add `credentials: 'include'` to the raw `fetch('/api/auth/logout', { method: 'POST' })`.
- **Effort:** trivial

### 4.9 — Reconcile `PricingPlanPriceLogEntry` ↔ `PricingPlanAmountChangeLogEntry` (CONTRACT-B1)
- **File:** `/Users/billyribeiro/Desktop/my-websites/swings/src/lib/api/types.ts:673`
- **Change:** delete the hand-written alias and import from `schema.d.ts` instead.
- **Effort:** trivial

### 4.10 — Add OpenAPI `#[derive(ToSchema)]` to missing response types (CONTRACT-B + drift)
- **Files:** `backend/src/handlers/{cart,catalog,blog,courses,products,pricing}.rs` etc.
- **Change:** add `#[utoipa::path(...)]` annotations to the 22 frontend types currently missing from the OpenAPI snapshot. Regenerate via `cargo test --test openapi_snapshot` with `UPDATE_OPENAPI_SNAPSHOT=1`. Then `pnpm gen:types`.
- **Effort:** medium

### 4.11 — Add a contract test that diffs runtime router vs OpenAPI snapshot
- **File (new):** `/Users/billyribeiro/Desktop/my-websites/swings/backend/tests/openapi_router_parity.rs`
- **Change:** at startup, walk the `Router` and assert every `(method, path)` pair has a matching `paths.<path>.<method>` entry in the OpenAPI snapshot. Catches future doubled-prefix and unmounted-router bugs.
- **Effort:** medium

---

## Phase 5 — Lint & format (both stacks)

### 5.1 — Run `prettier --write .` (LINT-2)
- **Change:** `pnpm exec prettier --write .` to fix all 223 formatting violations.
- **Verification:** `pnpm exec prettier --check .` exits 0
- **Effort:** trivial
- **Dependency:** none

### 5.2 — Investigate and fix Prettier parser failures on `_globalJsonLd` (LINT-3)
- **File:** `/Users/billyribeiro/Desktop/my-websites/swings/src/routes/+layout.svelte` (likely)
- **Change:** identify the offending block; if it's inside a `<script type="application/ld+json">`, ensure Prettier's embedded-language plugin handles it correctly (may need to upgrade `prettier-plugin-svelte` or wrap the content differently).
- **Verification:** `pnpm exec prettier --check src/routes/+layout.svelte` exits 0 cleanly
- **Effort:** small

### 5.3 — Add `prettier --check` to CI (`ci:frontend`)
- **File:** `/Users/billyribeiro/Desktop/my-websites/swings/package.json`
- **Change:** in the `ci:frontend` script, add `pnpm exec prettier --check .`.
- **Effort:** trivial

---

## Phase 6 — Architecture & pattern fixes

### 6.1 — Re-enable disabled Svelte ESLint rules (CFG-2)
- **File:** `/Users/billyribeiro/Desktop/my-websites/swings/eslint.config.js:45-49`
- **Change:** remove or set to `'warn'` each of:
  - `svelte/no-at-html-tags` — XSS posture; audit all `{@html}` sites and ensure a sanitizer wrapper
  - `svelte/require-each-key` — reconciliation safety
  - `svelte/no-dom-manipulating` — runes anti-pattern
  - `svelte/no-navigation-without-resolve`
  - `svelte/no-unused-svelte-ignore`
- **Verification:** `pnpm lint` runs and surfaces real findings; address each
- **Effort:** medium (audit + fix each surfaced site)

### 6.2 — Add the missing TS strict flags (CFG-1)
- **File:** `/Users/billyribeiro/Desktop/my-websites/swings/tsconfig.json`
- **Change:** add to `compilerOptions`:
  ```json
  "noImplicitReturns": true,
  "noFallthroughCasesInSwitch": true,
  "noUncheckedIndexedAccess": true,
  "exactOptionalPropertyTypes": true,
  "noUnusedLocals": true,
  "noUnusedParameters": true
  ```
- **Verification:** `pnpm check` — fix any newly-surfaced errors (likely a few; `noUncheckedIndexedAccess` typically surfaces several)
- **Effort:** medium

### 6.3 — Fix Vercel `vercel.json` to ship CSP headers on prerendered pages (CFG-3)
- **File:** `/Users/billyribeiro/Desktop/my-websites/swings/vercel.json`
- **Change:** add a `headers` block matching the policy in `hooks.server.ts:134`:
  ```json
  {
    "headers": [
      {
        "source": "/(.*)",
        "headers": [
          { "key": "Content-Security-Policy", "value": "default-src 'self'; ..." },
          { "key": "Strict-Transport-Security", "value": "max-age=31536000; includeSubDomains; preload" },
          { "key": "X-Content-Type-Options", "value": "nosniff" },
          { "key": "Referrer-Policy", "value": "strict-origin-when-cross-origin" },
          { "key": "Permissions-Policy", "value": "camera=(), microphone=(), geolocation=()" }
        ]
      }
    ],
    "rewrites": [...]
  }
  ```
- **Note:** the static CSP cannot include the per-request nonce; either drop the nonce on prerendered pages and use `'strict-dynamic'`, or accept that `<script>` policy on static pages is `'self' 'unsafe-inline'`.
- **Effort:** medium (CSP tuning is finicky)

### 6.4 — Gate Vercel preview rewrites by branch (CFG-4)
- **File:** `/Users/billyribeiro/Desktop/my-websites/swings/vercel.json:5`
- **Change:** replace the hard-coded production URL with an env-driven destination per environment. Vercel supports `VERCEL_ENV`-based env vars; configure `API_PROXY_DESTINATION` per environment in the Vercel dashboard.
- **Effort:** small

### 6.5 — Choose between `render.yaml` and `railway.toml` (CFG-5, CFG-6)
- **Decision:** Railway is the canonical host (per Vercel rewrite). Either:
  - **(A)** delete `render.yaml` and `backend/nixpacks.toml`, document that Render is no longer a deploy target.
  - **(B)** sync `render.yaml` envVars with `Config::assert_production_ready` (add 12+ missing vars) and add `APP_ENV: production`. Delete `nixpacks.toml`.
- **Effort:** small (A) / medium (B)

### 6.6 — Add HEALTHCHECK to Dockerfile (CFG-7)
- **File:** `/Users/billyribeiro/Desktop/my-websites/swings/Dockerfile`
- **Change:** in the runtime stage, install `curl` (or use a tiny static health-check binary) and add `HEALTHCHECK CMD curl -f http://127.0.0.1:3001/api/health || exit 1`.
- **Effort:** trivial

### 6.7 — Drop `libssl-dev` from Dockerfile builder (CFG-9)
- **File:** `/Users/billyribeiro/Desktop/my-websites/swings/Dockerfile:38`
- **Change:** remove `libssl-dev pkg-config` from the apt install list.
- **Verification:** `docker build .` succeeds; resulting binary still works
- **Effort:** trivial

---

## Phase 7 — Dead code, dependencies, config integrity

### 7.1 — Decide on `commerce/repo.rs` (TEST-3)
- **File:** `/Users/billyribeiro/Desktop/my-websites/swings/backend/src/commerce/repo.rs`
- **Decision required:** wire the 14 unused `pub async fn`s into `handlers/products.rs` (EC-01 refactor) OR delete the file.
- **Effort:** medium (wire) / trivial (delete)

### 7.2 — Wire or delete `email.rs` transactional surface (DEAD-3)
- **File:** `/Users/billyribeiro/Desktop/my-websites/swings/backend/src/email.rs:289,315,334,355`
- **Decision required:** the codebase appears to use the `notifications/...` pipeline for emails. The `email.rs` functions (`send_password_reset, send_welcome, send_subscription_confirmation, send_subscription_cancelled`) have no callers. Either route the flows through `email.rs` OR delete the dead surface.
- **Effort:** small

### 7.3 — Remove 8 unimported `.svelte` components (DEAD-1)
- **Files:** `lib/components/ui/{DateRangePicker,LivePriceTicker,Skeleton}.svelte`, `lib/components/admin/analytics/AnalyticsDashboard3d.svelte`, `lib/components/charts/{CohortHeatmap,GrowthChart,FunnelChart}.svelte`, `lib/components/forms/RepeaterField.svelte`
- **Decision required:** confirm each is actually unused (some may be planned features); delete or wire.
- **Effort:** trivial (delete) / large (wire)

### 7.4 — Add `deny.toml` (DEP-7)
- **File (new):** `/Users/billyribeiro/Desktop/my-websites/swings/backend/deny.toml`
- **Change:** create with the project's license allowlist (MIT, Apache-2.0, 0BSD, BSD-3-Clause, BSD-2-Clause, MPL-2.0, ISC, CC0-1.0, Unicode-DFS-2016) plus known exceptions. Reference: https://embarkstudios.github.io/cargo-deny/
- **Verification:** `cargo deny --manifest-path backend/Cargo.toml check licenses` — no rejections
- **Effort:** small

### 7.5 — Add `license` field to `swings-api` (DEP-6)
- **File:** `/Users/billyribeiro/Desktop/my-websites/swings/backend/Cargo.toml:1-4`
- **Change:** add `license = "UNLICENSED"` and `publish = false` to `[package]`.
- **Effort:** trivial

### 7.6 — Install `cargo-audit`, `cargo-outdated`, `cargo-udeps` on CI (DEP-10)
- **Files:** `/Users/billyribeiro/Desktop/my-websites/swings/.github/workflows/security.yml` (or wherever CI lives)
- **Change:** add `cargo install cargo-audit cargo-outdated cargo-udeps --locked` to the security job; run all three.
- **Effort:** small

### 7.7 — Bump `aws-sdk-s3` to collapse 9 lock-file duplicates (DEP-9)
- **Sub-task of Phase 1.5.** Already covered.

### 7.8 — Bump `rand` to `0.9` to collapse rand-family triplication (DEP-9)
- **File:** `/Users/billyribeiro/Desktop/my-websites/swings/backend/Cargo.toml:53`
- **Change:** `rand = "0.8"` → `rand = "0.9"`. Update call sites for any 0.8→0.9 API breaks (mostly `thread_rng()` → `rng()`).
- **Effort:** small

### 7.9 — Sync `.env.example` files with code (CFG cross-cutting)
- **Files:** `/Users/billyribeiro/Desktop/my-websites/swings/.env.example`, `/Users/billyribeiro/Desktop/my-websites/swings/backend/.env.example`
- **Change:** verify every key in the example files is read by the code; remove dead keys; add missing required keys (cross-reference with `Config::assert_production_ready`). Clarify the duplicate `STRIPE_SECRET_KEY` documentation.
- **Effort:** small

### 7.10 — Cleanup unreferenced TS type exports (DEAD-2)
- **Files:** `src/lib/api/types.ts`, `src/lib/api/admin-security.ts`, `src/lib/utils/animations.ts`, etc.
- **Change:** delete ~80 unused exports. Some (the `__resetXxxForTests` test fixtures) should be wired into the corresponding spec files; others are simply dead.
- **Effort:** medium

### 7.11 — Resolve TODOs (DEAD-4)
- 16 TODOs, none SECURITY/CRITICAL. Triage each:
  - `notifications/channels/{discord,slack,sms,push,in_app,webhook}.rs` — feature scaffolding; track in product backlog
  - `consent/integrity.rs:97` — scheduler not wired
  - `handlers/products.rs:709` — R2 cleanup on asset delete (cleanup follow-up)
  - others: doc fixes / minor refactors
- **Effort:** large (each is its own subtask)

### 7.12 — Decide on Tailwind CSS v4 (JS-5)
- **Decision:** AGENTS.md §1 references TailwindCSS v4 but `package.json` has zero Tailwind packages. Either install + wire it (per AGENTS.md) or update AGENTS.md to remove the reference.
- **Effort:** trivial (doc edit) / large (Tailwind install + utility-class adoption)

---

## Phase 8 — Test coverage gaps

### 8.1 — Fix browser-mode Vitest config (FETEST-3, BLOCKER for the test infrastructure itself)
- **File:** `/Users/billyribeiro/Desktop/my-websites/swings/vitest.browser.config.ts:9`
- **Change:**
  ```ts
  // before
  import { defineConfig } from 'vitest/config';
  // ...
  browser: { provider: 'playwright', ... }
  // after
  import { defineConfig } from 'vitest/config';
  import { playwright } from '@vitest/browser-playwright';
  // ...
  browser: { provider: playwright(), ... }
  ```
- **Verification:** `pnpm test:browser` runs all 6 spec files
- **Effort:** trivial
- **Dependency:** none

### 8.2 — Fix Playwright admin auth fixture (FETEST-2 Cluster D)
- **File:** `/Users/billyribeiro/Desktop/my-websites/swings/e2e/fixtures/auth.ts` (or wherever `authedAdminTest` lives)
- **Investigation:** verify the fixture establishes a session via `/api/auth/login` and stores the resulting cookies in the Playwright context.
- **Likely cause:** cookies stored under wrong domain, or session storage path changed in BFF migration.
- **Effort:** small to medium (depends on the cause)

### 8.3 — Filter Vercel analytics noise in Playwright console-error guard (FETEST-2 Cluster A)
- **File:** `/Users/billyribeiro/Desktop/my-websites/swings/e2e/smoke/home.spec.ts:38`
- **Change:** widen the regex to also catch the Firefox/WebKit phrasing:
  ```ts
  /Failed to load resource|ERR_CONNECTION|favicon|\/api\/analytics|_vercel\/(insights|speed-insights)/
  ```
- **Effort:** trivial

### 8.4 — Update pricing CTA test for new behavior (FETEST-2 Cluster B)
- **File:** `/Users/billyribeiro/Desktop/my-websites/swings/e2e/smoke/pricing.spec.ts:44`
- **Change:** update the assertion to reflect the new redirect destination (Stripe Checkout or `/pricing/monthly`).
- **Effort:** trivial

### 8.5 — Dismiss ConsentBanner before logout in auth specs (FETEST-2 Cluster C)
- **File:** `/Users/billyribeiro/Desktop/my-websites/swings/e2e/auth/register-login.spec.ts:38`
- **Change:** before calling `logout()`, dismiss the consent banner (click "Accept" or "Reject all"), or dismiss it in a `test.beforeEach`.
- **Effort:** trivial

### 8.6 — Add tests for the 5 new UI primitives (FETEST-4)
- **Files (new):**
  - `/Users/billyribeiro/Desktop/my-websites/swings/src/lib/components/ui/Tooltip.svelte.spec.ts`
  - `/Users/billyribeiro/Desktop/my-websites/swings/src/lib/components/ui/Toaster.svelte.spec.ts`
  - `/Users/billyribeiro/Desktop/my-websites/swings/src/lib/components/ui/ConfirmDialog.svelte.spec.ts`
  - `/Users/billyribeiro/Desktop/my-websites/swings/src/lib/components/ui/ConfirmDialogHost.svelte.spec.ts`
  - `/Users/billyribeiro/Desktop/my-websites/swings/src/lib/components/ui/DateRangePicker.svelte.spec.ts`
- **Coverage:** each: render, hover/focus, keyboard, escape, props.
- **Effort:** medium (5 components × ~100 LoC each)
- **Dependency:** Phase 8.1

### 8.7 — Add tests for `lib/api/client.ts` (FETEST-5)
- **File (new):** `/Users/billyribeiro/Desktop/my-websites/swings/src/lib/api/client.test.ts`
- **Coverage:** request, retry on 401, refresh flow, error parsing (covers Phase 4.1 fix), `getBlob` filename parsing.
- **Effort:** medium

### 8.8 — Add tests for `utils/checkout.ts` and `utils/safeHtml.ts` (FETEST-6)
- **Files (new):** `lib/utils/checkout.test.ts`, `lib/utils/safeHtml.test.ts`
- **Effort:** small

### 8.9 — Add tests for the 4 untested stores (FETEST-7)
- **Files (new):** `lib/stores/{auth,confirm,modal,toast}.svelte.spec.ts`
- **Coverage:** state transitions, persistence (auth → localStorage), queue ordering (confirm).
- **Effort:** medium

### 8.10 — Wire observability test (TEST-2)
- **File:** `/Users/billyribeiro/Desktop/my-websites/swings/backend/tests/observability.rs:72`
- **Change:** install observability middleware in `tests/support/app.rs::build_router`; remove `#[ignore]`.
- **Effort:** small (per `docs/wiring/OBSERVABILITY-WIRING.md`)

### 8.11 — Add unit tests for `services::pricing_rollout` and `services::storage` (TEST-4, TEST-5)
- **Files:** add inline `#[cfg(test)]` blocks; pure-function helpers like `generate_key`, `public_url_for_key` are cheap to test.
- **Effort:** small

### 8.12 — Install `cargo-llvm-cov` and add coverage report to CI (TEST-6)
- **Change:** `cargo install cargo-llvm-cov`; add to CI; target ≥ 80% line coverage on backend, ≥ 70% on frontend.
- **Effort:** small

---

## Suggested execution order

| Day | Phase items |
|---|---|
| **1** | 1.1 (doubled-prefix), 1.2 (mount forms), 1.4 (idempotency cookie), 1.5 (RUSTSEC), 1.6 (JWT secret), 4.1 (error shape), 4.2 (admin/users), 4.8 (logout cookie carry) — most of these are 1-line edits |
| **2** | 4.3 (subs list/stats), 4.4 (popups analytics), 4.5 (coupons stats), 4.6 (member self-service), 4.7 (auto Idempotency-Key) |
| **3** | 1.3 (forms builder decision), 2.6 (`$effect` migration), 2.7 (`untrack` in FormRenderer), 2.8 (admin/dashboard +error), 2.9 (root +error id fix) |
| **4** | 2.2 (drawer refactor sweep — all 17 sites), 2.3 (CommandPalette/EditorToolbar dialog), 2.4 (TradersModal) |
| **5** | 2.5 (input labels), 2.10 (tap targets), 8.3-8.5 (Playwright console/pricing/banner fixes), 8.1 (browser Vitest fix) |
| **6** | 8.2 (admin auth fixture), 5.1 (`prettier --write`), 5.2 (parser failure fix), 5.3 (`prettier --check` in CI) |
| **7** | 6.1 (Svelte ESLint rules), 6.2 (TS strict flags) — surfaces follow-up work |
| **8** | 6.3 (Vercel CSP headers), 6.4 (Vercel preview rewrites), 6.5 (Render decision), 6.6-6.7 (Dockerfile cleanup) |
| **9** | 7.1 (`commerce/repo.rs`), 7.2 (`email.rs`), 7.3 (unimported components), 7.4 (`deny.toml`), 7.5 (license field) |
| **10** | 7.6 (cargo-audit/outdated/udeps in CI), 7.7-7.8 (lock dedupe), 7.9 (`.env.example` sync), 7.10 (TS export cleanup) |
| **11+** | 8.6-8.9 (UI primitive tests, client.ts tests, util tests, store tests), 8.10 (observability test), 8.11-8.12 (Rust unit tests + coverage tooling) |

Total realistic effort: **~3 weeks at one engineer working through the list**, faster with parallel tracks. Phases 1-2 alone unblock the production runtime issues and are achievable in a single sprint.

---

End of plan.
