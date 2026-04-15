# Deployment Readiness Report

Generated: 2026-04-15T18:42:00Z

## Summary

- Total checks: 52
- Passed: 48
- Failed: 0
- Warnings: 4

## Section 1: vercel.json

- [✅ PASS] `vercel.json` exists in the project root.
- [✅ PASS] Contains a rewrite mapping `/api/:path*` to an external HTTPS URL.
- [⚠️ WARN] Destination uses a Railway-style host (`https://YOUR-SERVICE.up.railway.app/...`) and is **not** `onrender.com`, but **`YOUR-SERVICE` is still a placeholder**. Before production traffic, replace it with the real Railway public hostname (and keep it in sync with `API_URL` / Vercel `VITE_API_URL`).
- [✅ PASS] There is **no** `/uploads/:path*` rewrite (media is expected as absolute URLs).

**Full contents of `vercel.json`:**

```json
{
	"rewrites": [
		{
			"source": "/api/:path*",
			"destination": "https://YOUR-SERVICE.up.railway.app/api/:path*"
		}
	]
}
```

## Section 2: API Base URL Resolution

1. [`src/lib/api/resolvePublicApiBase.ts`](src/lib/api/resolvePublicApiBase.ts) — confirmed behavior:
   - [✅ PASS] Production + browser → `if (params.browser && !params.dev) return ''`
   - [✅ PASS] Production + server + `VITE_API_URL` set → returns trimmed URL (no trailing slash)
   - [✅ PASS] Production + server + `VITE_API_URL` not set → falls through to `return ''`
   - [✅ PASS] Dev + browser → first branch false; then `if (params.dev) return params.browser ? '' : ...` → `''`
   - [✅ PASS] Dev + server → `'http://127.0.0.1:3001'`

2. [`src/lib/api/publicApiBase.ts`](src/lib/api/publicApiBase.ts) — [✅ PASS] Calls `resolvePublicApiBase` with `import.meta.env.VITE_API_URL`, `import.meta.env.DEV`, and `browser` from `$app/environment`.

3. Tests — [✅ PASS] `pnpm vitest run src/lib/api/resolvePublicApiBase.test.ts` — **1 file, 6 tests passed** (exit code 0).

**Full contents of `resolvePublicApiBase.ts`:**

```typescript
/**
 * Pure resolver for tests and build-time behavior (no Svelte imports).
 *
 * Architecture (repo evidence):
 * - Frontend: `@sveltejs/adapter-vercel` (svelte.config.js)
 * - API: separate host (e.g. Render `backend/render.yaml`, `FRONTEND_URL` → Vercel)
 *
 * Production browser bundles must not fall back to `localhost` — that breaks Vercel users.
 * Set `VITE_API_URL` to the public API origin in the Vercel project (Production + Preview as needed).
 */
export function resolvePublicApiBase(params: {
	viteApiUrl: string | undefined;
	dev: boolean;
	browser: boolean;
}): string {
	// In production browsers, force same-origin requests so platform rewrites/proxies
	// can handle cross-origin API routing without CORS fragility.
	if (params.browser && !params.dev) {
		return '';
	}

	const raw = params.viteApiUrl;
	if (raw != null && String(raw).trim() !== '') {
		return String(raw).trim().replace(/\/$/, '');
	}
	if (params.dev) {
		return params.browser ? '' : 'http://127.0.0.1:3001';
	}
	// Production / preview build without VITE_API_URL: same-origin `/api/...` (only valid if you add edge rewrites).
	return '';
}
```

**Full contents of `publicApiBase.ts`:**

```typescript
import { browser } from '$app/environment';
import { resolvePublicApiBase } from '$lib/api/resolvePublicApiBase';

/**
 * Base URL for the Rust HTTP API (no trailing slash).
 *
 * - **Dev (browser):** `""` — Vite proxies `/api` → `vite.config.ts` target.
 * - **Dev (SSR):** `http://127.0.0.1:3001` — Node cannot use the browser proxy.
 * - **Production browser:** same-origin `/api` (Vercel rewrites to the Rust API). No CORS for page navigations.
 * - **Production SSR / build:** set `VITE_API_URL` so server-side loaders can reach the API host directly.
 */
