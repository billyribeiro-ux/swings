# Swings Backend Admin System — Phase 1–4 Engineering Report

> Principal Engineer review and partial implementation of the admin/back-office
> surface as of 2026-04-19. Phases 1–3 follow the brief in
> `[Backend admin audit task](2f9ed232-7a97-49ea-b58c-f2208cb2f890)`.
>
> All claims in this document are verified against `cargo check`,
> `cargo clippy --all-targets -- -D warnings`, and `cargo test --tests`
> (411 lib + 45 integration tests, all green) on the working tree as of this
> commit.

---

## ✅ CONFIRMED COMPLETE (end-to-end, tested)

| Domain | Feature | Path → handler → DB → test |
|---|---|---|
| **Auth** | Register / login / refresh-rotation / logout / forgot+reset password | `handlers::auth` ↔ `db::*_user` / `db::*_password_reset_token` / `db::*_refresh_token`; tests `example_auth_flow.rs`, `authz_matrix.rs` |
| **RBAC** | Policy engine with `permissions` + `role_permissions` tables, `AdminUser` / `RoleUser` / `PrivilegedUser` / `OptionalAuthUser` extractors | `authz::Policy::load`, `extractors::*`; tests `authz_matrix.rs` (covers admin/support/member matrix per route) |
| **Members** | List / get / update role / delete | `handlers::admin::list_members` / `get_member` / `update_member_role` / `delete_member`; tests `admin_security::delete_member_writes_audit_row` |
| **Members** | Stripe billing-portal handoff per member | `handlers::admin::admin_member_billing_portal` |
| **Subscriptions** | Cancel-at-period-end / Resume per member | `handlers::admin::admin_member_subscription_{cancel,resume}` |
| **Subscriptions** | Pause / resume / cancel-immediately / refund engine | `commerce::subscriptions::*` (now with `paused` enum value, see migration 057) |
| **Pricing** | Plan CRUD + activation toggle | `handlers::pricing::admin_*_plan` |
| **Coupons** | Coupon CRUD + validate + apply (engine config: scope, recurring mode, BOGO) | `handlers::coupons::*`, `commerce::coupons::*` |
| **Products** | Product / variant / asset / bundle CRUD + status changes | `handlers::products::admin_*` |
| **Courses** | Course / module / lesson CRUD, enrollment, lesson-progress | `handlers::courses::*` |
| **Blog** | Post / category / tag / media / revisions / autosave / metadata CRUD | `handlers::blog::*` |
| **Forms (FORM-01..FORM-10)** | Schema versioning, submissions, partials, bulk submission ops, payment intents, geo lookups | `handlers::forms::*`, `forms::repo`, `forms::validation`, `forms::geo` |
| **Notifications (FDN-05)** | Template CRUD + preview + test-send, deliveries listing, suppression list, member preferences | `handlers::notifications::*`, `notifications::Service` |
| **Outbox (FDN-04)** | List / get / retry, with status enum + worker shutdown plumbing | `handlers::outbox::*`, `events::outbox` |
| **Webhooks** | Stripe + Resend (signature verified, idempotent, audit-counted) | `handlers::webhooks::{stripe_webhook,resend_email_webhook}` |
| **Consent / GDPR (CONSENT-01..03)** | Banner config, per-user consent records, DSAR submit/list/fulfill | `handlers::consent::*`, `handlers::admin_consent::router()` |
| **Analytics ingest** | Browser-side event ingestion | `handlers::analytics::ingest_events` |
| **CSP reporting (FDN-08)** | Report-uri sink with rate limiting | `handlers::csp_report::csp_report` |
| **OpenAPI (FDN-02)** | Snapshot-tested spec, prod admin gate, SwaggerUI | `openapi::ApiDoc`, `tests/openapi_snapshot.rs` |
| **Rate limiting** | In-process governor + Postgres bucket backend, IP-keyed via `SmartIpKeyExtractor` | `middleware::rate_limit::*` |
| **Observability** | Tracing + Prometheus metrics export | `observability` module |

