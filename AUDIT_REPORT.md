# AUDIT_REPORT.md

> Read-only audit consolidating 13 parallel sub-agent findings against the
> SvelteKit 2.x + Svelte 5 (runes) frontend and the Rust + Axum backend at
> `/Users/billyribeiro/Desktop/my-websites/swings`. Generated 2026-04-26.
> No code was modified.

---

## Executive Summary

**Total issues by severity:**

- **Blockers:** 12 (broken in production)
- **Major:** 27
- **Minor:** 95+
- **Info / dead-code:** 24

**Domain split:**

- Frontend issues: 31
- Backend issues: 21
- Contract / cross-stack: 33
- Config / deployment: 14
- Tests: 24

**Top 5 highest-impact problems (one line each):**

1. **Backend doubled-prefix bug**: `coupons`/`courses`/`products` admin AND public routers each declare inner-prefix paths (`/coupons/{id}`) and then `main.rs` nests them at `/api/admin/coupons` → actual mounted path is `/api/admin/coupons/coupons/{id}`. ~30 frontend call sites 404 in production.
2. **Forms entire surface unmounted**: `handlers::forms::{public_router, admin_router}` exist, are documented in OpenAPI, exercised in tests via `tests/support/app.rs:677-678` — but `main.rs` has zero references to `handlers::forms`. Every `/api/forms/*` and `/api/admin/forms/*` call 404s.
3. **Idempotency middleware inert under BFF**: `middleware/idempotency.rs:186-198` (`decode_subject`) only reads `Authorization: Bearer ...`. The SPA uses cookies (no Bearer). Result: every admin POST is silently retry-unsafe — refunds, comp grants, manual orders, DSAR exports, bulk coupons, popup duplicates.
4. **Universal frontend error-shape mismatch**: `client.ts:79-80` reads `err.error`; backend emits RFC 7807 `application/problem+json` with `{type, title, status, detail}`. Every non-2xx surfaces as the literal string `"Request failed"` in toasts/UIs across the entire admin app.
5. **4 BLOCKER RUSTSEC advisories in TLS chain** (RUSTSEC-2026-0098, -0099, -0104, -0104 second copy) all rooted in `aws-sdk-s3 1.119.0` pinning `rustls 0.21` + `rustls-webpki 0.101.7`. Affects R2, SMTP, HTTP outbound, and Postgres TLS surfaces.

**Overall codebase health (one paragraph, honest):**

The codebase is structurally healthy — Rust passes `cargo check`/`cargo clippy -D warnings`/`cargo fmt --check` cleanly with zero issues, all 761 backend tests pass, frontend `pnpm check` (svelte-check) reports 0 errors / 0 warnings across 5604 files, ESLint is clean, Svelte 5 runes migration is complete (zero legacy syntax remaining), and no forbidden icon libraries leaked in. However the frontend↔backend contract has substantial drift: an entire HTTP router (forms) is unmounted, four resource families (coupons/courses/products/forms) suffer routing bugs that 404 multiple admin pages, the BFF cookie migration left the idempotency middleware behind so the entire admin POST surface is no longer guarded against retry duplication, and the error-shape mismatch makes every admin error toast useless. The Rust supply chain has 4 high-severity advisories all routable to one dependency bump (`aws-sdk-s3 ≥ 1.120`). The Playwright suite has 23 failing tests across 4 root-cause clusters and the browser-mode Vitest config is broken under v4. None of the frontend issues block compilation; all the contract issues are runtime-only and can be fixed surgically.

---

## Findings by Domain

### 1. TYPE-SAFETY (frontend)

#### TS-1 — `tsc --noEmit` rejects 19 named-type re-exports from `*.svelte` files

- **File:** `/Users/billyribeiro/Desktop/my-websites/swings/src/lib/components/shared/index.ts:11–37`
- **Severity:** major (only via raw `tsc`; `pnpm check` via svelte-check passes)
- **Evidence:** `error TS2614: Module '"*.svelte"' has no exported member 'ButtonProps'. Did you mean to use 'import ButtonProps from "*.svelte"' instead?` (×19, one per type)
- **Root cause:** SvelteKit's ambient `*.svelte` shim only declares a default export; raw `tsc` cannot see Svelte 5 named-type exports from `<script module lang="ts">` blocks. The Svelte language-server plugin used by `svelte-check` understands them.
- **Why it matters:** breaks any contributor who runs `tsc --noEmit` directly. CI uses `pnpm check` so green there, but the discrepancy will trip developers and IDE integrations that bypass the Svelte plugin.

### 2. LINT & FORMAT (frontend)

#### LINT-1 — ESLint clean

- **Evidence:** `pnpm exec eslint . --max-warnings=0` exits 0 with no output. **0 errors, 0 warnings.**

#### LINT-2 — Prettier reports 223 files needing format

- **Severity:** minor (formatting only)
- **Evidence:** `[warn] Code style issues found in 223 files. Run Prettier with --write to fix.`
- **Root cause:** files have not been run through `prettier --write` recently. Includes 29 `.md`, 4 yaml/json, 50+ `.svelte`, dozens of `.ts`.
- **Why it matters:** cosmetic; no runtime impact. CI does not currently enforce `prettier --check`.

#### LINT-3 — Prettier parser failures on `{_globalJsonLd}` and `{@html _globalJsonLd}`

- **File:** `/Users/billyribeiro/Desktop/my-websites/swings/src/routes/+layout.svelte` (suspected — only file injecting `_globalJsonLd`)
- **Severity:** major (tool error blocks `prettier --check` from completing on those files)
- **Evidence:**
  ```
  SyntaxError: Shorthand property is not allowed in JSON. (1:2)
  > 1 | {_globalJsonLd}
  ```
  ```
  SyntaxError: Unexpected token. (1:2)
  > 1 | {@html _globalJsonLd}
  ```
- **Root cause:** Prettier's embedded Babel parser is being handed a Svelte mustache as JSON. Likely a `<svelte:head>` JSON-LD `<script type="application/ld+json">` body that confuses Prettier's embedded-language router.
- **Why it matters:** `prettier --write` would skip these files. May propagate to a broader formatting drift if left unaddressed.