export function getPublicApiBase(): string {
	return resolvePublicApiBase({
		viteApiUrl: import.meta.env.VITE_API_URL,
		dev: import.meta.env.DEV,
		browser
	});
}
```

## Section 3: Server-Side Data Fetching

| File | API base source | `fetch` source | Issues |
|------|-----------------|----------------|--------|
| [`src/routes/blog/+page.server.ts`](src/routes/blog/+page.server.ts) | `getPublicApiBase()` via module `const API = getPublicApiBase()` | `load` destructuring `{ url, fetch }` | [✅ PASS] None |
| [`src/routes/blog/[slug]/+page.server.ts`](src/routes/blog/[slug]/+page.server.ts) | Same | `load` destructuring `{ params, fetch }` | [✅ PASS] None |

Both use SvelteKit `fetch` from the load argument (not a global/custom import).

## Section 4: SvelteKit API Routes (Stay on Vercel)

| File | External service | `$env/dynamic/private` for secrets? | Notes |
|------|------------------|--------------------------------------|-------|
| [`src/routes/api/create-checkout-session/+server.ts`](src/routes/api/create-checkout-session/+server.ts) | **Stripe** (`stripe.checkout.sessions.create`) | [✅ PASS] Yes — `env.STRIPE_SECRET_KEY` from `$env/dynamic/private`; public URLs via `$env/dynamic/public` | Does not call the Rust API |
| [`src/routes/api/greeks-pdf/+server.ts`](src/routes/api/greeks-pdf/+server.ts) | **None** (stub: logs email, TODO for email/list provider) | [⚠️ WARN] No — no secrets module; no real integration yet | [✅ PASS] Does not reference the Rust API |

## Section 5: Vite Dev Proxy

- [✅ PASS] Proxy under `server.proxy`
- [✅ PASS] Maps `/api` to `http://127.0.0.1:3001`
- [✅ PASS] `changeOrigin: true`

**Proxy block from [`vite.config.ts`](vite.config.ts):**

```typescript
	server: {
		proxy: {
			// Rust API (pnpm dev + cargo run in backend). Browser uses same-origin /api via getPublicApiBase().
			'/api': {
				target: 'http://127.0.0.1:3001',
				changeOrigin: true
			}
		}
	},
```

## Section 6: SvelteKit Adapter

- [✅ PASS] Imports `@sveltejs/adapter-vercel`
- [✅ PASS] Adapter configured with `runtime: 'nodejs22.x'` (plus `serviceWorker` / `prerender` settings)

**Full [`svelte.config.js`](svelte.config.js):**

```javascript
import process from 'node:process';
import adapter from '@sveltejs/adapter-vercel';

/**
 * Service worker registration policy (must stay aligned with `src/hooks.client.ts` + `$lib/client/service-worker-dev-policy`).
 * @see https://svelte.dev/docs/kit/configuration#serviceWorker
 */
const allowServiceWorkerInDev =
	process.env.PUBLIC_SERVICE_WORKER_IN_DEV === '1' ||
	process.env.PUBLIC_SERVICE_WORKER_IN_DEV === 'true';

const registerServiceWorker = process.env.NODE_ENV === 'production' || allowServiceWorkerInDev;

/** @type {import('@sveltejs/kit').Config} */
const config = {
	kit: {
		adapter: adapter({
			runtime: 'nodejs22.x'
		}),
		serviceWorker: {
			register: registerServiceWorker
		},
		prerender: {
			handleHttpError: 'warn',
			handleMissingId: 'warn',
			crawl: true,
			entries: ['/', '/about', '/courses', '/blog', '/pricing', '/pricing/monthly', '/pricing/annual']
		}
	}
};

export default config;
```

## Section 7: Backend Dockerfile

- [✅ PASS] Multi-stage: `rust:latest` builder + `debian:bookworm-slim` runtime
- [✅ PASS] Migrations copied: `COPY --from=builder /app/migrations ./migrations`
- [✅ PASS] `EXPOSE 3001`
- [✅ PASS] `CMD ["./swings-api"]`

[⚠️ WARN] Railway sets `PORT` dynamically; the image documents 3001 but the app reads `PORT` from env ([`backend/src/config.rs`](backend/src/config.rs)) — correct for Railway; no change required.

## Section 8: Backend Config — Production Guards

- [✅ PASS] `assert_production_ready()` exists in [`backend/src/config.rs`](backend/src/config.rs) and returns early unless `APP_ENV` is `production`; otherwise it builds a `missing` list and **panics** if non-empty.

**Variables / checks when `APP_ENV=production`:**

| Check | Name(s) |
|-------|---------|
| Non-empty config field | `DATABASE_URL` |
| Non-empty config field | `JWT_SECRET` |
| Non-empty config field | `API_URL` |
| Non-empty config field | `FRONTEND_URL` |
| Non-empty config field | `STRIPE_SECRET_KEY` |
| Non-empty config field | `STRIPE_WEBHOOK_SECRET` |
| Env var non-empty | `ADMIN_EMAIL` |
| Env var non-empty | `ADMIN_PASSWORD` |
| R2 bundle | `R2_ACCOUNT_ID`, `R2_ACCESS_KEY_ID`, `R2_SECRET_ACCESS_KEY`, `R2_BUCKET_NAME`, `R2_PUBLIC_URL` (must all succeed for `R2Storage::from_env()`) |

## Section 9: Backend Auth — Token Rotation