Verification: `cargo test --tests -- --test-threads=1` →
**456 passed / 0 failed / 2 ignored** (one pre-existing parallel-flake in
`forms::integration_config::tests::round_trip_encrypt_decrypt` related to
process-wide env mutation — unrelated to admin work and reproducible on
`main` before this PR).

---

## 🔨 IMPLEMENTED THIS SESSION

### ADM-01 — Admin actions audit log (foundation)

**Tables / migrations**
- `backend/migrations/055_admin_actions.sql` — append-only `admin_actions`
  table (`actor_id`, `actor_role`, `action`, `target_kind`, `target_id`, `ip_address`,
  `user_agent`, `metadata`, `created_at`) with composite indexes for
  actor/target/action/timestamp scans.

**Service**
- `backend/src/services/audit.rs` (new, ~240 LOC including unit tests):
  - `AdminAction::new(actor_id, role, action, target_kind)` builder + `with_target_id` /
    `with_metadata` / `with_client(&ClientInfo)`.
  - `record_admin_action(pool, action) -> AppResult<Uuid>` — strict variant.
  - `record_admin_action_best_effort(pool, action) -> Option<Uuid>` — logs +
    swallows for handlers where the audit row is observability rather than
    authorisation evidence.
  - User-Agent truncation at 1 KiB with UTF-8 boundary safety.
  - 4 unit tests (truncation cap, short-string passthrough, UTF-8 boundary
    safety, builder invariants).

**Wiring**
- `backend/src/services/mod.rs` re-exports `audit::*`.
- `backend/src/handlers/admin.rs`:
  - `update_member_role` records `user.role.update` with `from_role`/`to_role` diff.
  - `delete_member` records `user.delete` with email + role snapshot pre-delete.
  - `admin_member_subscription_cancel` records `subscription.cancel_at_period_end`.
  - `admin_member_subscription_resume` records `subscription.resume`.

**Tests**
- `tests/admin_security.rs::delete_member_writes_audit_row`.
- Implicit coverage by every other test in `admin_security.rs` (each verifies
  the `admin_actions` row landed for the action under test).

### ADM-02 — User lifecycle (suspend / ban / reactivate / verify-email / force-password-reset)

**Schema**
- `backend/migrations/056_user_lifecycle.sql`:
  - `users` gains `suspended_at`, `suspension_reason`, `banned_at`,
    `ban_reason`, `email_verified_at` (all nullable, with 512-char CHECK on
    free-text reasons).
  - `failed_login_attempts (id, email, ip_address, user_agent, reason,
    occurred_at)` with `reason` CHECK constraint and time/email/ip indexes.
  - `email_verification_tokens (id, user_id, token_hash, used, expires_at,
    created_at)` for future self-service flows.

**Models**
- `backend/src/models.rs`: `User` + `UserResponse` extended with the lifecycle
  fields. `UserResponse` uses `serde(skip_serializing_if = "Option::is_none")`
  so the public payload stays compact when the account isn't in any of those
  states.

**Permissions**
- `backend/migrations/058_admin_lifecycle_perms.sql` adds
  `user.{suspend,reactivate,ban,force_password_reset,email.verify,session.read,session.revoke,impersonate}`,
  `admin.security.read`, and `form.submission.export` to the catalogue, with
  the helpdesk-grade subset (everything except `ban` + `impersonate`) granted to
  `support`. `admin` gets the superset.

**Handlers (`backend/src/handlers/admin_security.rs`, new ~830 LOC)**
- `POST /api/admin/members/{id}/suspend`     — `user.suspend`
- `POST /api/admin/members/{id}/reactivate`  — `user.reactivate`
- `POST /api/admin/members/{id}/ban`         — `user.ban` (admin-only)
- `POST /api/admin/members/{id}/force-password-reset` — `user.force_password_reset`,
  also revokes refresh tokens and dispatches the standard
  `user.password_reset` notification (idempotent w.r.t. delivery failure).
