# Infrastructure Reference — Full Stack Deployment Architecture

**Service:** New SvelteKit + Rust/Axum Platform
**Date:** April 15, 2026
**Author:** Billy Ribeiro
**Status:** Production Architecture — Final

---

## Architecture Overview

```
Browser
  │
  ▼
Vercel (SvelteKit SSR + API Routes)          ← Frontend + BFF
  │
  ▼
Railway (Rust/Axum Binary)                    ← Backend API
  │
  ├──▶ Railway PostgreSQL 16                  ← Database (auto-linked)
  ├──▶ Cloudflare R2 (S3-Compatible)          ← Media Storage
  ├──▶ Stripe (Billing + Webhooks)            ← Payments
  └──▶ Resend (Transactional Email)           ← Mail provider

Cloudflare DNS                                ← DNS + CDN for media
```

**Data flow:** The browser NEVER talks directly to the Rust API. All requests go through SvelteKit's server-side routes (`+page.server.ts`, `+server.ts`) which proxy to the Rust API. The Rust API URL is never exposed to the client.

---

## 1. Frontend — Vercel Pro

| Detail                | Value                                                  |
| --------------------- | ------------------------------------------------------ |
| **Service**           | Vercel                                                 |
| **Plan**              | Pro                                                    |
| **Cost**              | $20/month                                              |
| **Framework**         | SvelteKit with `@sveltejs/adapter-vercel`              |
| **Package manager**   | pnpm (override in Vercel project settings)             |
| **Build machine**     | Turbo (30 vCPU, 60 GB RAM — default on Pro)            |
| **Bandwidth**         | 1 TB/month included                                    |
| **Edge requests**     | 10M/month included                                     |
| **Function duration** | Up to 800s with Fluid Compute                          |
| **Function memory**   | Up to 4 GB                                             |
| **Concurrent builds** | 12                                                     |
| **Daily deploys**     | 6,000                                                  |
| **Split routing**     | `false` (default — single function, fewer cold starts) |

### Vercel Environment Variables

| Variable            | Value                        | Scope           |
| ------------------- | ---------------------------- | --------------- |
| `PRIVATE_API_URL`   | `https://api.yourdomain.com` | Server only     |
| `PUBLIC_APP_URL`    | `https://yourdomain.com`     | Client + Server |
| `STRIPE_PUBLIC_KEY` | `pk_live_...`                | Client + Server |

### SvelteKit Configuration

```javascript
// svelte.config.js
import adapter from '@sveltejs/adapter-vercel';

export default {
	kit: {
		adapter: adapter({
			runtime: 'nodejs22.x',
			split: false
		})
	}
};
```

### Security Notes

- CVE-2026-27118 (SvelteSpill cache deception) — patched February 2026. Keep `@sveltejs/kit` and `@sveltejs/adapter-vercel` on latest versions.
- Skew protection — enable in Vercel project settings → Advanced.

---

## 2. Backend API — Railway Pro

| Detail              | Value                                                |
| ------------------- | ---------------------------------------------------- |
| **Service**         | Railway                                              |
| **Plan**            | Pro                                                  |
| **Cost**            | $20/month base + usage (includes $20 usage credit)   |
| **Runtime**         | Rust binary via Docker                               |
| **Framework**       | Axum 0.8.x                                           |
| **Database driver** | SQLx 0.8.x                                           |
| **Deploy method**   | GitHub push → auto-build from `backend/Dockerfile`   |
| **Region**          | US East (closest to Vercel)                          |
| **Max resources**   | 32 GB RAM, 32 vCPU (Pro limit)                       |
| **Networking**      | Public HTTPS domain auto-provisioned                 |
| **Spend limit**     | Set to $50/month (configurable in Railway dashboard) |

### Railway Environment Variables

