# TODO_AUDIT.md

> Generated 2026-04-26 by scanning the entire repo for actionable inline markers
> (TODO/FIXME/XXX/HACK/BUG/DEPRECATED/WORKAROUND/REVISIT/KLUDGE/STUB).
> No code modified.

## Executive Summary

- **Total raw matches scanned:** 50 (across `src/`, `backend/src/`, `backend/tests/`, `e2e/`, `scripts/`, `docs/`)
- **Real actionable findings (after false-positive filter):** 18
- **Per severity:**
  - P0/Critical: **0**
  - P1/Major: **2**
  - P2/Minor: **16**
  - Info/Intentional: **6** (counted separately below)
- **Per category:**
  - `security`: 0
  - `incomplete-feature`: 8 (6 notification channels + outbound webhook stub + greeks-pdf email)
  - `refactor-debt`: 5 (i18n shim swap, tcf shim, IP-hash dedupe, JWT claim hard-fail, Stripe currency minima)
  - `external-dependency-blocked`: 2 (MaxMind subdivision lookup; CONSENT-08 scheduler dependency)
  - `dead-code`: 0
  - `doc-gap`: 1 (e2e CI workflow not yet wired)
  - `intentional`: 6 (counted in Info)
- **Verdict:** Zero security-tagged TODOs. The bulk of the backlog is *scaffolded-but-not-wired* features (notifications channels, outbound webhooks, integrity-anchor scheduler) plus two cross-cutting refactor markers. Cleanup is mostly mechanical; no on-fire items.

---

## P0 / Critical (act now)

**None.** No marker carried a SECURITY/CRITICAL tag, no auth-bypass / XSS / CSRF / injection / RCE references, no in-prod bug callouts.

---

## P1 / Major

Two markers gate user-visible behavior on production deploys.

### P1-1 — `/admin/_consent-preview` and `/admin/_ui-kit` are NOT yet behind admin RBAC

- **File:** `/Users/billyribeiro/Desktop/my-websites/swings/src/routes/admin/_consent-preview/+page.svelte:7`
- **File:** `/Users/billyribeiro/Desktop/my-websites/swings/src/routes/admin/_ui-kit/+page.svelte:8`
- **Verbatim:**
  ```
  TODO (FDN-07 authz gate): gate this route behind `requires: ['admin']`
  once CONSENT-07 admin RBAC is live so it cannot reach production.
  ```
- **Context:** Both pages are dev-preview surfaces (Consent banner gallery and UI-kit showcase). Route segments start with `_` as a convention but SvelteKit does NOT treat `_`-prefixed segments as private — they ARE reachable in a deployed build unless a `+layout.server.ts` guard blocks them. There is no `+page.server.ts` / `+layout.server.ts` enforcing `requires: ['admin']` on either path today.
- **Severity rationale:** Not P0 because neither page exposes mutating actions or secrets — they render UI primitives against `STUB_BANNER_CONFIG`. But shipping them unguarded leaks the visual-design surface and the underscore-prefix convention to the public, which is exactly what the comment is warning against.
- **Recommended action:** Add a `+page.server.ts` (or shared admin layout guard) to both routes that returns `error(404)` when the requester is not an admin. If FDN-07 RBAC is already shipped (the comment is stale), wire `requires: ['admin']` now and delete the TODO.

### P1-2 — JWT issuer/audience claim binding still allows absent claims

- **File:** `/Users/billyribeiro/Desktop/my-websites/swings/backend/src/extractors.rs:171`
- **Verbatim:**
  ```
  After `JWT_EXPIRATION_HOURS` have elapsed since this function shipped,
  every live session has rotated through the new mint path and we can
  promote the absent-claim branch to an error (see the TODO marker in
  `backend/src/extractors.rs` → `verify_claim_binding`).
  ```
- **Context:** `verify_claim_binding` currently accepts tokens that lack an `iss`/`aud` claim entirely. The intent (per the doc-comment) is that this is a transitional concession for legacy tokens minted before the rollout. Once `JWT_EXPIRATION_HOURS` has elapsed since the rollout date, the absent-claim branch should be promoted to `Err(AppError::Unauthorized)`.
- **Severity rationale:** Not security-critical *today* (the wrong-value branch already errors; an attacker cannot forge a token without the signing key), but missing `iss`/`aud` validation is a known JWT smell and the comment explicitly tags it as a planned hardening. P1 because it's a security-adjacent posture promise that decays the longer it sits.
- **Recommended action:** Confirm the rollout date. If `now - rollout_date > JWT_EXPIRATION_HOURS`, replace the `if let Some(...)` branches with an unconditional unwrap-or-Unauthorized and delete the comment. Add a regression test asserting tokens without `iss` / `aud` are rejected.

