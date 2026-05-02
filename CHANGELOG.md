# Changelog

All notable changes described in this file are grouped by release date.  
Timestamps use the operator-facing calendar date attached to the change list.

> **Convention (from 2026-05-01 onward):** every session that changes code
> must prepend a new `## YYYY-MM-DD HH:MM ET — <title>` section at the top
> of this log. Each entry documents the _why_ and _impact_; the diff is the
> full record of _what_.

---

## 2026-05-02 04:35 ET — Architectural CLS fix on `/admin/members` + dev-DB cleanup

Two work items shipped in one session.

### Fixed: filter-tab clicks on `/admin/members` produced visible page drift

User report: clicking "All roles / Admins / Members / Any status / Active /
Suspended / Banned" tabs caused "huge drift" — every element below the
table jumped as the row count changed. The previous CLS work
(`f67af54`) had added `silent: true` to mutation handlers (delete /
ban / suspend) so they'd avoid flashing the skeleton, but **filter
handlers, search, and pagination still flipped `loading=true`**, which
unmounted the table → mounted the 5-row skeleton → mounted the new
table at whatever row count came back. Two layout shifts per click.

Fixed in two passes:

1. **`silent: true` on every non-mount call site of `loadMembers()`**
   — `changeRole`, `changeStatus`, `clearMemberFilters`,
   `scheduleSearchReload`, `commitSearchNow`, prev-page, next-page.
   The list stays mounted, the `<tbody>` swaps in place. This alone
   takes mutation-CLS to 0.0000 across all 14 filter clicks ×
   2 viewports.

2. **Reserved-space layout architecture** so the underlying
   class-of-bug ("table-wrap height = N × row-h, where N is variable")
   cannot recur on a larger dataset. Single source of truth: a
   `PER_PAGE = 15` script constant + CSS custom properties
   (`--row-h: 3.6rem`, `--cards-row-h: 9rem`, etc.) on
   `.members-page`. From there:
   - **Cards block (mobile) and table-wrap (≥768px) always mounted**,
     each with `min-height: PER_PAGE × row-h + chrome`. Going from a
     filter that returns 15 rows to one that returns 0 (or vice
     versa) cannot resize either container.
   - **Empty state renders inline** inside the reserved box (centered
     via `display: grid; place-items: center` on the table-wrap)
     instead of unmounting the wrapper to swap in a standalone empty
     panel.
   - **Skeleton renders exactly `PER_PAGE` rows** (was 5) and real
     `<tr>` rows are locked to `height: var(--row-h)` so the
     loading → ready transition is a pure in-place row swap with no
     height delta. Content variability (long names / wrapped emails)
     can no longer drift row height between filter swaps.
   - **Pagination slot always reserved** — when `totalPages <= 1` we
     render phantom controls with `visibility: hidden` so the slot's
     vertical space is preserved. Without this, going from a
     multi-page filter result to a single-page one would unmount the
     row and bounce the page footer up by ~3rem.
   - **Live count badge extracted** from the descriptive subtitle
     (`{total} total members — manage roles…`) into a dedicated
     `.members-page__count` pill next to the H1. The dynamic value
     no longer changes character count inside a wrapping paragraph,
     which was the single largest contributor to first-paint CLS
     (~0.034 desktop, traced to a `<p>` line-wrap shift).

### Added: mutation-CLS gate spec + load-time CLS gate

`e2e/admin/members-filter-cls.spec.ts` — authenticated Playwright
spec that runs against both viewports (1280×800 desktop / 390×844
mobile), captures `PerformanceObserver({ type: 'layout-shift',
buffered: true })`, and asserts:

- **Load-time CLS < 0.02** (threshold gate; reserved-space invariant).
- **Per-click delta < 0.01** for each of 7 filter tabs.

Final numbers: filter-click delta = **0.0000** on all 14 click events
(7 tabs × 2 viewports). Load-time = **0.0051 desktop / 0.0066 mobile**
(was 0.0421 / 0.0113 before the badge extraction — ~88% / ~42%
reduction respectively). The remaining ~5–7 thousandths is 1px
font-metric drift after web-font swap — a known web-platform issue,
20× below Google's "good CLS" threshold of 0.1.

When load-time CLS > 0.001, the spec dumps offending DOM sources
(tag + outerHTML + rect deltas) so future regressions name the
offender automatically.

### Cleaned: local dev DB — drop test data, keep the operator

Cleanup of the local Postgres on `:5434`:

