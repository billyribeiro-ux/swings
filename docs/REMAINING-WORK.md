# Remaining Work — Implementation Plan

> **Last revised:** 2026-04-25
> **Author:** Principal-engineer audit (read-only sweep)
> **Companion docs:** [`AUDIT.md`](../AUDIT.md), [`AGENTS.md`](../AGENTS.md),
> [`docs/RUNBOOK.md`](./RUNBOOK.md), [`docs/stripe-local-testing.md`](./stripe-local-testing.md),
> [`docs/stripe-pricing-models.md`](./stripe-pricing-models.md).
> **Scope:** SvelteKit frontend, Rust/Axum backend, Postgres, Stripe, Resend,
> R2, Vercel/Railway. Production domain
> `precisionoptionsignals.com` (frontend) +
> `swings-production.up.railway.app` (API).

---

## Executive summary

`swings` is in good structural shape — the foundational pillars (FDN-_),
admin platform (ADM-_), commerce (EC-_), forms (FORM-_), consent
(CONSENT-_), popups (POP-_) and observability scaffolding all landed,
along with 76 forward-only migrations, ~480 backend lib tests, ~67
frontend unit tests, and a production-grade audit/RBAC catalogue. The
**primary risk surface is no longer "does the feature exist" but
"does the feature enforce its own contract end-to-end"** — three classes
of gap dominate:

1. **Half of the admin handlers do not call `policy.require()`.** The
   newer admin handlers (`admin_audit`, `admin_dsar`, `admin_orders`,
   `admin_subscriptions`, `admin_settings`, `admin_security`,
   `admin_roles`, `admin_ip_allowlist`, `admin_impersonation`,
   `admin_members`) consistently gate on the FDN-07 permission catalogue.
   The legacy ones (`blog`, `courses`, `coupons`, `popups`, `products`,
   `pricing`, `forms`, `admin_consent`, `notifications`, `outbox`) gate
   only on the `AdminUser` role string and never call
   `policy.require()`. A `support`-role user with `admin.dashboard.read`
   can today reach mutator handlers in those domains because the role
   extractor accepts any non-`member` role with the dashboard perm.

2. **The Stripe webhook handler covers only three event types.** The
   on-ramp (`checkout.session.completed`,
   `customer.subscription.{created,updated,deleted}`) is wired; the
   _post-purchase lifecycle_ (`invoice.payment_failed`, `invoice.paid`,
   `charge.refunded`, `payment_intent.payment_failed`,
   `customer.subscription.trial_will_end`, dunning, dispute) is not.
   The pricing-rollout / one-time-product / form-payment-intent code
   paths therefore complete in the database but never reflect later
   real-world payment-state changes back into our local mirror.

3. **Several backend admin surfaces have no admin UI.** The backend
   exposes `/api/admin/notifications/templates`,
   `/api/admin/notifications/deliveries`,
   `/api/admin/notifications/suppression`, `/api/admin/outbox/*`,
   `/api/admin/consent/*` (write API), and `/api/admin/forms/*`
   submissions exports — but no SvelteKit pages render them. Consent
   admin (`/admin/consent`) has only a 217-line landing page; the
   per-resource sub-pages (`banner`, `categories`, `services`, `policies`,
   `log`) do not call any `/api/admin/consent/*` write endpoint.

Beyond that, the auth model is missing MFA, the BFF cookie pattern
called for in `AUDIT.md` §3.1 #1 is still pending (admin tokens live in
`localStorage`), the test suite has no Stripe-webhook integration
fixture, and the `OpenAPI` snapshot omits a sizeable chunk of public
GETs. None of these are ship-blockers in isolation; together they are
the next 4–6 weeks of work.

---

## Status snapshot

Legend: ✅ shipped · ◐ partial / stubby · ✗ missing · n/a not in scope.

| Domain                                                              | Backend CRUD                                         | Backend RBAC (`policy.require`)           | Frontend CRUD UI                               | Audit row                   | Idempotency                         | Tests                             |
| ------------------------------------------------------------------- | ---------------------------------------------------- | ----------------------------------------- | ---------------------------------------------- | --------------------------- | ----------------------------------- | --------------------------------- |
| Members lifecycle (suspend/ban/email/sessions)                      | ✅ `admin_security.rs`                               | ✅ 11 calls                               | ✅ `members/[id]` + `members/manage`           | ✅                          | n/a (gated POSTs idempotent by id)  | ✅ `admin_security.rs` (13 tests) |
| Members search + manual create                                      | ✅ `admin_members.rs`                                | ✅ 2 calls                                | ✅ `members/manage/+page.svelte`               | ✅                          | ✅                                  | ✅ `admin_members.rs` (12 tests)  |
| Member self profile / dashboard                                     | ✅ `member.rs`                                       | ✗ AuthUser only — see §3.2 #4 of AUDIT.md | ✅                                             | n/a                         | n/a                                 | ◐                                 |
| Watchlists                                                          | ✅ admin.rs                                          | ✗ AdminUser only                          | ✅ list-only                                   | ◐ best-effort               | ✗                                   | ✗                                 |
| Blog (posts/categories/tags/media)                                  | ✅ `blog.rs` (1402 LOC)                              | ✗ no `policy.require` (zero calls)        | ✅ rich editor                                 | ✅ `audit_admin` everywhere | ✗ not wired                         | ✗                                 |
| Courses / modules / lessons                                         | ✅ `courses.rs`                                      | ✗ no `policy.require`                     | ✅                                             | ✅                          | ✗                                   | ✗                                 |
| Coupons (incl. bulk, engine refactor)                               | ✅ `coupons.rs` (1274 LOC, EC-11)                    | ✗ no `policy.require`                     | ✅                                             | ✅                          | ✗                                   | ✗                                 |
| Popups + variants + analytics                                       | ✅ `popups.rs` (1323 LOC)                            | ✗ no `policy.require`                     | ✅                                             | ✅                          | ✗                                   | ✗                                 |
| Products + variants + assets + bundles                              | ✅ `products.rs`                                     | ✗ no `policy.require`                     | ◐ list + edit only                             | ✅                          | ✗                                   | ✗                                 |
| Pricing plans + rollout                                             | ✅ `pricing.rs` + `services::pricing_rollout`        | ✗ no `policy.require`                     | ✅ `subscriptions/plans`                       | ✅                          | ✅ rollout requires Idempotency-Key | ✗                                 |
| Orders (manual create/void/refund/CSV)                              | ✅ `admin_orders.rs`                                 | ✅ 6 calls                                | ✅ `orders/+page.svelte`                       | ✅                          | ✅                                  | ✅ 18 tests                       |
| Subscriptions (comp / extend / cycle override)                      | ✅ `admin_subscriptions.rs`                          | ✅ 4 calls                                | ✅ `subscriptions/manual`                      | ✅                          | ✅                                  | ✅ 13 tests                       |
| Stripe webhooks                                                     | ◐ 3 event types                                      | n/a webhook                               | n/a                                            | ✗ never audits              | ✅ via `processed_webhook_events`   | ◐ signature + tampering only      |
| Public/admin checkout (`SvelteKit remote fn`)                       | ✅ `src/routes/api/checkout.remote.ts`               | n/a                                       | ✅ pricing pages                               | n/a                         | ◐ stripe key reused                 | ✗ no E2E                          |
| Cart (guest + authed)                                               | ✅ `cart.rs`                                         | n/a (member or anon)                      | ✗ no admin UI for cart inspection              | n/a                         | ✗                                   | ✗                                 |
| Audit log viewer + FTS                                              | ✅ `admin_audit.rs`                                  | ✅ 3 calls                                | ✅ `audit/+page.svelte`                        | n/a self                    | n/a (read-only)                     | ✅ 12 tests                       |
| DSAR (request, export, erase, sweep)                                | ✅ `admin_dsar.rs` (889 LOC)                         | ✅ 6 calls + dual-control                 | ✅ `dsar/+page.svelte`                         | ✅                          | ✅                                  | ✅ 15 + sweep + r2 + async        |
| Security (IP allowlist, impersonation, roles)                       | ✅ all 3 handlers                                    | ✅ 21 calls combined                      | ✅ all 3 sub-pages                             | ✅                          | n/a (gated POST)                    | ✅ 11 + 21 + 10                   |
| Settings (typed catalogue)                                          | ✅ `admin_settings.rs`                               | ✅ 5 calls                                | ✅ `settings/+page.svelte` + `settings/system` | ✅                          | ✗ not on writes                     | ✅ 9 tests                        |
| Forms builder + submissions + payment intents                       | ✅ `forms.rs` (928 LOC)                              | ✗ no `policy.require`                     | ◐ list + edit; **no submissions table page**   | ✅                          | ✗                                   | ✗                                 |
| Consent banner / categories / services / policies / log / integrity | ✅ `admin_consent.rs` (810 LOC)                      | ✗ no `policy.require`                     | ◐ landing page only — sub-pages exist as stubs | ◐ outbox event only         | ✗                                   | ✗                                 |
| Consent DSAR (member-side)                                          | ✅ `consent.rs::admin_router` + `public_dsar_router` | ✗ no `policy.require`                     | n/a (admin DSAR is the path)                   | ✅                          | n/a                                 | ✗                                 |
| Author profile                                                      | ✅ `users` columns from 005_user_author_profile.sql  | n/a                                       | ✅ `author/+page.svelte`                       | ✅                          | n/a                                 | ✗                                 |
| Analytics ingest + summary                                          | ✅ `analytics.rs` + `admin.rs::analytics_*`          | ✗ no `policy.require`                     | ✅ `analytics/+page.svelte`                    | n/a                         | n/a                                 | ✗                                 |
| Notifications (templates / deliveries / suppression)                | ✅ `notifications.rs` admin router                   | ✗ no `policy.require`                     | **✗ no admin UI at all**                       | ✅                          | ✗                                   | ✗ outside provider/webhook        |
| Outbox viewer / retry                                               | ✅ `outbox.rs`                                       | ✗ no `policy.require`                     | **✗ no admin UI**                              | ✅                          | n/a                                 | ✅ 5 tests                        |
| Email verification + resend                                         | ✅ `auth.rs::verify_email`, `resend_verification`    | n/a public                                | ✗ no UI page (uses link in mail only)          | n/a                         | n/a                                 | ✗                                 |
| Forgot / reset password                                             | ✅ `auth.rs`                                         | n/a public                                | ✅ `/forgot-password`, `/reset-password`       | n/a                         | n/a                                 | ✗                                 |
| MFA / passkeys                                                      | ✗                                                    | ✗                                         | ✗                                              | ✗                           | ✗                                   | ✗                                 |

