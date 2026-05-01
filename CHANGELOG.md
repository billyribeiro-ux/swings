# Changelog

All notable changes described in this file are grouped by release date.  
Timestamps use the operator-facing calendar date attached to the change list.

> **Convention (from 2026-05-01 onward):** every session that changes code
> must prepend a new `## YYYY-MM-DD HH:MM ET — <title>` section at the top
> of this log. Each entry documents the *why* and *impact*; the diff is the
> full record of *what*.

---

## 2026-05-01 15:55 ET — Membership platform hardening (Phase A + B)

### Why this exists

End-to-end audit + fix of every real "is this user paid / is this user
allowed" path on the platform. Revealed and closed five production-grade
gaps that would each have failed an external pen-test review.

### Security gaps closed

1. **Ban / suspend now revoke active sessions** — `extractors::AuthUser` and
   `OptionalAuthUser` re-check `users.banned_at` + `users.suspended_at` on
   every authenticated request. Before this change, a banned user retained
   full access for up to `JWT_EXPIRATION_HOURS` (default 24h). Cost is one
   indexed PK lookup per authed call. Lazy-unsuspend logic preserved (an
   expired time-boxed suspension self-heals on the next request).
2. **`PaidUser` extractor** — new typed extractor for subscription-gated
   handlers (`extractors.rs`). Allows only `Active` / `Trialing`. Admins
   bypass for QA. `PastDue` / `Unpaid` / `Canceled` / `Paused` all bounce.
3. **Course enrollment gate** — `POST /api/member/courses/{id}/enroll` now
   honours `is_free` / `is_included_in_subscription` / `price_cents`:
   - free courses → any authenticated user
   - sub-included → requires Active/Trialing sub (or admin)
   - pay-per-course (price > 0, not free, not sub-included) → 403 until the
     pay-per-course purchase ledger lands
4. **Lesson content redaction** — `GET /api/courses/{slug}` strips
   `content` / `content_json` / `video_url` from non-`is_preview` lessons
   when the caller is not entitled. Preview lessons stay public for
   marketing. Admin always sees the full payload.
5. **`subscriptions.canceled_at` is now populated** — the
   `customer.subscription.deleted` webhook stamps the column. COALESCE
   guarantees a Stripe retry of the same event does not overwrite the
   first cancellation timestamp.

### Money type unification (i64 SSOT)

**Migration 081** widens every "money in cents" column from `INTEGER` (max
~$21.4M) to `BIGINT` (max ~$92 quintillion):

- `pricing_plans.amount_cents`
- `courses.price_cents`
- `subscriptions.grandfathered_price_cents`
- `sales_events.amount_cents`
- `coupons.min_purchase_cents`, `coupons.max_discount_cents`
- `coupon_usages.discount_applied_cents`

13 Rust struct fields flipped from `i32` → `i64` in lockstep
(`models::{Course, CreateCourseRequest, UpdateCourseRequest, CourseListItem,
PricingPlan, CreatePricingPlanRequest, UpdatePricingPlanRequest,
PricingRolloutPreview, PricingPlanAmountChangeLogEntry,
AnalyticsRecentSale, Coupon, CreateCouponRequest, UpdateCouponRequest,
CouponValidationResponse, CouponUsage}`, plus `db::{RecentSaleRow,
SubscriptionRow}` and `handlers::pricing::PriceProtectionRequest`,
`handlers::coupons::ApplyCouponRequest`). `db::pricing_monthly_annual_cents`
return type updated. OpenAPI snapshot regenerated; frontend types
regenerated in lockstep. `i64` is now the single source of truth for
money end-to-end (Postgres BIGINT ↔ Rust i64 ↔ JSON number).

`SubscriptionStatus` and `SubscriptionPlan` enums gained `Copy + Eq` —
they're unit-only enums, the missing `Copy` was forcing pointless `.clone()`
calls and blocking idiomatic pattern matching at usage sites.

### New integration tests (43 total, all green)

- **`lifecycle_revocation.rs`** (12 tests): banned-user-on-active-token →
  401; suspended → 401; expired suspension self-heals; hard-deleted user →
  401; admin ban → 401 (no admin path leak); future-dated ban still
  blocks; ban does not leak via `OptionalAuthUser`; refresh after ban
  fails; un-ban / un-suspend symmetry.