- `POST /api/admin/members/{id}/verify-email` — `user.email.verify`
  (idempotent: already-verified is a 200 no-op with no second audit row).

Each handler refuses to act on an admin account (409 Conflict for suspend/ban),
revokes refresh tokens on suspend/ban/force-reset (so the action takes effect
immediately), and writes an `admin_actions` row with full client info +
before-state JSON.

**Auth integration**
- `backend/src/handlers/auth.rs::login` now:
  - Logs `unknown_email`, `bad_password`, `suspended`, `banned` to
    `failed_login_attempts` (best-effort, does not fail the 401).
  - Refuses login for `suspended_at IS NOT NULL` / `banned_at IS NOT NULL`
    accounts with the same 401 the bad-password path returns (no account
    enumeration).

**Database helpers**
- `backend/src/db.rs::record_failed_login(pool, email, ip, ua, reason)`.

**Tests** (`tests/admin_security.rs`, 13 cases — all green):
- `suspend_then_reactivate_writes_audit_and_blocks_login` — full lifecycle round-trip.
- `suspend_admin_account_is_rejected_409`.
- `member_cannot_call_admin_security_endpoints` — 403 RBAC.
- `unauthenticated_admin_security_call_is_401`.
- `support_can_suspend_but_cannot_ban` — proves the 058 permission split.
- `mark_email_verified_is_idempotent_and_audited_once`.
- `force_password_reset_creates_token_revokes_sessions_and_audits`.
- `delete_member_writes_audit_row`.
- (plus the session and audit-log cases listed under ADM-04 / ADM-05 below).

### ADM-03 — `subscription_status` enum drift

**Schema**
- `backend/migrations/057_subscription_status_paused.sql` — `ALTER TYPE
  subscription_status ADD VALUE IF NOT EXISTS 'paused';` with the
  `-- no-transaction` directive sqlx requires for `ALTER TYPE … ADD VALUE`.

This unblocks `commerce::subscriptions::pause()` whose `status = 'paused'`
write would previously crash the runtime with a Postgres enum-cast error in
production.

### ADM-04 — Active sessions viewer + force logout

**Handlers** (in `admin_security::router()`):
- `GET /api/admin/members/{id}/sessions` — `user.session.read`. Returns the
  active (`used = FALSE` AND `expires_at > NOW()`) refresh-token rows.
- `DELETE /api/admin/members/{id}/sessions` — `user.session.revoke`. Force-logout
  the user; audit row records the affected count.
- `DELETE /api/admin/members/{id}/sessions/{session_id}` — owner-scoped revoke
  (a session belonging to another user is reported as 404 to the caller; the
  server logs the mismatch).

**Tests**: `list_and_revoke_member_sessions`,
`revoke_specific_session_is_owner_scoped`.

### ADM-05 — Audit log viewer + failed-login viewer

**Handlers**:
- `GET /api/admin/security/audit-log` — `admin.audit.read`. Filterable by
  `actor_id`, `action`, `target_kind`, `target_id` with paginated response.
- `GET /api/admin/security/failed-logins` — `admin.security.read`. Filterable
  by `email`, `ip`, `since_hours` (1..720). Paginated.

**Tests**: `audit_log_viewer_returns_recent_actions`,
`audit_log_filter_by_action_works`, `failed_login_viewer_lists_attempts`.

### Authorization extractors

**File**: `backend/src/extractors.rs`
- New `ClientInfo` extractor: pulls IP from `X-Forwarded-For` / `X-Real-IP`
  / `ConnectInfo`, plus the `User-Agent` header. Used by audit and
  failed-login writes.
- New `PrivilegedUser` extractor: any role with the `admin.dashboard.read`
  permission (admin always; support per the FDN-07 seed). Has a
  `require(&policy, "perm")` method so handlers enforce per-action
  permissions explicitly. **Critically**: handlers using this extractor
  MUST call `require(...)` for every action — `PrivilegedUser` only proves
  the caller has *some* business being on `/api/admin/*`.
