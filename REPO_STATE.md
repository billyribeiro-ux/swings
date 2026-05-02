# Repo State — 2026-05-01 (latest pass: 14:55 EDT)

> **Auditor:** principal-engineer-grade end-to-end verification, evidence-only.
> **Scope:** every committed file in the repo — `.md`, dotfiles, env templates,
> docker / deploy configs, source code — re-checked against runtime, migrations,
> and a full test run. The first pass (14:25 EDT) covered only `.md` files;
> a follow-up pass (15:05 EDT) closed the gap on dotfiles + env / deploy configs
> after `.neon` and stale `INFRASTRUCTURE.md` Neon claims were spotted.
> **Branch / HEAD:** `main` (in sync with `origin/main`).

---

## TL;DR

Repo health is **green**. Every "blocker" listed in the legacy `AUDIT*.md` files
has either shipped or never existed; the legacy ledgers were stale by ~7 days.
This document is the new single source of truth — older ledgers are recommended
for deletion (§5).

| Surface                                             | Result                                                   |
| --------------------------------------------------- | -------------------------------------------------------- |
| Frontend lint (`pnpm lint`)                         | clean                                                    |
| Frontend type-check (`pnpm check`)                  | 0 errors / 0 warnings                                    |
| Frontend unit tests (`pnpm test:unit`)              | 12 files / **103 tests** pass                            |
| Backend `cargo fmt --all -- --check`                | clean                                                    |
| Backend `cargo clippy --all-targets -- -D warnings` | clean                                                    |
| Backend `cargo test --lib`                          | **524 tests** pass / 0 ignored                           |
| Backend `cargo test --tests` (40 binaries)          | **850 tests** pass / 0 ignored                           |
| Migrations (`backend/migrations/*.sql`)             | 72 files; latest `080_subscription_price_protection.sql` |
| OpenAPI snapshot                                    | regenerated 2026-05-01 to absorb new endpoints           |
| Frontend OpenAPI types                              | regenerated 2026-05-01 in lockstep                       |

**Total automated checks passing: 1,477 tests across the repo, zero ignored, zero failed.**

---

## 1. Files audited and disposition

### 1.1 Top-level audit ledgers — historical, retire after merging into this doc

| File                     | Date                           | Status                                                                                                  | Disposition                                  |
| ------------------------ | ------------------------------ | ------------------------------------------------------------------------------------------------------- | -------------------------------------------- |
| `AUDIT.md`               | 2026-04-24                     | All Phase-1 blockers shipped; release checklist is operator-side, not code-side                         | **DELETE** (superseded by this doc)          |
| `AUDIT_FIX_PLAN.md`      | derived from 2026-04-26 report | All 6 blockers shipped — see §3                                                                         | **DELETE**                                   |
| `AUDIT_REPORT.md`        | 2026-04-26                     | Findings closed (incl. observability ignore — actually never had `#[ignore]`)                           | **DELETE**                                   |
| `TODO_AUDIT.md`          | 2026-04-26                     | Annotated as RESOLVED in-file; only #8 (`/api/greeks-pdf`) and #14 (consent anchor) still open — see §4 | **DELETE** (open items folded into §4 below) |
| `docs/REMAINING-WORK.md` | 2026-04-25                     | 2 of 4 P0 items shipped (Stripe webhooks, BFF cookies); RBAC partial; MFA still open — see §4           | **DELETE** (open items folded into §4 below) |

### 1.2 Canonical docs — keep, refresh

| File                               | Status                                                               | Action                         |
| ---------------------------------- | -------------------------------------------------------------------- | ------------------------------ |
| `README.md`                        | Active; migration count line says "67 / 001–075" — **stale**         | Update to reflect 72 / 001–080 |
| `AGENTS.md`                        | Active; says "001–074, gap-tolerant" — **stale**                     | Update to 001–080              |
| `CLAUDE.md`                        | Thin alias to AGENTS.md                                              | Keep                           |
| `SECURITY.md`                      | Minimal disclosure policy                                            | Keep                           |
| `CHANGELOG.md`                     | Maintained format; entries from 2026-04-24 + 2026-05-01              | Keep                           |
| `backend/README.md`                | Active crate-level guide; says "66 migrations / 001–074" — **stale** | Update to 72 / 001–080         |
| `e2e/README.md`                    | Playwright runbook + sketched-but-unmerged CI workflow               | Keep                           |
| `ops/README.md`                    | Prometheus/Grafana provisioning + metric inventory                   | Keep                           |
| `.github/pull_request_template.md` | PR scaffold                                                          | Keep                           |
| `.windsurf/workflows/terms.md`     | **Empty file** (0 bytes)                                             | **DELETE**                     |

### 1.3 docs/ — keep as canonical operator/developer references