- **`subscription_lifecycle_access.rs`** (13 tests): per-status
  `is_active` reporting (Active/Trialing/PastDue/Unpaid/Canceled/Paused/no
  row); transitions (Active→Canceled, PastDue→Active, Trial→Active);
  `canceled_at` populated on cancel; COALESCE preserves first
  cancellation across replays; anonymous → 401.
- **`course_access_gates.rs`** (18 tests): free-course visibility +
  enrollment for anonymous & members; sub-included course content
  redaction matrix (anon, unsubscribed, active, trialing, past_due,
  canceled, admin); enrollment gating per status; pay-per-course 403;
  unpublished course 404 for everyone.

### Verification

```
cargo fmt --all -- --check     → clean
cargo clippy --all-targets     → clean (-D warnings)
cargo test --lib               → 524 pass / 0 ignored
cargo test --tests             → 893 pass / 0 ignored / 0 failed (43 binaries)
pnpm lint                      → clean
pnpm check                     → 0 errors / 0 warnings
pnpm test:unit                 → 12 files / 103 tests pass
```

### Followups

- Real Stripe E2E (Phase C, blocked on operator pasting real
  `sk_test_*` keys into `backend/.env` and root `.env`). Will land in
  `docs/STRIPE-E2E-RESULTS-2026-05-01.md`.
- Pay-per-course purchase ledger (`course_purchases` table + flow); the
  enrollment gate currently 403s on this case as a hard block — better
  than letting anyone enroll without paying.

---

## 2026-05-01 15:05 ET — Env + dev-config audit; retire stale Render and Neon claims

### Why this exists

The 14:55 audit pass covered only `.md` files and missed two stale dotfiles
plus a doc cluster that still claimed Neon as the production database when
production has run on Railway PostgreSQL since at least 2026-04-15. This
follow-up pass closes that gap by auditing every dotfile + env + deploy
config end-to-end.

### Verified, classified, kept

| File | Verdict | Why |
|---|---|---|
| `.editorconfig`, `.npmrc`, `.nvmrc`, `.prettierrc`, `.prettierignore` | KEEP | standard tooling configs |
| `.sqlfluff` | KEEP | live config consumed by `.github/workflows/sql-lint.yml` |
| `.trivyignore` | KEEP | live config consumed by `.github/workflows/security.yml`; currently empty list (intentional — file documents the suppression policy and provides a curated home for any future entry) |
| `.vercelignore`, `.dockerignore`, `.gitignore` | KEEP | active build excludes |
| `.mcp.json` | KEEP | project-level MCP server registration (Svelte + rust-analyzer) |
| `project.inlang/settings.json` | KEEP | inlang IDE / Sherlock / Fink tooling source-of-truth + planned migration path for `src/lib/i18n/paraglide.ts` shim |
| `backend/.env.example`, `.env.example` | KEEP | committed templates; documented `SWINGS_ALLOW_HTTP_WEBHOOKS` (the one env var production code reads but the template did not list) |

### Deleted

- **`.neon`** — Neon CLI org-id pin (`{"orgId":"org-dark-rice-71594132"}`).
  No code, no doc, no workflow read it. Production runs Railway PostgreSQL
  per `docs/DEPLOYMENT.md` (canonical). Stale leftover from an early Neon
  experiment that never shipped.

### Stale Render / Neon claims rewired

- `Dockerfile`, `.dockerignore`, `README.md`, `AGENTS.md`, `backend/README.md`
  — Render references stripped (Render is no longer a deploy target;
  `render.yaml` does not exist; Railway is canonical).
- `Dockerfile` preamble — stale `AUDIT_FIX_PLAN Phase 6.7` pointer to a
  deleted ledger replaced with a concrete explanation that survives.
- `docs/INFRASTRUCTURE.md` § 3 — full rewrite from "Database — Neon Scale"
  to "Database — Railway PostgreSQL", including connection string format,
  sqlx pool tuning that matches what `Config::from_env` actually reads,
  and accurate migration count (72 forward-only, versions 001–080).
  Replaced two embedded SQL snippets (047, 048) that documented two
  long-shipped migrations with a pointer to `backend/migrations/`.
- `docs/INFRASTRUCTURE.md` cost summary — Neon line removed; total
  re-priced to reflect Railway-bundled PostgreSQL ($40-60/month).
- `docs/INFRASTRUCTURE.md` deployment checklist — "Create Neon Scale
  account" replaced with "Provision the Railway PostgreSQL add-on".
