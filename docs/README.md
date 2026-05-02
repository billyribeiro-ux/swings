# Documentation index

> **Last revised:** 2026-05-01
> **Maintainer:** swings core team
> **Convention:** every document in this tree begins with a `Last revised`
> header. If you ship a behavioural change, bump that header in the same
> commit. Stale docs are bugs.

This directory is the canonical home for all long-form documentation.
The root [`README.md`](../README.md) is the entry point and links here.

---

## Active documents

| Document                                                             | Purpose                                                                                                                                                                                                                    | Audience                          |
| -------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------- |
| [`RUNBOOK.md`](./RUNBOOK.md)                                         | Diagnosis + remediation for every Prometheus alert defined in [`../ops/prometheus/admin-alerts.rules.yml`](../ops/prometheus/admin-alerts.rules.yml). Includes SQL probes, decision trees, and benign-vs-hostile guidance. | On-call SRE, backend engineers    |
| [`DEPLOYMENT.md`](./DEPLOYMENT.md)                                   | End-to-end Go-Live guide: Vercel (frontend) + Railway (Rust API + Postgres) + Postmark (SMTP). Covers DNS, secrets, smoke tests, rollback.                                                                                 | Anyone shipping a new environment |
| [`INFRASTRUCTURE.md`](./INFRASTRUCTURE.md)                           | Full-stack topology: services, networks, secrets, identity boundaries, and data flows. The architectural source of truth.                                                                                                  | Architects, security reviewers    |
| [`SEO_RUNBOOK.md`](./SEO_RUNBOOK.md)                                 | SEO operating standard: meta-tag policy, sitemap discipline, structured data, and the `pnpm ci:seo` checks.                                                                                                                | Frontend, content, marketing      |
| [`ci.md`](./ci.md)                                                   | What CI runs, why each check exists, and how to debug a red build.                                                                                                                                                         | Anyone fighting a CI failure      |
| [`google-tracking-setup-guide.md`](./google-tracking-setup-guide.md) | GA4 + GTM + Search Console setup for the SvelteKit app.                                                                                                                                                                    | Marketing / data                  |
| [`stripe-local-testing.md`](./stripe-local-testing.md)               | Test-mode keys, `stripe listen`, DB price IDs, test cards, signup + checkout E2E on localhost.                                                                                                                             | Developers                        |
| [`stripe-pricing-models.md`](./stripe-pricing-models.md)             | Stripe Price IDs vs DB-driven `price_data`: pros/cons, enterprise norms, and Swings setup for each.                                                                                                                        | Engineers, RevOps                 |
| [`STRIPE-E2E-QA.md`](./STRIPE-E2E-QA.md)                             | Release-day QA drill for Stripe webhook expansion: test card matrix, manual checklist, verification queries.                                                                                                               | Engineers, QA                     |

---

## Repo-level references

| Document                               | Purpose                                                                               |
| -------------------------------------- | ------------------------------------------------------------------------------------- |
| [`../CHANGELOG.md`](../CHANGELOG.md)   | Dated record of every behaviour-affecting change. The canonical historical reference. |
| [`../REPO_STATE.md`](../REPO_STATE.md) | Most recent end-to-end audit snapshot — current status, open items, deletion ledger.  |

---

## Wiring guides ([`./wiring/`](./wiring/))

Integrator-facing docs describing how a self-contained backend
subsystem is hooked into the main binary. Read these when adding or
debugging a similar subsystem.

| Document                                                                 | Subsystem                                                       |
| ------------------------------------------------------------------------ | --------------------------------------------------------------- |
| [`wiring/FDN-06-WIRING.md`](./wiring/FDN-06-WIRING.md)                   | `backend/src/common/{money,geo,ua,html}` utility crate          |
| [`wiring/FDN-TESTHARNESS-WIRING.md`](./wiring/FDN-TESTHARNESS-WIRING.md) | In-process integration-test harness (`backend/tests/support/`)  |
| [`wiring/OBSERVABILITY-WIRING.md`](./wiring/OBSERVABILITY-WIRING.md)     | `backend/src/observability/` — tracing + Prometheus scaffolding |

---

## How to add or modify a doc

1. Pick the right home:
   - **Will an on-call engineer read this at 03:00?** → `RUNBOOK.md` or
     a new top-level doc in `docs/`.
   - **Is this how a subsystem is plugged into `main.rs`?** → a new
     `docs/wiring/<NAME>.md`.
   - **Is this a one-shot report or audit ledger?** Capture the result
     in [`../CHANGELOG.md`](../CHANGELOG.md) and, if it materially shifts
     the state of the repo, refresh [`../REPO_STATE.md`](../REPO_STATE.md)
     with evidence. Do not create one-shot ledger files in `docs/`.
2. Add a `> **Last revised:** YYYY-MM-DD` header at the very top.
3. Cross-link from this index and from the root [`README.md`](../README.md)
   if it's user-facing.
4. If the doc replaces an older one, delete the old file in the same
   commit. **No "v1 / v2" doc pairs** — they always drift.