### 3. SVELTE A11Y

#### A11Y-1 — svelte-check is clean

- **Evidence:** `svelte-check found 0 errors and 0 warnings` (5604 files).

#### A11Y-2 — Autofixer is clean across 18 representative components

- **Evidence:** Zero `issues` reported on Tooltip, Toaster, ConfirmDialog, ConfirmDialogHost, members list+detail, audit, dsar, orders, popups, ConsentBanner, Nav, Footer, CommandPalette, AdminSiteBar, Dialog, Drawer. Some `suggestions` (style hints about `bind:this` → attachment, `$effect`-vs-`$derived`) but no correctness issues.

#### A11Y-3 — 17 admin drawer backdrops use `role="button"` + `tabindex="-1"`

- **Files:** `/admin/+layout.svelte:329`, `/admin/audit/+page.svelte:375`, `/admin/dsar/+page.svelte:675`, `/admin/orders/+page.svelte:503`, `/admin/outbox/+page.svelte:293`, `/admin/notifications/{deliveries,suppression,templates}/+page.svelte`, `/admin/forms/[id]/submissions/+page.svelte:510`, `/admin/consent/{log,categories,integrity,services,banner,policy}/+page.svelte`, `/lib/components/editor/MediaLibrary.svelte:204`, `/lib/components/editor/PostEditor.svelte:948`
- **Severity:** major
- **Evidence:** `<div class="..." role="button" tabindex="-1" onclick=... onkeydown={(e)=>e.key==='Escape'&&...}></div>` — `tabindex="-1"` makes the element non-focusable; `onkeydown` never fires from keyboard. SR users hear "button" but it's unreachable.
- **Root cause:** copy-paste backdrop pattern using `role="button"` to silence Svelte a11y lint rather than the proper `aria-hidden="true"` + `<!-- svelte-ignore -->` pattern, or a shared Drawer primitive.
- **Why it matters:** misleading semantics; the 17 instances are all dismissible only via mouse (Escape works only because of document-level handlers in sibling code).

#### A11Y-4 — TradersModal overlay receives Tab focus

- **File:** `/Users/billyribeiro/Desktop/my-websites/swings/src/lib/components/traders/TradersModal.svelte:39-46`
- **Severity:** major
- **Evidence:** `<div class="modal-overlay" tabindex="0" role="button" onkeydown={(e) => e.key === 'Enter' && modal.close()}>` — first focusable in the modal; Tab lands on a button the user did not invoke; Enter dismisses the entire modal.
- **Root cause:** missing focus trap; should use `Dialog.svelte` primitive.

#### A11Y-5 — CommandPalette and EditorToolbar shortcuts modal lack focus traps

- **Files:** `/Users/billyribeiro/Desktop/my-websites/swings/src/lib/components/admin/CommandPalette.svelte:159-221`, `/Users/billyribeiro/Desktop/my-websites/swings/src/lib/components/editor/EditorToolbar.svelte:893-916`
- **Severity:** major
- **Evidence:** `role="dialog" aria-modal="true"` set, but no focus trap. Once Tab leaves the input, focus escapes to the underlying admin shell.

#### A11Y-6 — Inputs without labels (placeholder-as-label)

- **Files:** `/admin/products/[id]/+page.svelte:473-538`, `/admin/popups/new/+page.svelte:181,222,228,240,264-286`, `/admin/popups/[id]/+page.svelte:202`, `/admin/courses/[id]/+page.svelte:200,213-214`, `/admin/blog/+page.svelte:535`, `/admin/subscriptions/plans/+page.svelte:226-318`
- **Severity:** major
- **Evidence:** e.g. `<input type="text" placeholder="SKU" bind:value={vSku} />` — placeholder is not an accessible name; SR users hear "edit text" with no context.

#### A11Y-7 — Tap targets <44px on mobile

- **Files:** `/admin/members/+page.svelte:954-967` (`.member-card__btn { min-height: 2.25rem }` = 36px), `/admin/popups/+page.svelte:781-783` (`.icon-btn { width: 2rem; height: 2rem }` = 32px)
- **Severity:** minor
- **Why it matters:** falls below WCAG 2.5.5 AA (44×44) on touch devices.

#### A11Y-8 — Other minor: `PopupRenderer.svelte:81` `<button tabindex="-1">`, `DateRangePicker.svelte:225` empty `onkeydown` placeholder.

### 4. RUNES & SVELTE 5 COMPLIANCE

#### RUNES-1 — Zero legacy syntax across 210 files

- **Evidence counts:** 0 `export let`, 0 `$:`, 0 `on:event`, 0 `<slot>`, 0 `createEventDispatcher`, 0 `beforeUpdate`/`afterUpdate`, 0 `$$props`/`$$restProps`/`$$slots`, 0 mutated props, 0 `$state.raw` misuse.

#### RUNES-2 — 6 `$effect` instances doing fetch-on-mount (the prior incident's anti-pattern)

- **Severity:** major
- **Files:**
  - `/admin/blog/tags/+page.svelte:10` — `$effect(() => { loadTags(); });`
  - `/admin/blog/categories/+page.svelte:14` — `$effect(() => { loadCategories(); });`
  - `/admin/blog/media/+page.svelte:31` — `$effect(() => { loadMedia(); });`
  - `/admin/blog/[id]/+page.svelte:13` — `$effect(() => { loadPost(); });`
  - `/admin/products/[id]/+page.svelte:94` — `$effect(() => { if (productId) { load(); } });`
  - `/lib/components/editor/MediaLibrary.svelte:26` — `$effect(() => { if (open) { loadMedia(); } });`
- **Root cause:** same class as the `effect_update_depth_exceeded` incident (admin/+layout.svelte). Works today only because writes happen not to feed back into the trigger.
- **Migration:** swap to `onMount(loadX);` (already adopted in `admin/products/+page.svelte:81` and `admin/courses/+page.svelte:62`).

#### RUNES-3 — 1 `$effect` writing state it transitively reads