**Reading guide:** the rows where backend RBAC is "✗ no `policy.require`"
are the §3 "Phase 3" target list; rows where the backend column shows
**◐** plus a missing UI are the §2 "Phase 2" target list; the Stripe
webhook ◐ is §4 in its own right because it is unique in shape.

---

## Phase 1 — Foundational (P0, ~2 weeks)

### 1.1 Stripe webhook coverage

- **What:** wire all post-purchase Stripe events into
  `backend/src/handlers/webhooks.rs` instead of the current 3 (subscription
  created/updated/deleted + checkout.session.completed).
- **Why:** today, a successful purchase flows in but a refund, dunning,
  trial-end, or subscription pause never updates our DB — the Stripe
  Dashboard and our admin UI silently diverge. Failed-payment retries
  also never page anyone.
- **Where:**
  - `backend/src/handlers/webhooks.rs:110` (the `match event_type`
    block — extend with additional arms).
  - `backend/src/db.rs` — add `record_invoice_payment_failed`,
    `record_invoice_paid`, `record_charge_refunded` helpers.
  - `backend/src/notifications/templates.rs` — register
    `subscription.payment_failed`, `subscription.trial_ending`,
    `order.refund.issued` templates.
- **Acceptance criteria:**
  - All seven additional event types parsed:
    `invoice.payment_failed`, `invoice.paid`,
    `customer.subscription.trial_will_end`,
    `customer.subscription.paused`, `customer.subscription.resumed`,
    `charge.refunded`, `payment_intent.payment_failed`.
  - Each writes a row to `admin_actions` (or its own dedicated table)
    keyed by Stripe event id so the audit log shows the sequence
    end-to-end.
  - `subscriptions.status` reflects `past_due` → `unpaid` → `canceled`
    when Stripe steps a member through dunning.
  - Refund handler reconciles `orders.refund_amount_cents` (already in
    `035_orders.sql`) and emits `order.refunded` outbox event.
  - Integration test in new `backend/tests/stripe_webhooks.rs` exercises
    each arm with a forged-but-valid signature.
- **Estimated effort:** **L** (5–7 d).

### 1.2 BFF cookie / server-side admin gate (carry-forward from `AUDIT.md` §3.1 #1)

- **What:** replace `localStorage` JWT custody with HttpOnly `Secure`
  `SameSite=Lax` cookies and SSR-gate `/admin/**` + `/dashboard/**`.
- **Why:** any reflected XSS on an admin page is currently full admin
  takeover. F17 in `AUDIT.md` sanitised the known sinks; cookie custody
  is the structural fix.
- **Where:**
  - New `src/routes/admin/+layout.server.ts` and
    `src/routes/dashboard/+layout.server.ts`.
  - `src/hooks.server.ts` — verify cookie + role; redirect/error before
    render.
  - New `/api/auth/cookie-login` endpoint or wrap existing `/api/auth/login`
    response with `Set-Cookie`.
  - Backend: `extractors.rs` already accepts `Authorization: Bearer …`;
    add a fallback path that reads the cookie when the BFF is the caller.
- **Acceptance criteria:**
  - `curl -i https://app/admin` (no cookie) returns a 303 from the
    SvelteKit edge — never reaches the client SPA.
  - `localStorage.access_token` never set after login.
  - Playwright `e2e/admin/security.spec.ts` updated to cookie priming.
  - Cookie carries no PII, only the access JWT; refresh path moves to
    server (cookie rotated by SvelteKit, refresh JWT never leaves the
    server).
- **Estimated effort:** **L** (5–7 d).

### 1.3 Wire RBAC enforcement on legacy admin handlers

Promote the `admin.require(&state.policy, "<perm>")` pattern from
`admin_security.rs` (which already does it on every mutator) into the
ten handlers that today skip it. See **Phase 3** for the full audit.

- **What:** add `admin.require()` (or `privileged.require()`) to every
  POST/PUT/DELETE/PATCH route in: `blog`, `courses`, `coupons`,
  `popups`, `products`, `pricing`, `forms`, `admin_consent`,
  `notifications`, `outbox`.
- **Why:** the FDN-07 permission catalogue exists and is loaded at boot,
  but none of these handlers consult it. A `support`-role token (which
  has `admin.dashboard.read` per `021_rbac.sql`) can today hit
  `POST /api/admin/blog/posts` and create a post.
- **Where:** every admin route in the listed handlers — the route table
  is in each handler's `admin_router()`.
- **Acceptance criteria:** `grep -nE "AdminUser" backend/src/handlers/*.rs`
  followed by absence of `policy.require` in the same function body
  returns **zero** non-test hits.
- **Estimated effort:** **M** (3–4 d) — mechanical but must touch every
  handler.

### 1.4 MFA for admin / support roles (TOTP first)

- **What:** add TOTP-based MFA enrolment + verification for any user
  with role ≠ `member`. WebAuthn / passkeys is a follow-up.
- **Why:** the platform protects member commerce data. A privileged
  account with only password + email recovery does not meet a 2026 bar
  for SaaS that handles PII + payments.
- **Where:** new migration `077_mfa_secrets.sql`, new
  `backend/src/handlers/auth_mfa.rs`, new `notifications` template
  `user.mfa.enabled`, frontend `/admin/security/mfa/+page.svelte`.
- **Acceptance criteria:**
  - Enrolment: `POST /api/auth/mfa/enroll` returns provisioning URI.
  - Verification: `POST /api/auth/mfa/verify` accepts a 6-digit TOTP.
  - Login flow: when role ≠ `member` and `mfa_enabled = true`, login
    issues a partial JWT with `mfa_required = true` until `/api/auth/mfa/verify`
    is called.
  - Recovery codes generated (10 single-use), shown once, hashed in DB.
  - Audit row on enrol / disable / failed-attempt.
