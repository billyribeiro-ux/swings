# Deployment Guide — Go Live on Vercel + Railway

**Stack:** SvelteKit (Vercel) + Rust/Axum (Railway) + Postgres (Railway) + Postmark (SMTP)
**Monthly cost:** $40 ($20 Vercel Pro + $20 Railway Pro)
**Time to complete:** One afternoon
**Code changes required:** One file (`vercel.json`)

---

## Prerequisites

Before you start, have these accounts ready:

- [ ] **Vercel** account (sign up at vercel.com)
- [ ] **Railway** account (sign up at railway.com)
- [ ] **Postmark** account (sign up at postmarkapp.com — free, no credit card)
- [ ] **Stripe** account (you already have this)
- [ ] **Domain** registered and DNS accessible (Cloudflare, Namecheap, wherever)
- [ ] **GitHub** repo with your codebase pushed

---

## Step 1: Railway — Deploy the Rust API + Postgres

### 1.1 Create a Railway project

1. Go to railway.com → New Project
2. Name it (e.g., "precision-signals")

### 1.2 Add Postgres

1. Inside your project, click "New" → "Database" → "PostgreSQL"
2. Railway provisions a Postgres instance immediately
3. Click on the Postgres service → "Variables" tab → copy the `DATABASE_URL`
4. This is your production database. Migrations run automatically when the API starts.

### 1.3 Deploy the Rust API

1. In the same project, click "New" → "GitHub Repo"
2. Select your repo
3. Railway will detect the Dockerfile. **Important:** Set the root directory to `backend/` in the service settings (Settings → Source → Root Directory = `/backend`)
4. Railway will build and deploy the Docker image

### 1.4 Set environment variables

Click on your API service → "Variables" tab → add each variable:

| Variable | Value |
|---|---|
| `DATABASE_URL` | Click "Add Reference" → select the Postgres service's `DATABASE_URL` (Railway auto-links it) |
| `JWT_SECRET` | Generate one: run `openssl rand -hex 32` in your terminal |
| `JWT_EXPIRATION_HOURS` | `24` |
| `REFRESH_TOKEN_EXPIRATION_DAYS` | `30` |
| `APP_ENV` | `production` |
| `ADMIN_EMAIL` | Your admin email |
| `ADMIN_PASSWORD` | A strong password |
| `ADMIN_NAME` | `Billy Ribeiro` |
| `PORT` | `3001` |
| `API_URL` | (set this AFTER step 1.5 — you need the Railway domain first) |
| `FRONTEND_URL` | `https://yourdomain.com` (or your Vercel URL for now) |
| `CORS_ALLOWED_ORIGINS` | `https://yourdomain.com,https://www.yourdomain.com` |
| `STRIPE_SECRET_KEY` | `sk_live_...` (from Stripe dashboard) |
| `STRIPE_WEBHOOK_SECRET` | (set this AFTER step 5) |
| `UPLOAD_DIR` | `/app/uploads` |
| `SMTP_HOST` | `smtp.postmarkapp.com` |
| `SMTP_PORT` | `587` |
| `SMTP_USER` | Your Postmark Server API Token (from step 3) |
| `SMTP_PASSWORD` | Same Postmark Server API Token |
| `SMTP_FROM` | `noreply@yourdomain.com` |

### 1.5 Generate a public domain

1. Click on your API service → "Settings" → "Networking" → "Generate Domain"
2. Railway gives you something like `your-service-production.up.railway.app`
3. Go back to Variables and set `API_URL` = `https://your-service-production.up.railway.app`

### 1.6 Add a persistent volume for uploads

1. Click on your API service → "Settings" → "Volumes"
2. Mount path: `/app/uploads`
3. This ensures uploaded media survives redeployments

### 1.7 Optional: Custom domain

1. In "Settings" → "Networking" → "Custom Domain"
2. Add `api.yourdomain.com`
3. Railway gives you a CNAME target — add it to your DNS
4. Update `API_URL` to `https://api.yourdomain.com`

### 1.8 Verify the API is running

Once deployed, visit:

```
https://your-service-production.up.railway.app/api/blog/posts
```

You should get `{"data":[],"total":0,"page":1,"per_page":20,"total_pages":0}` — an empty blog listing. That means the API is running and migrations executed successfully.

---

## Step 2: Vercel — Deploy the SvelteKit Frontend

### 2.1 Import the project

1. Go to vercel.com → "Add New Project"
2. Import your GitHub repo
3. Vercel auto-detects SvelteKit
4. **Override install command:** `pnpm install`
5. Click "Deploy"

### 2.2 Set environment variables

Go to your Vercel project → "Settings" → "Environment Variables":