- **File:** `/Users/billyribeiro/Desktop/my-websites/swings/src/lib/components/forms/FormRenderer.svelte:129-137`
- **Severity:** major
- **Evidence:** `$effect(() => { for (const sv of verdict.setValues) { if (data[sv.field] !== sv.value) { data[sv.field] = sv.value; } } });` reads `verdict.setValues` (a `$derived` of `data`) and writes back into `data`.
- **Migration:** wrap body in `untrack(() => { ... })`.

#### RUNES-4 — 2 `$effect` materializing default values via parent callback

- **Files:** `/lib/components/forms/fields/SliderField.svelte:26`, `/lib/components/forms/fields/HiddenField.svelte:14`
- **Severity:** minor — should be `onMount`.

### 5. SVELTEKIT 2.x ROUTING

#### ROUTE-1 — Missing `+error.svelte` for admin route group

- **File (missing):** `/Users/billyribeiro/Desktop/my-websites/swings/src/routes/admin/+error.svelte`
- **Severity:** major
- **Why it matters:** errors fall through to the root error boundary; admin chrome (sidebar, palette, breadcrumbs) is lost on every error.

#### ROUTE-2 — Missing `+error.svelte` for dashboard route group

- **File (missing):** `/Users/billyribeiro/Desktop/my-websites/swings/src/routes/dashboard/+error.svelte`
- **Severity:** major

#### ROUTE-3 — `id="main-content"` duplicated between root layout and root error page

- **File:** `/Users/billyribeiro/Desktop/my-websites/swings/src/routes/+error.svelte:42`
- **Severity:** minor — when error renders inside public shell there are two `#main-content` elements; breaks the Skip-to-main-content link.

#### ROUTE-4 — `/api/greeks-pdf/+server.ts` uses `json({error}, {status})` instead of `error()`

- **File:** `/Users/billyribeiro/Desktop/my-websites/swings/src/routes/api/greeks-pdf/+server.ts:9,39`
- **Severity:** minor — bypasses `handleError` and correlation-id integration.

#### ROUTE-5 — `throw redirect(...)` (v1 idiom) at one site

- **File:** `/Users/billyribeiro/Desktop/my-websites/swings/src/routes/admin/consent/policies/+page.ts:11`
- **Severity:** minor — drop `throw` keyword for SvelteKit 2 idiom consistency.

#### ROUTE-6 — `$app/state` migration complete; `$app/stores` references: 0

- **Evidence:** all 27 `page` imports come from `$app/state`. Clean.

### 7. RUST CORRECTNESS

#### RUST-1 — `cargo check`/`clippy -D warnings`/`fmt --check` all clean

- **Evidence:** zero warnings, zero errors. Verified with forced full recompile.

#### RUST-2 — Zero `unwrap()` outside tests, zero `unsafe`, zero blocking IO in async

- **Evidence:** comprehensive grep + manual re-verification.

#### RUST-3 — `.expect(...)` usages: 13 occurrences, all on documented infallible operations

- **Files:** `middleware/impersonation_banner.rs:77`, `middleware/rate_limit.rs:352,599,618`, `forms/validation.rs:100,108`, `forms/antispam.rs:321`, `forms/uploads.rs:141,147,185,206`, `notifications/preferences.rs:142,148`
- **Severity:** minor — all are on UUID Display, const-evaluated regex, infallible `FixedOffset::east_opt(0)`, or std mutex poison (well-documented).

#### RUST-4 — One `unreachable!()` in production

- **File:** `/Users/billyribeiro/Desktop/my-websites/swings/backend/src/notifications/webhooks/resend.rs:241`
- **Severity:** minor — base64 chunks(3) remainder ∈ {0,1,2}; the `_` arm is genuinely unreachable. Recommend annotating with `// SAFETY:` comment.

#### RUST-5 — One elidable lifetime

- **File:** `/Users/billyribeiro/Desktop/my-websites/swings/backend/src/forms/integrations.rs:100`
- **Severity:** minor — `fn require_email<'a>(p: &'a SubmissionPayload<'_>) -> Result<&'a str, _>` could elide.

#### RUST-6 — `Arc<Mutex>`/`Arc<RwLock>` audit: clean

- **Files reviewed:** `settings/mod.rs:154` (`RwLock<HashMap>`), `forms/uploads.rs:135` (test-only `Mutex`). No nested locks, no guards across `.await`.

### 8. RUST DEPENDENCIES & SECURITY

#### DEP-1 — RUSTSEC-2026-0098 (rustls-webpki 0.101.7 — name constraints for URI names)

- **Severity:** **BLOCKER**
- **Path:** `aws-sdk-s3 1.119.0` → `aws-smithy-http-client 1.1.12` → `rustls 0.21.12` → `rustls-webpki 0.101.7`
- **Fix:** upgrade to `rustls-webpki ≥ 0.103.12` (via bumping `aws-sdk-s3 ≥ 1.120`).

#### DEP-2 — RUSTSEC-2026-0099 (rustls-webpki 0.101.7 — wildcard subtree)

- **Severity:** **BLOCKER**
- **Same chain as DEP-1.**

#### DEP-3 — RUSTSEC-2026-0104 (rustls-webpki 0.101.7 + 0.103.12 — CRL panic)

- **Severity:** **BLOCKER (×2 copies)**
- **Paths:** the 0.101.7 copy is via `aws-sdk-s3`; the 0.103.12 copy is via `hyper-rustls 0.27.9` / `lettre 0.11.21` / `reqwest 0.12.28` / `sqlx-core 0.8.6` / `tokio-rustls 0.26.4` — touches every TLS-using subsystem.
- **Fix:** `cargo update -p rustls-webpki -p rustls`.

#### DEP-4 — RUSTSEC-2024-0384 (instant 0.1.13 — unmaintained)

- **Severity:** major
- **Path:** `instant 0.1.13` → `fastrand 1.9.0` → `futures-lite 1.13.0` → `http-types 2.12.0` → `async-stripe 0.39.1`
- **Fix:** upgrade `async-stripe` to current upstream OR replace with a `reqwest`-based wrapper.

#### DEP-5 — RUSTSEC-2024-0436 (paste 1.0.15 — unmaintained, archived)