- **Estimated effort:** **L** (5+ d).

---

## Phase 2 — CRUD completeness (P1, 2–3 weeks)

Each item below corresponds to a backend route that exists today but
either has no admin UI or only has a list page. File paths under
`src/routes/admin/` are absolute relative to repo root.

### 2.1 Notifications admin (templates / deliveries / suppression)

- **What:** add `/admin/notifications/templates`,
  `/admin/notifications/deliveries`, `/admin/notifications/suppression`
  pages.
- **Backend already exists:**
  `backend/src/handlers/notifications.rs:39` (`admin_router`) — list /
  create / get / update / preview / test-send / list-deliveries /
  list-suppression / add / remove suppression.
- **Acceptance:**
  - Templates: list, create, edit, preview rendered HTML, send-test to a
    chosen recipient.
  - Deliveries: filter by template / status / recipient with pagination.
  - Suppression: search / add manual entry / remove (audit-row backed).
- **Effort:** **M** (3–4 d).

### 2.2 Outbox admin (`/admin/outbox`)

- **What:** add list + retry + drilldown.
- **Backend:** `backend/src/handlers/outbox.rs:128` (list),
  `outbox.rs:223` (get), `outbox.rs:258` (retry).
- **Acceptance:** retry re-enqueues the event and increments
  `outbox_events.attempts`; UI shows `next_run_at`, `last_error`,
  `aggregate_type/id`.
- **Effort:** **S** (1–2 d).

### 2.3 Consent admin sub-pages

- **What:** flesh out the four sub-pages currently mounted as routes —
  banner, categories, services, policies — to actually call the existing
  write API. The current `admin/consent/+page.svelte` is a 217-LOC
  landing card.
- **Backend already exists:** `admin_consent.rs:46`.
- **Acceptance:** banner edit page POSTs to
  `/api/admin/consent/banners`; categories, services edits work; policy
  versioning shown read-only because the table is append-only.
- **Effort:** **M** (3–4 d).

### 2.4 Forms — submissions table + per-form bulk actions

- **What:** add `/admin/forms/[id]/submissions/+page.svelte` to render
  rows from `GET /api/admin/forms/{id}/submissions` with filter +
  bulk-update (`POST /api/admin/forms/{id}/submissions/bulk`).
- **Backend:** `backend/src/handlers/forms.rs:79` (`admin_router`)
  already has list + bulk-update + CSV export.
- **Acceptance:** CSV export download works; archive-bulk + delete-bulk
  surfaced; per-row JSON inspector.
- **Effort:** **M** (2–3 d).

### 2.5 Watchlists CRUD UI completion

- **What:** the watchlists list (`/admin/watchlists/+page.svelte`,
  747 L) renders but the list-only nav is misleading — there is a `new/`
  - `[id]/` route, but the list does not surface the publish/unpublish
    toggle outcome reliably and the alerts surface (`/api/admin/watchlists/{id}/alerts`)
    is not exposed.
- **Backend:** `backend/src/handlers/admin.rs:54` —
  `/watchlists/{id}/alerts` GET/POST + `/alerts/{id}` PUT/DELETE.
- **Acceptance:** alerts panel inside `watchlists/[id]` lists, creates,
  edits, deletes alerts; toast feedback on toggle.
- **Effort:** **M** (2 d).

### 2.6 Products — variants / assets / bundles UI

- **What:** the products list + edit page exists (`products/+page.svelte`,
  888 L; `products/[id]/+page.svelte`) but variant CRUD, asset add/remove,
  and bundle composition are not surfaced.
- **Backend:** `backend/src/handlers/products.rs:432` (variants),
  `:597` (assets), `:707` (bundle items).
- **Acceptance:** variant grid editable in-page; assets with thumbnail
  - delete; bundle composition via product picker.
- **Effort:** **M** (3 d).

### 2.7 Member detail — subscription + sessions tabs

- **What:** `members/[id]/+page.svelte` exists but does not surface the
  `admin_security.rs` sessions endpoint (`GET/DELETE
/api/admin/security/members/{id}/sessions`) nor the
  `force-password-reset` / `verify-email` actions in a unified panel.
- **Backend:** `admin_security.rs:46` routes.
- **Acceptance:** sessions tab lists active refresh tokens; revoke
  one-by-one + revoke-all; force-password-reset emits the reset email;
  verify-email button bypasses outbound email when set.
- **Effort:** **S** (1–2 d).

### 2.8 Cart admin inspector

- **What:** add `/admin/carts/+page.svelte` (read-only) so support can
  view a member's cart by user_id or anonymous_id.
- **Backend:** `backend/src/handlers/cart.rs:106` is member/anon-scoped;
  add a thin `/api/admin/carts/{user_or_anon}` GET.
- **Acceptance:** read-only render of cart lines + totals; useful when
  triaging "the discount didn't apply" tickets.
- **Effort:** **S** (1 d).

### 2.9 Subscription detail page

- **What:** `subscriptions/+page.svelte` (1118 L) is a list. There is no
  per-subscription detail page that surfaces the audit trail of comps,
  extensions, billing-cycle overrides for that one row.
- **Backend:** `admin_subscriptions.rs::by_user` exists; add
  `GET /api/admin/subscriptions/{id}` and the ledger query.
- **Acceptance:** drilldown shows the full action history (audit_log
  filtered by `target_id = subscription.id`) and current vs proposed
  plan.
- **Effort:** **M** (2 d).

### 2.10 Members impersonation banner UX

- **What:** `middleware/impersonation_banner::stamp` already adds
  response headers (ADM-07) but no global frontend banner is rendered to
  the impersonator showing "you are acting as X".
- **Where:** `src/routes/+layout.svelte` should read the
  `X-Impersonation-Active` header from the `me` response and render a
  red bar with an "exit" CTA.
- **Acceptance:** banner appears within 1 navigation; "exit" hits
  `/api/auth/logout` (which the `auth.rs::logout` already
  understands as "end impersonation, do not log target out").
- **Effort:** **S** (1 d).

---

## Phase 3 — RBAC enforcement audit (P1, ~1 week)

The FDN-07 catalogue (migration `021_rbac.sql` + the 058 / 063 / 064 /
066 / 067 / 068 / 069 perm-extension migrations) defines ~70
permissions. The `Policy` is loaded at boot
(`backend/src/main.rs:216`). Eleven admin handlers already use it
correctly (21 + 6 + 6 + 11 + 4 + 5 + 6 + 4 + 3 + 2 = **66 `require`
calls**). Ten admin handlers do not.

### 3.1 Handlers missing `policy.require` (must add)

For each, the verb-suffixed permissions to use already exist in the
catalogue.

| Handler             | File                                    | Routes count    | Suggested perms                                                                                    |
| ------------------- | --------------------------------------- | --------------- | -------------------------------------------------------------------------------------------------- |
| Blog                | `backend/src/handlers/blog.rs`          | 24 admin routes | `blog.post.create/update_*/delete_*/publish`, `blog.media.upload/delete_*`, `blog.category.manage` |
| Courses             | `backend/src/handlers/courses.rs`       | 7 admin routes  | `course.manage` (single perm covers all admin mutations)                                           |
| Coupons             | `backend/src/handlers/coupons.rs`       | 9 admin routes  | `coupon.manage`, `coupon.read_any` for GETs                                                        |
| Popups              | `backend/src/handlers/popups.rs`        | 10 admin routes | `popup.manage`, `popup.read_analytics` for analytics                                               |
| Products            | `backend/src/handlers/products.rs`      | 14 admin routes | new `product.manage` perm via migration `077_product_perms.sql`                                    |
| Pricing             | `backend/src/handlers/pricing.rs`       | 8 admin routes  | `subscription.plan.manage`                                                                         |
| Forms               | `backend/src/handlers/forms.rs`         | 4 admin routes  | `form.manage`, `form.submission.read_any`, `form.submission.delete_any`                            |
| Admin consent       | `backend/src/handlers/admin_consent.rs` | 11 admin routes | new `consent.config.manage` (migration), reuse `consent.log.read_any`                              |
| Notifications admin | `backend/src/handlers/notifications.rs` | 8 admin routes  | `notification.template.manage`, `notification.broadcast.create`                                    |
| Outbox              | `backend/src/handlers/outbox.rs`        | 3 admin routes  | `admin.outbox.read`, `admin.outbox.retry`                                                          |