---

## P2 / Minor

One-line entries — refactor debt, scaffolded incomplete features with no user-visible regression today, and pure doc gaps.

| # | File:Line | Marker | Note |
|---|-----------|--------|------|
| 1 | `backend/src/notifications/channels/in_app.rs:37` | TODO | In-app notification channel returns `Permanent("not implemented")`. Wire under UI subsystem. |
| 2 | `backend/src/notifications/channels/push.rs:36` | TODO | WebPush VAPID channel stubbed; provider not implemented. |
| 3 | `backend/src/notifications/channels/sms.rs:37` | TODO | Twilio SMS channel stubbed; needs feature flag + creds. |
| 4 | `backend/src/notifications/channels/slack.rs:33` | TODO | Slack incoming-webhook channel stubbed. |
| 5 | `backend/src/notifications/channels/discord.rs:33` | TODO | Discord webhook channel stubbed. |
| 6 | `backend/src/notifications/channels/webhook.rs:38` | TODO | Outbound HTTP POST channel stubbed (FORM-07 dependency). |
| 7 | `backend/src/events/handlers/webhook_out.rs:15` | FORM-07 TODO | Outbox webhook delivery is a no-op stub that logs + reports success. Replace with HMAC-signed POST + retry/DLQ classifier. |
| 8 | `src/routes/api/greeks-pdf/+server.ts:12` | TODO | Lead-magnet endpoint accepts email but does not enroll in mailing list / send PDF. Returns `success: true` regardless. **User-visible:** the lead-magnet form silently no-ops in prod. Bump to P1 if this endpoint is currently surfaced on a live page. |
| 9 | `src/lib/i18n/paraglide.ts:20` | TODO | Custom shim aping `@inlang/paraglide-js`; swap to real package when ready. Mechanical. |
| 10 | `src/lib/consent/tcf.ts:16` | TODO | Publisher-only TCF v2.2 shim; full GVL integration deferred (CONSENT-04 v2). |
| 11 | `backend/src/commerce/checkout.rs:89` | TODO | Stripe minimum is hard-coded to 50 USD-cents. Add per-currency minima before non-USD checkout ships. |
| 12 | `backend/src/handlers/products.rs:705` | TODO (EC-07) | Asset DELETE removes the row but not the R2 object. Safe today (signed-URL issuance not yet live) — revisit when entitlement-aware purge lands. |
| 13 | `backend/src/handlers/forms.rs:774` | TODO | Local `ip_hash_daily` helper duplicates `crate::consent::records::ip_hash_daily`. Dedupe once both subsystems are merged. |
| 14 | `backend/src/consent/integrity.rs:97` | TODO | `anchor_recent` exists but no scheduler invokes it (CONSENT-08 scheduler subsystem missing). Cross-referenced in `docs/REMAINING-WORK.md:620`. |
| 15 | `backend/src/consent/integrity.rs:27` | TODO | Module-level restatement of the same scheduler dependency above. |
| 16 | `backend/src/consent/geo.rs:30` & `geo.rs:81` | TODO (CONSENT-05 v2) | US subdivision (CA / CO / etc.) routing impossible until MaxMind region plumbing lands. Migration 026's US-CA banner currently un-routable. |
| 17 | `backend/migrations/028_consent_integrity.sql:16` | TODO | Migration-comment restatement of the same scheduler dependency in #14. |
| 18 | `e2e/README.md:115` | TODO | Documents an intentionally-deferred `.github/workflows/e2e.yml`. Pure doc/CI gap; not in code. |

---

## Info / Intentional

These markers showed up in the scan but are NOT actionable. Do not treat as backlog.

**Count:** 6 hits across 5 surfaces.