| Variable | Value | Environment |
|---|---|---|
| `VITE_API_URL` | `https://your-service-production.up.railway.app` (your Railway API URL) | Production + Preview |
| `PUBLIC_APP_URL` | `https://yourdomain.com` | Production |
| `PUBLIC_APP_URL` | `https://your-project.vercel.app` | Preview |
| `STRIPE_SECRET_KEY` | `sk_live_...` | Production |
| `STRIPE_SECRET_KEY` | `sk_test_...` | Preview |

### 2.3 Update `vercel.json`

In your repo, update `vercel.json` to point at Railway:

```json
{
  "rewrites": [
    {
      "source": "/api/:path*",
      "destination": "https://your-service-production.up.railway.app/api/:path*"
    }
  ]
}
```

Commit and push. Vercel auto-deploys.

### 2.4 Add custom domain

1. Vercel project → "Settings" → "Domains"
2. Add `yourdomain.com` and `www.yourdomain.com`
3. Vercel gives you DNS records — add them to your DNS provider
4. Once verified, Vercel auto-provisions SSL

### 2.5 Verify the frontend

Visit your Vercel URL. The landing page should load. Navigate to `/blog` — it should show an empty blog listing (no posts yet). Navigate to `/admin` — you should see the admin login screen.

---

## Step 3: Postmark — Set Up Email

### 3.1 Create account and server

1. Sign up at postmarkapp.com (free, no credit card)
2. Create a "Server" (e.g., "Precision Signals Production")
3. Copy the "Server API Token" — this is your SMTP username AND password

### 3.2 Verify your sending domain

1. In Postmark → "Sender Signatures" → "Add Domain"
2. Enter `yourdomain.com`
3. Postmark gives you DNS records to add:

| Type | Name | Value |
|------|------|-------|
| TXT | `20xxxxxxxx._domainkey.yourdomain.com` | (Postmark provides this — DKIM key) |
| CNAME | `pm-bounces.yourdomain.com` | `pm.mtasv.net` |

4. Add these records in your DNS provider
5. Click "Verify" in Postmark — both should go green

### 3.3 Verify it works

Once Railway has the SMTP env vars set (from Step 1.4), the backend will initialize the email service on next deploy. Check Railway logs for:

```
Email service initialized (SMTP: smtp.postmarkapp.com)
```

Test by registering a new user — they should receive a welcome email. Test forgot password — you should receive a reset email with a working link.

### 3.4 Free tier limits

Postmark free tier = 100 emails/month. That covers roughly 30-50 new signups (welcome email + headroom for password resets). When you approach that limit, upgrade to Basic ($15/month, 10,000 emails).

---

## Step 4: Update CORS on Railway

Now that you know your production Vercel domain, go back to Railway and update:

```
CORS_ALLOWED_ORIGINS=https://yourdomain.com,https://www.yourdomain.com
FRONTEND_URL=https://yourdomain.com
```

If you also want Vercel preview deployments to work, add the pattern:

```
CORS_ALLOWED_ORIGINS=https://yourdomain.com,https://www.yourdomain.com,https://your-project-*.vercel.app
```

Note: Wildcard CORS patterns may or may not work depending on your Axum CORS setup. If previews don't work, add the specific preview URL when needed.

---

## Step 5: Stripe Webhook

### 5.1 Create the webhook endpoint

1. Stripe Dashboard → Developers → Webhooks → "Add endpoint"
2. Endpoint URL: `https://your-service-production.up.railway.app/api/webhooks/stripe`
   (or `https://api.yourdomain.com/api/webhooks/stripe` if you set up a custom domain)
3. Select events:
   - `checkout.session.completed`
   - `customer.subscription.created`
   - `customer.subscription.updated`
   - `customer.subscription.deleted`
   - `invoice.payment_succeeded`
   - `invoice.payment_failed`
4. Click "Add endpoint"
5. Copy the "Signing secret" (starts with `whsec_`)

### 5.2 Set the webhook secret in Railway

Go to Railway → API service → Variables:

```
STRIPE_WEBHOOK_SECRET=whsec_...
```

Railway redeploys automatically.

### 5.3 Test the webhook

In Stripe Dashboard → Webhooks → your endpoint → "Send test webhook"

Select `checkout.session.completed` and send. Check Railway logs — you should see:

```
Stripe webhook received: checkout.session.completed (evt_test_...)
```

---

## Step 6: Smoke Test Everything

Run through this checklist manually:

### Public pages
- [ ] Landing page loads at `yourdomain.com`
- [ ] `/about` loads
- [ ] `/blog` loads (empty is fine)
- [ ] `/courses` loads
- [ ] `/pricing` loads
- [ ] `/pricing/monthly` and `/pricing/annual` load