| Variable                | Value                                         |
| ----------------------- | --------------------------------------------- |
| `DATABASE_URL`          | Reference Railway Postgres `DATABASE_URL`     |
| `JWT_SECRET`            | Strong random 64-char string                  |
| `APP_ENV`               | `production`                                  |
| `ADMIN_EMAIL`           | Admin email address                           |
| `ADMIN_PASSWORD`        | Strong admin password                         |
| `API_URL`               | `https://api.yourdomain.com`                  |
| `FRONTEND_URL`          | `https://yourdomain.com`                      |
| `CORS_ALLOWED_ORIGINS`  | `https://yourdomain.com,https://*.vercel.app` |
| `PORT`                  | `3001` (or Railway's `$PORT`)                 |
| `STRIPE_SECRET_KEY`     | `sk_live_...`                                 |
| `STRIPE_WEBHOOK_SECRET` | `whsec_...`                                   |
| `R2_ACCOUNT_ID`         | Cloudflare Account ID                         |
| `R2_ACCESS_KEY_ID`      | R2 API token access key                       |
| `R2_SECRET_ACCESS_KEY`  | R2 API token secret                           |
| `R2_BUCKET_NAME`        | Bucket name                                   |
| `R2_PUBLIC_URL`         | `https://media.yourdomain.com`                |
| `SMTP_HOST`             | `smtp.postmarkapp.com`                        |
| `SMTP_PORT`             | `587`                                         |
| `SMTP_USER`             | Postmark Server API Token                     |
| `SMTP_PASSWORD`         | Postmark Server API Token (same as user)      |
| `SMTP_FROM`             | `noreply@yourdomain.com`                      |

### Rust Dependencies (additions to existing Cargo.toml)

```toml
aws-sdk-s3 = "1"
aws-config = "1"
tower_governor = "0.6"
```

---

## 3. Database — Railway PostgreSQL

| Detail                 | Value                                                              |
| ---------------------- | ------------------------------------------------------------------ |
| **Service**            | Railway PostgreSQL                                                 |
| **Plan**               | Pro add-on (provisioned alongside the API service)                 |
| **Engine**             | PostgreSQL 16                                                      |
| **Cost**               | Included in Railway Pro base + usage (storage + RAM)               |
| **Networking**         | Private link from API service via `postgres.railway.internal`      |
| **TLS**                | `sslmode=require` (Railway terminates TLS at the network edge)     |
| **Region**             | US East (paired with the API service)                              |
| **Backups**            | Daily snapshots retained per Railway Pro policy                    |
| **Connection pooling** | None at the platform layer — sqlx pool sizes are tuned client-side |

### Connection String Format

```
postgres://postgres:<password>@postgres.railway.internal:5432/railway?sslmode=require
```

Use the **private** Railway-internal hostname inside the API service (no
public exposure). Keep `DATABASE_URL` as a Railway service-link reference
so credential rotation propagates automatically.

### sqlx Pool Tuning

The `db::create_pool` factory in `backend/src/db.rs` reads the
`PGPOOL_*` env vars (see `backend/.env.example`) and falls back to safe
defaults sized for a 2-vCPU container. Override only when ops metrics
demand it:

```rust
let pool = sqlx::postgres::PgPoolOptions::new()
    .max_connections(cfg.pgpool_max)        // default: 10
    .min_connections(cfg.pgpool_min)        // default: 0
    .acquire_timeout(cfg.pgpool_acquire_timeout)
    .idle_timeout(cfg.pgpool_idle_timeout)
    .max_lifetime(cfg.pgpool_max_lifetime)
    .connect(&cfg.database_url)
    .await?;
```

### Migrations

All 72 forward-only migrations in `backend/migrations/`
(versions `001–080`, gap-tolerant) run automatically on API startup via
`sqlx::migrate!()` in `backend/src/main.rs`. SHA-384 checksums are
enforced; editing a migration after it has been applied to any
environment fails the next boot.

---

## 4. Media Storage — Cloudflare R2

| Detail            | Value                                                                |
| ----------------- | -------------------------------------------------------------------- |
| **Service**       | Cloudflare R2                                                        |
| **Plan**          | Free tier (then pay-as-you-go)                                       |
| **Cost**          | $0/month (free tier: 10 GB storage, 1M Class A ops, 10M Class B ops) |
| **Storage rate**  | $0.015/GB-month (after free tier)                                    |
| **Egress**        | $0 (always free)                                                     |
| **API**           | S3-compatible                                                        |
| **CDN**           | Cloudflare global network (330+ data centers)                        |
| **Custom domain** | `media.yourdomain.com`                                               |

### Bucket Configuration

| Setting       | Value                                                            |
| ------------- | ---------------------------------------------------------------- |
| Bucket name   | `yourdomain-media`                                               |
| Public access | Enabled (or custom domain)                                       |
| CORS          | Allow `https://yourdomain.com`                                   |
| Cache-Control | `public, max-age=31536000, immutable` (set per-object on upload) |

### R2 API Token

Create in Cloudflare Dashboard → R2 → Manage R2 API Tokens:

| Permission          | Value                   |
| ------------------- | ----------------------- |
| Object Read & Write | Yes                     |
| Bucket scope        | `yourdomain-media` only |

### Storage Key Format

```
media/{year}/{month}/{8-char-uuid}-{sanitized-filename}
```

Example: `media/2026/04/a1b2c3d4-hero-image.webp`

### What Replaces What

| Before                                | After                                            |
| ------------------------------------- | ------------------------------------------------ |
| `UPLOAD_DIR` on local disk            | R2 bucket                                        |
| `ServeDir` at `/uploads/*`            | Direct access via `media.yourdomain.com`         |
| `media.storage_path` = local path     | `media.storage_path` = R2 key                    |
| `media.url` = `{API_URL}/uploads/...` | `media.url` = `https://media.yourdomain.com/...` |

---

## 5. Transactional Email — Postmark Basic

| Detail              | Value                                                         |
| ------------------- | ------------------------------------------------------------- |
| **Service**         | Postmark                                                      |
| **Plan**            | Basic                                                         |
| **Cost**            | $15/month                                                     |
| **Included emails** | 10,000/month                                                  |
| **Overage**         | $1.80/1,000 emails                                            |
| **Deliverability**  | ~98.5% inbox placement                                        |
| **Integration**     | SMTP relay (drop-in replacement for existing `lettre` config) |

### SMTP Configuration

| Setting  | Value                              |
| -------- | ---------------------------------- |
| Host     | `smtp.postmarkapp.com`             |
| Port     | `587` (TLS)                        |
| Username | Your Server API Token              |
| Password | Your Server API Token (same value) |
| From     | `noreply@yourdomain.com`           |

### DNS Records for Domain Verification

Postmark requires DKIM and Return-Path records. Add these in Cloudflare DNS:

| Type  | Name                                   | Value                  |
| ----- | -------------------------------------- | ---------------------- |
| TXT   | `20xxxxxxxx._domainkey.yourdomain.com` | (provided by Postmark) |
| CNAME | `pm-bounces.yourdomain.com`            | `pm.mtasv.net`         |

### Email Types Sent

| Email                     | Trigger                     | Stream        |
| ------------------------- | --------------------------- | ------------- |
| Welcome                   | User registration           | Transactional |
| Password reset            | Forgot password request     | Transactional |
| Email verification        | Registration / email change | Transactional |
| Subscription confirmation | Stripe checkout completed   | Transactional |
| Subscription cancellation | User cancels                | Transactional |

---

## 6. DNS — Cloudflare Free

| Detail       | Value                                      |
| ------------ | ------------------------------------------ |
| **Service**  | Cloudflare                                 |
| **Plan**     | Free                                       |
| **Cost**     | $0/month                                   |
| **Features** | DNS, DDoS protection, free SSL, page rules |

### DNS Records

| Type  | Name          | Value                          | Proxy Status           |
| ----- | ------------- | ------------------------------ | ---------------------- |
| CNAME | `@`           | `cname.vercel-dns.com`         | DNS only (gray cloud)  |
| CNAME | `www`         | `cname.vercel-dns.com`         | DNS only (gray cloud)  |
| CNAME | `api`         | `your-service.up.railway.app`  | DNS only (gray cloud)  |
| CNAME | `media`       | `your-bucket.r2.dev`           | Proxied (orange cloud) |
| TXT   | `@`           | Vercel domain verification TXT | N/A                    |
| TXT   | DKIM selector | Postmark DKIM value            | N/A                    |
| CNAME | `pm-bounces`  | `pm.mtasv.net`                 | DNS only               |

**Important:** Vercel and Railway manage their own TLS certificates. Do NOT proxy these through Cloudflare (use DNS-only / gray cloud). Only proxy the `media` subdomain through Cloudflare for CDN caching.

---

## 7. Payments — Stripe

| Detail               | Value                                            |
| -------------------- | ------------------------------------------------ |
| **Service**          | Stripe                                           |
| **Plan**             | Pay-as-you-go                                    |
| **Cost**             | 2.9% + $0.30 per transaction                     |
| **Rust crate**       | `async-stripe`                                   |
| **Webhook endpoint** | `https://api.yourdomain.com/api/webhooks/stripe` |
| **Webhook signing**  | `hmac` + `sha2` (existing implementation)        |

### Webhook Events to Subscribe

| Event                           | Purpose                  |
| ------------------------------- | ------------------------ |
| `checkout.session.completed`    | New subscription created |
| `customer.subscription.created` | Subscription record sync |
| `customer.subscription.updated` | Plan change / renewal    |
| `customer.subscription.deleted` | Cancellation             |
| `invoice.payment_succeeded`     | Payment confirmation     |
| `invoice.payment_failed`        | Failed payment handling  |

### Idempotency

New `processed_webhook_events` table prevents duplicate processing. Every webhook is checked against `event_id` before processing. Events older than 30 days are cleaned up automatically.

---

## 8. Auth Hardening

### Refresh Token Rotation

| Feature         | Implementation                                     |
| --------------- | -------------------------------------------------- |
| Family tracking | `family_id` column on `refresh_tokens` table       |
| Token rotation  | Each refresh issues new token, marks old as `used` |
| Reuse detection | If `used` token is presented, revoke entire family |
| Login           | Creates new `family_id` per session                |

### Rate Limiting

| Endpoint                    | Limit       | Window                |
| --------------------------- | ----------- | --------------------- |
| `/api/auth/login`           | 5 requests  | Per 60 seconds per IP |
| `/api/auth/register`        | 10 requests | Per 60 minutes per IP |
| `/api/auth/forgot-password` | 3 requests  | Per 60 minutes per IP |

Implementation: `tower-governor` with `SmartIpKeyExtractor` (reads `X-Forwarded-For` behind Railway's proxy).

### Session Cookies (SvelteKit Frontend Pattern)

The SvelteKit frontend manages browser sessions:

1. User submits login form → SvelteKit `+page.server.ts` action calls Rust API `/api/auth/login`
2. Rust API returns JWT + refresh token in response body
3. SvelteKit action sets `HttpOnly`, `Secure`, `SameSite=Strict` cookie containing the tokens
4. On subsequent requests, SvelteKit `+page.server.ts` reads cookie and forwards JWT as `Authorization: Bearer <token>` to Rust API
5. Browser never sees or stores the JWT directly — XSS cannot steal it

---

## 9. Domain Structure

| Domain                 | Destination               | Purpose              |
| ---------------------- | ------------------------- | -------------------- |
| `yourdomain.com`       | Vercel                    | SvelteKit frontend   |
| `www.yourdomain.com`   | Vercel (redirect to apex) | SEO canonical        |
| `api.yourdomain.com`   | Railway                   | Rust/Axum API        |
| `media.yourdomain.com` | Cloudflare R2 via CDN     | Uploaded media files |

---

## 10. Monthly Cost Summary

| Service                    | Plan          | Monthly Cost            |
| -------------------------- | ------------- | ----------------------- |
| Vercel                     | Pro           | $20                     |
| Railway                    | Pro           | $20 base + ~$5-15 usage |
| Railway PostgreSQL         | Pro add-on    | included in Railway Pro |
| Cloudflare R2              | Free tier     | $0                      |
| Cloudflare DNS             | Free          | $0                      |
| Resend                     | Free / Pro    | $0 (free tier) — $20    |
| Stripe                     | Pay-as-you-go | 2.9% + $0.30/txn        |
| **Total (infrastructure)** |               | **$40-60/month**        |

**At launch with light traffic: ~$60/month**
**With moderate traffic and active members: ~$70-80/month**
**Stripe fees are deducted from revenue, not a fixed infrastructure cost**

---

## 11. New Database Migrations

The full migration set is in `backend/migrations/`. See
`backend/README.md` for thematic groupings (auth, commerce, RBAC, DSAR, etc.).

---

## 12. Deployment Checklist

### One-Time Setup

- [ ] Create Vercel Pro account and import frontend repo
- [ ] Configure pnpm as install command in Vercel project settings
- [ ] Set Vercel environment variables (PRIVATE_API_URL, PUBLIC_APP_URL, STRIPE_PUBLIC_KEY)
- [ ] Create Railway Pro account and connect backend repo
- [ ] Set Railway spend limit to $50/month
- [ ] Provision the Railway PostgreSQL add-on next to the API service
- [ ] Reference the add-on's `DATABASE_URL` from the API service env
- [ ] Create Cloudflare account and add domain
- [ ] Create R2 bucket with public access
- [ ] Create R2 API token (Object Read & Write)
- [ ] Set all R2 env vars in Railway
- [ ] Create Postmark account and verify sending domain (DKIM + Return-Path DNS)
- [ ] Set Postmark SMTP env vars in Railway
- [ ] Configure Stripe webhook endpoint: `https://api.yourdomain.com/api/webhooks/stripe`
- [ ] Set Stripe env vars in Railway
- [ ] Set all remaining Railway env vars (JWT_SECRET, APP_ENV, ADMIN_EMAIL, etc.)
- [ ] Configure DNS records in Cloudflare (see Section 6)
- [ ] Enable Vercel domain and verify
- [ ] Attach custom domain to Railway service (`api.yourdomain.com`)
- [ ] Set up R2 custom domain (`media.yourdomain.com`)
- [ ] Deploy backend — verify migrations run
- [ ] Deploy frontend — verify SSR works
- [ ] Test login, registration, password reset
- [ ] Test Stripe checkout flow end-to-end
- [ ] Test media upload (verify R2 storage)
- [ ] Test webhook delivery (Stripe CLI or dashboard test event)
- [ ] Verify email delivery (Postmark activity log)

### Per-Deploy (Automated)

- [ ] `git push` to `main` → Vercel auto-deploys frontend
- [ ] `git push` to `main` → Railway auto-builds Dockerfile and deploys backend
- [ ] SQLx migrations run automatically on backend startup
- [ ] Preview deployments created for PRs (Vercel + Railway Pro)

---

## 13. What You Don't Need Yet

| Thing                 | Why Not                                     | When to Add                                                        |
| --------------------- | ------------------------------------------- | ------------------------------------------------------------------ |
| Redis                 | Traffic doesn't warrant a cache layer       | When you need session storage, job queues, or API response caching |
| Cargo workspace split | Compile times aren't a bottleneck yet       | When full rebuilds exceed 3-5 minutes                              |
| Multi-region          | Single US East region serves fine           | When you have significant international traffic                    |
| CDN for API responses | Cloudflare free plan in front of Railway    | When API response caching would help                               |
| Background job queue  | Stripe webhooks and analytics handle inline | When you need async email sends, report generation, etc.           |
| Monitoring/APM        | Railway built-in metrics + Prometheus       | When you need distributed tracing (OpenTelemetry → Grafana)        |
| Log aggregation       | Railway logs + structured `tracing` output  | When you need searchable log history beyond Railway's retention    |

---

_End of document._
