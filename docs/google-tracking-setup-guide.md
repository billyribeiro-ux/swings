# Google & Marketing Tracking — Setup Guide (Swings)

This document explains **what this project supports today**, what **“Google CAPI”** usually means, which **keys and IDs** each Google product needs, and **where you would configure them** (Vercel, Railway, local `.env`) if you add integrations.

It is **not** a substitute for Google’s official documentation; use it as a checklist and architecture map for this codebase.

---

## Table of contents

1. [What this repo has today](#1-what-this-repo-has-today)
2. [What people mean by “Google CAPI”](#2-what-people-mean-by-google-capi)
3. [Google Analytics 4 (GA4) — browser vs server](#3-google-analytics-4-ga4--browser-vs-server)
4. [Google Ads — conversions and API](#4-google-ads--conversions-and-api)
5. [Google reCAPTCHA](#5-google-recaptcha)
6. [Where to put secrets (Vercel vs Railway vs local)](#6-where-to-put-secrets-vercel-vs-railway-vs-local)
7. [If you implement server-side events — suggested architecture](#7-if-you-implement-server-side-events--suggested-architecture)
8. [Verification checklist](#8-verification-checklist)
9. [Official Google references](#9-official-google-references)

---

## 1. What this repo has today

| Capability | Status | Location / notes |
|------------|--------|-------------------|
| **First-party analytics** (page views, sessions, CTA impressions/clicks) | Implemented | Browser: `src/lib/analytics/AnalyticsBeacon.svelte`, `src/lib/analytics/cta.ts` → API `POST /api/analytics/events` → Postgres |
| **Admin analytics dashboard** | Implemented | `src/routes/admin/analytics/+page.svelte` → `GET /api/admin/analytics/summary`, revenue from `sales_events` |
| **Vercel Speed Insights** (Core Web Vitals–style performance) | Implemented | `src/routes/+layout.svelte` — `injectSpeedInsights()` from `@vercel/speed-insights/sveltekit` |
| **GA4 / gtag (browser pixel)** | **Not implemented** | No Measurement ID in `app.html` or layout |
| **GA4 Measurement Protocol (server → GA4)** | **Not implemented** | No `api_secret`, no MP `fetch` code |
| **Google Ads Conversion API / offline conversions** | **Not implemented** | No Ads API credentials or upload jobs |
| **Google Tag Manager (GTM)** | **Not implemented** | No GTM container snippet |
| **reCAPTCHA** | **Not implemented** | No site key / verify endpoint |

**Implication:** There is **no file or env var in this repo today** where you “paste Google CAPI keys” and get results. You must **add** the integration (code + env vars) first.

---

## 2. What people mean by “Google CAPI”

“CAPI” in ads/analytics circles often refers to **server-side** or **API-based** event delivery (Meta calls theirs *Conversions API*). Google’s closest concepts are:

| Product | Server-side mechanism | Typical “keys” |
|---------|----------------------|----------------|
| **GA4** | [Measurement Protocol](https://developers.google.com/analytics/devguides/collection/protocol/ga4) | Measurement ID `G-…` + **API secret** |
| **Google Ads** | [Google Ads API](https://developers.google.com/google-ads/api/docs/start) / offline conversions | Developer token, OAuth client ID/secret, refresh token, customer IDs, conversion action resource names |
| **Enhanced conversions** (Ads) | Hashed PII sent with conversions | Tied to Ads tagging / API — not a single “CAPI key” |

If someone says “enter my Google CAPI keys,” **clarify** whether they mean **GA4 MP**, **Google Ads API**, or **reCAPTCHA** — the credentials are different.

---

## 3. Google Analytics 4 (GA4) — browser vs server

### 3A. Browser tagging (gtag.js) — simplest for page views

**What you need**

- **Measurement ID**: `G-XXXXXXXXXX` (GA4 → Admin → Data streams → Web stream).

**Where it would go (if you add it)**

- Either inject the gtag snippet in `src/app.html`, or load it from `src/routes/+layout.svelte` with `svelte:head` / a small component.
- Use **environment-specific** IDs (e.g. separate GA4 properties for prod vs staging) via `import.meta.env.PUBLIC_GA_MEASUREMENT_ID` (you must define `PUBLIC_*` in Vercel and locally).

**Pros:** Fast to add; good for basic page_view.  
**Cons:** Ad blockers; less control for sensitive server-only events.

### 3B. GA4 Measurement Protocol (server → GA4)

**What you need** (same property, server credentials)

| Name | Example | Where to find |
|------|---------|----------------|
| **Measurement ID** | `G-XXXXXXXXXX` | GA4 → Admin → Data streams |
| **API secret** | Long random string | Same stream → **Measurement Protocol API secrets** |

**Endpoint (conceptually)**

- `POST https://www.google-analytics.com/mp/collect?measurement_id=G-...&api_secret=...`
- Or the **debug** endpoint for validation while building.

**Payload**

- JSON with `client_id` (or `app_instance_id` for apps), `events` array with `name` and `params`.

**Where secrets would live**

- **Never** expose `api_secret` in the browser bundle.
- Prefer **Vercel server** (`+server.ts` routes) or **Railway (Rust)** to call MP, using env vars only on the server.

**Correct results depend on**

- Valid `measurement_id` + `api_secret` pair from the **same** GA4 stream.
- Stable `client_id` if you want sessions to stitch (often mirror what gtag would use, or generate server-side with rules you document).
- Event names and parameters matching what you care about in GA4 (and [recommended events](https://support.google.com/analytics/answer/9267735) if you use them).

---

## 4. Google Ads — conversions and API

Server-side Google Ads tracking is **not** one key; it is usually:

| Credential | Purpose |
|------------|---------|
| **Developer token** | API access (approved for use) |
| **OAuth 2.0 client ID & secret** | App credentials |
| **Refresh token** | Long-lived access on behalf of a user with access to the Ads account |
| **Customer ID** (MCC / manager if used) | `123-456-7890` formatted string in API |
| **Conversion action** | Resource name or ID for the conversion you’re recording |

**Typical flows**

- **Offline conversions** (GCLID, order ID, etc.) uploaded via API.
- **Enhanced conversions** (hashed email/phone) per Google’s hashing rules.

**Where secrets would live**

- Only on a **secure server** (Railway backend or Vercel serverless), never in client JS.

This repo **does not** include Ads API client code; adding it is a dedicated project (dependencies, error handling, idempotency, privacy review).

---

## 5. Google reCAPTCHA

If you add bot protection to forms (login, contact, lead magnets):

| Key | Public? | Typical env name (your choice) |
|-----|---------|--------------------------------|
| **Site key** | Yes (browser) | e.g. `PUBLIC_RECAPTCHA_SITE_KEY` |
| **Secret key** | No | e.g. `RECAPTCHA_SECRET_KEY` (Vercel/Railway only) |

**Flow**

1. Frontend loads reCAPTCHA, gets a **token**.
2. Backend verifies token with Google’s **siteverify** endpoint using the **secret**.

Not implemented in Swings today.

---

## 6. Where to put secrets (Vercel vs Railway vs local)

### Local development

- Copy `.env.example` patterns; use a **local** `.env` (gitignored) for secrets.
- For any `PUBLIC_*` variable, Vite/SvelteKit exposes it to the **browser** — do not put secrets there.

### Vercel (SvelteKit frontend)

- **Dashboard:** Project → Settings → Environment Variables.
- **Browser-safe:** prefix with `PUBLIC_` (e.g. `PUBLIC_GA_MEASUREMENT_ID` if you add gtag).
- **Server-only:** no `PUBLIC_` prefix (e.g. `GA4_API_SECRET`, `RECAPTCHA_SECRET_KEY`).
- Redeploy after changing variables.

### Railway (Rust API backend)

- **Dashboard:** Service → Variables, or `railway variable set -s <service> KEY=value`.
- Use for **backend-only** secrets (e.g. if Rust calls GA4 MP or verifies reCAPTCHA — only if you implement it).

### Rule of thumb

| Variable type | Vercel | Railway | `PUBLIC_` in SvelteKit |
|---------------|--------|---------|-------------------------|
| GA4 Measurement ID (browser gtag) | Yes | No | Yes |
| GA4 API secret (MP) | Server route only | Optional (Rust) | **Never** |
| Google Ads OAuth / tokens | Server only | Server only | **Never** |
| reCAPTCHA site key | Yes | N/A | Yes |
| reCAPTCHA secret | Server only | Server only | **Never** |

---

## 7. If you implement server-side events — suggested architecture

This is **recommended** for Swings’ stack (Vercel + Railway) if you want GA4 MP or Ads uploads without exposing secrets.

1. **Define events** you care about (e.g. `purchase`, `sign_up`, `generate_lead`) and **where** they fire (Stripe webhook success, registration API, thank-you page load).
2. **Choose the sender**
   - **Option A:** SvelteKit `+server.ts` on Vercel (Node) calls GA4 MP with `fetch`.
   - **Option B:** Rust backend on Railway calls GA4 MP after trusted events (webhooks already hit Railway).
3. **Store secrets** only on that sender’s host.
4. **Pass minimal PII**; hash per Google’s docs for enhanced conversions.
5. **Log failures** and retry idempotently where possible (webhooks may redeliver).

**First-party analytics** in this repo remains useful for **your** admin dashboard; GA4/Ads are for **Google’s** reporting and ads optimization — they complement each other.

---

## 8. Verification checklist

After you add any Google integration:

- [ ] **GA4 DebugView** (if using MP debug endpoint or DebugView-enabled client) shows events.
- [ ] **Real-time** reports in GA4 show activity from a test browser.
- [ ] **Ads** test conversions (if applicable) appear in the conversion action’s diagnostics (per Google’s workflow).
- [ ] **No secrets** in Git, client bundles, or public env vars.
- [ ] **Consent** (if you use a CMP): wire tagging to consent mode per your legal/policy requirements.

---

## 9. Official Google references

- GA4 Measurement Protocol: https://developers.google.com/analytics/devguides/collection/protocol/ga4  
- GA4 recommended events: https://support.google.com/analytics/answer/9267735  
- Google Ads API start: https://developers.google.com/google-ads/api/docs/start  
- reCAPTCHA: https://developers.google.com/recaptcha  

---

## Quick reference — Swings analytics you already have

| Piece | Path / endpoint |
|-------|------------------|
| Page view beacon | `src/lib/analytics/AnalyticsBeacon.svelte` |
| CTA tracking | `src/lib/analytics/cta.ts`, used e.g. in `Pricing.svelte`, `Hero.svelte`, `FinalCta.svelte` |
| Ingest API | `POST /api/analytics/events` (Railway) |
| Admin summary | `GET /api/admin/analytics/summary` |
| Admin revenue series | `GET /api/admin/analytics/revenue` |
| Speed Insights | `src/routes/+layout.svelte` (`injectSpeedInsights`) |

---

*Last updated to reflect the Swings codebase layout. If you add GA4, GTM, Ads, or reCAPTCHA, update this file with the exact env var names and file paths you introduce.*