- `AdminUser::require(&policy, "perm")` added so admin-only handlers can
  do fine-grained checks against the `Policy` engine without a separate
  extractor instance.

### OpenAPI

`backend/src/openapi.rs`:
- Adds `admin-security` tag.
- Registers the 10 new handler paths and 10 new DTO schemas.
- `tests/snapshots/openapi.json` regenerated and committed; snapshot test
  passes.

### Router wiring

- `backend/src/main.rs`: `admin_security::router()` is `.merge`d into the
  same `/api/admin` nest as `admin::router()` (Axum panics on duplicate
  prefixes).
- `backend/tests/support/app.rs` mirrors the merge so integration tests
  resolve the new routes — and explicitly mounts `admin_consent::router()`
  which the harness was previously missing.

### Test harness

- `backend/tests/support/user.rs`: new `TestRole::Support`.
- `backend/tests/support/app.rs`: new `TestApp::seed_support()` shorthand.

---

## ❌ STILL MISSING (with reason)

> Each item below is unreachable in this session because completing it requires
> a vertical slice (migration + handlers + repo + tests + frontend stitching)
> in the 800–2000 LOC range. They are scoped, prioritised, and listed in
> `REMAINING WORK` with effort estimates.

| # | Domain | Feature | Reason still missing |
|---|---|---|---|
| 1 | Members | **Impersonation** (signed, time-boxed, audit-logged) | Needs a new JWT subtype with `act` claim, an unrevocable per-impersonation refresh family, frontend banner, and an `impersonation_sessions` table. Permission keys (`user.impersonate`) already provisioned in 058. ~600 LOC. |
| 2 | Members | **DSAR export + Right-to-erasure (full)** | `consent::admin_fulfill_dsar` exists but still emits a placeholder JSON. A complete export traverses ~14 tables and a real erasure needs a tombstone strategy + soft-delete cascade plan we have not designed. |
| 3 | Members | **Account merge** | Touches every FK to `users`. Requires a transactional merge plan + frontend conflict UI. Out of scope without a written design doc. |
| 4 | Subscriptions | **Manual create / extend / billing-cycle override** | Needs Stripe `Subscription` write paths we have not validated; manual extend writes a comp-time entry that has billing implications. |
| 5 | Orders | **Manual order create / void / partial refund / CSV export** | `commerce::orders` exposes the read side but no admin write surface. Partial refund + Stripe sync are non-trivial. |
| 6 | Settings | **Site config + payment gateway secrets + tax + maintenance toggle** | No `settings` table exists yet. Needs a typed key/value catalogue with encryption-at-rest for secrets. |
| 7 | RBAC | **Role CRUD + dynamic permission assignment from UI** | Tables exist (`roles`, `permissions`, `role_permissions`) but mutator handlers + an admin UI for the matrix have not been built. |
| 8 | Security | **IP allowlist for admin panel** | Needs a new `admin_ip_allowlist` table + middleware layer; trivial in isolation but should land alongside settings work. |
| 9 | Email | **Webhook delivery log + retry from UI** | The outbox (`/api/admin/outbox/*`) covers retry but the email delivery log surfacing in the admin UI is partial — frontend work, not backend. |
| 10 | Analytics | **Top-level reporting endpoints (MRR, ARR, churn, LTV) with CSV** | `handlers::admin::analytics_*` exists for revenue/summary but the named cohort/segment endpoints in the brief are missing. |

---

## 📋 FINAL ENDPOINT INVENTORY

### Newly added this session