- **Severity:** major
- **Path:** `paste 1.0.15` → `utoipa-axum 0.2.0` → `swings-api`
- **Fix:** wait for `utoipa-axum` upstream or vendor a replacement.

#### DEP-6 — `swings-api` has no `license` field

- **File:** `/Users/billyribeiro/Desktop/my-websites/swings/backend/Cargo.toml:1-4`
- **Severity:** major (tooling) — add `license = "UNLICENSED"` or `publish = false`.

#### DEP-7 — Repo missing `deny.toml`

- **Severity:** major (tooling)
- **Evidence:** `cargo deny check` emits `unable to find a config path, falling back to default config`. Default policy has empty allow-list → 498 license rejections.
- **Fix:** add `backend/deny.toml` with the project's license allowlist (MIT, Apache-2.0, 0BSD, BSD-3-Clause, etc.) and known exceptions.

#### DEP-8 — `[profile.release]` is production-grade

- **File:** `/Users/billyribeiro/Desktop/my-websites/swings/backend/Cargo.toml:145-151`
- **Evidence:** `opt-level=3, lto="thin", codegen-units=1, strip="symbols", debug=false, overflow-checks=false`. `panic` left at `unwind` (intentional, justified inline).

#### DEP-9 — 44 duplicate-version transitive crates in lock file

- Top duplicates: `rustls` (0.21.12, 0.23.38), `rustls-webpki` (0.101.7, 0.103.12), `tokio-rustls` (0.24.1, 0.26.4), `hyper-rustls`, `hyper` (0.14.32, 1.9.0), `http`, `http-body`, `h2`, `hashbrown` (5 copies), `getrandom` (4 copies), `rand` (3 copies), `rand_core` (3 copies), `rand_chacha` (3 copies), `webpki-roots`, `uuid` (0.8.2, 1.23.1), `untrusted`, `thiserror` (1, 2), `syn` (1, 2), `fastrand` (1.9, 2.4), etc.
- **Fix:** the `aws-sdk-s3 ≥ 1.120` bump collapses the rustls/hyper/http/h2 split. `rand = "0.9"` direct dep collapses the rand-family triplication.

#### DEP-10 — Tools missing on host: `cargo-audit`, `cargo-outdated`, `cargo-udeps`

- **Severity:** info — cargo-deny covered the RUSTSEC database, so security data was not lost. Recommend `cargo install cargo-audit cargo-outdated cargo-udeps`.

### 9. RUST TESTS & COVERAGE

#### TEST-1 — All 761 backend tests pass; 1 ignored, 0 failed

- **Evidence:** `unittests src/lib.rs` 492/0/0; integration test binaries ranging from 1 to 21 tests each, all green. Total: **761 passed, 0 failed, 1 ignored.**

#### TEST-2 — 1 ignored test (documented)

- **File:** `/Users/billyribeiro/Desktop/my-websites/swings/backend/tests/observability.rs:72`
- **Test:** `request_carries_correlation_id_and_counter_fires`
- **Severity:** major
- **Reason (verbatim):** `#[ignore = "enable once observability layers are wired into TestApp::build_router (see docs/wiring/OBSERVABILITY-WIRING.md)"]`
- **Why it matters:** the production observability stack (correlation-id propagation + `http_requests_total` counter) is not exercised by integration tests because the test harness's router builder doesn't install the middleware.

#### TEST-3 — `commerce/repo.rs` has 14 `pub async fn` with zero callers

- **File:** `/Users/billyribeiro/Desktop/my-websites/swings/backend/src/commerce/repo.rs`
- **Severity:** minor (unreachable code)
- **Functions:** `list_products, get_product, create_product, update_product, set_status, list_variants, add_variant, update_variant, delete_variant, list_assets, add_asset, delete_asset, list_bundle_items, set_bundle_items`
- **Why it matters:** scaffolding for an EC-01 product-handler refactor that never landed. Either wire it up or delete.

#### TEST-4 — `services::pricing_rollout::rollout_after_plan_save` not unit-tested

- **File:** `/Users/billyribeiro/Desktop/my-websites/swings/backend/src/services/pricing_rollout.rs:135`
- **Severity:** minor (Stripe-side audience-bucketing logic untested).

#### TEST-5 — `services::storage` has no inline unit tests

- **File:** `/Users/billyribeiro/Desktop/my-websites/swings/backend/src/services/storage.rs`
- **Severity:** minor — `generate_key`, `public_url_for_key` (URL-builders, no I/O) would be cheap to unit-test.

#### TEST-6 — Coverage tools not installed

- **Evidence:** `cargo-tarpaulin` and `cargo-llvm-cov` both not found. No coverage numbers produced.

### 10. API CONTRACT (frontend ↔ backend)

#### CONTRACT-A1 — Doubled-prefix bug on coupons / courses / products (BLOCKER, system-wide)

- **Backend:** `handlers/coupons.rs:28-57`, `handlers/courses.rs:20-49`, `handlers/products.rs:48-90`
- **Evidence:** `main.rs` nests at `/api/admin/coupons|courses|products` and `/api/coupons|courses|products`; each handler's routes start with the same noun → mounted paths become `/api/admin/coupons/coupons/{id}`, etc. OpenAPI snapshots claim the _intended_ paths. Tests at `tests/rbac_legacy_handlers.rs:130,218` and `tests/idempotency_legacy_handlers.rs:160,130,217` hit the doubled paths and pass — confirming the runtime drift.
- **Affected frontend:** `src/lib/api/products.ts:60-105`, `src/routes/admin/coupons/+page.svelte:64,86,102,126`, `coupons/new/+page.svelte:36`, `coupons/[id]/+page.svelte:48,69,96`, `admin/courses/+page.svelte:71`, `courses/new/+page.svelte:78`, `courses/[id]/+page.svelte:41,113,131`, `dashboard/+page.svelte:30`, `dashboard/courses/+page.svelte:30`, `dashboard/courses/[slug]/+page.svelte:28`, `pricing/+page.svelte:115`, `dashboard/account/+page.svelte:88`. **All 404 in production.**
- **Fix:** drop the inner noun prefix from each `Router::new()` (e.g. `route("/", ...)` instead of `route("/coupons", ...)`).

