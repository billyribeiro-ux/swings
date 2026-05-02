# Forensic Audit — Index

**Started:** 2026-05-01
**Repo:** `/Users/billyribeiro/Desktop/my-websites/swings`
**Branch:** `main` at `17e20d1`
**Standard:** Principal-Engineer-Level-7. Every finding has a stable ID. Every artifact is reproducible from a committed script.

---

## Phase status

- [x] 1. Surface mapping — COMPLETE 2026-05-02
- [ ] 2. Static forensics (3 parallel)
- [ ] 3. Dynamic confirmation
- [ ] 4. Severity ranking
- [ ] 5. Implementation plan
- [ ] 6. Regression lock-in

### Phase 1 summary

- **Routes mapped: 282** — backend 278, sveltekit 2, webhook 2.
- **Distinct middleware constructs:** 12 — `admin_ip_allowlist::enforce`, `rate_limit::admin_mutation_rate_limit`, `idempotency::enforce`, `rate_limit::login_layer`, `rate_limit::register_layer`, `rate_limit::forgot_password_layer`, `rate_limit::member_layer`, `rate_limit::webhooks_layer`, `rate_limit::form_submit_layer`, `rate_limit::form_partial_layer`, `rate_limit::popup_event_layer`, `rate_limit::popup_submit_layer`, `rate_limit::coupon_apply_layer`, `rate_limit::consent_record_layer`, `clamp_body_size` (csp), plus the 7 root-level layers (`metrics_handle Extension`, `metrics::http_middleware`, `correlation::middleware`, `impersonation_banner::stamp`, `maintenance_mode::enforce`, `CorsLayer`, `TraceLayer`).
- **Distinct authz constructs:** 7 — `AdminUser`, `PrivilegedUser`, `AuthUser`, `MaybeAuthUser`, `OptionalAuthUser`, `PaidUser`, `RoleUser` (declared, unused). Plus `policy.require()` / `admin.require()` / `privileged.require()` with **67 distinct permission strings** observed in handler bodies.
- **Tables touched by admin flows:** ~80. See `01-surface-map/db-touchpoints.md` for the per-table read/write/lock matrix.
- **Structural surprises:**
  - Duplicate prefix mount at main.rs:691 + main.rs:700 — both `admin_consent::router()` and `consent::admin_router()` nest under `/api/admin/consent`. Each carries its own `idempotency::enforce` layer, but the route sets are disjoint so axum routes deterministically.
  - Asymmetric `/api/member` mount stack: main.rs:755 wraps `idempotency` + `rate_limit::member_layer`; main.rs:762 (courses member_router) and main.rs:763 (notifications member_router) share the same prefix but inherit NEITHER layer.
  - `/api/admin/audit` (main.rs:616) is the only admin-tree nest that does NOT chain `.layer(idempotency::enforce)` — every sibling does.
  - `RoleUser` extractor is defined (extractors.rs:590) but used by zero routes — reserved for future migration per the docstring.
  - SvelteKit `/admin/**` is CSR-only (`+layout.ts: ssr=false`), so the entire frontend admin surface lives on a single client-rendered shell with no server-side load functions or form actions.

---

## Workspace layout

```
forensic-audit/
├── 00-INDEX.md                  ← this file (updated after each phase)
├── 01-surface-map/              ← machine-checkable inventory (Phase 1)
│   ├── routes.json
│   ├── middleware-chain.md
│   ├── auth-graph.md
│   ├── db-touchpoints.md
│   └── dependency-graph.svg
├── 02-static-findings/          ← F-NNN-<slug>.md files (Phase 2)
├── 03-dynamic-artifacts/        ← per-finding repro evidence (Phase 3)
│   └── F-NNN/
│       ├── repro.sh
│       ├── request.http
│       ├── response.txt
│       ├── db-state-before.sql
│       ├── db-state-after.sql
│       └── server.log
├── 04-falsified-fixes/          ← prior-fix-<sha>.md (Phase 3)
├── 05-severity/                 ← severity-matrix.md (Phase 4)
├── 06-fix-plan/                 ← FIX-NNN entries (Phase 5)
├── 07-regression-suite/         ← test files referencing F-NNN (Phase 6)
└── 99-tools/                    ← shared helper scripts
```

---

## Findings ledger

(Empty until Phase 2 begins.)

| ID | Title | Phase | Status |
|----|-------|-------|--------|

---

## Anti-lie protocol

Before any phase reports complete, the dispatched agent must:
1. Pick 3 random findings from its own output.
2. Re-open the cited file at the cited line numbers. Confirm the code is what was quoted.
3. Re-run the cited reproducer command. Confirm output matches captured artifact.
4. Append a self-audit log to the final message.

If any check fails, the agent's output is `UNTRUSTED` and re-run from scratch.

---

## Naming conventions

- Findings: `F-001`, `F-002`, ... (zero-padded, sequential, never reused)
- Fixes: `FIX-001` resolves one or more `F-NNN`
- Slugs in filenames: lowercase-kebab-case
- Every code citation: markdown link `[file.ts:42](path/to/file.ts#L42)`