- **1,622 leaked `test_*` schemas dropped** — the Rust integration
  test harness creates a per-test ephemeral schema and is supposed
  to drop it on teardown; previous crash-paths left them behind.
  This was also the cause of `pg_dump` failing with
  `out of shared memory` (locking 1,622 × ~90 dependent objects
  exceeded `max_locks_per_transaction`). Dropped via 1,622
  individual auto-commit `DROP SCHEMA … CASCADE` statements
  (`/tmp/drop-test-schemas.sql`) outside the data-cleanup
  transaction.
- **45 of 46 user rows deleted**, leaving only the operator
  (`b9d628f7-42af-4d78-af93-cb13ba0b092b` / `billy.ribeiro@icloud.com`).
  CASCADE FKs swept dependents (subscriptions, refresh_tokens,
  addresses, course_enrollments, etc.) automatically.
- **Content/seed tables wiped**: pricing_plans, coupons, popups +
  variants + submissions + events, blog_posts + categories + tags
  + revisions, courses + modules + lessons, forms + versions +
  submissions, products + variants + categories, watchlists +
  alerts, analytics events/sessions, outbox events, idempotency
  keys, rate-limit buckets, consent records, dsar_jobs/requests,
  impersonation_sessions, stripe_webhook_audit, etc.
- **Preserved**: operator's `users` row, operator's
  `admin_actions` (91 rows — operator audit trail), `permissions`
  + `role_permissions` (RBAC matrix from `021_rbac.sql`),
  `consent_categories` / `consent_services` / `consent_policies` /
  `consent_banner_configs` (config tables), `popup_types`,
  `app_settings`, `notification_templates`, `_sqlx_migrations`.
- Single transaction with `ON_ERROR_STOP=1`. Pre-cleanup
  `pg_dump` (public schema only) saved at
  `/tmp/swings-backup-20260502-091016.sql` (1.0 MB) — restorable
  with `psql … < <backup>`.

Verified by post-cleanup `POST /api/auth/login` against the
operator credentials → HTTP 200, JWT issued.

### Verification

- `pnpm exec svelte-check` → 4427 files / **0 errors / 0 warnings**.
- `pnpm lint` → clean.
- `pnpm test:unit` → **103/103 passing** across 12 test files.
- `e2e/admin/members-filter-cls.spec.ts` → **2/2 passing** (desktop
  + mobile).

### Files changed

- [src/routes/admin/members/+page.svelte](src/routes/admin/members/+page.svelte)
  — `silent: true` on filter/search/pagination call sites; reserved-space
  architecture (CSS custom properties, always-mounted wrappers,
  inline empty state, reserved pagination slot, count-badge
  extraction); `PER_PAGE` lifted to script constant.
- [e2e/admin/members-filter-cls.spec.ts](e2e/admin/members-filter-cls.spec.ts)
  — new authenticated CLS gate spec, both viewports.
- [e2e/admin/forensic-cls-dashboard.spec.ts](e2e/admin/forensic-cls-dashboard.spec.ts)
  — new forensic sweep for member-facing `/dashboard/*` routes
  (mirror of the admin sweep, no CLS assertions, JSON evidence
  to `/tmp/forensic/cls-dash-*.json`).
- [CLAUDE.md](CLAUDE.md) — migration ceiling bumped from "001–087"
  to "highest: 091" (the file in `backend/migrations/` reflects
  this; CLAUDE.md was lagging).

---

## 2026-05-01 23:30 ET — Removed "Recent sign-ups" from admin home

The admin home page was rendering a "Recent sign-ups" table as the
second-to-last section. That belongs on `/admin/members` (which already
has a full members list with filters), not on the operator's main
landing — admin home is for ops health (KPIs, quick actions), not
member-acquisition feeds. Removed the section in
`src/routes/admin/+page.svelte` and cleaned up:

- The orphan `formatDate()` helper (only used by the removed section)
- All orphan CSS (`.admin-dash__link`, `.admin-dash__empty`,
  `.admin-dash__table-wrap`, `.admin-dash__cards`, the entire
  `.admin-table*` block, the entire `.member-card*` block)
- The reduced-motion rule that referenced `.member-card`

`pnpm check` now: 4426 files / 0 errors / **0 warnings** (was 0 errors /
31 warnings before the cleanup, all of them dead-CSS from this section).

---

## 2026-05-01 23:15 ET — Clean-DB pass: seed strip, blog-post 500 fix, ellipsis member actions, consent banner z-index

Shipped under commit `0597bb1` (squashed by the operator). This entry
documents what landed and the verification gate for it.

### Fixed: `PUT /api/admin/blog/posts/{id}` 500 on every update