### Auth
- [ ] Navigate to `/register` — create a test account
- [ ] Check your email — welcome email received from Postmark
- [ ] Navigate to `/login` — log in with the test account
- [ ] `/dashboard` loads with "Welcome back, [Name]"
- [ ] Log out works

### Admin
- [ ] Navigate to `/admin` — log in with `ADMIN_EMAIL` / `ADMIN_PASSWORD`
- [ ] Dashboard loads with stats (all zeros is fine)
- [ ] Create a blog post — save as draft, then publish
- [ ] Navigate to `/blog` — the post appears
- [ ] Navigate to `/blog/[slug]` — the post renders with author info
- [ ] Upload a media file in the admin blog editor — it saves and displays
- [ ] Create a watchlist with an alert
- [ ] Create a course with a module and lesson

### Stripe
- [ ] Click "Subscribe" on the pricing page
- [ ] Complete Stripe Checkout with test card `4242 4242 4242 4242`
- [ ] Redirected to `/success` page
- [ ] Check Railway logs — webhook received
- [ ] Check `/dashboard/account` — subscription shows as active

### Password reset
- [ ] Navigate to `/admin/forgot-password`
- [ ] Enter your email
- [ ] Check email — reset link received
- [ ] Click the link — `/admin/reset-password?token=...` loads
- [ ] Set new password — success message
- [ ] Log in with new password

---

## Step 7: Set Railway Spend Limit

1. Railway → Account → Billing → Usage
2. Set a monthly spend limit of `$50`
3. This prevents runaway costs if something goes wrong

---

## DNS Summary

If you're using Cloudflare (or any DNS provider), here are all the records you need:

| Type | Name | Value | Proxy |
|------|------|-------|-------|
| CNAME | `@` | `cname.vercel-dns.com` | DNS only |
| CNAME | `www` | `cname.vercel-dns.com` | DNS only |
| CNAME | `api` (optional) | `your-service.up.railway.app` | DNS only |
| TXT | `@` | Vercel domain verification value | N/A |
| TXT | DKIM selector | Postmark DKIM value | N/A |
| CNAME | `pm-bounces` | `pm.mtasv.net` | DNS only |

---

## What You Have When Done

| Layer | Service | Cost |
|-------|---------|------|
| Frontend + SSR | Vercel Pro | $20/month |
| API + Database | Railway Pro (Rust + Postgres) | $20/month |
| Email | Postmark Free | $0/month |
| Payments | Stripe | 2.9% + 30¢/txn |
| DNS | Your existing provider | $0 |
| **Total** | | **$40/month** |

### What's live:
- Marketing pages (landing, about, pricing, courses)
- Blog with full CMS (admin panel, categories, tags, revisions, media)
- Member registration, login, JWT auth with token rotation
- Stripe subscriptions (checkout, billing portal, cancel/resume)
- Member dashboard (watchlists, courses, account)
- Admin dashboard (stats, members, watchlists, courses, pricing, coupons, popups)
- Analytics ingest
- Email (welcome, password reset, subscription confirmations)
- Rate limiting on auth endpoints
- Webhook idempotency

### What to add later (when revenue justifies it):
- **Postmark Basic** ($15/month) — when free tier runs out
- **Cloudflare R2** ($0 free tier) — when media uploads outgrow Railway volume
- **Neon Postgres** ($5-20/month) — when you need branching/PITR
- **Custom domain on Railway** — `api.yourdomain.com` instead of `.up.railway.app`

---

## Emergency: If Something Breaks

### API won't start
Check Railway logs. The most common issue is missing env vars — `APP_ENV=production` enforces that all required variables are set. If any are missing, the process panics with a clear error listing what's missing.

### CORS errors in browser
Make sure `CORS_ALLOWED_ORIGINS` in Railway includes the exact origin the browser is on (scheme + host, no trailing slash). `https://yourdomain.com` and `https://www.yourdomain.com` are different origins.

### Stripe webhooks failing
Check Railway logs for the specific error. Common issues: wrong `STRIPE_WEBHOOK_SECRET`, or the endpoint URL in Stripe doesn't match the Railway URL. The webhook handler logs every event it processes.

### Email not sending
Check Railway logs for `Email service initialized`. If you see `SMTP_USER not configured — email sending is disabled`, the Postmark env vars aren't set. If you see `Failed to initialize email service`, the SMTP credentials are wrong.

### Database connection errors
Railway Postgres uses internal networking by default. Make sure `DATABASE_URL` uses the Railway-provided internal URL (not the external one) for lowest latency. The internal URL is auto-injected when you reference the Postgres service variable.

---

*Ship it.*