**Report: implemented** (see [`backend/src/handlers/auth.rs`](backend/src/handlers/auth.rs))

| Requirement | Status |
|-------------|--------|
| Login creates a new `family_id` per session | [✅ PASS] `generate_tokens` uses `let family_id = Uuid::new_v4()` before `store_refresh_token` (also used for register) |
| Refresh: if `used` is true, revoke family | [✅ PASS] `if stored.used { db::delete_refresh_tokens_by_family(...); return Err(...) }` |
| Refresh marks old token used before issuing new | [✅ PASS] `db::mark_refresh_token_used(&state.db, stored.id).await?` then new refresh stored |
| New refresh uses same `family_id` | [✅ PASS] `store_refresh_token(..., stored.family_id, false)` |

## Section 10: Backend Rate Limiting

[`backend/src/middleware/rate_limit.rs`](backend/src/middleware/rate_limit.rs) — all three layers exist:

| Layer | Burst | Refill period | Effective steady rate (comment in code) |
|-------|-------|---------------|----------------------------------------|
| `login_layer()` | 5 | 12s | ~5 requests/minute per IP |
| `register_layer()` | 10 | 360s | 10 requests/hour per IP |
| `forgot_password_layer()` | 3 | 1200s | 3 requests/hour per IP |

[`backend/src/handlers/auth.rs`](backend/src/handlers/auth.rs) router — [✅ PASS] `/register` uses `register_layer()`, `/login` uses `login_layer()`, `/forgot-password` uses `forgot_password_layer()`.

## Section 11: Webhook Idempotency

[`backend/src/handlers/webhooks.rs`](backend/src/handlers/webhooks.rs):

- [✅ PASS] Before handling, `db::try_claim_stripe_webhook_event(&state.db, event_id, event_type)` claims the event.
- [✅ PASS] If claim returns `Ok(false)` (duplicate), responds **`200 OK`** via `return StatusCode::OK` without re-processing.
- [✅ PASS] Old rows: ~1% of requests run `db::cleanup_old_stripe_webhook_events` (30-day retention per [`backend/src/db.rs`](backend/src/db.rs)).

[`backend/migrations/017_webhook_idempotency.sql`](backend/migrations/017_webhook_idempotency.sql):

- [✅ PASS] Table **`processed_webhook_events`** exists with **`event_id TEXT PRIMARY KEY`**.

## Section 12: Media Backend

[`backend/src/services/storage.rs`](backend/src/services/storage.rs):

- [✅ PASS] `MediaBackend` enum: `Local { upload_dir }` and `R2(R2Storage)`
- [✅ PASS] `resolve()` tries `R2Storage::from_env()` first, falls back to local with warning
- [✅ PASS] R2 only when all `R2_*` vars are set and non-empty (`from_env` rules)

[`backend/src/main.rs`](backend/src/main.rs):

- [✅ PASS] `let media_backend = services::MediaBackend::resolve(config.upload_dir.clone());`
- [✅ PASS] `mount_local_uploads = !(state.config.is_production() && state.media_backend.is_r2())` — `/uploads` **ServeDir** only when not (production **and** R2); in production with R2, local static uploads are not mounted.

## Section 13: Database Migrations

Ordered list under [`backend/migrations/`](backend/migrations/):

1. `001_initial.sql`
2. `002_blog.sql`
3. `003_password_resets.sql`
4. `004_media_title.sql`
5. `005_user_author_profile.sql`
6. `006_post_format.sql`
7. `007_post_meta.sql`
8. `008_media_focal.sql`
9. `009_analytics.sql`
10. `010_normalize_user_emails.sql`
11. `011_courses.sql`
12. `012_pricing_plans.sql`
13. `013_coupons.sql`
14. `014_analytics_enhanced.sql`
15. `015_popups.sql`
16. `016_blog_trash_meta.sql`
17. `017_webhook_idempotency.sql`
18. `018_refresh_token_families.sql`

- [✅ PASS] Files numbered **001** through **018**
- [✅ PASS] No gaps
- [✅ PASS] No duplicate numbers (18 distinct prefixes)

## Section 14: Build Verification

| Command | Result |
|---------|--------|
| `pnpm check` | [✅ PASS] `svelte-check found 0 errors and 0 warnings` |
| `pnpm lint` | [✅ PASS] `eslint .` (exit 0) |
| `pnpm build` | [✅ PASS] Vite build + `@sveltejs/adapter-vercel` done (exit 0) |
| `cd backend && cargo check` | [✅ PASS] exit 0 |
| `cd backend && cargo clippy -- -D warnings` | [✅ PASS] exit 0 |

Rust toolchain was available; no backend checks skipped.

## Section 15: Environment Variable Inventory

### Vercel (frontend + SvelteKit serverless)