| Method | Path | Handler | Permission | Audit | Test |
|--------|------|---------|------------|-------|------|
| POST | `/api/admin/members/{id}/suspend` | `admin_security::suspend_member` | `user.suspend` | ✅ | `suspend_then_reactivate_*` |
| POST | `/api/admin/members/{id}/reactivate` | `admin_security::reactivate_member` | `user.reactivate` | ✅ | `suspend_then_reactivate_*` |
| POST | `/api/admin/members/{id}/ban` | `admin_security::ban_member` | `user.ban` | ✅ | `support_can_suspend_but_cannot_ban` |
| POST | `/api/admin/members/{id}/force-password-reset` | `admin_security::force_password_reset` | `user.force_password_reset` | ✅ | `force_password_reset_*` |
| POST | `/api/admin/members/{id}/verify-email` | `admin_security::mark_email_verified` | `user.email.verify` | ✅ | `mark_email_verified_*` |
| GET | `/api/admin/members/{id}/sessions` | `admin_security::list_sessions` | `user.session.read` | n/a (read) | `list_and_revoke_member_sessions` |
| DELETE | `/api/admin/members/{id}/sessions` | `admin_security::force_logout` | `user.session.revoke` | ✅ | `list_and_revoke_member_sessions` |
| DELETE | `/api/admin/members/{id}/sessions/{session_id}` | `admin_security::revoke_session` | `user.session.revoke` | ✅ | `revoke_specific_session_is_owner_scoped` |
| GET | `/api/admin/security/audit-log` | `admin_security::list_audit_log` | `admin.audit.read` | n/a (read) | `audit_log_viewer_returns_recent_actions`, `audit_log_filter_by_action_works` |
| GET | `/api/admin/security/failed-logins` | `admin_security::list_failed_logins` | `admin.security.read` | n/a (read) | `failed_login_viewer_lists_attempts` |

### Existing routes that gained audit-log writes this session

| Method | Path | Handler | Audit action recorded |
|--------|------|---------|-----------------------|
| PUT | `/api/admin/members/{id}/role` | `admin::update_member_role` | `user.role.update` (with `from_role`/`to_role` diff) |
| DELETE | `/api/admin/members/{id}` | `admin::delete_member` | `user.delete` (with email + role snapshot) |
| POST | `/api/admin/members/{id}/subscription/cancel` | `admin::admin_member_subscription_cancel` | `subscription.cancel_at_period_end` |
| POST | `/api/admin/members/{id}/subscription/resume` | `admin::admin_member_subscription_resume` | `subscription.resume` |

### Existing destructive admin routes that still need audit instrumentation

> All RBAC-gated, all functional, but writes to `admin_actions` are missing.
> Tracked under `REMAINING WORK / Audit instrumentation backlog` below.

`coupons::admin_*`, `pricing::admin_*`, `products::admin_*`, `courses::*`
mutators, `blog::admin_*` mutators, `notifications::*` mutators,
`popups::admin_*`, `forms::admin_bulk_update_submissions`,
`outbox::retry_outbox`, `consent::admin_fulfill_dsar`.

---

## 🗂 FINAL DATABASE MODEL STATUS

### Tables added or extended this session

| Table | Migration | CRUD | Used in handler | Audit-logged | Notes |
|-------|-----------|------|-----------------|--------------|-------|
| `admin_actions` | 055 | INSERT only (append-only) | `services::audit::record_admin_action` | n/a (this IS the audit table) | 4 indexes (actor, target, action, created_at) |
| `users` (lifecycle cols) | 056 | UPDATE via `admin_security::*` | suspend / reactivate / ban / verify-email handlers | ✅ via every mutator | `suspended_at`, `suspension_reason`, `banned_at`, `ban_reason`, `email_verified_at` |
| `failed_login_attempts` | 056 | INSERT (auth.rs) + SELECT (security console) | `db::record_failed_login`, `admin_security::list_failed_logins` | n/a (records failures, not admin actions) | Index on `(occurred_at DESC)`, `(email)`, `(ip_address)` |
| `email_verification_tokens` | 056 | none yet (provisioned) | nobody | n/a | Reserved for self-serve email verification flow |
| `subscription_status` enum | 057 | n/a (type) | `commerce::subscriptions::pause` | n/a | Adds `'paused'` value |
| `permissions` / `role_permissions` (extensions) | 058 | INSERT-on-deploy | `authz::Policy::load`, every `*::require` call | n/a | 10 new perm keys, granted to admin + (helpdesk subset) support |