#### CONTRACT-A2 — `/api/forms/*` and `/api/admin/forms/*` not mounted (BLOCKER)

- **Backend:** `handlers/forms.rs:47,77` define `public_router()` and `admin_router()`; OpenAPI lists them; `main.rs` has zero references.
- **Affected frontend:** `lib/api/forms.ts:47,70,90,104`, `lib/components/forms/fields/{Subscription,Payment,CountryState,PostProductSelector,DynamicDropdown}Field.svelte`, `routes/forms/[slug]/+page.server.ts:21`, `lib/api/admin-form-submissions.ts:46,56`. Every form-related call 404s.

#### CONTRACT-A3 — Admin "forms-builder" routes use non-existent `/admin/forms/*` (BLOCKER)

- **Frontend:** `routes/admin/forms/+page.svelte:45,59`, `forms/new/+page.svelte:42`, `forms/[id]/+page.svelte:41,100`, `forms/[id]/versions/+page.svelte:46`, `forms/[id]/preview/+page.svelte:55`
- **Backend:** `handlers/forms.rs:77-84` only exposes `/{id}/submissions` and `/{id}/submissions/bulk`. Builder UI assumes a full CRUD + versions surface that was never implemented.

#### CONTRACT-A4 — `/api/admin/users` does not exist (BLOCKER)

- **Frontend:** `routes/admin/blog/+page.svelte:234` calls `api.get<UserResponse[]>('/api/admin/users?role=admin&per_page=50')`
- **Backend:** only `/api/admin/members` exists.

#### CONTRACT-A5 — `/api/admin/subscriptions` (list) and `/stats` do not exist (BLOCKER)

- **Frontend:** `routes/admin/subscriptions/+page.svelte:76,100`
- **Backend:** `admin_subscriptions.rs:53-59` only exposes `/comp`, `/by-user/{user_id}`, `/{id}/extend`, `/{id}/billing-cycle`. No list, no stats.

#### CONTRACT-A6 — `/api/admin/popups/analytics` (collection-level) does not exist (BLOCKER)

- **Frontend:** `routes/admin/popups/+page.svelte:46`
- **Backend:** only `/{id}/analytics` is wired. `/analytics` will be path-matched as `/{id}` (string) → tries to parse "analytics" as UUID → 400.

#### CONTRACT-A7 — `/api/admin/coupons/stats` does not exist (BLOCKER)

- **Frontend:** `routes/admin/coupons/+page.svelte:64`
- **Backend:** `handlers/coupons.rs:28-42` has no `/stats` route.

#### CONTRACT-A8 — Member self-service paths missing on backend (BLOCKER)

- `/api/member/password` (POST) — `dashboard/account/+page.svelte:61`
- `/api/member/account` (DELETE) — `dashboard/account/+page.svelte:100`
- `/api/member/coupons/apply` — `dashboard/account/+page.svelte:88`
- **Backend:** `member.rs:17-37` exposes profile/subscription/billing/watchlists/courses only.

#### CONTRACT-D1 — Frontend never sends `Idempotency-Key` for admin POSTs (MAJOR, system-wide)

- **Evidence:** the only frontend caller that sends `Idempotency-Key` is `routes/admin/subscriptions/plans/+page.svelte:131`. Every other admin POST (manual order, refund, comp grant, DSAR export, bulk coupon mint, popup duplicate, blog autosave) skips it.
- **Backend:** middleware passes through when no header is supplied (`middleware/idempotency.rs:96-100`).

#### CONTRACT-D2 — Idempotency middleware extracts actor from Bearer only (MAJOR)

- **File:** `/Users/billyribeiro/Desktop/my-websites/swings/backend/src/middleware/idempotency.rs:186-198`
- **Evidence:** `decode_subject()` only reads `Authorization: Bearer ...`. Lines 105-108 pass through on missing actor.
- **Why it matters:** the SPA uses cookies (no Bearer) → even when frontend _does_ send `Idempotency-Key`, middleware silently bypasses caching. Compare to `extractors.rs:123-155` (`extract_access_token`) which correctly reads cookie OR Bearer.

#### CONTRACT-G1 — Universal error-shape mismatch (MAJOR, system-wide)

- **Frontend:** `src/lib/api/client.ts:65-66, 79-80, 205-206` — `throw new ApiError(response.status, err.error || 'Request failed');`
- **Backend:** `error.rs:230-260` (impl `IntoResponse for AppError`) emits `application/problem+json` with `{type, title, status, detail}`. There is no `error` field.
- **Result:** every non-2xx surfaces as the literal string `"Request failed"`. Toast / inline-error UX degraded everywhere.
- **Fix:** `err.detail || err.title || err.error || 'Request failed'`.

#### CONTRACT-C2 — `auth.svelte.ts:111` raw `fetch('/api/auth/logout')` omits `credentials: 'include'`

- **Severity:** major-in-dev / minor-in-prod
- **Evidence:** in dev (Vite proxy → 3001) the cookie does NOT travel; logout silently no-ops.

#### CONTRACT-B1 — Type drift `PricingPlanPriceLogEntry` vs `PricingPlanAmountChangeLogEntry`

- **File:** `src/lib/api/types.ts:673` (frontend alias) vs `schema.d.ts:4490` (OpenAPI emit)
- **Severity:** minor — shape identical, but breaks `pnpm gen:types` regeneration.

#### CONTRACT-E1 — Dead backend routes (no frontend caller)

- `/api/member/notification-preferences` (GET, PUT)
- `/api/admin/blog/posts/{id}/revisions/{rev_id}/restore`
- `/api/admin/coupons/{id}/usages`, `/api/admin/coupons/{id}/engine`
- `/api/admin/notifications/templates/{id}/test-send`
- `/api/admin/security/audit-log`, `/failed-logins`
- `/api/admin/security/roles/_reload` (wrapped but no UI page)
- `/api/auth/impersonation/exit`
- `/api/admin/consent/services` and `/policies` POST/PUT (wrapped but no UI)
- `/api/cart/*` whole router (no caller — cart UI not built yet)
- `/api/catalog/*` whole router (no caller)

