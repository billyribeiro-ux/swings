# Audit Response Report

**Date:** 2026-04-19
**Repository:** `swings`
**Scope:** Validation of an external 16‑point "what's missing" audit against the actual codebase.

---

## Executive Summary

The audit lists 16 categories of supposedly missing functionality. After auditing the repository, **the majority of items are already implemented**, often at production quality. Seven items are genuine gaps worth tracking.

| Status | Count | Categories |
|---|---|---|
| Already implemented | 11 | 1, 2 (most), 4, 5, 6, 7 (most), 8 (most), 9 (most), 10, 12, 13 (partial), 14, 15, 16 |
| Genuine gap or thin | 7 | 3, 2 (verification/magic links), 7 (CAPTCHA), 8 (impersonation), 9 (backups/DR), 11 (caching), 13 (a11y) |

---

## 1. Already Implemented (Audit is Inaccurate)

### 1.1 Authorization beyond tier checks
Real RBAC with a hot‑reloadable policy cache.

- `backend/src/authz.rs`
- `backend/migrations/021_rbac.sql` — `role_permissions` catalogue
- `backend/src/lib.rs` — `AppState.policy: Arc<authz::Policy>` with `Policy::reload`
- `backend/tests/authz_matrix.rs` — server‑side enforcement tests

### 1.2 Password reset & session handling
- Password reset table: `backend/migrations/003_password_resets.sql`
- UI: `src/routes/admin/forgot-password/+page.svelte`, `src/routes/admin/reset-password/+page.svelte`
- Refresh token rotation with reuse detection: `backend/migrations/018_refresh_token_families.sql`
- E2E coverage: `e2e/auth/register-login.spec.ts`, `invalid-credentials.spec.ts`, `rate-limit.spec.ts`

### 1.3 File uploads / media storage
- `backend/src/services/storage.rs`
- `backend/src/forms/uploads.rs`
- `backend/migrations/033_form_uploads.sql`
- `AppState.media_backend` wired in `backend/src/lib.rs`

### 1.4 Email system
- Core: `backend/src/email.rs`
- Notifications service: `backend/src/notifications/` (`templates.rs`, `preferences.rs`, `suppression.rs`, `unsubscribe.rs`, `send.rs`)
- Multi‑channel: `notifications/channels/` (email, sms, push, slack, discord, in_app, webhook)
- Provider integration: `backend/src/notifications/webhooks/resend.rs`
- Tests: `backend/tests/notifications.rs`, `resend_provider.rs`, `resend_webhook.rs`

### 1.5 Observability
- `backend/src/observability/` — `correlation.rs`, `tracing_json.rs`, `metrics.rs`, `handler.rs`
- `backend/tests/observability.rs`
- Documented in `OBSERVABILITY-WIRING.md`

### 1.6 Rate limiting
- `backend/src/middleware/rate_limit.rs`
- `backend/migrations/022_rate_limits.sql`
- Distributed (Postgres) backend with in‑process governor fallback (`AppState.rate_limit`)
- E2E: `e2e/auth/rate-limit.spec.ts`

### 1.7 Admin dashboard
Extensive admin surface under `src/routes/admin/`:
`members`, `subscriptions`, `coupons`, `products`, `courses`, `popups`, `forms`, `analytics`, `consent`, `watchlists`, `dsar`, `settings`.

### 1.8 Migrations
50+ ordered SQL migrations in `backend/migrations/` covering RBAC, outbox, notifications, consent, forms, commerce, popups.

### 1.9 Webhook resilience
- Idempotency: `backend/migrations/017_webhook_idempotency.sql`
- Outbox pattern: `backend/src/events/{outbox,dispatcher,worker}.rs`, `migrations/019_outbox.sql`
- Source tagging for replay: `migrations/023_webhook_source.sql`
- Graceful drain via `AppState.outbox_shutdown`
- Tests: `backend/tests/outbox.rs`

### 1.10 Security hardening
- CSP report endpoint: `backend/src/handlers/csp_report.rs`
- Refresh token families, RBAC, distributed rate limiting
- Crate hardening: `#![forbid(unsafe_code)]` and `#![deny(warnings)]` in `backend/src/lib.rs`
- Trivy scan config (`.trivyignore`), `SECURITY.md`

### 1.11 Testing depth
- Backend integration: `authz_matrix`, `notifications`, `observability`, `outbox`, `openapi_snapshot`, `resend_provider`, `resend_webhook`
- Frontend unit: `src/lib/forms/validate.test.ts`, `src/hooks.server.test.ts`, `src/routes/page.svelte.spec.ts`
- E2E: `e2e/` with auth, popups, smoke suites