- `backend/README.md` — `db.rs` annotation changed from "Neon-tuned" to
  "env-tuned via PGPOOL_*"; `DATABASE_URL` row updated to drop the
  Neon-specific framing.
- `backend/src/main.rs` — pool-tuning comment retargeted away from Neon.

### Auditor's note

The first audit pass treated dotfiles as inert config noise and only
inspected `.md` files. That left `.neon` and the `INFRASTRUCTURE.md` Neon
cluster in place, contradicting `docs/DEPLOYMENT.md` and the actual
production setup. The user caught both — this pass closes the gap, and
`REPO_STATE.md` is updated to reflect "every committed file" as the
audit scope going forward, not just `.md`.

### Verification

```
cargo fmt --all -- --check     → clean
cargo clippy --all-targets     → clean (-D warnings)
pnpm lint                      → clean
pnpm check                     → 0 errors / 0 warnings
docker compose config          → both compose files parse
```

---

## 2026-05-01 14:55 ET — Repo-wide ledger consolidation + stale-doc retirement

### What changed and why

End-to-end audit of every `.md` file in the repo, every TODO, every blocker —
verified against code with grep + a full test run (1,477 tests, 0 failed,
0 ignored). The legacy ledgers (`AUDIT*.md`, `TODO_AUDIT.md`,
`docs/REMAINING-WORK.md`, `docs/archive/`) were stale by ~7 days; every
"blocker" they listed was either shipped or never existed. They were actively
misleading new readers.

### New canonical doc

- **`REPO_STATE.md`** — single source of truth for repo health, with evidence
  (file paths, line numbers, test counts) for every claim. Includes the
  6 surviving open items (none are blockers).

### Deletions (14 files, ~21,800 lines of stale ledger content)

```
AUDIT.md
AUDIT_FIX_PLAN.md
AUDIT_REPORT.md
TODO_AUDIT.md
docs/REMAINING-WORK.md
docs/archive/                       (entire directory, 8 files)
.windsurf/workflows/terms.md        (empty file)
test-results/                       (gitignored Playwright debris)
```

Git history preserves every prior version — nothing destroyed, just retired
from the working tree.

### Doc backlinks rewired

Every code + doc pointer to the deleted files retargeted to the live source
of truth:

- `README.md`, `AGENTS.md`, `backend/README.md` — migration count corrected
  from `67 / 001–075` → `72 / 001–080`; archive backlinks replaced with
  pointers to `migrations/021_rbac.sql`.
- `docs/README.md` — `archive/` section removed; `CHANGELOG.md` and
  `REPO_STATE.md` added to the index.
- `docs/RUNBOOK.md`, `docs/STRIPE-E2E-QA.md`, `docs/wiring/OBSERVABILITY-WIRING.md`
  — dead `docs/archive/*` and `docs/REMAINING-WORK.md` references stripped.
- 11 source files (`backend/src/**`, `src/lib/**`, `eslint.config.js`,
  `Cargo.toml`, `deny.toml`, `audit.toml`, etc.) — module-level comments and
  config preambles retargeted away from deleted ledgers.

### Code housekeeping landed in the same commit

- OpenAPI snapshot (`backend/tests/snapshots/openapi.json`) regenerated to
  cover the rollout-preview + price-protection endpoints added in the
  prior session.
- Frontend OpenAPI types (`src/lib/api/schema.d.ts`) and the hand-written
  `src/lib/api/types.ts` updated in lockstep — added `PricingRolloutPreview`
  type and the `skipped_grandfathered` field on `AdminStripeRolloutSummary`.
- `cargo fmt --all` mechanical fixes to test files.

### Verification

```
pnpm lint                      → clean
pnpm check                     → 0 errors / 0 warnings
pnpm test:unit                 → 12 files / 103 tests pass
cargo fmt --all -- --check     → clean
cargo clippy --all-targets     → clean (-D warnings)
cargo test --lib               → 524 pass / 0 ignored
cargo test --tests             → 850 pass / 0 ignored / 0 failed (40 binaries)
```

---

## 2026-05-01 14:30 ET — Membership/auth hardening, grandfather price protection, rollout preview

### Migration