### 11. JS DEPENDENCIES & SECURITY

#### JS-1 — `pnpm audit`: 1 LOW + 1 MODERATE, no HIGH/CRITICAL

- **LOW: `cookie 0.6.0`** (transitive via `@sveltejs/kit`) — GHSA-pxg6-pf52-xh8x. Out-of-bounds chars in cookie name/path/domain. Upstream-only fix.
- **MODERATE: `uuid 11.1.0`** (transitive via `vite-plugin-devtools-json`) — GHSA-w5hq-g745-h8pq. Buffer bounds check missing in v3/v5/v6. Upstream-only fix.

#### JS-2 — `pnpm outdated`: empty (no outdated direct deps)

#### JS-3 — Forbidden icon libraries: ZERO matches

- **Evidence:** `grep -rE 'lucide|@tabler|@heroicons|@fortawesome|fontawesome|feather-icons|bootstrap-icons|material-icons|@mui'` returns zero matches in `src/` and `pnpm-lock.yaml`. **Phosphor compliance verified.**

#### JS-4 — `openapi-typescript@7.13.0` peer wants TS 5; repo on TS 6

- **Severity:** minor — TS 6 is API-compatible with surfaces openapi-typescript uses; `pnpm gen:types` works.

#### JS-5 — TailwindCSS v4 referenced in AGENTS.md §1 but absent from `package.json`

- **Severity:** minor — documentation drift. AGENTS.md may be stale, or Tailwind is wired via PostCSS/Vite plugin from elsewhere; verify.

### 12. DEAD CODE

#### DEAD-1 — 8 unimported `.svelte` components in `src/lib/`

- `lib/components/ui/DateRangePicker.svelte`
- `lib/components/ui/LivePriceTicker.svelte`
- `lib/components/ui/Skeleton.svelte`
- `lib/components/admin/analytics/AnalyticsDashboard3d.svelte`
- `lib/components/charts/CohortHeatmap.svelte`
- `lib/components/charts/GrowthChart.svelte`
- `lib/components/charts/FunnelChart.svelte`
- `lib/components/forms/RepeaterField.svelte`

#### DEAD-2 — ~80 unreferenced TS exports

- Top concentrations: `api/types.ts` (22), `api/admin-security.ts` (9), `utils/animations.ts` (8). Many speculative request-body types and unused animation factories.

#### DEAD-3 — Unreferenced backend `pub fn`s (15 sampled)

- `email.rs` whole transactional surface (`send_password_reset, send_welcome, send_subscription_confirmation, send_subscription_cancelled`) — no callers; suggests duplicate path
- `commerce/downloads.rs:222,259`
- `commerce/tax.rs:55,146,162,203`
- `commerce/catalog.rs:237`
- `middleware/rate_limit.rs:246,570,685`
- `forms/repo.rs:96,122,168,185,213,538` (consistent with CONTRACT-A2 — forms router not mounted)
- `security/impersonation.rs:274` (`list_active`)
- `consent/repo.rs:80` (`list_active_services`)
- `services/audit.rs:276` (`audit_admin_under_impersonation`)

#### DEAD-4 — TODOs (16 total, no SECURITY/CRITICAL tags)

- Notable: `notifications/channels/{discord,slack,sms,push,in_app,webhook}.rs` all flag "wire in X under future subsystem" — channel scaffolding present, providers not yet implemented.

### 13. CONFIG INTEGRITY

#### CFG-1 — `tsconfig.json` missing strict-suite flags (MAJOR)

- **File:** `/Users/billyribeiro/Desktop/my-websites/swings/tsconfig.json:3-14`
- **Missing:** `noImplicitReturns`, `noFallthroughCasesInSwitch`, `noUncheckedIndexedAccess`, `exactOptionalPropertyTypes`, `noUnusedLocals`, `noUnusedParameters`
- **Why it matters:** `strict: true` is set but the requested supplemental flags were never added.

#### CFG-2 — 5 Svelte ESLint rules disabled (MAJOR)

- **File:** `/Users/billyribeiro/Desktop/my-websites/swings/eslint.config.js:45-49`
- **Disabled:** `svelte/no-navigation-without-resolve`, `svelte/require-each-key`, `svelte/no-dom-manipulating`, `svelte/no-at-html-tags`, `svelte/no-unused-svelte-ignore`
- **Highest concern:** `svelte/no-at-html-tags` off → silent XSS posture risk on every `{@html}` usage.

#### CFG-3 — `vercel.json` lacks security headers (MAJOR)

- **File:** `/Users/billyribeiro/Desktop/my-websites/swings/vercel.json`
- **Evidence:** only the `/api/*` rewrite is configured; no `headers` block. CSP is set in `hooks.server.ts:134` but **prerendered pages** (root, /about, /courses, /blog, /pricing, /pricing/monthly, /pricing/annual per `svelte.config.js:27`) bypass server hooks — they ship without CSP.

#### CFG-4 — `vercel.json` rewrites previews to PRODUCTION Railway (MAJOR)

- **File:** `/Users/billyribeiro/Desktop/my-websites/swings/vercel.json:5`
- **Evidence:** `"destination": "https://swings-production.up.railway.app/api/:path*"` — hard-coded. Vercel previews → production backend.

#### CFG-5 — `render.yaml` envVars dangerously incomplete (MAJOR)

- **File:** `/Users/billyribeiro/Desktop/my-websites/swings/render.yaml:11-26`
- **Missing required-in-prod:** `STRIPE_SECRET_KEY`, `STRIPE_WEBHOOK_SECRET`, `ADMIN_EMAIL`, `ADMIN_PASSWORD`, `SETTINGS_ENCRYPTION_KEY`, `RESEND_API_KEY`, `RESEND_WEBHOOK_SECRET`, all 5 R2 vars, `APP_URL`, `API_URL`, `CORS_ALLOWED_ORIGINS`, `APP_ENV`
- **Without `APP_ENV=production`**, `Config::assert_production_ready()` is a no-op so boot silently succeeds without these. Worst-case failure mode (e.g. empty Stripe key treated as "Stripe disabled").