### 1.12 Billing
- `backend/src/commerce/`: `subscriptions.rs`, `memberships.rs`, `coupons.rs`, `tax.rs`, `orders.rs`, `checkout.rs`, `cart.rs`, `downloads.rs`, `reports.rs`
- Migrations: `036_tax.sql`, `041_subscriptions_v2.sql`, `041_coupons_refactor.sql`, `042_memberships.sql`, `040_products.sql`, `037_user_downloads.sql`

### 1.13 Legal / consent
- Public pages: `src/routes/terms/`, `src/routes/privacy/`, `src/routes/about/`
- Consent service: `backend/src/consent/` (records, integrity, dsar_export, regions)
- Migrations: `024_consent.sql`–`028_consent_integrity.sql`
- Admin UI: banner / policies / services + `dsar`
- Client module: `src/lib/consent/`

### 1.14 UX baseline
`src/lib/components/ui/` includes `Button`, `EmptyState`, `Skeleton`, `Toast`, `ConfirmDialog`, `Footer`, `Nav`, plus more in `landing/`, `forms/`, `admin/`, `editor/`, `popups/`, `traders/`, `charts/`, `consent/`, `shared/`.

---

## 2. Genuine Gaps

These are the items where the audit is correct.

### 2.1 Multi‑tenancy / organizations / teams
**Status:** Not present.
No `organizations`, `workspaces`, `org_id`, `tenant`, or invite flows in `backend/`.
**Impact:** Fine for single‑user SaaS; expensive to retrofit if B2B/team plans are ever needed.
**Recommendation:** Decide now whether multi‑tenant is on the roadmap. If yes, model `organizations` + `memberships` with `tenant_id` on every owned table before more data piles up.

### 2.2 Email verification + magic links
**Status:** Missing.
Grep for `verify_email`, `email_verification`, `magic_link` returns nothing in `backend/`. Password reset exists; account verification and passwordless login do not.
**Recommendation:** Slot into existing `notifications` + `password_resets` patterns. Reuse the token table shape, add `purpose` column, add verification gate in `handlers/auth.rs`.

### 2.3 CAPTCHA / bot protection
**Status:** Honeypot only.
`backend/src/forms/antispam.rs` has a honeypot, but no Turnstile/hCaptcha/reCAPTCHA integration on auth or public endpoints.
**Recommendation:** Add Cloudflare Turnstile (free, privacy‑friendly) on register/login/forgot‑password and any public form. Verify server‑side in `handlers/auth.rs` and `handlers/forms.rs`.

### 2.4 Backups & disaster recovery
**Status:** Undocumented.
Migrations are excellent, but no backup cadence, restore drill, or DR runbook in `deployment-guide.md` or `Infrastructure.md`.
**Recommendation:** Document Postgres PITR cadence, retention, and a quarterly restore drill. Add a `DR_RUNBOOK.md` with RTO/RPO targets.

### 2.5 Caching strategy
**Status:** No layer.
No `cache`, `revalidate`, `stale`, or SWR patterns in `backend/src`. No Redis/CDN cache layer evident.
**Recommendation:** Probably premature today. When needed, start with HTTP caching headers + `stale-while-revalidate` on read‑heavy endpoints (`pricing`, `catalog`, `blog`) before reaching for Redis.

### 2.6 Admin impersonation / safe support tools
**Status:** Likely missing.
Members/subscriptions admin exists, but no impersonation flow or "act as user" audit‑logged tool was found.
**Recommendation:** Add a signed, time‑boxed impersonation token; log every action to a dedicated `admin_actions` table. Gate behind a permission already modelled in `authz.rs`.

### 2.7 Accessibility review
**Status:** Components exist, no formal a11y harness.
No axe / pa11y / lighthouse‑a11y CI step.
**Recommendation:** Add `@axe-core/playwright` to existing E2E suite. Run on key flows (landing, register, login, dashboard, checkout).

---

## 3. Prioritized Backlog

| Priority | Item | Why |
|---|---|---|
| P0 | Email verification | Standard SaaS expectation; low effort given existing email pipeline |
| P0 | CAPTCHA on auth/public forms | Cheap insurance against credential stuffing and form spam |
| P1 | Backup & DR runbook | Documentation only, but critical for production confidence |
| P1 | Admin impersonation + audit log | Support efficiency, low risk if scoped properly |
| P2 | Multi‑tenant model | Only if roadmap requires teams/B2B |
| P2 | A11y CI step | Quick win; protects against regressions |
| P3 | Caching layer | Defer until traffic justifies |

---

## 4. Conclusion

The repository is significantly more mature than the audit portrays. Of the 16 raised concerns, **9 are fully addressed**, **2 are partially addressed**, and **5 are genuine gaps** (with multi‑tenancy and caching being optional depending on roadmap).

The two highest‑ROI items to action immediately are **email verification** and **CAPTCHA on auth**, both of which integrate cleanly with the existing `notifications` and `rate_limit` infrastructure.