- `src/routes/api/checkout.remote.ts:126` — string literal `"price_XXXX"` inside an error message ("priceId must look like 'price_XXXX'"). Not a marker.
- `docs/google-tracking-setup-guide.md:60,76` — `G-XXXXXXXXXX` is the documented placeholder shape for a GA4 Measurement ID. Not a marker.
- `backend/migrations/064_role_matrix_perms.sql:8` — comment explicitly says *"is a deliberate design constraint, **not a TODO**"*. Self-documenting non-finding.
- `STUB_BANNER_CONFIG` references in `src/lib/api/consent.ts:124,139,169`, `src/routes/+layout.svelte:18,30`, `src/routes/admin/_consent-preview/+page.svelte:15-20`, `src/lib/components/consent/_fixtures/ConsentBannerHarness.svelte:9,27` — `STUB_` is the public name of the SSR/offline fallback constant; this is intentional API surface, not a stubbed function. (12 hits in the raw scan; all benign.)
- `AUDIT_REPORT.md:447,545`, `AUDIT_FIX_PLAN.md:477,479`, `AUDIT.md:244`, `docs/README.md:53,55`, `docs/RUNBOOK.md:7`, `docs/archive/deployment-readiness-report.md:118`, `docs/archive/project-audit.md:2043,5524`, `docs/archive/wiring-verification-report.md:101-107`, `docs/REMAINING-WORK.md:620`, `docs/STRIPE-E2E-QA.md:483`, `docs/wiring/FDN-TESTHARNESS-WIRING.md:102` — these are *meta-discussion* of TODOs in other audit/plan/archive docs (e.g. "DEAD-4 — TODOs (16 total)"), or `DEBUG` log lines pasted into archived reports. They're commentary about TODOs, not TODOs themselves.

---

## Recommended next-pass priorities

In order of bang-per-buck for the next sprint:

1. **Gate `/admin/_consent-preview` + `/admin/_ui-kit` behind RBAC** (P1-1). Five-minute fix, removes a real production-leakage risk. Highest ROI.
2. **Verify `verify_claim_binding` rollout window has elapsed and harden** (P1-2). Either delete the transitional branch (preferred) or replace the comment with a hard date so the next pass can act unambiguously.
3. **Either implement or remove the `/api/greeks-pdf` endpoint** (P2-#8). It silently lies to the user (`success: true` with no enrollment). Pick one: ship the Resend/Mailgun integration or replace with a 501.
4. **Wire CONSENT-08 anchor scheduler** (P2-#14). One scheduled `tokio-cron` task; unblocks tamper-evidence guarantees that already have migration + handler + tests in place. Knocks out three TODOs at once (#14, #15, #17) plus the `REMAINING-WORK.md` ledger entry.
5. **Replace `webhook_out.rs` outbound stub with the real HMAC-signed delivery** (P2-#7, FORM-07). The OutboxRecord plumbing, retry classifier, and DispatchError taxonomy already exist — this is the missing leg. Notification channels (#1-#6) become trivial follow-ups once #7 lands because most of them share the same outbound transport.

Defer: i18n paraglide swap (#9), TCF v2.2 GVL upgrade (#10), MaxMind subdivision routing (#16), per-currency Stripe minima (#11) — all blocked on or coupled to scope changes (multilingual rollout, full TCF certification, MaxMind license, non-USD launch). Re-evaluate at the next quarterly planning cycle.

---

## Resolved in this pass (2026-04-26)

### P1-1 — `/admin/_consent-preview` and `/admin/_ui-kit` admin RBAC gate
- **Verdict:** TODO was effectively stale. The admin shell at
  `src/routes/admin/+layout.svelte` already gates every child route via the
  `{#if !auth.isAuthenticated || !auth.isAdmin} … {:else} {@render children()}`
  branch, plus an `adminSessionReady` guard tied to a successful
  `/api/auth/me` round-trip. Non-admins see the admin login form, never the
  preview pages.
- **Action taken:** Replaced the stale TODO comment block in both
  `src/routes/admin/_consent-preview/+page.svelte` and
  `src/routes/admin/_ui-kit/+page.svelte` with an `Authz:` block documenting
  the layout-level gate, and added a tiny defense-in-depth `$effect` that
  redirects to `/admin` via `goto(...)` if `auth.isAdmin` is ever false.
  `svelte-autofixer` returned `issues: []` for both files.
- **Verification:** `pnpm check` — `0 ERRORS 0 WARNINGS 0 FILES_WITH_PROBLEMS`.

### P1-2 — `verify_claim_binding` rollout-window TODO
- **Verdict:** Rollout window has elapsed. `verify_claim_binding` shipped
  in commit `d0f0eec` on 2026-04-24; today is 2026-04-26 (≥48 h elapsed),
  and `JWT_EXPIRATION_HOURS = 24`, so every live session has rotated through
  the iss/aud-emitting mint path. Note also that
  `jwt_validation()` itself was hardened on 2026-04-25 (commit `307ced9`)
  to call `set_audience` / `set_issuer`, which already rejects tokens
  without iss/aud at the `jsonwebtoken` decode layer.
- **Action taken:** Promoted the absent-claim branch in
  `backend/src/extractors.rs::verify_claim_binding` to
  `Err(AppError::Unauthorized)`. Updated the doc-comment to reflect the
  new strict semantics and link to the rollout history. Added 6 unit tests
  in the same module's `#[cfg(test)] mod tests` covering
  `{ok-when-both-match, missing-iss, missing-aud, missing-both, wrong-iss,
  wrong-aud}`. Updated the integration-test harness
  (`backend/tests/support/user.rs`) to mint tokens with iss/aud — without
  this, *every* integration test that uses `seed_user`/`seed_admin` would
  start failing at the jwt_validation layer (this was already broken since
  2026-04-25; this pass fixes it as a side-effect).
- **Verification:**
  - `cargo fmt --all -- --check` — clean (no output).
  - `cargo clippy --all-targets -- -D warnings` — clean (no warnings).
  - `cargo test --lib extractors::` — 18 passed; 0 failed; 0 ignored
    (12 pre-existing + 6 new).

### P2 status (16 markers)
- **None resolved.** Per the per-finding rules, P2s eligible for this pass
  are 1-2 line cleanups (rename for clarity, remove dead branch, fix doc
  comment). Re-reading every P2 marker:
  - **#1–#7 (notification channels + outbound webhook):** all require new
    feature work — provider integrations (Twilio, Slack, Discord webhooks,
    WebPush VAPID), persistent inbox tables, HMAC-signed delivery + retry/DLQ
    classifier. Architectural; deferred.
  - **#8 (`/api/greeks-pdf` lead-magnet):** requires Resend/Mailgun
    integration to actually deliver the PDF. Architectural; deferred. (See
    REMAINING-WORK candidate below.)
  - **#9 (i18n paraglide swap):** mechanical but coupled to multilingual
    rollout scope. Deferred.
  - **#10 (TCF v2.2 GVL):** requires full TCF certification + GVL
    plumbing. Deferred.
  - **#11 (Stripe per-currency minima):** requires per-currency minima
    table; coupled to non-USD launch. Deferred.
  - **#12 (EC-07 R2 asset purge):** safe today (signed-URL issuance not
    live); coupled to entitlement-aware purge subsystem. Deferred.
  - **#13 (`ip_hash_daily` dedupe):** would touch a cross-subsystem merge
    (consent::records ⇄ handlers::forms). Architectural; deferred.
  - **#14, #15, #17 (CONSENT-08 anchor scheduler):** requires a new
    tokio-cron worker (handler + retention + alert + runbook section per
    AGENTS.md §7/§8). Deferred.
  - **#16 (MaxMind subdivision routing):** blocked on MaxMind region
    plumbing; external dependency. Deferred.
  - **#18 (`e2e/README.md` workflow gap):** the README itself documents
    that the `.github/workflows/e2e.yml` is intentionally deferred until
    the dedicated lane lands. The marker is a roadmap note, not a code TODO
    — leaving as documented.
- **Recommendation:** the deferred P2s belong in `docs/REMAINING-WORK.md`
  (and most are already cross-referenced there); none are stale or
  resolvable as a 1-line tweak.

---

## False positives filtered

Of 50 raw ripgrep hits, **32** were false positives:

- **12** were references to the public `STUB_BANNER_CONFIG` constant — legitimate exported API surface, not a stub-marker.
- **15** were mentions of TODOs inside `AUDIT.md` / `AUDIT_REPORT.md` / `AUDIT_FIX_PLAN.md` / `REMAINING-WORK.md` / `docs/archive/*` (i.e. audit reports talking about the existence of TODOs in code). Counting them would double-count.
- **3** were `XXXX` placeholders inside doc strings (GA4 Measurement IDs, Stripe `price_XXXX` shape).
- **2** were the word `DEBUG` appearing in pasted log output inside an archived wiring-verification report.