- **`backend/migrations/080_subscription_price_protection.sql`** (new)
  - Adds `grandfathered_price_cents INTEGER`, `grandfathered_currency TEXT DEFAULT 'usd'`, and `price_protection_enabled BOOLEAN NOT NULL DEFAULT FALSE` to `subscriptions`.
  - Adds partial index `idx_subscriptions_price_protected` on `pricing_plan_id WHERE price_protection_enabled = TRUE` for fast audit queries.
  - Seeds two new permissions: `subscription.price_protection.manage` (admin only).

### Backend — models

- **`backend/src/models.rs`**
  - `Subscription`: three new fields: `grandfathered_price_cents: Option<i32>`, `grandfathered_currency: Option<String>`, `price_protection_enabled: bool`.
  - `PricingStripeRollout`: new `skip_price_protected: bool` field (default `true`).
  - `AdminStripeRolloutSummary`: new `skipped_grandfathered: usize` field.
  - New `PricingRolloutPreview` struct: `total_in_audience`, `would_update`, `would_skip_grandfathered`, `current_amount_cents`, `currency`.

### Backend — pricing rollout service

- **`backend/src/services/pricing_rollout.rs`**
  - `rollout_after_plan_save()`: skips any subscription with `price_protection_enabled = true` and counts them in `skipped_grandfathered`.
  - New `preview_rollout()`: dry-run that returns `PricingRolloutPreview` counts without calling Stripe.

### Backend — pricing handlers

- **`backend/src/handlers/pricing.rs`**
  - New `GET /api/admin/pricing/plans/{id}/rollout-preview` (`admin_rollout_preview`): returns preview counts; requires `subscription.plan.manage`.
  - New `POST /api/admin/pricing/subscriptions/{sub_id}/price-protection` (`admin_toggle_price_protection`): toggles grandfather flag per subscription; requires `subscription.price_protection.manage`; writes audit row.
  - `RolloutPreviewParams` and `PriceProtectionRequest` DTOs added.

### Integration tests

- **`backend/tests/auth_membership.rs`** (18 tests, all new)
  - Registration: success + BFF cookie check, duplicate 409, weak password, bad email.
  - Login gates: banned → 401, suspended → 401, expired suspension auto-lifted → 200.
  - RBAC: member → 403 on subscriptions / members / audit / pricing; unauthenticated → 401.
  - Refresh rotation: new pair returned, spent token rejected.
  - Logout: prevents refresh reuse.
  - Password reset: forgot-password always 200 (no enumeration), invalid token 4xx.
  - Email verification: token row created in DB on register.

- **`backend/tests/pricing_rollout.rs`** (8 tests, all new)
  - Preview endpoint returns correct total + zero grandfathered when none protected.
  - Preview reflects protected subscriptions in `would_skip_grandfathered`.
  - Toggle endpoint enables then disables protection; verifies DB state both ways.
  - Toggle returns 404 for unknown subscription.
  - RBAC: member blocked from preview and toggle endpoints.
  - Preview returns 404 for unknown plan.

### Admin frontend

- **`src/routes/admin/subscriptions/plans/+page.svelte`**
  - Two-step Stripe rollout confirmation: first "Save" with rollout enabled fetches preview and shows member counts; second "Confirm & Push to Stripe" commits.
  - Results banner now surfaces `skipped_grandfathered` count ("X grandfathered member(s) kept their price.").
  - State: `rolloutPreview`, `rolloutPreviewLoading`, `showRolloutConfirm`; `fetchRolloutPreview()` helper.
  - New `.rollout-confirm` CSS block for the confirmation card.
  - Svelte autofixer confirmed zero issues post-edit.

---

## 2026-05-01 10:45 ET — Full-backend audit + observability fixes

### Audit scope

Five parallel streams audited the entire backend and admin dashboard:
HTTP handlers, services/workers, middleware, database migrations, domain
modules, RBAC, integration/unit/E2E test coverage, and the admin frontend.

### All-clear findings (no code changes needed)