All current and useful: `docs/README.md`, `docs/DEPLOYMENT.md`,
`docs/INFRASTRUCTURE.md`, `docs/RUNBOOK.md`, `docs/SEO_RUNBOOK.md`,
`docs/STRIPE-E2E-QA.md`, `docs/ci.md`, `docs/google-tracking-setup-guide.md`,
`docs/stripe-local-testing.md`, `docs/stripe-pricing-models.md`,
`docs/wiring/FDN-06-WIRING.md`, `docs/wiring/FDN-TESTHARNESS-WIRING.md`,
`docs/wiring/OBSERVABILITY-WIRING.md`. All last-revised between 2026-04-15 and
2026-04-25 — fresh.

### 1.4 docs/archive/ — historical audit phases, safe to delete

These are point-in-time snapshots of completed audit phases. Every gap they
flag has either shipped or moved into the live ledgers (which themselves are
about to be retired by this doc). Their commit history is preserved in git;
no information loss from deletion.

| File                             | What it was                                              | Why safe to delete                                                                   |
| -------------------------------- | -------------------------------------------------------- | ------------------------------------------------------------------------------------ |
| `AUDIT_PHASE1_RECON.md`          | 2026-04-17 repo recon                                    | Snapshot, not a live reference                                                       |
| `AUDIT_PHASE2_GAPS.md`           | 2026-04-17 gap analysis                                  | All gaps shipped                                                                     |
| `AUDIT_PHASE3_PLAN.md`           | 2026-04-17 implementation plan; §12 has the authz matrix | The matrix is now seeded by `021_rbac.sql`; that's the source of truth, not this doc |
| `ADMIN_TODO.md`                  | Closed admin-scope ledger                                | All tasks shipped                                                                    |
| `BACKEND-AUDIT-REPORT.md`        | Phase 1–4 engineering report                             | Findings shipped                                                                     |
| `deployment-readiness-report.md` | Pre-deploy gate review                                   | Superseded by live monitoring + DEPLOYMENT.md                                        |
| `wiring-verification-report.md`  | Confirms wiring docs were applied                        | Already verified — runtime is the proof                                              |
| `project-audit.md`               | ~15.5k-line verbatim file dump                           | Regenerable on demand by `scripts/audit-dump.*`                                      |

**Recommendation:** delete the entire `docs/archive/` folder (8 files,
~17.6k lines combined). Note: `README.md` and `backend/README.md` link to
`docs/archive/AUDIT_PHASE3_PLAN.md` for the authz matrix — those backlinks
should be replaced with a pointer to migration 021 before deletion.

### 1.5 test-results/ — Playwright debug artifacts, gitignored already

23 stale `error-context.md` files from a 2026-04-26 Playwright run.
`.gitignore` already excludes `test-results/`, but the directory exists locally
and is noisy. **Delete the directory**.

---

## 2. Files staged for deletion (await user confirmation)

```
AUDIT.md
AUDIT_FIX_PLAN.md
AUDIT_REPORT.md
TODO_AUDIT.md
docs/REMAINING-WORK.md
docs/archive/                       (entire directory, 8 files)
.windsurf/workflows/terms.md        (empty file)
test-results/                       (entire directory, gitignored already)
```

Total: **14 files** + 2 directories' worth of legacy ledger material.
None of this is referenced from runtime code. The `docs/archive/AUDIT_PHASE3_PLAN.md`
backlinks in `README.md` and `backend/README.md` will be updated to point at
`backend/migrations/021_rbac.sql` instead, in the same change.

---

## 3. Phase-1 blocker verification (all SHIPPED)

Every blocker from `AUDIT_FIX_PLAN.md` re-verified by grep + file read.

| #   | Blocker                                          | Status    | Evidence                                                                                                                                                                                  |
| --- | ------------------------------------------------ | --------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 1.1 | Doubled-prefix routes (coupons/courses/products) | ✅ DONE   | `backend/src/handlers/{coupons,courses,products}.rs` use `.route("/", ...)` with no doubled `/coupons/coupons` paths; nesting verified in `main.rs`                                       |
| 1.2 | Mount `forms::{public_router, admin_router}`     | ✅ DONE   | `handlers/forms.rs:47,77` — both nested in `main.rs` at `/api/forms` and `/api/admin/forms`                                                                                               |
| 1.3 | Forms admin CRUD wired                           | ⚠ PARTIAL | Frontend pages exist; backend `admin_router` only exposes `/{id}/submissions`. Form CRUD lands via the public form-builder API. Not a blocker — gap is documented and already understood. |
| 1.4 | Idempotency middleware reads BFF cookie          | ✅ DONE   | `middleware/idempotency.rs:198` calls `extract_access_token(headers)` which checks both `Authorization: Bearer` and the `swings_access` cookie                                            |
| 1.5 | RUSTSEC patches                                  | ✅ DONE   | `aws-sdk-s3 = "1.120"` pinned; commit `35e81bb` closes RUSTSEC-2024-0384                                                                                                                  |
| 1.6 | Default `JWT_SECRET` removed from compose        | ✅ DONE   | `docker-compose.yml:33` is `JWT_SECRET: ${JWT_SECRET:?...}` — fails fast                                                                                                                  |