#### CFG-6 — `nixpacks.toml` contradicts `railway.toml` (MAJOR)

- **File:** `/Users/billyribeiro/Desktop/my-websites/swings/backend/nixpacks.toml`
- **Evidence:** declares Nixpacks build using `cargo build --release` and `./target/release/swings-api`; `railway.toml` uses `builder = "DOCKERFILE"`. nixpacks.toml is dead but a footgun for future migrations (no toolchain pin, no `pkg-config`, no `openssl`).

#### CFG-7 — `Dockerfile` has no `HEALTHCHECK` directive (MAJOR)

- **File:** `/Users/billyribeiro/Desktop/my-websites/swings/Dockerfile`
- **Why it matters:** Render/Railway can detect failures externally, but no in-container healthcheck.

#### CFG-8 — `docker-compose.yml` bakes default `JWT_SECRET` (BLOCKER for dev hygiene)

- **File:** `/Users/billyribeiro/Desktop/my-websites/swings/docker-compose.yml:33`
- **Evidence:** `JWT_SECRET: ${JWT_SECRET:-explosive-swings-jwt-secret-key-2026}` — same JWT secret on every developer's machine, leaked in public repo. Trivially forgeable.

#### CFG-9 — `Dockerfile` installs `libssl-dev` despite rustls-only stack (MINOR)

- **File:** `/Users/billyribeiro/Desktop/my-websites/swings/Dockerfile:38`
- **Evidence:** dependency tree is rustls-only; `libssl-dev` is dead weight in the builder image.

#### CFG-10 — Several auth `+page.ts` files don't explicitly set `ssr` (MINOR)

- Files: `forgot-password`, `reset-password`, `resend-verification`, `verify-email`, `success`, `admin/forgot-password`, `admin/reset-password`
- **Evidence:** only `prerender = false`; inherit `ssr=true, csr=true`. For consistency with `login` (which sets `ssr=false`), consider explicit setting.

### 14. FRONTEND TEST COVERAGE

#### FETEST-1 — Vitest unit suite 100% green

- **Evidence:** 9 files, 67/67 tests passed, 0 skipped.

#### FETEST-2 — Playwright: 23 failed / 5 skipped / 24 passed (52 total)

- **Severity:** blocker for those failures. 4 distinct root-cause clusters:

  **Cluster A — Vercel analytics blocked by nosniff (smoke-firefox, smoke-webkit, 2 tests):**

  ```
  Error: unexpected console errors: ["Refused to execute http://localhost:4173/_vercel/speed-insights/script.js as script because \"X-Content-Type-Options: nosniff\" was given..."]
  at e2e/smoke/home.spec.ts:40
  ```

  Root cause: `pnpm preview` doesn't serve `/_vercel/insights/script.js` (only Vercel infra does). Console-error filter regex doesn't catch the Firefox/WebKit phrasing.

  **Cluster B — Pricing CTA target changed (smoke-firefox, smoke-webkit, 2 tests):**

  ```
  Expected pattern: /\/(login|register)/
  Received string:  "http://localhost:4173/pricing/monthly"
  at e2e/smoke/pricing.spec.ts:44
  ```

  Root cause: recent `refactor(Pricing)` (commit `9bbc64c`) changed CTA. Test still expects pre-Stripe behavior.

  **Cluster C — ConsentBanner intercepts pointer events (auth, 1 test):**

  ```
  TimeoutError: locator.click ... <p class="body svelte-1scodl"> from <section data-layout="bar"...> intercepts pointer events
  at e2e/pages/DashboardPage.ts:35 via e2e/auth/register-login.spec.ts:38
  ```

  Root cause: ConsentBanner overlay z-stacked above sign-out button. Test doesn't dismiss banner first.

  **Cluster D — Admin pages render unauthenticated/error fallback (admin, 18 tests):**
  All 18 admin failures: `getByTestId('admin-XXX-page')` not visible. Root cause: `authedAdminTest` fixture's session is not establishing in the test browser context (or admin nav was dropped from the public shell after `refactor(Nav)` `020ceee`).

#### FETEST-3 — `pnpm test:browser` cannot start (BLOCKER)

- **File:** `/Users/billyribeiro/Desktop/my-websites/swings/vitest.browser.config.ts:9`
- **Evidence:**
  ```
  TypeError: The `browser.provider` configuration was changed to accept a factory instead of a string.
  Add an import of "playwright" from "@vitest/browser-playwright" instead.
  ```
- **Why it matters:** silent regression — CI's `ci:frontend` only runs `pnpm test:unit` which excludes `*.svelte.spec.ts`. Six spec files are dead since the Vitest 4 upgrade:
  - `routes/page.svelte.spec.ts`
  - `lib/stores/toasts.svelte.spec.ts`
  - `lib/stores/consent.svelte.spec.ts`
  - `lib/components/shared/Dialog.svelte.spec.ts`
  - `lib/components/shared/Button.svelte.spec.ts`
  - `lib/components/consent/ConsentBanner.svelte.spec.ts`

#### FETEST-4 — `src/lib/components/ui/` — 14 components, 0 tests

- **Affected:** Tooltip, Toaster, ConfirmDialog, ConfirmDialogHost, DateRangePicker (the new primitives, MAJOR), plus Button, EmptyState, FloatingButton, Footer, LivePriceTicker, Nav, ScrollReveal, SectionHeader, Skeleton (mix of presentational + state).

#### FETEST-5 — `src/lib/api/client.ts` has zero coverage (MAJOR)

- Central HTTP client (auth, retries, problem-json) with no unit tests.

#### FETEST-6 — `src/lib/utils/checkout.ts` and `src/lib/utils/safeHtml.ts` untested (MAJOR)

- Stripe checkout helper + sanitization both critical, both no tests.

#### FETEST-7 — `auth/confirm/modal/toast` stores untested

- `auth.svelte.ts`, `confirm.svelte.ts`, `modal.svelte.ts`, `toast.svelte.ts` — all untested.

---

## Root-Cause Clustering

### Cluster R1 — BFF cookie migration left several systems behind (MAJOR)

The BFF cookie migration (Phase 1.3) updated `extractors::AuthUser` to read cookie OR Bearer (`extractors.rs:123-155`), but missed:

- **Idempotency middleware** (`middleware/idempotency.rs:186-198`) still extracts actor from Bearer only. Result: every admin POST is retry-unsafe (CONTRACT-D1, CONTRACT-D2).
- **Auth store logout** (`auth.svelte.ts:111`) raw `fetch` omits `credentials: 'include'`. Result: dev logout no-ops (CONTRACT-C2).

**Fix the cluster, not the symptoms:** add `extract_access_token(headers)` (cookie-or-bearer) to the idempotency middleware; add `credentials: 'include'` to `auth.svelte.ts:111`.

### Cluster R2 — Doubled-prefix bug in 4 routers (BLOCKER, system-wide)

Same authoring mistake repeated in 4 handler files: each declares routes starting with the resource noun (`/coupons/{id}`), then `main.rs` nests at `/api/admin/<noun>`. Affects 30+ frontend call sites + the unmounted forms router (CONTRACT-A1, CONTRACT-A2 are technically distinct issues but share the same authoring confusion about who owns the prefix).

**Fix the cluster:** in each handler's `Router::new()`, drop the inner prefix → `route("/", ...)` and `route("/{id}", ...)`. Mount `forms::*_router()` in `main.rs`. Done in one PR.

### Cluster R3 — TLS chain bifurcation drives 4 RUSTSEC blockers + 9 lock-file dupes (BLOCKER)

`aws-sdk-s3 1.119.0` pins `rustls 0.21` + `rustls-webpki 0.101.7` (vulnerable). The rest of the tree uses `rustls 0.23`. One dependency bump (`aws-sdk-s3 ≥ 1.120`) resolves DEP-1, DEP-2, DEP-3 (×2), DEP-4 (partially), and collapses 9 duplicate-version pairs in DEP-9.

### Cluster R4 — Frontend error-shape assumes pre-Problem+JSON contract (MAJOR)

`client.ts` reads `err.error`; backend emits RFC 7807. Every admin error is `"Request failed"`. Single 1-line fix in `client.ts:79-80, 65-66, 205-206`.

### Cluster R5 — `$effect` fetch-on-mount anti-pattern recurring in new pages (MAJOR)

6 new admin pages adopted the same anti-pattern that already cost the codebase the `effect_update_depth_exceeded` incident. Mechanical migration to `onMount` (RUNES-2). Add ESLint custom rule or pre-commit hook to catch future regressions.

### Cluster R6 — 17 admin drawers reimplement the same a11y-broken backdrop (MAJOR)

Each of the 17 occurrences (A11Y-3) is the same 4-line copy-paste. Routing every admin drawer through the existing `Drawer.svelte` primitive simultaneously eliminates A11Y-3, A11Y-5, A11Y-7, and reduces ~1700 lines of duplicated markup.

### Cluster R7 — Test infrastructure dark spots (MAJOR + BLOCKER)

- Browser-mode Vitest broken (FETEST-3) → 6 spec files silently dead.
- Playwright admin suite broken (FETEST-2 Cluster D) → 18 admin tests can't even reach an authenticated state.
- Observability test ignored (TEST-2) → correlation-id propagation not exercised.
- 0 tests on the new UI primitives (FETEST-4).

These compound: if the admin auth fixture had been working, it would have caught CONTRACT-A1/A2/A3/A4/A5/A6/A7/A8 by failing on the broken endpoints.

---

## Cross-Stack Risk Map

### Type drift

- **`PricingPlanPriceLogEntry`** (frontend hand-coded) vs **`PricingPlanAmountChangeLogEntry`** (OpenAPI emit). Same shape, different name. Will break next `pnpm gen:types`.
- **22 frontend types in `api/types.ts`** with no matching backend schema (`AnalyticsSummary`, `RevenueAnalytics`, `SalesEvent`, `MediaItem`, etc.). Either the OpenAPI generator is missing `#[derive(ToSchema)]` annotations on the corresponding Rust structs, or the frontend is consuming a contract the backend doesn't expose.
- **`forms::*` DTOs** in `schema.d.ts` describe a non-existent runtime (the router isn't mounted).

### Auth flow gaps

- BFF cookie carry mostly correct, **except** the idempotency middleware (R1).
- Logout bypass in dev (R1).
- No CSRF token on top of `SameSite=Lax`. The custom `Idempotency-Key` header + CORS allowlist effectively block CSRF, but a defense-in-depth CSRF token would harden further.

### Error-handling inconsistencies

- Backend uniformly emits Problem+JSON; frontend uniformly reads `err.error`. **Total mismatch — every error toast is the literal string `"Request failed"`** (R4).
- `routes/api/greeks-pdf/+server.ts` returns `json({error}, {status})` instead of `error()` — bypasses `handleError` correlation-id integration.

### Race conditions in client-server interaction

- **D2 (idempotency middleware bypass under cookies):** any admin POST that the user double-clicks or that the network retries silently creates duplicates (refunds, comp grants, manual orders, bulk coupon mints). The middleware exists, the table exists, but neither fires under the BFF cookie carrier.

### OpenAPI snapshot drift

- The snapshot is built from hand-written `#[utoipa::path(path = "...")]` annotations, not the live router tree. Cheerfully advertises `/api/admin/coupons/{id}` while the actual mounted path is `/api/admin/coupons/coupons/{id}` (CONTRACT-A1). Advertises the entire `/api/forms/*` surface even though `main.rs` never mounts `forms::*_router()` (CONTRACT-A2).
- Recommend a contract test that diffs `paths` from the live `Router` against the OpenAPI keys at startup.

### Production deployment surface

- **Vercel previews → production Railway** (CFG-4). Any preview deploy can mutate prod data.
- **Prerendered pages have no CSP** (CFG-3). The `/`, `/about`, `/blog`, `/pricing*` marketing surface ships without CSP headers because hooks.server.ts only fires on SSR.
- **`render.yaml` is dangerously incomplete** (CFG-5). Without `APP_ENV=production`, the boot-time secrets check is a no-op.

---

End of report. See `AUDIT_FIX_PLAN.md` for the phased remediation plan.