| Area | Result |
|------|--------|
| Admin mutation `policy.require` enforcement | All 31 handlers compliant |
| Admin mutation `audit_admin` recording | All 31 handlers compliant |
| Idempotency-Key middleware on all admin POST/PUT/DELETE | Fully wired |
| `unwrap` / `expect` / `panic!` in non-test production code | Zero violations |
| Handler registration — orphaned or unregistered handlers | None found |
| Database table ↔ HTTP endpoint coverage | 100% |
| Background worker graceful-shutdown paths | All 5 workers correct |
| Migration sequence 001–079 (gaps 029/040 intentional) | Clean |
| Migration foreign-key ordering | No violations |
| RBAC permission matrix: handler calls vs. seeded migrations | 37/37 match |
| Domain modules completeness (commerce, consent, popups, forms, notifications, pdf) | All fully implemented |
| Admin frontend: idempotency keys auto-injected by API client | Correct |
| Admin frontend: BFF HttpOnly-cookie auth pattern | Correctly implemented |
| Admin frontend: route auth guards | All protected, no gaps |
| Admin frontend: TypeScript strict mode, zero `any` types | Confirmed |
| Backend integration tests — `#[ignore]` violations | Zero (policy maintained) |
| Backend integration tests — handler coverage | 36 tests, all 31 handlers covered |

### False positive resolved

- **`webhooks.rs` line 1259 `expect("valid hmac key")`** — initially flagged
  as a production-code violation. Confirmed on re-read: the call lives inside
  the `#[cfg(test)]` block (`make_signature` test helper). The production
  path at line 196 uses `match … { Err(_) => return false }`. **No fix needed.**

### Fixed — `outbox_last_success_unixtime` Prometheus gauge missing

- **File:** [`backend/src/events/worker.rs`](backend/src/events/worker.rs)
- **Rule:** AGENTS.md §7 — every worker must emit `*_last_success_unixtime`
  so the runbook can detect a stalled loop.
- **Change:** After each non-empty dispatch batch (`Ok(n)` arm), worker now
  emits `metrics::gauge!("outbox_last_success_unixtime").set(...)`.
  Added `use chrono::Utc` import.
- **Impact:** Prometheus staleness alert for the outbox worker now has data to
  fire on; previously the alert would never trigger regardless of worker state.

### Fixed — `dsar_export_last_success_unixtime` Prometheus gauge missing

- **File:** [`backend/src/services/dsar_worker.rs`](backend/src/services/dsar_worker.rs)
- **Rule:** AGENTS.md §7 — same as above.
- **Change:** After each successful job export inside `process_job`, worker
  now emits `metrics::gauge!("dsar_export_last_success_unixtime").set(...)`.
  Uses existing `chrono::Utc` full-path style (no new import needed).
- **Impact:** Operators can detect a stuck DSAR export pipeline via Prometheus
  without manually querying `dsar_jobs` row states.

### Known gaps documented (no fix in this session)

- **E2E coverage: ~13% of routes.** Admin feature CRUD pages (blog, courses,
  consent, coupons, products, notifications, popups, forms, subscriptions),
  member dashboard, and public utility pages have no Playwright spec. Backend
  API correctness is fully covered by 36 integration tests.
- **Frontend component unit tests: ~30 components untested.** Landing page
  and chrome/layout components (static presentation, minimal logic). Admin
  feature components rely on E2E coverage instead of isolated unit tests.

---

## 2026-04-24 — Subscription price rollout & catalog ↔ Stripe control

> **Note on the date:** this entry was authored under the heading **April 24, 2024** per project request. The engineering work ships with repository state current as of **2026-04-24** (migrations, OpenAPI snapshot, and coordinated frontend/backend commits).

### Summary

Operators can **choose** what happens to **existing Stripe subscriptions** when they change a row in `pricing_plans`: either leave Stripe alone (historical default — new checkouts only) or **push** the updated commercial terms to every targeted member subscription after the database save.

### Database

- **`076_subscriptions_pricing_plan_id.sql`**
  - Adds nullable `subscriptions.pricing_plan_id` (`UUID`, FK → `pricing_plans.id`, `ON DELETE SET NULL`).
  - Partial index `idx_subscriptions_pricing_plan_id` for fast rollout candidate queries.

### Stripe ↔ Swings linking

- **Checkout (`src/routes/api/checkout.remote.ts`)**
  - When checkout is created from a **`planSlug`**, the session now includes  
    `subscription_data.metadata.swings_pricing_plan_id = <catalog plan UUID>`.
  - `priceId`-only checkouts intentionally omit this metadata (no catalog row is implied).

- **Webhooks (`backend/src/handlers/webhooks.rs`)**
  - `customer.subscription.updated` parses `metadata.swings_pricing_plan_id` and passes it into `db::upsert_subscription`, so the local mirror learns which catalog plan a subscription was purchased from.
  - `checkout.session.completed` continues to upsert with `NULL` catalog link until the subscription webhook arrives.
  - `customer.subscription.deleted` preserves `pricing_plan_id` while flipping status.