`db::create_blog_revision` decoded `COALESCE(MAX(revision_number), 0) + 1`
into `(i64,)`, but `blog_revisions.revision_number` is `INT4` per
`002_blog.sql`. PostgreSQL returns INT4 from `MAX(int4) + int4`, so once a
single revision existed the read failed with
`ColumnDecode { i64 vs INT4 }` and the entire admin update flow 500'd
the moment the operator tried to save a second time. The first save
silently passed because the COALESCE branch returned `0` — but in PG that
literal was BIGINT-coerced before `+ 1`, so it accidentally worked. Fix
swaps the tuple to `(i32,)` and drops the now-redundant `as i32` cast on
the bind. Comment in the source documents the trap.

Regression test: `backend/tests/admin_blog_post_update.rs` —
`update_blog_post_twice_succeeds_and_writes_revisions` creates a post,
then PUTs **twice** (the second call exercises the revision INSERT
against a non-empty table, which is exactly the path that 500'd), and
asserts `blog_revisions` ends with rows numbered `[1, 2]`.

### Stripped seeded / placeholder data for clean end-to-end testing

Two new forward-only migrations DELETE the demo content seeded by older
migrations so a fresh DB renders empty states everywhere:

- **`086_strip_seeded_pricing_plans.sql`** — `DELETE FROM pricing_plans
WHERE slug IN ('monthly', 'annual')`. The operator builds the real
  catalog through the admin pricing UI on a clean DB.
- **`087_strip_seeded_popup_templates.sql`** — `DELETE FROM popups WHERE
is_template = TRUE`. The popup-templates seed in `052_popup_templates.sql`
  was confusing the empty-state UX.

Frontend: `Testimonials.svelte` had three hard-coded fake testimonials
(Michael Chen, Sarah Martinez, David Thompson) plus a fake-stat block
("18,000+ Active Traders / 4.9/5 / 95% Renewal Rate"). The `testimonials`
constant is now `[]` with a `TODO(testimonials)` pointing at the future
`GET /api/testimonials` endpoint, and the section is wrapped in
`{#if testimonials.length > 0}` so it just doesn't render until real
content exists.

Other founder-claim copy (about-page stats, real product feature copy)
was deliberately left intact — those are product positioning, not "demo
data," and changing them is a product decision.

### Replaced admin members icon-cluster with an ellipsis (⋯) action menu

`admin/members/+page.svelte` was rendering a row of 5 icon buttons per
member (Edit / Suspend / Ban / Promote / Delete), each wrapped in a
Tooltip. The cluster was visually noisy and tap targets crowded each
other on dense rows. Replaced both the mobile-card and tablet-table
clusters with a single `DotsThreeVerticalIcon` trigger driving a new
`ActionMenu` component — items in this order:

1. View profile (`UserIcon`)
2. Edit profile (`PencilSimpleIcon`)
3. Suspend sign-in / Lift suspension (`ClockCountdownIcon`, label
   switches on `suspended_at`)
4. **Promote to admin / Demote to member** (`ShieldCheckIcon`, label
   switches on `role === 'admin'`) — explicitly added because the
   operator asked for an obvious admin-grant path
5. Ban / Lift ban (`ProhibitIcon`)
6. ── divider ──
7. **Delete account** (`TrashIcon`, danger variant — red on hover)

### New `ActionMenu` primitive

`src/lib/components/shared/ActionMenu.svelte` (426 LOC) +
`ActionMenuItem.svelte` (118 LOC) + `ActionMenuDivider.svelte` (13 LOC).
Mirrors the engineering quality of the polished `Tooltip`:
portals to `document.body` to escape ancestor `overflow:hidden`,
viewport-clamped placement with auto-flip, single-active coordination
across instances (only one menu open at a time), full keyboard nav
(Arrow Up/Down, Home/End, Tab/Shift-Tab, Escape), focus management
(opens to first item on keyboard, returns to trigger on close),
proper ARIA wiring (`role="menu"` + `role="menuitem"` +
`aria-orientation` + `aria-haspopup` + `aria-expanded`), 120ms
fade/slide animation honouring `prefers-reduced-motion`. Visual style
matches the new tooltip palette (near-black + teal-tinted edge).

Exported from `src/lib/components/shared/index.ts` for reuse.

### Fixed consent banner being clipped by mobile bottom tab bar

`src/lib/components/consent/ConsentBanner.svelte` z-index was
`var(--z-40)` (40), but the dashboard's mobile bottom tab bar from the
21:00 ship is at `z-index: 50`. On `/dashboard/*` and `/admin/*` mobile
viewports the bottom of the banner was being painted UNDER the tab bar.
Two fixes:

- Bumped banner `z-index` to `60` so it floats above the tab bar
- On `< 768px`, added `inset-block-end: calc(56px + env(safe-area-inset-bottom, 0px))`
  for both `bar` and `box` layouts so the banner sits ABOVE the tab bar
  with a comfortable gap — not just stacked on top of it. Desktop is
  unaffected.

### Verification gate (commit `0597bb1`)

| Check                                       | Result                                                                          |
| ------------------------------------------- | ------------------------------------------------------------------------------- |
| `cargo fmt --all -- --check`                | clean (exit 0)                                                                  |
| `cargo clippy --all-targets -- -D warnings` | clean (0 warnings)                                                              |
| `cargo test --lib`                          | **524 passed / 0 failed / 0 ignored**                                           |
| `cargo test --test admin_blog_post_update`  | **1 / 1** new regression test passes                                            |
| `cargo test --test admin_coupons_create`    | 4 / 4 still pass                                                                |
| `cargo test --test member_subscriptions`    | 13 / 13 still pass                                                              |
| Migration replay (fresh DB, `001..087`)     | all apply; `pricing_plans=0`, `template_popups=0`                               |
| `pnpm check`                                | **4426 files / 0 errors / 0 warnings**                                          |
| `pnpm lint`                                 | 0 errors, 1 pre-existing warning (`forensic-cls.spec.ts` unused arg, unrelated) |
| `pnpm test:unit -- --run`                   | **103 / 103 passed**                                                            |

---

## 2026-05-01 22:30 ET — Polish: tooltip refresh, coupon-create 422 fix, icon audit

### Fixed: `POST /api/admin/coupons` 422 on every submit

Two compounding bugs caused every authenticated coupon-create request to
fail at the `Json<>` extractor:

1. **`DiscountType` had no serde rename.** The enum derived
   `Serialize` / `Deserialize` directly, so the wire format expected
   `"Percentage"` / `"FixedAmount"` / `"FreeTrial"` (Rust variant names
   verbatim). The SPA was sending `"percentage"` / `"fixed"` /
   `"free_trial"` — none matched. Added
   `#[serde(rename_all = "snake_case")]` so the wire shape matches the
   SQL enum (`fixed_amount` etc.) and the SPA convention.
2. **Stale field names in the new-coupon UI.** The form was sending
   `value`, `min_purchase`, `max_discount`, `start_date`, `expiry_date`,
   `active` instead of the canonical `discount_value`,
   `min_purchase_cents`, `max_discount_cents`, `starts_at`,
   `expires_at`, `is_active`. The dropdown also used `"fixed"` instead
   of `"fixed_amount"`. And dollar amounts were sent as raw numbers
   (would have stored `5¢` instead of `$5`).

Fix touched both pages:

- `src/routes/admin/coupons/new/+page.svelte` — corrected field names,
  added `dollarsToCents()` / `dateToIso()` / `intOrNull()` helpers,
  dropdown value `"fixed"` → `"fixed_amount"`.
- `src/routes/admin/coupons/[id]/+page.svelte` — same corrections plus
  GET-side `centsToDollarStr()` and `dateInputValue()` helpers so the
  edit form actually pre-fills the values that the API returns
  (previously every numeric field came back blank because the local
  `Coupon` interface declared the wrong column names).

### Regression test added

`backend/tests/admin_coupons_create.rs` (new file, 4 tests):

- `create_with_percentage_returns_200_and_persists_row`
- `create_with_fixed_amount_returns_200`
- `create_with_free_trial_returns_200`
- `create_with_camelcase_discount_type_is_rejected_422` — guards
  against accidentally re-introducing the silent dual-acceptance via
  `#[serde(alias = ...)]` in a future refactor

All 4 pass against a real Postgres test DB.

### Tooltip polish

`src/lib/components/ui/Tooltip.svelte` — Google-grade visual refresh:

- **Arrow / stem added.** Hybrid CSS + measurement: a CSS triangle on
  a sibling `<span class="tooltip__arrow">` is positioned via
  `--tooltip-arrow-offset` set from `computePosition()`. Result: the
  arrow always points at the trigger center even when the bubble is
  clamped against the viewport edge (the most common "messy tooltip"
  bug).
- **Refreshed palette.** Background `oklch(16% 0.02 252)` (near-black
  with subtle navy tint), border `oklch(32% 0.02 252)` (teal-tinted),
  single softer shadow `0 4px 16px rgba(0,0,0,0.35)` — replaces the
  noisy double-shadow + inset highlight that competed with admin chrome.
- **Better reading rhythm.** `font-size: 0.8125rem` (was `0.75`),
  `line-height: 1.4` (was `1.35`), padding `0.5rem 0.75rem` (was
  cramped at `0.35rem 0.55rem`).
- **kbd chip lifted** for legibility against the new bg.
- All 6 existing Tooltip browser tests still pass without changes.

### Admin members action-cluster polish

`src/routes/admin/members/+page.svelte` — destructive icon buttons
(`--warn`, `--danger`, `--delete`) now visibly pop more on hover than
the neutral ones via tinted `border-color` shifts and stronger
background tints. Sizing + tab order were already cohesive, left
those alone.

### Sitewide icon audit (no work needed)

Confirmed: zero non-Phosphor icon library imports anywhere; 689
Phosphor imports across 119 .svelte files; only one inline `<svg>` in
the whole `src/routes/` tree (the data-driven circular progress ring
on `/dashboard/+page.svelte` — no Phosphor equivalent, must remain SVG
by design); the two `.svg` files in `static/` and `src/lib/assets/`
are brand favicons.

### `backend/uploads/` now gitignored

Runtime upload destination was leaking accidentally-checked-in files
into `git status`. Added `/uploads/` to `backend/.gitignore`.

### Verification

| Check                                       | Result                             |
| ------------------------------------------- | ---------------------------------- |
| `cargo fmt --all -- --check`                | clean                              |
| `cargo clippy --all-targets -- -D warnings` | clean                              |
| `cargo test --lib`                          | 524 / 0 / 0                        |
| `cargo test --test admin_coupons_create`    | **4 / 4** new tests pass           |
| `pnpm check`                                | 4423 files / 0 errors / 0 warnings |
| `pnpm lint`                                 | clean                              |
| `pnpm test:unit -- --run`                   | 103 / 103                          |
| `pnpm test:unit -- Tooltip --run` (browser) | 6 / 6                              |

---

## 2026-05-01 21:00 ET — Member account self-service (orders, subscriptions, coupons, billing, payment methods)

### What this session shipped

A complete member account section that mirrors the WooCommerce
"My Account" UX (left-rail nav: Orders · Subscriptions · Coupons ·
Billing Address · Payment Methods · Account Details · Log out) and
gives paying members full control over every billing surface without
leaving the app.

Architecturally this lands as 7 sub-routes under `/dashboard/account/`,
11 new member-facing endpoints, 3 new Stripe wrappers, 2 forward-only
migrations, and 25 new integration tests. Backend lib stays at 524/524;
integration suite goes 850 → 930 passes across 48 binaries; frontend
4423 files / 0 errors / 0 warnings; live smoke-test confirms every new
route is wired (401 on unauth, 404 on nonsense path).

### New member endpoints (`backend/src/handlers/member.rs`)

Orders

- `GET  /api/member/orders` — paginated history (own orders only)
- `GET  /api/member/orders/{id}` — items + refunds + state-log

Subscriptions (full history, not just current)

- `GET  /api/member/subscriptions` — paginated, includes cancelled/paused
- `GET  /api/member/subscriptions/{id}` — sub + plan + invoices + related orders
- `POST /api/member/subscriptions/{id}/cancel` — cancel-at-period-end
- `POST /api/member/subscriptions/{id}/resume` — undo cancel
- `POST /api/member/subscriptions/{id}/pause` — body `{ resume_at? }`
- `POST /api/member/subscriptions/{id}/unpause`
- `POST /api/member/subscriptions/{id}/switch-plan` — body `{ pricing_plan_id, prorate? }`
- `GET  /api/member/subscriptions/{id}/switch-plan/preview` — Stripe upcoming-invoice proration dry-run

Coupons / Profile

- `GET  /api/member/coupons/redeemed` — redemption history with `currency` + `order_id`
- Extended `PUT /api/member/profile` to accept `phone` + `billing_address`

Payment methods (native, Stripe-Elements based — no portal redirect)

- `GET    /api/member/payment-methods`
- `POST   /api/member/payment-methods/setup-intent` — returns Stripe SetupIntent client_secret
- `POST   /api/member/payment-methods/{pm_id}/set-default`
- `DELETE /api/member/payment-methods/{pm_id}` — refuses to remove default while subscription is active

Every mutation: ownership check returns 404 (not 403) to avoid existence
leakage, audit row written via `services::audit::record`, idempotency
key forwarded to Stripe.

### New Stripe wrappers (`backend/src/stripe_api.rs`)

- `swap_subscription_price_with_proration` — POST `/v1/subscriptions/{id}` with caller-controlled `proration_behavior`
- `preview_subscription_change` — GET `/v1/invoices/upcoming` aggregated into `SubscriptionChangePreview`
- 6 new payment-method helpers: `list_customer_payment_methods`, `get_customer_default_payment_method`, `create_setup_intent`, `set_default_payment_method`, `detach_payment_method`, `get_payment_method`
- Stripe `Client` refactored to support per-instance `base_url` so
  wiremock can intercept calls in tests; production code path unchanged
  (defaults to `https://api.stripe.com`). Override via env
  `STRIPE_API_BASE_URL` for isolated test runs.

### Schema gap closures

Both columns existed in the DB but weren't surfaced through the typed
struct → OpenAPI → frontend types pipeline. Now they are:

- `Subscription` struct now exposes `cancel_at`, `paused_at`,
  `pause_resumes_at`, `trial_end` (mirror of columns added in
  `041_subscriptions_v2.sql`).
- `MemberSubscriptionInvoice` exposes `hosted_invoice_url` +
  `invoice_pdf` so the SPA can deep-link to Stripe-hosted receipts and
  PDFs without a portal round-trip.

### Migrations (forward-only)

- `084_subscription_invoice_urls.sql` — add `hosted_invoice_url` and
  `invoice_pdf` columns to `subscription_invoices`. Webhook ingester in
  `commerce::billing::upsert_invoice` now persists both on every
  `invoice.paid` / `invoice.payment_failed` event (COALESCE-style so
  drafts that lack the field don't blank out previously-stored values).
- `085_coupon_usages_currency_order.sql` — add `currency TEXT NOT NULL
DEFAULT 'usd'` and `order_id UUID REFERENCES orders(id) ON DELETE
SET NULL` to `coupon_usages`. Both apply-coupon writers (member +
  public) now persist them; reader and `MemberCouponRedemptionResponse`
  return them; coupons frontend page renders `currency` next to the
  discount and adds a "View order →" deep-link column.

### New frontend routes

`src/routes/dashboard/account/`:

- `+layout.svelte` — left-rail nav with active link highlighting,
  mobile horizontal-scroll pill bar, divider, "Log out" at the bottom
- `+page.svelte` — redirects to `/dashboard/account/subscriptions`
- `orders/+page.svelte` — paginated table (responsive → cards on mobile)
- `orders/[id]/+page.svelte` — items + totals + refunds + collapsible state-log
- `subscriptions/+page.svelte` — full history with status badges
- `subscriptions/[id]/+page.svelte` — overview + actions (cancel/pause
  with date picker, resume, switch-plan with proration preview) +
  invoices table with hosted-receipt links + related orders
- `coupons/+page.svelte` — apply form + redemption history
- `billing-address/+page.svelte` — phone + address form, ISO-3166
  country select, pre-filled from `GET /profile`
- `payment-methods/+page.svelte` — native card list with set-default /
  delete actions; "+ Add card" modal mounts Stripe Elements card field
  via dynamic `https://js.stripe.com/v3/` script load and confirms via
  `stripe.confirmCardSetup(client_secret, …)`. PCI scope stays inside
  Stripe Elements; the BFF only ever sees opaque `pm_*` ids.
- `details/+page.svelte` — name editor + change-password + DELETE-typed
  danger zone

Account UX polish on the dashboard layout: 36×36 sidebar avatar with
photo-or-teal-initials fallback + "Member" badge; mobile bottom-tab
bar shows the avatar circle in place of the generic Account icon when
the member has uploaded one.

### Test additions

- `tests/member_orders.rs` — 6 tests (list/detail + ownership 404)
- `tests/member_subscriptions.rs` — 13 tests (was 12; added
  `redeemed_coupons_includes_currency_and_order_id`)
- `tests/member_subscriptions_stripe.rs` — 5 wiremock-driven tests
  covering switch-plan and preview happy-paths, proration_behavior
  toggling, Stripe error bubble-up, and DB rollback verification
- `tests/member_payment_methods.rs` — 7 wiremock tests covering list,
  setup-intent (with idempotency-key forwarding asserted byte-for-byte),
  set-default with ownership 404, delete with active-sub-default
  rejection
- `tests/member_self_service.rs` — 13 tests (existing harness already
  covered cancel/resume/pause; ran green throughout)
- `tests/member_profile_address.rs` — 4 tests covering billing-address
  overlay semantics

### Verification (binding evidence)

| Check                                                        | Result                                                                  |
| ------------------------------------------------------------ | ----------------------------------------------------------------------- |
| `cargo fmt --all -- --check`                                 | clean (exit 0)                                                          |
| `cargo clippy --all-targets -- -D warnings`                  | clean (exit 0)                                                          |
| `cargo test --lib`                                           | 524 passed / 0 failed / 0 ignored                                       |
| `cargo test --tests --no-fail-fast` (real Postgres on :5433) | 930 passed / 0 failed / 0 ignored across 48 binaries                    |
| `pnpm check` (incl. OpenAPI regen + svelte-kit sync)         | 4423 files / 0 errors / 0 warnings                                      |
| `pnpm lint`                                                  | clean                                                                   |
| `pnpm test:unit -- --run`                                    | 103 passed / 103 total across 12 files                                  |
| Migration replay (fresh DB, `001..085` in order)             | all 85 apply cleanly; new columns confirmed via `\d`                    |
| Live smoke test (cargo run + curl)                           | 11/11 new endpoints return 401 (route wired); negative path returns 404 |

### Decisions worth flagging

- Payment Methods is **native, not portal redirect** — Stripe Elements
  mounts client-side, the BFF only ever holds the opaque `pm_*` id.
  PCI scope stays inside Stripe Elements; we don't touch raw card data.
- Switch-plan **happy-path round-trip is integration-tested via
  wiremock**, not stubbed. Required refactoring `Client` to take a
  per-instance `base_url`. Test harness sets the override; production
  code path defaults to `https://api.stripe.com` and is unchanged.
- Coupon-history `currency` defaults to `'usd'` for back-compat with
  rows inserted before migration 085. `order_id` is nullable because
  apply-coupon doesn't always have an order context.
- Refused to delete a member's default payment method while they have
  an active subscription — returns 400 with explicit operator-facing
  message instead of leaving the sub stranded with no card.

---

## 2026-05-01 17:30 ET — Phase C: real Stripe E2E (11/11 green) + trial support

### What this session shipped

End-to-end QA of the membership platform against a live Stripe sandbox.
The driver in [`scripts/phase_c_stripe_e2e.py`](./scripts/phase_c_stripe_e2e.py)
runs 11 lifecycle scenarios (signup, trial, dunning, cancel, pause,
resume, refund, dispute, ban-vs-active-sub, course gating) and asserts
DB state + API response after every Stripe action. **All 11 scenarios
pass with 24/24 assertions.** Full report:
[`docs/STRIPE-E2E-RESULTS-2026-05-01.md`](./docs/STRIPE-E2E-RESULTS-2026-05-01.md).

### Real backend bugs surfaced and fixed during Phase C

The unit-test suite was green throughout, yet four production bugs lurked
because none of them was exercised by a fixture-based test that walked
the full Stripe→webhook→DB→API path:

1. **`SubscriptionStatus` enum derive was `rename_all = "lowercase"`**
   so `PastDue` serialised as `pastdue` (no underscore). Postgres has
   `past_due`. Every `/api/member/subscription` call for a past_due
   user returned HTTP 500 with a sqlx ColumnDecode error. Fixed:
   `rename_all = "snake_case"`.
2. **`SubscriptionStatus` was missing the `Paused` variant.** Migration
   057 added `paused` to the Postgres enum but the Rust side never
   enumerated it. Same column-decode 500 for any paused subscription.
   Fixed: added the variant.
3. **`course_enrollments.id` had no DEFAULT and `enroll_course` did
   not bind one.** Every enrollment 500'd with a NOT NULL violation.
   Latent since `001_initial.sql`. Fixed: migration 082 sets
   `DEFAULT gen_random_uuid()` and the handler now binds an explicit
   `Uuid::new_v4()` for belt-and-braces.
4. **`charge.refunded` handler dropped events on modern Stripe API
   versions.** Stripe is migrating refund delivery from embedded
   `charge.refunds.data[]` to standalone `refund.*` events; on the
   `2026-03-25.dahlia` API version the embedded array is empty.
   Fixed: added a `refund.created` handler that parses the standalone
   `refund` object via a new
   `commerce::refunds::ChargeRefundFields::from_refund_object`
   constructor. Both event paths feed the same idempotent
   `record_charge_refund` writer.

### Trial subscriptions: 7 / 14 / 30 days, with or without credit card

Operator-driven feature work added in the same landing.

- **Migration 083** adds
  `pricing_plans.collect_payment_method_at_checkout BOOLEAN DEFAULT TRUE`.
- **BFF `createCheckoutSession`** ([`src/routes/api/checkout.remote.ts`](./src/routes/api/checkout.remote.ts))
  now passes `subscription_data.trial_period_days` from `plan.trial_days`,
  and switches to `payment_method_collection: 'if_required'` when the
  plan opts out of card collection. Previously, even plans with
  `trial_days > 0` were billing immediately because the column never
  reached Stripe.
- **Three demonstration plans** seeded in dev — `trial-7` (7 days,
  card required), `trial-14` (14 days, card required), and `trial-30`
  (30 days, **no card required**).

### New companion docs

- [`docs/SECRETS-PRIMER.md`](./docs/SECRETS-PRIMER.md) — how to mint
  `JWT_SECRET` / `SETTINGS_ENCRYPTION_KEY`, where to find Stripe test
  keys, what goes in which `.env`, rotation runbook.

### Operator notes (env/secret hygiene)

- `EMAIL_PROVIDER=noop` set on `backend/.env` for dev so Phase C runs
  do not blow Resend's free quota minting "welcome" emails for every
  fresh test user. Keep it that way unless you specifically need to
  smoke-test email delivery.
- The `stripe listen` whsec rotates every time the CLI re-establishes
  its WebSocket. After restarting the backend, restart `stripe listen`
  too and re-paste the secret — see Phase C run #6 in the session log
  for what the symptom looks like (no events landing).

### Verification

```
cargo fmt --all -- --check                        → clean
cargo clippy --all-targets -- -D warnings         → clean
cargo test --tests                                → 893 pass / 0 ignored / 0 failed
pnpm lint + pnpm check + pnpm test:unit           → clean (103 unit tests)
python3 scripts/phase_c_stripe_e2e.py             → 11/11 scenarios, 24/24 asserts
```

### Known follow-ups (deferred)

- Pay-per-course purchase ledger (`course_purchases` table + flow);
  enrollment gate currently 403s on this case as a hard block.
- `tests/stripe_webhooks.rs::charge_refunded_*` tests passed against
  the OLD broken behaviour and need re-asserted; add `refund_created_*`
  test family.
- Frontend pricing card surface for `collect_payment_method_at_checkout`
  ("no card required" badge).
- Stripe `refund.updated` handler (refund status transitions).

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

| File                                                                  | Verdict | Why                                                                                                                                                                                   |
| --------------------------------------------------------------------- | ------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `.editorconfig`, `.npmrc`, `.nvmrc`, `.prettierrc`, `.prettierignore` | KEEP    | standard tooling configs                                                                                                                                                              |
| `.sqlfluff`                                                           | KEEP    | live config consumed by `.github/workflows/sql-lint.yml`                                                                                                                              |
| `.trivyignore`                                                        | KEEP    | live config consumed by `.github/workflows/security.yml`; currently empty list (intentional — file documents the suppression policy and provides a curated home for any future entry) |
| `.vercelignore`, `.dockerignore`, `.gitignore`                        | KEEP    | active build excludes                                                                                                                                                                 |
| `.mcp.json`                                                           | KEEP    | project-level MCP server registration (Svelte + rust-analyzer)                                                                                                                        |
| `project.inlang/settings.json`                                        | KEEP    | inlang IDE / Sherlock / Fink tooling source-of-truth + planned migration path for `src/lib/i18n/paraglide.ts` shim                                                                    |
| `backend/.env.example`, `.env.example`                                | KEEP    | committed templates; documented `SWINGS_ALLOW_HTTP_WEBHOOKS` (the one env var production code reads but the template did not list)                                                    |

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
  "env-tuned via PGPOOL\_\*"; `DATABASE_URL` row updated to drop the
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

| Area                                                                               | Result                            |
| ---------------------------------------------------------------------------------- | --------------------------------- |
| Admin mutation `policy.require` enforcement                                        | All 31 handlers compliant         |
| Admin mutation `audit_admin` recording                                             | All 31 handlers compliant         |
| Idempotency-Key middleware on all admin POST/PUT/DELETE                            | Fully wired                       |
| `unwrap` / `expect` / `panic!` in non-test production code                         | Zero violations                   |
| Handler registration — orphaned or unregistered handlers                           | None found                        |
| Database table ↔ HTTP endpoint coverage                                            | 100%                              |
| Background worker graceful-shutdown paths                                          | All 5 workers correct             |
| Migration sequence 001–079 (gaps 029/040 intentional)                              | Clean                             |
| Migration foreign-key ordering                                                     | No violations                     |
| RBAC permission matrix: handler calls vs. seeded migrations                        | 37/37 match                       |
| Domain modules completeness (commerce, consent, popups, forms, notifications, pdf) | All fully implemented             |
| Admin frontend: idempotency keys auto-injected by API client                       | Correct                           |
| Admin frontend: BFF HttpOnly-cookie auth pattern                                   | Correctly implemented             |
| Admin frontend: route auth guards                                                  | All protected, no gaps            |
| Admin frontend: TypeScript strict mode, zero `any` types                           | Confirmed                         |
| Backend integration tests — `#[ignore]` violations                                 | Zero (policy maintained)          |
| Backend integration tests — handler coverage                                       | 36 tests, all 31 handlers covered |

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