**Acceptance:**

1. `grep -nE "AdminUser|PrivilegedUser" backend/src/handlers/*.rs |
xargs -I{} grep -L "policy.require\\|policy.has" {}` returns empty.
2. Every admin POST/PUT/PATCH/DELETE pairs `policy.require()` with
   `services::audit::audit_admin*()`.
3. New CI check: a Rust unit-test under
   `backend/tests/authz_matrix.rs` walks the OpenAPI tag list and asserts
   each admin route has at least one `policy.require` call by inspecting
   `axum::Router` visit order (or use a `#[admin_handler]` attribute
   macro).

**Estimated effort:** **M** (3–4 d). Mechanical.

### 3.2 Permissions defined but never checked

Cross-referencing `021_rbac.sql` plus 058/063/064/066/067/068/069 with
the call sites:

| Permission                                                                                              | Defined in | Currently checked?                                     |
| ------------------------------------------------------------------------------------------------------- | ---------- | ------------------------------------------------------ |
| `blog.post.update_own` / `_any`, `delete_own` / `_any`, `publish`                                       | 021        | ✗ — author-vs-admin distinction collapses to AdminUser |
| `blog.media.delete_own` / `_any`                                                                        | 021        | ✗                                                      |
| `blog.category.manage`                                                                                  | 021        | ✗                                                      |
| `course.manage`, `course.enroll.other`, `course.progress.read_any`                                      | 021        | ✗                                                      |
| `coupon.manage`, `coupon.read_any`                                                                      | 021        | ✗                                                      |
| `popup.manage`, `popup.read_analytics`                                                                  | 021        | ✗                                                      |
| `subscription.plan.manage`                                                                              | 021        | ✗ (used by `pricing.rs`)                               |
| `form.manage`, `form.submission.read_any`, `form.submission.delete_any`                                 | 021        | ✗                                                      |
| `notification.template.manage`, `notification.broadcast.create`                                         | 021        | ✗                                                      |
| `admin.outbox.read`, `admin.outbox.retry`                                                               | 021        | ✗                                                      |
| `dsar.fulfill`                                                                                          | 021        | ✓ (admin_dsar)                                         |
| `admin.audit.read`                                                                                      | 021        | ✓ (admin_audit + admin_security)                       |
| `admin.role.manage`, `admin.permission.manage`                                                          | 021        | ✓ (admin_roles)                                        |
| `user.suspend / reactivate / ban / force_password_reset / email.verify / session.read / session.revoke` | 058        | ✓ (admin_security)                                     |
| `admin.ip_allowlist.read / manage`                                                                      | 059        | ✓ (admin_ip_allowlist)                                 |
| `admin.impersonation.create` (`IMPERSONATE_PERMISSION`)                                                 | 060        | ✓ (admin_impersonation)                                |
| `admin.settings.read / read_secret / write`                                                             | 063        | ✓ (admin_settings)                                     |
| `admin.role_matrix.read / manage` (`PERM_READ`/`PERM_MANAGE` in admin_roles)                            | 064        | ✓ (admin_roles)                                        |
| `admin.member.read / create`                                                                            | 066        | ✓ (admin_members)                                      |
| `admin.subscription.read / comp / extend / billing_cycle.override`                                      | 067        | ✓ (admin_subscriptions)                                |
| `admin.order.read / create / void / refund / export`                                                    | 068        | ✓ (admin_orders)                                       |
| `admin.dsar.read / export / erase.request / erase.approve`                                              | 069        | ✓ (admin_dsar)                                         |

Net: **17 distinct permissions** are defined, granted to roles, but
never enforced by any handler. They become live the moment Phase 3.1
lands.

---

## Phase 4 — Stripe checkout end-to-end testing (P0/P1, ~1 week)

### 4a. Local setup verification

| Item                                                                                                                         | Source of truth                                                                      | Verified?                                        |
| ---------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------ | ------------------------------------------------ |
| `STRIPE_SECRET_KEY=sk_test_…` in `backend/.env` and root `.env`                                                              | `docs/stripe-local-testing.md:42`                                                    | Documented                                       |
| `STRIPE_WEBHOOK_SECRET=whsec_…` in `backend/.env`                                                                            | same                                                                                 | Documented; rotated each `stripe listen` restart |
| `pnpm stripe:listen` script in `package.json`                                                                                | Should map to `stripe listen --forward-to http://127.0.0.1:3001/api/webhooks/stripe` | Confirmed in docs                                |
| Pricing plans seeded with `stripe_price_id` (optional) **or** valid `amount_cents`+`stripe_product_id` for `price_data` mode | `pricing_plans` table; admin UI under `/admin/subscriptions/plans`                   | Schema present                                   |

**Action items in this section:**

1. Add a `pnpm stripe:fixtures` script that seeds two test plans
   (`monthly`, `annual`) into `pricing_plans` with `is_active=true` and
   either Stripe-Dashboard-created Prices or
   `(amount_cents, currency, interval)` populated for `price_data` mode.
   Today this is a manual `psql` step.
2. Add a `pnpm stripe:doctor` script that:
   - GETs `/api/pricing/plans` and asserts ≥1 active plan.
   - Calls Stripe via `sk_test_…` to verify each plan resolves either
     to a real Price ID or has `stripe_product_id` set.
   - Pings `/api/webhooks/stripe` with a deliberately-bad signature and
     asserts a 401.

### 4b. Test card matrix