| Variable | Required | Purpose |
|----------|----------|---------|
| `VITE_API_URL` | Recommended (SSR/blog/sitemap) | Public Railway API origin for server-side `fetch` to Rust API when not relying on same-origin relative URLs |
| `PUBLIC_APP_URL` | Required for checkout UX | Base URL for Stripe success/cancel redirects in [`create-checkout-session`](src/routes/api/create-checkout-session/+server.ts) |
| `STRIPE_SECRET_KEY` | Required for checkout | Stripe server secret in SvelteKit API route (`$env/dynamic/private`) |
| `PUBLIC_STRIPE_MONTHLY_PRICE_ID` | Required for monthly pricing flow | Stripe Price ID (client/pricing pages) |
| `PUBLIC_STRIPE_ANNUAL_PRICE_ID` | Required for annual pricing flow | Stripe Price ID |
| `PUBLIC_SERVICE_WORKER_IN_DEV` | Optional | Enable SW in dev ([`svelte.config.js`](svelte.config.js)) |

*Note: Stripe **webhook** secret and subscription sync live on the **Railway** Rust service, not this table.*

### Railway (backend)

| Variable | Required | Purpose |
|----------|----------|---------|
| `DATABASE_URL` | Yes | Postgres connection |
| `JWT_SECRET` | Yes | JWT signing |
| `JWT_EXPIRATION_HOURS` | Optional (default 24) | Access token lifetime |
| `REFRESH_TOKEN_EXPIRATION_DAYS` | Optional (default 30) | Refresh token lifetime |
| `PORT` | Optional (default 3001) | Listen port (Railway often overrides) |
| `FRONTEND_URL` | Yes (production guard) | CORS fallback / app links |
| `CORS_ALLOWED_ORIGINS` | Recommended | Explicit allowed origins (comma-separated); falls back to `FRONTEND_URL` |
| `API_URL` | Yes (production guard) | Public API base URL (emails, links, webhooks) |
| `APP_URL` | Optional | Default `http://localhost:5173` in code |
| `APP_ENV` | Optional (`development` default) | Set `production` to enable strict startup checks |
| `STRIPE_SECRET_KEY` | Yes if `APP_ENV=production` | Stripe API from Rust |
| `STRIPE_WEBHOOK_SECRET` | Yes if `APP_ENV=production` | Verify Stripe webhooks |
| `ADMIN_EMAIL` | Yes if `APP_ENV=production` | Admin seed |
| `ADMIN_PASSWORD` | Yes if `APP_ENV=production` | Admin seed |
| `ADMIN_NAME` | Optional | Admin display name |
| `UPLOAD_DIR` | Optional (`./uploads`) | Local media directory |
| `R2_ACCOUNT_ID` | Yes if `APP_ENV=production` (current code) | R2 S3 endpoint |
| `R2_ACCESS_KEY_ID` | Yes if `APP_ENV=production` | R2 credentials |
| `R2_SECRET_ACCESS_KEY` | Yes if `APP_ENV=production` | R2 credentials |
| `R2_BUCKET_NAME` | Yes if `APP_ENV=production` | R2 bucket |
| `R2_PUBLIC_URL` | Yes if `APP_ENV=production` | Public base URL for R2 objects |
| `SMTP_HOST` | Optional | Mail relay (Postmark: `smtp.postmarkapp.com`) |
| `SMTP_PORT` | Optional | Typically 587 |
| `SMTP_USER` | Optional* | If empty, email service disabled in [`main.rs`](backend/src/main.rs) |
| `SMTP_PASSWORD` | Optional* | Postmark server token |
| `SMTP_FROM` | Optional | From address |

\*For production email, treat SMTP vars as required operationally even if the binary can start without them.

### Postmark (DNS)

Typical records when Postmark verifies a sender domain (exact values come from the Postmark dashboard for your domain):

| Record type | Name (example) | Purpose |
|-------------|----------------|---------|
| TXT | `@` or delegated subdomain | **DKIM** — Postmark signing keys |
| CNAME | `pm-bounces` or return-path host | **Return-Path** / bounce domain |
| TXT | `@` | **SPF** — authorize Postmark to send for the domain |

## Action Items

1. [⚠️ WARN] Replace `YOUR-SERVICE` in [`vercel.json`](vercel.json) with the live Railway hostname before production.
2. [⚠️ WARN] Ensure Vercel `VITE_API_URL` and Railway `API_URL` stay aligned with that hostname.
3. [⚠️ WARN] [`greeks-pdf`](src/routes/api/greeks-pdf/+server.ts) is a stub — no real email or PDF delivery; implement or disable the feature for production expectations.
4. [⚠️ WARN] With `APP_ENV=production`, the backend **requires** full R2 configuration today ([`assert_production_ready`](backend/src/config.rs)); plan R2 (or a deliberate non-production `APP_ENV` strategy) before go-live if you expected disk-only media under production mode.