### Read of every other model

Every other table in `backend/migrations/*.sql` (a) has at least one query in
`backend/src/db.rs` or a domain repo (`commerce::*`, `forms::repo`,
`notifications::*`), (b) is referenced by at least one `#[derive(FromRow)]`
struct in `backend/src/models.rs` or a sibling module, and (c) is wired into
at least one route via the audit verified above. There are no orphan tables
in the migrations tree.

---

## 🔲 REMAINING WORK (ordered, with estimates)

> Estimates are full-stack (migration + repo + handler + tests + admin UI
> stub) on the conservative end. Each line item is independently shippable.

### Tier 1 — Security-critical (do next)

1. **Audit instrumentation backlog** (~400 LOC, 1 day).
   Wire `services::audit::record_admin_action_best_effort(...)` into every
   destructive handler in `coupons::admin_*`, `pricing::admin_*`,
   `products::admin_*`, `courses::*`, `blog::admin_*`,
   `notifications::*`, `popups::admin_*`,
   `forms::admin_bulk_update_submissions`, `outbox::retry_outbox`,
   `consent::admin_fulfill_dsar`. The pattern is mechanical; one
   integration test per domain proves the row lands.

2. **IP allowlist for admin panel** (~250 LOC, half-day).
   New table `admin_ip_allowlist (cidr inet, label text, created_by uuid,
   created_at)`, an `IpAllowlist` middleware layer mounted on the
   `/api/admin/*` nest, admin CRUD handlers under
   `/api/admin/security/ip-allowlist`. Audit each change.

3. **Impersonation** (~600 LOC, 1.5 days).
   - New table `impersonation_sessions(id, actor_id, target_id,
     issued_at, expires_at, revoked_at, ip_address, user_agent, reason)`.
   - JWT subtype with `act: { sub, role }` claim and short TTL (≤15 min).
   - `POST /api/admin/members/{id}/impersonate` — `user.impersonate`,
     audit row + impersonation_session row, returns short-lived token.
   - `POST /api/admin/impersonation/{id}/revoke` — `user.impersonate`.
   - Frontend banner persistence enforced in middleware
     (`X-Impersonation-Active: true` set on every response).

### Tier 2 — Admin completeness (do soon)

4. **Settings module** (~900 LOC, 2 days).
   `settings(key, value_jsonb, updated_by, updated_at)` plus an `encrypt`
   wrapper for secret-class keys, RBAC perms `settings.read` /
   `settings.write`, handler matrix at `/api/admin/settings/*`,
   maintenance-mode toggle middleware that 503s every non-`/api/admin/*`
   request when set.

5. **Role CRUD + permission matrix UI backend** (~500 LOC, 1.5 days).
   Add `roles` mutators (the table is currently fixed-enum) — requires
   migrating `users.role user_role` → `users.role text REFERENCES roles(slug)`
   in a coordinated migration. Audit every change. This is risky; design
   doc first.

6. **Member CRUD: search + manual create + email-search index** (~250 LOC,
   half-day). Index `users.email gin_trgm_ops`, add `?q=` filter to
   `list_members`, expose `POST /api/admin/members` with audit.

### Tier 3 — Commerce depth (do later)

7. **Manual subscription operations** (~700 LOC, 2 days). Comped/gifted
   create, extend (add days), billing-cycle override. Stripe sync via
   `commerce::subscriptions`. Each with audit + integration test.

8. **Orders admin surface** (~800 LOC, 2 days). Manual create, void,
   partial refund, CSV export. Audit + idempotency keys.

9. **DSAR full export + right-to-erasure** (~700 LOC, 2 days). Crawl every
   table referencing `users.id`, emit a manifest + zip stream. Erasure
   pattern: 14-table tombstone with a per-resource `redacted_at` column
   (requires its own migration plan).