Use the official Stripe test cards (https://docs.stripe.com/testing#cards).
For each, document the user flow, the webhook events fired by Stripe,
the DB rows expected on our side, and the email expected from Resend.

| Scenario                        | Card (PAN)                              | Expected Stripe events                                                            | Expected DB state                                                                                                              | Expected email                            |
| ------------------------------- | --------------------------------------- | --------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------ | ----------------------------------------- |
| Success — credit                | `4242 4242 4242 4242`                   | `checkout.session.completed`, `customer.subscription.created`, `invoice.paid`     | `subscriptions` row `status='active'`, `pricing_plan_id` set, `users.stripe_customer_id` set, `audit_log` "checkout completed" | `subscription.confirmed`                  |
| Success — debit                 | `4000 0566 5566 5556`                   | same                                                                              | same                                                                                                                           | same                                      |
| 3DS required                    | `4000 0027 6000 3184`                   | `checkout.session.completed` (after 3DS approval), then subscription/invoice arms | same as success                                                                                                                | same                                      |
| 3DS declined                    | `4000 0082 6000 3178`                   | `checkout.session.expired`                                                        | no subscription row created                                                                                                    | none                                      |
| Insufficient funds              | `4000 0000 0000 9995`                   | `checkout.session.expired` (initial) or `invoice.payment_failed` (renewal)        | row stays `incomplete` then deleted, or moves to `past_due`                                                                    | `subscription.payment_failed` (Phase 1.1) |
| Stolen card                     | `4000 0000 0000 9979`                   | `checkout.session.expired`                                                        | none                                                                                                                           | none                                      |
| Disputed (chargeback)           | `4000 0000 0000 0259`                   | `charge.dispute.created`                                                          | `subscriptions` should freeze pending review (new column?)                                                                     | `subscription.disputed`                   |
| Attaches but fails first charge | `4000 0000 0000 0341`                   | `customer.subscription.created`, then `invoice.payment_failed` immediately        | row created `status='past_due'`                                                                                                | dunning                                   |
| Trial then card declines        | `4000 0000 0000 0341` with trial period | `customer.subscription.trial_will_end` (3d before), then `invoice.payment_failed` | trial→active→past_due→canceled cascade                                                                                         | trial-ending + payment_failed             |
| 100% off coupon                 | any                                     | `checkout.session.completed`, `customer.subscription.created` (no invoice)        | as success but `coupons_usages` += 1                                                                                           | `subscription.confirmed`                  |

### 4c. Webhook flows to test

Each row should land in `backend/tests/stripe_webhooks.rs` (new file).
Use the same forged-but-valid signature pattern from
`webhooks.rs:464` (`make_signature`).

1. **New subscription (success):** post a synthetic
   `checkout.session.completed` for a known user_id; assert the
   `subscriptions` row exists and an `audit_log` row is written.
2. **3DS challenge:** synthetic `payment_intent.requires_action` →
   wait → `checkout.session.completed`. Assert no double-write.
3. **Failed payment / dunning:** post
   `invoice.payment_failed`; assert `subscriptions.status = 'past_due'`
   and a notification is enqueued.
4. **Subscription canceled at period end:** post
   `customer.subscription.updated` with `cancel_at_period_end=true`;
   assert local mirror reflects.
5. **Subscription canceled immediately:** post
   `customer.subscription.deleted`; assert `status='canceled'`
   and email enqueued (already in code today).
6. **Upgrade with proration:** post
   `customer.subscription.updated` carrying new `items.data[0].price`;
   assert `pricing_plan_id` moves to the new catalog row (uses
   `metadata.swings_pricing_plan_id` per
   `webhooks.rs:223`).
7. **Refund issued:** post `charge.refunded` with the refunded amount;
   assert `orders.refund_amount_cents` updated and audit row written.
8. **Invoice paid (renewal):** post `invoice.paid` for the second cycle;
   assert nothing changes structurally but a `subscription.renewed`
   audit row lands.
9. **Customer portal entry:** call `POST
/api/member/billing-portal` (`backend/src/handlers/member.rs:150`);
   assert returned URL is a `https://billing.stripe.com/...` and the
   call records an audit row.
10. **Webhook signature failure (forged event):** post payload with bad
    signature; assert 401 and **no DB write**.
11. **Idempotency-Key collision:** post the same event id twice;
    assert second call returns 200 with no DB delta (already exercised
    by `processed_webhook_events` table — extend test to verify
    explicitly).
12. **Pricing plan rollout (existing subs migration to new prices):**
    after `PUT /api/admin/pricing/plans/{id}` with
    `stripe_rollout.push_to_stripe_subscriptions=true`, assert each
    targeted Stripe sub gets a `customer.subscription.updated` event,
    and the subsequent webhook lands `pricing_plan_id` correctly.

### 4d. Test runner / harness

**Recommended:** extend `backend/tests/` with `stripe_webhooks.rs`
that uses the `TestApp` harness in `backend/tests/support/` and the
`make_signature` helper from `webhooks.rs:464`. Keep this offline
(no live Stripe API). Pattern after `tests/admin_idempotency.rs`
which already exercises the Idempotency middleware end-to-end.

**Optional layer:** add a Playwright spec in `e2e/checkout/` that
drives the live Stripe test-mode hosted Checkout (using `4242…`)
and waits for the webhook reconciliation by polling
`/api/admin/subscriptions?email=…` until `status='active'`. Run
nightly only — daytime CI cannot tolerate the latency.

### 4e. Manual QA checklist (printable)

Operator-facing; suitable for a release-day go-live drill.

1. `pnpm dev:all` and `pnpm stripe:listen` both running. Paste the
   freshly minted `whsec_…` into `backend/.env`. Restart the API.
2. `pnpm stripe:doctor` (Phase 4a item 2) — green.
3. Browse to `http://localhost:5173/pricing`. Click **Subscribe** on the
   monthly plan.
4. On the Stripe Checkout page enter `4242 4242 4242 4242` / future
   expiry / `123` CVC / any zip. Submit.
5. Land on the SvelteKit success page.
6. Visit `/admin/subscriptions` — the new sub should appear with
   `status=active`.
7. Visit `/admin/orders` — invoice for the first cycle should appear.
8. Visit `/admin/audit` — at least one `subscription.upserted` action
   row attributed to the webhook actor.
9. In the Stripe Dashboard (test mode), refund the invoice.
10. Inside ~30 s, `/admin/orders` should show the refund (Phase 1.1
    dependency — flag if missing).
11. Cancel the subscription from the Stripe Dashboard. `/admin/subscriptions`
    should flip to `canceled`.
12. Repeat steps 3-7 with `4000 0000 0000 9995` (insufficient funds)
    and verify the failure path.
13. Repeat with `4000 0027 6000 3184` (3DS) and complete the challenge.
14. Use the `100off` test coupon (configure under `/admin/coupons`) and
    verify the price drops to $0 on the Stripe Checkout page; confirm
    success path completes without an `invoice.paid` (Stripe skips
    it when amount is 0).
15. From `/admin/subscriptions/plans`, edit the monthly plan's
    `amount_cents` and toggle "also update existing Stripe subscriptions".
    Send the request with an `Idempotency-Key` header (UUIDv4). Expect a
    `pricing_plan.stripe_rollout` row in `admin_actions` and a Stripe
    `customer.subscription.updated` event back per existing sub.

---

## Phase 5 — Operational readiness (P2, ongoing)

### 5.1 Observability gaps

- **Missing alerts:** `stripe_webhook_signature_rejected_total` —
  alert on > 1/min sustained 5 m. Today the handler logs but no
  Prometheus counter is incremented. Add to
  `backend/src/observability/metrics.rs` and wire in
  `webhooks.rs:69`.
- **Missing alert:** `stripe_webhook_unhandled_event_total{type=…}`
  to surface event types Stripe is sending that we silently `_=>{}` on
  (`webhooks.rs:129`). High-cardinality on `type` label is acceptable
  given Stripe's finite event taxonomy.
- **Missing runbook entry:** `Phase 1.1` events need entries in
  `docs/RUNBOOK.md` — invoice payment failed, refund issued, dispute
  created. Each should reference the `audit_log` query to triage.
- **Worker liveness:** every worker emits `*_last_success_unixtime`
  per AGENTS.md §7; add an alert
  `WorkerStalledMoreThan2x{worker=…}` set to 2× the configured
  cadence. Today only some workers have this alert in
  `admin-alerts.rules.yml`.

### 5.2 Rate limiting

- `RATE_LIMIT_BACKEND` — confirm prod is `postgres` (the default
  `inprocess` does not survive a multi-replica deploy). Add the
  `Config::assert_production_ready` check.
- The webhook layer is 500/min/source. Stripe retry storms can spike
  briefly to 200/m; the 500 ceiling is fine but the per-source key
  uses the source IP — Stripe sends from a CIDR pool, so de-dup by
  `(source_ip, event_id_prefix)` may need adjustment if storms
  accumulate.
- Member POSTs (`/api/member/*`) are not rate-limited at all today.
  Add a per-user bucket modelled on the admin mutation limiter
  (`middleware/rate_limit.rs::admin_mutation_rate_limit`).

### 5.3 Idempotency

- The middleware is mounted on `/api/admin/subscriptions`,
  `/api/admin/orders`, `/api/admin/dsar` (see `main.rs:506`–`531`).
  Land it on the remaining admin mutation trees per `AUDIT.md` §3.2 #3:
  `admin_blog`, `admin_courses`, `admin_pricing` (already required for
  rollout but not enforced for plan create/delete), `admin_coupons`,
  `admin_popups`, `admin_products`, `admin_consent`, `admin_forms`,
  `admin_outbox`, `admin_notifications`. Either wire or document why
  not.
- GC worker (`services::idempotency_gc`) is running. Confirm
  Prometheus shows `idempotency_gc_last_success_unixtime` advancing.

### 5.4 Background workers

| Worker                                          | Status                           | `*_last_success_unixtime`                   |
| ----------------------------------------------- | -------------------------------- | ------------------------------------------- |
| Outbox dispatcher                               | ✅ running, 4 concurrent         | needs metric? confirm in `events/worker.rs` |
| Audit retention                                 | ✅ running, 1h cadence           | confirmed                                   |
| DSAR worker                                     | ✅ running, 30s cadence          | confirmed                                   |
| DSAR artefact sweep                             | ✅ running, 1h cadence           | confirmed                                   |
| Idempotency GC                                  | ✅ running, 5m cadence           | confirmed                                   |
| **Missing:** consent integrity anchor scheduler | ✗ TODO `consent/integrity.rs:97` | n/a                                         |
| **Missing:** dunning sweeper (P1.1 dependency)  | ✗                                | n/a                                         |

### 5.5 PII / log hygiene

- `webhooks.rs:311` already scrubs `customer_email`. Confirm the
  remaining log lines in `webhooks.rs::handle_*` carry only opaque
  Stripe IDs.
- `auth.rs::issue_email_verification` logs token hash, not raw token —
  good. Same for `forgot_password` (per `AUDIT.md` F1).

---

## Phase 6 — Polish (P3)

### 6.1 Documentation

- New runbook entries (Phase 1.1, 5.1).
- Wiring docs:
  - `docs/wiring/STRIPE-WEBHOOKS-WIRING.md` covering
    `webhooks.rs::stripe_webhook` + signature verification + retry
    semantics.
  - `docs/wiring/RBAC-WIRING.md` codifying the
    `policy.require + audit_admin` pattern (Phase 3 pre-req).
  - `docs/wiring/MFA-WIRING.md` once Phase 1.4 lands.
- ADRs (`docs/ADR/`): auth model (BFF cookie vs JWT), event outbox,
  CSP, media backend (R2 vs local), Stripe pricing model selection.
- Migrate `docs/RUNBOOK.md` → `docs/runbooks/*.md` per
  `AUDIT.md` §3.4.

### 6.2 OpenAPI / TS SDK

- The snapshot `backend/tests/snapshots/openapi.json` carries 144
  paths today; the actual route table is materially larger (counted
  from `main.rs` + each `*_router` ~ 230). Either generate from the
  router or expand the curated list in `backend/src/openapi.rs`.
- After Phase 3.1 lands, regenerate:
  `cd backend && cargo test openapi_snapshot -- --nocapture`
  (will diff and fail if drift); then `pnpm gen:types` to refresh
  `src/lib/api/schema.d.ts`.

### 6.3 Test coverage targets

| Layer                  | Today                 | Target                                  |
| ---------------------- | --------------------- | --------------------------------------- |
| Backend unit           | ~480                  | 80% line coverage                       |
| Backend integration    | ~202                  | every admin mutator + every webhook arm |
| Frontend unit (Vitest) | 67                    | 70% line coverage on `$lib/`            |
| Playwright smoke       | 4 specs (CI gate)     | unchanged                               |
| Playwright admin       | 2 specs (manual gate) | nightly job in CI                       |
| Stripe checkout E2E    | ✗                     | nightly job (Phase 4d)                  |

Specific untested handlers (no entry under `backend/tests/`):
`blog`, `courses`, `coupons`, `popups`, `products`, `pricing`,
`forms`, `admin_consent`, `notifications`, `outbox` (5 outbox tests
exist but cover the worker, not the handler), `analytics`,
`csp_report`. Each should get at least one happy-path + one
unauthorized + one validation-error integration test once Phase 3.1
adds the policy gates.

---

## Risk register

| #   | Risk                                                                                                                                                                   | Severity | Likelihood                                        | Mitigation                           |
| --- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------- | ------------------------------------------------- | ------------------------------------ |
| R1  | Stripe webhook only handles 3 of ~25 relevant event types; refunds, dunning, disputes silently diverge                                                                 | High     | Will happen on first dispute                      | Phase 1.1                            |
| R2  | Legacy admin handlers gate only on AdminUser, not the FDN-07 perm; support role can mutate blog/courses/coupons                                                        | High     | Already true in prod                              | Phase 1.3 / Phase 3                  |
| R3  | Admin JWT in `localStorage`; any reflected XSS = full takeover                                                                                                         | High     | Mitigated by F17 sanitisation but structural flaw | Phase 1.2                            |
| R4  | No MFA on admin / support; sole factor is password + email recovery                                                                                                    | High     | Industry baseline                                 | Phase 1.4                            |
| R5  | No automated Stripe E2E test; regressions discovered only by users                                                                                                     | Medium   | Inevitable on next refactor                       | Phase 4                              |
| R6  | DSAR erasure is dual-control but admin_consent admin write surface has no `policy.require`; an over-scoped support token could currently mutate banner/category config | Medium   | Phase 3.1 closes                                  | Phase 3                              |
| R7  | Idempotency middleware is opt-in per nest; missing on blog/courses/coupons/popups/products → admin double-clicks could double-create                                   | Medium   | Operator-facing                                   | Phase 5.3                            |
| R8  | `OUTBOX_WORKERS=4` × `PGPOOL_MAX=10` plus 4 background workers leaves <6 connections for HTTP under burst                                                              | Low      | Spikes only                                       | Document in `docs/INFRASTRUCTURE.md` |
| R9  | Notifications + outbox + consent admin pages missing → operators must run `psql` for triage                                                                            | Low      | Daily operational friction                        | Phase 2.1 / 2.2 / 2.3                |
| R10 | OpenAPI snapshot under-counts paths (~144 of ~230) → frontend SDK incomplete; some endpoints called by handwritten clients                                             | Low      | Drift creeps in                                   | Phase 6.2                            |

---

## Suggested execution order

| Week | Phase items in flight                                                                                                     |
| ---- | ------------------------------------------------------------------------------------------------------------------------- |
| 1    | **Phase 1.1** (Stripe webhooks) + **Phase 1.3** (RBAC wiring on legacy) in parallel; **Phase 4d** harness scaffolding     |
| 2    | **Phase 1.2** (BFF cookie) lands; **Phase 4** test matrix + manual QA drill                                               |
| 3    | **Phase 2.1–2.3** (notifications, outbox, consent UIs); **Phase 5.1** alerts + runbook                                    |
| 4    | **Phase 1.4** (MFA TOTP); **Phase 2.4–2.6** (forms, watchlists, products UIs); **Phase 5.2** rate-limit hardening         |
| 5    | **Phase 2.7–2.10** (member detail, cart inspector, sub detail, impersonation banner); **Phase 5.3** idempotency expansion |
| 6    | **Phase 6** polish — docs, OpenAPI regen, SDK regen, missing handler tests                                                |

**Critical path:** R1 (webhooks) and R2 (RBAC) gate everything because
they affect production correctness; both must land before BFF cookies
or MFA shift the security perimeter. The BFF cookie work (1.2) and
RBAC wiring (1.3) are independent and can be done in parallel by two
engineers.

---

## Appendix A — Migration summary

Read `head -3` of every migration to extract the FDN- / ADM- /
EC- / FORM- / CONSENT- / POP- / AUTH- prefix that names the
subsystem.

| Version | File                                    | Subsystem  | One-liner                                                                                                                                                   |
| ------- | --------------------------------------- | ---------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 001     | `001_initial.sql`                       | FDN        | Bootstrap: `users`, `subscriptions`, `refresh_tokens`, watchlists, alerts; enums `user_role`, `subscription_plan`, `subscription_status`, `trade_direction` |
| 002     | `002_blog.sql`                          | BLOG       | Blog posts + media + post_status enum                                                                                                                       |
| 003     | `003_password_resets.sql`               | AUTH       | password_reset_tokens table                                                                                                                                 |
| 004     | `004_media_title.sql`                   | BLOG       | media.title column                                                                                                                                          |
| 005     | `005_user_author_profile.sql`           | AUTH       | user bio/position/website columns                                                                                                                           |
| 006     | `006_post_format.sql`                   | BLOG       | post.format column                                                                                                                                          |
| 007     | `007_post_meta.sql`                     | BLOG       | post_meta key/value sidecar                                                                                                                                 |
| 008     | `008_media_focal.sql`                   | BLOG       | media focal_x/focal_y                                                                                                                                       |
| 009     | `009_analytics.sql`                     | ANL        | analytics_sessions + analytics_events                                                                                                                       |
| 010     | `010_normalize_user_emails.sql`         | AUTH       | lowercase email canonicalisation                                                                                                                            |
| 011     | `011_courses.sql`                       | EDU        | courses + modules + lessons + progress                                                                                                                      |
| 012     | `012_pricing_plans.sql`                 | EC         | pricing_plans + change-log                                                                                                                                  |
| 013     | `013_coupons.sql`                       | EC         | coupons v1 + usages                                                                                                                                         |
| 014     | `014_analytics_enhanced.sql`            | ANL        | sales_events for revenue funnel                                                                                                                             |
| 015     | `015_popups.sql`                        | POP        | popups v1 + submissions + events                                                                                                                            |
| 016     | `016_blog_trash_meta.sql`               | BLOG       | pre_trash_status / trashed_at columns                                                                                                                       |
| 017     | `017_webhook_idempotency.sql`           | FDN        | processed_webhook_events                                                                                                                                    |
| 018     | `018_refresh_token_families.sql`        | AUTH       | rotation + reuse detection                                                                                                                                  |
| 019     | `019_outbox.sql`                        | FDN-04     | transactional outbox table                                                                                                                                  |
| 020     | `020_notifications.sql`                 | FDN-05     | templates + deliveries + suppression + preferences                                                                                                          |
| 021     | `021_rbac.sql`                          | FDN-07     | RBAC catalogue (perms, role_perms) + author/support roles                                                                                                   |
| 022     | `022_rate_limits.sql`                   | FDN-08     | Postgres-backed sliding-window rate limit                                                                                                                   |
| 023     | `023_webhook_source.sql`                | FDN-09     | webhook idempotency multi-source key                                                                                                                        |
| 024     | `024_consent.sql`                       | CONSENT-01 | banners / categories / services / policies                                                                                                                  |
| 025     | `025_consent_log.sql`                   | CONSENT-03 | consent_records + dsar_requests                                                                                                                             |
| 026     | `026_consent_regions.sql`               | CONSENT-05 | regional banner config seeds                                                                                                                                |
| 027     | `027_forms.sql`                         | FORM-01    | form schema + fields + submissions                                                                                                                          |
| 028     | `028_consent_integrity.sql`             | CONSENT-07 | tamper-evidence anchor table                                                                                                                                |
| 030     | `030_products.sql`                      | EC-01      | digital-goods product model                                                                                                                                 |
| 031     | `031_cart.sql`                          | EC-03      | cart by user_id or anonymous_id                                                                                                                             |
| 032     | `032_form_partials.sql`                 | FORM-04    | save-and-resume                                                                                                                                             |
| 033     | `033_form_uploads.sql`                  | FORM-05    | chunked file uploads                                                                                                                                        |
| 034     | `034_form_payments.sql`                 | FORM-08    | Stripe payment intents from form submissions                                                                                                                |
| 035     | `035_orders.sql`                        | EC-05      | orders + refunds + state-transition log                                                                                                                     |
| 036     | `036_tax.sql`                           | EC-08      | manual tax rates + exempt customers                                                                                                                         |
| 037     | `037_user_downloads.sql`                | EC-07      | digital delivery grants                                                                                                                                     |
| 038     | `038_product_search.sql`                | EC-02      | tsvector + faceting + nested categories                                                                                                                     |
| 039     | `039_addresses.sql`                     | EC-04      | address book                                                                                                                                                |
| 041     | `041_subscriptions_v2.sql`              | EC-09      | pause/resume, prorations, dunning, switching                                                                                                                |
| 042     | `042_memberships.sql`                   | EC-10      | membership tiers                                                                                                                                            |
| 043     | `043_coupons_refactor.sql`              | EC-11      | Money-native, BOGO-aware coupons                                                                                                                            |
| 050     | `050_popups_targeting.sql`              | POP-01     | expanded triggers/targeting                                                                                                                                 |
| 051     | `051_popup_variants.sql`                | POP-02     | A/B variants                                                                                                                                                |
| 052     | `052_popup_templates.sql`               | POP-03     | template library                                                                                                                                            |
| 053     | `053_popup_visitor_state.sql`           | POP-05     | per-visitor frequency cap                                                                                                                                   |
| 054     | `054_popup_attributions.sql`            | POP-06     | revenue attribution                                                                                                                                         |
| 055     | `055_admin_actions.sql`                 | ADM-01     | admin/support audit log                                                                                                                                     |
| 056     | `056_user_lifecycle.sql`                | ADM-02     | user lifecycle columns + auth telemetry                                                                                                                     |
| 057     | `057_subscription_status_paused.sql`    | ADM-03     | enum value `paused` for subscription_status                                                                                                                 |
| 058     | `058_admin_lifecycle_perms.sql`         | ADM-04     | perm catalogue extension for member lifecycle                                                                                                               |
| 059     | `059_admin_ip_allowlist.sql`            | ADM-06     | IP allowlist table + perms                                                                                                                                  |
| 060     | `060_impersonation_sessions.sql`        | ADM-07     | impersonation server-side state                                                                                                                             |
| 061     | `061_impersonation_notification.sql`    | ADM-07-α   | impersonation notification template                                                                                                                         |
| 062     | `062_app_settings.sql`                  | ADM-08     | typed settings catalogue                                                                                                                                    |
| 063     | `063_settings_perms.sql`                | ADM-08     | perms for settings                                                                                                                                          |
| 064     | `064_role_matrix_perms.sql`             | ADM-09     | perms for role/perm matrix admin                                                                                                                            |
| 065     | `065_users_search.sql`                  | ADM-10     | indexes for admin user search                                                                                                                               |
| 066     | `066_member_admin_perms.sql`            | ADM-10     | perms for member admin                                                                                                                                      |
| 067     | `067_subscription_admin.sql`            | ADM-11     | manual subscription ops admin surface                                                                                                                       |
| 068     | `068_orders_admin_perms.sql`            | ADM-12     | perms for orders admin                                                                                                                                      |
| 069     | `069_dsar_admin.sql`                    | ADM-13     | admin-initiated DSAR + dual-control erasure                                                                                                                 |
| 070     | `070_admin_actions_fts.sql`             | ADM-14     | FTS over admin_actions                                                                                                                                      |
| 071     | `071_idempotency_keys.sql`              | ADM-15     | idempotency cache table                                                                                                                                     |
| 072     | `072_audit_retention.sql`               | ADM-16     | audit retention configuration                                                                                                                               |
| 073     | `073_dsar_export_async.sql`             | ADM-17     | async DSAR export pipeline                                                                                                                                  |
| 074     | `074_idempotency_gc_settings.sql`       | ADM-20     | tunable settings for idempotency GC                                                                                                                         |
| 075     | `075_email_verification_template.sql`   | AUTH-01    | email verification template                                                                                                                                 |
| 076     | `076_subscriptions_pricing_plan_id.sql` | EC-09b     | link Stripe subs to catalog plan                                                                                                                            |

Gaps `029, 040, 044–049` are intentional reservations
(see AGENTS.md §5).

---

## Appendix B — Permission matrix

The full catalogue is in `021_rbac.sql` plus the
`058 / 063 / 064 / 066 / 067 / 068 / 069` perm-extension migrations.
Below is the production matrix (enforced today) cross-referenced with
the **handler that calls `policy.require()` for it**.

| Permission key                                                                                                      | member | author | support    | admin | Enforced by handler                                     |
| ------------------------------------------------------------------------------------------------------------------- | ------ | ------ | ---------- | ----- | ------------------------------------------------------- |
| `user.self.read` / `update`                                                                                         | ✓      | ✓      | ✓          | ✓     | implicit (member.rs uses AuthUser)                      |
| `user.other.read`                                                                                                   | —      | —      | ✓          | ✓     | ✗ (admin.rs::list_members uses AdminUser)               |
| `user.other.update` / `delete`                                                                                      | —      | —      | —          | ✓     | ✗                                                       |
| `user.suspend` / `reactivate` / `ban` / `force_password_reset` / `email.verify` / `session.read` / `session.revoke` | —      | —      | varies     | ✓     | **admin_security.rs**                                   |
| `blog.post.create` / `update_own` / `delete_own` / `publish`                                                        | —      | ✓      | —          | ✓     | ✗ — Phase 3.1                                           |
| `blog.post.update_any` / `delete_any`                                                                               | —      | —      | —          | ✓     | ✗ — Phase 3.1                                           |
| `blog.media.upload` / `delete_own`                                                                                  | —      | ✓      | —          | ✓     | ✗                                                       |
| `blog.media.delete_any`                                                                                             | —      | —      | —          | ✓     | ✗                                                       |
| `blog.category.manage`                                                                                              | —      | —      | —          | ✓     | ✗                                                       |
| `course.read_enrolled` / `enroll.self` / `progress.read_self`                                                       | ✓      | ✓      | ✓          | ✓     | implicit                                                |
| `course.read_any` / `progress.read_any`                                                                             | —      | —      | ✓          | ✓     | ✗ — Phase 3.1                                           |
| `course.manage` / `enroll.other`                                                                                    | —      | —      | —          | ✓     | ✗                                                       |
| `coupon.apply`                                                                                                      | ✓      | ✓      | ✓          | ✓     | implicit                                                |
| `coupon.read_any`                                                                                                   | —      | —      | ✓          | ✓     | ✗                                                       |
| `coupon.manage`                                                                                                     | —      | —      | —          | ✓     | ✗                                                       |
| `order.mine.read` / `invoice.read_self`                                                                             | ✓      | ✓      | ✓          | ✓     | implicit                                                |
| `order.any.read` / `invoice.read_any`                                                                               | —      | —      | ✓          | ✓     | **admin_orders.rs** (PERM_READ alias)                   |
| `order.refund.create`                                                                                               | —      | —      | ✓ (capped) | ✓     | **admin_orders.rs** (PERM_REFUND)                       |
| `order.any.update`                                                                                                  | —      | —      | —          | ✓     | **admin_orders.rs** (PERM_VOID)                         |
| `subscription.mine.read` / `manage`                                                                                 | ✓      | ✓      | ✓          | ✓     | implicit                                                |
| `subscription.any.read`                                                                                             | —      | —      | ✓          | ✓     | **admin_subscriptions.rs** (PERM_READ)                  |
| `subscription.any.manage`                                                                                           | —      | —      | —          | ✓     | **admin_subscriptions.rs** (PERM_COMP / EXTEND / CYCLE) |
| `subscription.plan.manage`                                                                                          | —      | —      | —          | ✓     | ✗ — Phase 3.1 (pricing.rs)                              |
| `popup.submit` / `event`                                                                                            | ✓      | ✓      | ✓          | ✓     | implicit                                                |
| `popup.manage`                                                                                                      | —      | —      | —          | ✓     | ✗                                                       |
| `popup.read_analytics`                                                                                              | —      | —      | —          | ✓     | ✗                                                       |
| `form.submit`                                                                                                       | ✓      | ✓      | ✓          | ✓     | implicit                                                |
| `form.manage`                                                                                                       | —      | —      | —          | ✓     | ✗                                                       |
| `form.submission.read_any`                                                                                          | —      | —      | ✓          | ✓     | ✗                                                       |
| `form.submission.delete_any`                                                                                        | —      | —      | —          | ✓     | ✗                                                       |
| `consent.record` / `log.read_self` / `dsar.submit`                                                                  | ✓      | ✓      | ✓          | ✓     | implicit                                                |
| `consent.log.read_any`                                                                                              | —      | —      | ✓          | ✓     | ✗ — admin_consent.rs                                    |
| `dsar.fulfill`                                                                                                      | —      | —      | ✓          | ✓     | **admin_dsar.rs**                                       |
| `notification.mine.read` / `mark_read`                                                                              | ✓      | ✓      | ✓          | ✓     | implicit                                                |
| `notification.broadcast.create`                                                                                     | —      | —      | —          | ✓     | ✗                                                       |
| `notification.template.manage`                                                                                      | —      | —      | —          | ✓     | ✗                                                       |
| `admin.dashboard.read`                                                                                              | —      | —      | ✓          | ✓     | **PrivilegedUser extractor**                            |
| `admin.audit.read`                                                                                                  | —      | —      | ✓          | ✓     | **admin_audit.rs**, **admin_security.rs**               |
| `admin.role.manage` / `permission.manage`                                                                           | —      | —      | —          | ✓     | **admin_roles.rs**                                      |
| `admin.outbox.read` / `retry`                                                                                       | —      | —      | ✓          | ✓     | ✗ — outbox.rs                                           |
| `admin.ip_allowlist.read` / `manage`                                                                                | —      | —      | —          | ✓     | **admin_ip_allowlist.rs**                               |
| `admin.impersonation.create` (ADM-07)                                                                               | —      | —      | —          | ✓     | **admin_impersonation.rs**                              |
| `admin.settings.read` / `read_secret` / `write`                                                                     | —      | —      | partial    | ✓     | **admin_settings.rs**                                   |
| `admin.role_matrix.read` / `manage`                                                                                 | —      | —      | —          | ✓     | **admin_roles.rs**                                      |
| `admin.member.read` / `create`                                                                                      | —      | —      | ✓          | ✓     | **admin_members.rs**                                    |
| `admin.subscription.read` / `comp` / `extend` / `billing_cycle.override`                                            | —      | —      | varies     | ✓     | **admin_subscriptions.rs**                              |
| `admin.order.read` / `create` / `void` / `refund` / `export`                                                        | —      | —      | varies     | ✓     | **admin_orders.rs**                                     |
| `admin.dsar.read` / `export` / `erase.request` / `erase.approve`                                                    | —      | —      | varies     | ✓     | **admin_dsar.rs**                                       |

**Implicit** = the route's `AuthUser` / `AdminUser` extractor proves
the role string but the per-action permission is not consulted.
Phase 3 promotes every "✗" entry above to a real
`policy.require()` call.

---

## Audit Phase 7 — deferred items

### 7.6 — CI install of cargo tools (deferred)

`backend/deny.toml` is now in the repo (Phase 7.4) and `cargo deny check
licenses` runs clean locally. We have NOT wired `cargo deny`,
`cargo audit`, or `cargo geiger` into the GitHub Actions workflow yet —
that needs `.github/workflows/*.yml` edits which are out of scope for the
dead-code / deps / config sweep. To pick this up:

1. Add a `licenses-and-advisories` job to the existing Rust workflow
   that runs `cargo install --locked cargo-deny` (cached) and then
   `cargo deny --manifest-path backend/Cargo.toml check licenses advisories bans`.
2. Run on push + PR to `main`; non-blocking warning while the
   advisory ignore list is empty.
3. Optional: add `cargo audit` as a redundant safety net — `deny check
advisories` reads the same RustSec database so it is mostly
   duplicative.

### 7.8 — `rand 0.8 → 0.9` bump (deferred)

`backend/Cargo.toml` keeps `rand = "0.8"`. The 0.9 bump renames
`thread_rng() → rng()` and `Rng::gen() → Rng::random()`. The current
codebase has 13+ call sites across 7 files (`commerce/downloads.rs`,
`consent/records.rs`, `popups/gamified.rs`, `events/outbox.rs`,
`handlers/{webhooks, coupons, forms}.rs`, `notifications/unsubscribe.rs`)
plus the `gen_range` / `gen_bool` rename (also breaking). The dep-graph
audit shows the 0.8 / 0.9 triplication is benign (only ~50 KB extra
binary), so the bump is deferred until a quiet window where the rename
can be done in one shot.

---

### 8.12 — `cargo-llvm-cov` install + CI coverage report (deferred)

Phase 8 closes the test-coverage gaps surfaced by the audit (browser
specs, e2e fixture cookie carry, UI primitive specs, store specs,
client-error-shape coverage, observability wiring, pure-helper unit
tests for `services::storage` + `services::pricing_rollout`). What did
NOT land was the line-coverage tooling itself:

1. Install `cargo-llvm-cov` on the developer + CI toolchain
   (`cargo install cargo-llvm-cov --locked` plus the
   `llvm-tools-preview` rustup component).
2. Add a `cargo llvm-cov --workspace --html` step to `ci:backend`
   that uploads the report as a build artifact.
3. Set per-crate floors (target >= 80% line coverage on the backend,
   > = 70% on the frontend) and gate the CI job on
   > `--fail-under-lines`.

Reason for deferral: `cargo-llvm-cov` requires a one-off rustup
component install in the CI image, which is a separate Render /
GitHub-Actions config change outside the repo's `Cargo.toml`. Tracked
to a follow-up that bundles the toolchain bump with `cargo-audit`,
`cargo-outdated`, and `cargo-udeps` (Phase 7.6) in a single CI image
refresh.

---

_End of report._