### HTTP API

- **`PUT /api/admin/pricing/plans/{id}`** (`backend/src/handlers/pricing.rs`)
  - Request body may include optional **`stripe_rollout`** (`PricingStripeRollout` in `backend/src/models.rs`):
    - `push_to_stripe_subscriptions` (`bool`) — when `true`, after the plan row + change log are written, the server walks matching `subscriptions` rows and calls Stripe for each.
    - `audience` (`linked_subscriptions_only` | `linked_and_unlinked_legacy_same_cadence`) — controls whether only linked rows are touched or legacy unlinked rows with the same monthly/annual cadence are included.
  - If `push_to_stripe_subscriptions` is `true`, the request **must** include an **`Idempotency-Key`** HTTP header (8–255 characters). Missing/short keys return **400**.
  - Rollout is only allowed when at least one **billing** field changed in the same request (`amount_cents`, `stripe_price_id`, `currency`, `interval`, or `interval_count`).
  - Response shape is now **`AdminUpdatePricingPlanResponse`**: all `PricingPlan` fields at the top level **plus** optional `stripe_rollout` (`targeted`, `succeeded`, `failed[]` with per-subscription error strings). HTTP **200** is returned even when some Stripe calls fail; inspect `stripe_rollout.failed`.

- **`GET /api/admin/pricing/plans/price-log`**
  - Returns the latest `amount_cents` adjustments (joins `pricing_change_log`, `pricing_plans`, `users`) for the admin price history panel.

### Stripe rollout implementation

- New module **`backend/src/services/pricing_rollout.rs`**
  - Retrieves each Stripe subscription with `expand[]=items.data`.
  - **Exactly one** subscription item is supported; multi-item subscriptions return a descriptive **400** for that member.
  - **Model A** — updates the item to the catalog’s `stripe_price_id`.
  - **Model B** — sends inline `SubscriptionPriceData` using `stripe_product_id`, `amount_cents`, currency, and interval from the catalog row. Missing product id is rejected with a clear **400**.
  - Each Stripe `Subscription::update` uses a fresh **Stripe idempotency key** derived from the admin key + plan id + stable index so transport retries never double-apply.
  - **Proration:** the code intentionally omits `proration_behavior` and relies on **Stripe’s default** for subscription updates (`create_prorations`). Documented rationale: `async-stripe` exposes two incompatible `SubscriptionProrationBehavior` enums (subscription vs subscription_item); wiring the wrong one silently fails type-check or mis-serialises.

### Auditing

- Successful rollouts emit an additional **`pricing_plan.stripe_rollout`** row in `admin_actions` with targeted / succeeded / failed counts.
- The existing **`pricing_plan.update`** audit row remains unchanged.

### Frontend

- **`src/routes/admin/subscriptions/plans/+page.svelte`**
  - Fixes API paths to the real backend routes: **`/api/admin/pricing/plans`**, **`/api/admin/pricing/plans/price-log`**, **`PUT /api/admin/pricing/plans/{id}`**.
  - Edit form adds **“Also update existing Stripe subscriptions after save”** with audience radios.
  - When enabled, the client sends `stripe_rollout` in the JSON body and an `Idempotency-Key` header (UUID).
  - Displays a short post-save status line summarising Stripe outcomes.

- **`src/lib/api/types.ts`** — documents `PricingStripeRollout`, `AdminUpdatePricingPlanResponse`, and related DTOs for hand-written clients.

### OpenAPI / TypeScript

- `backend/tests/snapshots/openapi.json` regenerated; `pnpm gen:types` refreshes `src/lib/api/schema.d.ts`.

### Documentation

- **`docs/stripe-pricing-models.md`** — new section _Pushing catalog edits to existing Stripe subscriptions_ plus code pointers.

### Operational caveats

1. **`linked_and_unlinked_legacy_same_cadence`** can touch the wrong people if you run **multiple** active monthly (or annual) catalog plans — unlinked rows are only disambiguated by `subscription_plan` enum, not SKU.
2. Legacy subscribers created **before** this release will have **`pricing_plan_id = NULL`** until the next `customer.subscription.updated` webhook refreshes metadata from Stripe — they only appear in the legacy audience bucket.
3. Stripe must be configured (`STRIPE_SECRET_KEY`); otherwise rollout errors surface in `stripe_rollout.failed`.

---