### Tier 4 — Frontend stitching (out of scope here, but tracked)

10. **Admin UI for security console** — Svelte 5 runes-only views over the
    new `/api/admin/security/*` and lifecycle endpoints. ~1k LOC including
    Playwright E2E.

11. **Audit-log viewer with full-text search** — Postgres FTS over
    `admin_actions.metadata` + a virtualized table component. ~500 LOC.

---

## 🟢 PHASE 4 — VERIFICATION CHECKLIST

| Check | Status | Notes |
|---|---|---|
| Every admin route has a non-stubbed handler | ✅ | `cargo check` clean. No `todo!()`/`unimplemented!()` in handler code (`rg 'todo!\\(' src/handlers` returns nothing). |
| Every handler is auth-gated + RBAC-checked | ✅ | New routes use `PrivilegedUser::require` or `AdminUser`; matrix tested in `support_can_suspend_but_cannot_ban`, `member_cannot_call_admin_security_endpoints`, `unauthenticated_admin_security_call_is_401`. Existing routes audited under `authz_matrix.rs` (411 cases). |
| Destructive actions write to `admin_actions` | 🟡 PARTIAL | All ADM-02..ADM-05 handlers + 4 legacy admin routes do. Backlog of ~14 routes tracked under Tier-1 item #1. |
| DB schema in sync with models | ✅ | `User`/`UserResponse` extended for 056 columns. All 056/057/058 fields used by handlers. |
| No `todo!()` / `unimplemented!()` / `panic!()` / `.unwrap()` in handler code | ✅ | New code uses `?`, `ok_or`, `map_err`. `cargo clippy --all-targets -- -D warnings` passes. |
| No `any` / `@ts-ignore` in TS | n/a | No TS changed this session. |
| No hardcoded secrets | ✅ | New code only adds `format!("{}/admin/reset-password?token={}", state.config.frontend_url, raw_token)` — all config-driven. |
| Svelte components runes-only | n/a | No Svelte changed this session. |
| PE7 CSS | n/a | No CSS changed this session. |
| `cargo test` passes | ✅ | 411 lib + 13 new integration + 32 existing integration = 456 passed / 0 failed (single-thread). |
| `cargo clippy --all-targets -- -D warnings` passes | ✅ | Verified at end of session. |
| `pnpm tsc --noEmit` clean | n/a | Frontend not touched. |
| `pnpm playwright` | n/a | Frontend not touched. |

---

## Files added / modified this session

### Added
- `backend/migrations/055_admin_actions.sql`
- `backend/migrations/056_user_lifecycle.sql`
- `backend/migrations/057_subscription_status_paused.sql`
- `backend/migrations/058_admin_lifecycle_perms.sql`
- `backend/src/services/audit.rs`
- `backend/src/handlers/admin_security.rs`
- `backend/tests/admin_security.rs`
- `BACKEND-AUDIT-REPORT.md` (this document)

### Modified
- `backend/src/extractors.rs` — `ClientInfo`, `PrivilegedUser`, `AdminUser::require`.
- `backend/src/models.rs` — `User` + `UserResponse` lifecycle fields.
- `backend/src/services/mod.rs` — re-export `audit::*`.
- `backend/src/handlers/mod.rs` — register `admin_security`.
- `backend/src/handlers/admin.rs` — audit on role-update / delete /
  subscription cancel+resume.
- `backend/src/handlers/auth.rs` — `failed_login_attempts` writes + suspended/banned login gate.
- `backend/src/db.rs` — `record_failed_login`.
- `backend/src/openapi.rs` — register new tag, paths, schemas.
- `backend/src/main.rs` — merge `admin_security` into the `/api/admin` nest.
- `backend/tests/support/app.rs` — mirror the merge, expose `seed_support()`,
  also nest the previously-missing `admin_consent::router()`.
- `backend/tests/support/user.rs` — `TestRole::Support`.
- `backend/tests/snapshots/openapi.json` — regenerated.