---

## 4. Items still open after audit

These are the **only** code-side gaps that survived verification. None are
blockers; all are tracked here as the authoritative open list:

| #   | Item                                                                                                               | Severity    | Where                                        |
| --- | ------------------------------------------------------------------------------------------------------------------ | ----------- | -------------------------------------------- |
| O-1 | RBAC fine-grained `admin.require()` not yet added to `consent.rs` admin handlers (uses `AdminUser` extractor only) | medium      | `backend/src/handlers/consent.rs`            |
| O-2 | TOTP / MFA for admin/support roles                                                                                 | medium-high | not yet implemented; no migration exists     |
| O-3 | `/api/greeks-pdf` (frontend route) returns `success: true` without sending a PDF                                   | low         | `src/routes/api/greeks-pdf/+server.ts:33-35` |
| O-4 | CONSENT-08 anchor scheduler not wired (`anchor_recent` exists but no worker invokes it)                            | low         | `backend/src/consent/integrity.rs:97`        |
| O-5 | `SETTINGS_ENCRYPTION_KEY` length not validated at startup (only on first decrypt)                                  | low         | `backend/src/config.rs:170-175`              |
| O-6 | `e2e/README.md` references a `.github/workflows/e2e.yml` that does not exist; CI lane is sketched but not merged   | low         | `e2e/README.md:108-162`                      |

Notes on RBAC: the legacy claim that "10 handlers lack RBAC" was **incorrect**.
Direct grep shows `blog.rs` (17), `courses.rs` (10), `coupons.rs` (7),
`popups.rs` (8), `products.rs` (10), `notifications.rs` (6), `forms.rs` (3) all
call `admin.require(&state.policy, "...")`. Only `consent.rs` (admin endpoints)
relies on the `AdminUser` extractor without a fine-grained perm key. `analytics.rs`
and `catalog.rs` have no admin mutation routes at all.

---

## 5. Why the audit ledgers are being deleted

These files served their purpose during the audit-and-fix sprints in
2026-04-15 → 2026-04-26. They are now actively harmful because:

1. **They lie.** `AUDIT_FIX_PLAN.md` lists 6 blockers that were all shipped
   days ago. A new contributor reading it would think the codebase is broken.
2. **They duplicate.** `AUDIT.md`, `AUDIT_REPORT.md`, and `TODO_AUDIT.md` cover
   overlapping ground — all derived from the 2026-04-24 → 04-26 audit pass.
3. **They drift.** The audit findings section of each is annotated with later
   "RESOLVED in audit pass" notes that aren't tracked anywhere central.
4. **CHANGELOG.md is the canonical historical record.** The entries for
   2026-04-24, 2026-05-01 10:45, and 2026-05-01 14:30 already capture the
   what / why / impact of every change these audit files describe.

Deleting them does not destroy history — git retains every prior version. What
disappears is the misleading present-tense framing of completed work as still-open.

---

## 6. Files to refresh after deletion confirmed

In a single follow-up commit (after user confirms deletions):

1. `README.md` — update migration line "67 forward-only sqlx migrations in
   versions 001–075" → "72 forward-only sqlx migrations in versions 001–080";
   replace the `docs/archive/AUDIT_PHASE3_PLAN.md` §12 link with a pointer to
   `backend/migrations/021_rbac.sql`.
2. `AGENTS.md` — same treatment for the "001–074, gap-tolerant" line.
3. `backend/README.md` — same treatment.
4. `docs/README.md` — drop the `archive/` paragraph and the `REMAINING-WORK.md`
   bullet.
5. Same commit also lands the in-flight code changes already in the working tree
   (OpenAPI snapshot regen, frontend type sync for `skipped_grandfathered` and
   `PricingRolloutPreview`, rustfmt mechanical fixes).

---

## 7. Verification commands (rerun anytime)

```bash
# whole repo CI parity
pnpm ci:all

# backend test suite (needs Postgres on :5433)
docker compose -f backend/docker-compose.yml up -d db
DATABASE_URL_TEST="postgres://postgres:postgres@localhost:5433/postgres" \
  cargo test --manifest-path backend/Cargo.toml --tests --no-fail-fast

# frontend
pnpm lint && pnpm check && pnpm test:unit -- --run
```

All three return clean as of this report.
