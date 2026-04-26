# Wiring Verification Report

Generated: 2026-04-15T19:57:11Z

## Summary

- **Failed:** 0 (no ❌ FAIL outcomes)
- **Warnings:** Multiple ⚠️ items (CLI limitations, optional integrations, redirect behavior) — see sections below
- **Skipped:** 0 (Vercel CLI was available; production domain tested with redirect follow-up)

---

## Section 1: Secrets Rotation

| Check                                  | Status  | Notes                                                                                                                                                                                     |
| -------------------------------------- | ------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `railway variables get JWT_SECRET`     | ⚠️ WARN | Railway CLI v4.37.x has no `variable get` / `variables get` subcommand (`unrecognized subcommand 'get'`). Verification used `railway variable list -s swings -k` instead (see Section 3). |
| `JWT_SECRET` present and non-empty     | ✅ PASS | Confirmed present; value not reproduced here.                                                                                                                                             |
| `ADMIN_PASSWORD` present and non-empty | ✅ PASS | Confirmed present; value not reproduced here.                                                                                                                                             |

**Report:** Secrets exist and are set (non-empty).

**Rotation note:** It is not possible in this run to prove that current values differ from any secrets that may have appeared in earlier terminal or chat logs. For a strict rotation guarantee, set new values in Railway again and treat any previously exposed material as compromised.

---

## Section 2: Railway Upload Volume

| Check                     | Status  | Notes                                                                                                 |
| ------------------------- | ------- | ----------------------------------------------------------------------------------------------------- |
| Volume for `/app/uploads` | ✅ PASS | `railway volume list --json` shows one volume on service `swings` with `"mountPath": "/app/uploads"`. |
| Mount path exact          | ✅ PASS | `/app/uploads`                                                                                        |

---

## Section 3: Railway Environment Variables

Source: `railway variable list -s swings -k` (names and values were inspected for this report; **secret values are not copied below**).

| Variable               | Expected                                             | Status  | Notes                                                                           |
| ---------------------- | ---------------------------------------------------- | ------- | ------------------------------------------------------------------------------- |
| `DATABASE_URL`         | Exists, non-empty                                    | ✅ PASS | Present; not printed (contains credentials).                                    |
| `JWT_SECRET`           | Exists, non-empty                                    | ✅ PASS | Present; not printed.                                                           |
| `APP_ENV`              | `development` or `production`                        | ✅ PASS | **`development`**                                                               |
| `ADMIN_EMAIL`          | Non-empty                                            | ✅ PASS | Present; not printed.                                                           |
| `ADMIN_PASSWORD`       | Non-empty                                            | ✅ PASS | Present; not printed.                                                           |
| `ADMIN_NAME`           | Exists                                               | ✅ PASS | Present; not printed.                                                           |
| `PORT`                 | Exists                                               | ✅ PASS | **`3001`**                                                                      |
| `API_URL`              | `https://swings-production.up.railway.app`           | ✅ PASS | **`https://swings-production.up.railway.app`**                                  |
| `FRONTEND_URL`         | `https://precisionoptionsignals.com` (not localhost) | ✅ PASS | **`https://precisionoptionsignals.com`**                                        |
| `CORS_ALLOWED_ORIGINS` | Contains `https://precisionoptionsignals.com`        | ✅ PASS | **`https://precisionoptionsignals.com,https://www.precisionoptionsignals.com`** |
| `UPLOAD_DIR`           | `/app/uploads`                                       | ✅ PASS | **`/app/uploads`**                                                              |

**Optional / expected gaps**

| Variable                | Status                | Notes                                                                 |
| ----------------------- | --------------------- | --------------------------------------------------------------------- |
| `STRIPE_SECRET_KEY`     | ⚠️ WARN               | Not set — expected for now.                                           |
| `STRIPE_WEBHOOK_SECRET` | ⚠️ WARN               | Not set — expected for now.                                           |
| `R2_*`                  | ✅ PASS (as intended) | Not set; using local uploads per app logs.                            |
| `SMTP_*`                | ⚠️ WARN               | Not set; email disabled until provider (e.g. Postmark) is configured. |

---

## Section 4: Railway API Health Check

| Endpoint                   | HTTP code                                                    | Status  |
| -------------------------- | ------------------------------------------------------------ | ------- |
| `GET /api/blog/posts`      | 200                                                          | ✅ PASS |
| Body shape                 | JSON with `data`, `total`, `page`, `per_page`, `total_pages` | ✅ PASS |
| `GET /api/blog/categories` | 200                                                          | ✅ PASS |
| `GET /api/pricing/plans`   | 200                                                          | ✅ PASS |
| `GET /api/courses/courses` | 200                                                          | ✅ PASS |

Sample `GET /api/blog/posts` body (truncated): `{"data":[],"total":0,"page":1,"per_page":20,"total_pages":0}`

---

## Section 5: Railway Logs

Command: `railway logs -s swings --deployment --lines 20 --latest`

| Criterion                      | Status  |
| ------------------------------ | ------- | ------------------------------------------ |
| No panic messages              | ✅ PASS |
| No `ERROR` level lines         | ✅ PASS |
| `Swings API listening on port` | ✅ PASS |
| `Admin user seeded`            | ✅ PASS |
| `SMTP_USER not configured`     | ⚠️ WARN | Expected until SMTP is configured.         |
| `R2 not configured`            | ⚠️ WARN | Expected while using local `/app/uploads`. |

**Last 20 lines (as captured):**

```
Mounting volume on: /var/lib/containers/railwayapp/bind-mounts/d9600cb5-9b0a-41ff-8acc-6dad257eef1a/vol_pzo2u2i58w48rj4w
Starting Container
2026-04-15T19:55:35.818609Z  INFO swings_api::db: Admin user seeded (password unchanged if email already existed): billy.ribeiro@icloud.com
2026-04-15T19:55:35.818655Z  WARN swings_api: SMTP_USER not configured — email sending is disabled
2026-04-15T19:55:35.818662Z  WARN swings_api::services::storage: R2 not configured (R2 configuration error: R2_ACCOUNT_ID not set); using local uploads at /app/uploads
2026-04-15T19:55:35.819664Z  INFO swings_api: Swings API listening on port 3001
2026-04-15T19:55:44.944544Z DEBUG request{method=GET uri=/api/blog/posts?page=1&per_page=12 version=HTTP/1.1}: tower_http::trace::on_request: started processing request
2026-04-15T19:55:44.948799Z DEBUG request{method=GET uri=/api/blog/categories version=HTTP/1.1}: tower_http::trace::on_request: started processing request
2026-04-15T19:55:45.025197Z DEBUG request{method=GET uri=/api/blog/posts?page=1&per_page=12 version=HTTP/1.1}: tower_http::trace::on_response: finished processing request latency=80 ms status=200
2026-04-15T19:55:45.025197Z DEBUG request{method=GET uri=/api/blog/categories version=HTTP/1.1}: tower_http::trace::on_response: finished processing request latency=76 ms status=200
2026-04-15T19:56:54.690795Z DEBUG request{method=GET uri=/api/blog/posts version=HTTP/1.1}: tower_http::trace::on_response: finished processing request latency=15 ms status=200
2026-04-15T19:56:54.911642Z DEBUG request{method=GET uri=/api/blog/categories version=HTTP/1.1}: tower_http::trace::on_request: started processing request
2026-04-15T19:56:55.112537Z DEBUG request{method=GET uri=/api/courses/courses version=HTTP/1.1}: tower_http::trace::on_response: finished processing request latency=16 ms status=200
```

---

## Section 6: vercel.json

| Check                        | Status  |
| ---------------------------- | ------- | ----------------------------------------------------- |
| Rewrite destination          | ✅ PASS | `https://swings-production.up.railway.app/api/:path*` |
| No `/uploads/:path*` rewrite | ✅ PASS | Only `/api/:path*` present.                           |

**Full file contents:**

```json
{
	"rewrites": [
		{
			"source": "/api/:path*",
			"destination": "https://swings-production.up.railway.app/api/:path*"
		}
	]
}
```

---

## Section 7: Vercel Environment Variables

Source: `vercel env ls` + `vercel env pull --environment production` (temporary file removed after inspection).

| Variable            | Expected                                   | Status  | Verified value                             |
| ------------------- | ------------------------------------------ | ------- | ------------------------------------------ |
| `VITE_API_URL`      | `https://swings-production.up.railway.app` | ✅ PASS | `https://swings-production.up.railway.app` |
| `PUBLIC_APP_URL`    | `https://precisionoptionsignals.com`       | ✅ PASS | `https://precisionoptionsignals.com`       |
| `STRIPE_SECRET_KEY` | Not set yet                                | ⚠️ WARN | Not listed in `vercel env ls` — expected.  |

---

## Section 8: Frontend → API Connectivity

| Check                      | Status  | Notes                                                                                                            |
| -------------------------- | ------- | ---------------------------------------------------------------------------------------------------------------- |
| `curl` status without `-L` | ⚠️ WARN | **`307`** to `https://precisionoptionsignals.com/api/blog/posts` — follow redirects for a true end-to-end check. |
| `curl -sSL` status         | ✅ PASS | **200** after redirects.                                                                                         |
| JSON body vs Railway       | ✅ PASS | Same empty-list shape: `{"data":[],"total":0,"page":1,"per_page":20,"total_pages":0}`                            |

---

## Section 9: CORS Verification

Command: `curl -s -D - -o /dev/null -H "Origin: https://precisionoptionsignals.com" -H "Access-Control-Request-Method: GET" -X OPTIONS https://swings-production.up.railway.app/api/blog/posts`

| Criterion                      | Status  | Actual                                                                          |
| ------------------------------ | ------- | ------------------------------------------------------------------------------- |
| Status code                    | ✅ PASS | **200**                                                                         |
| `access-control-allow-origin`  | ✅ PASS | `https://precisionoptionsignals.com`                                            |
| `access-control-allow-methods` | ✅ PASS | `GET,POST,PUT,DELETE,OPTIONS` (includes **GET, POST, PUT, DELETE** as required) |

---

## Section 10: Auth Endpoint Smoke Test

| Request                               | HTTP code | Expected | Status  |
| ------------------------------------- | --------- | -------- | ------- |
| `POST /api/auth/login` (invalid body) | 401       | 401      | ✅ PASS |
| `GET /api/auth/me` (no token)         | 401       | 401      | ✅ PASS |

---

## Action Items (Blocking — must fix before go-live)

_None. All checks that were marked ❌ FAIL resolved to pass._

---

## Action Items (Non-blocking — can fix later)

1. **Section 1:** Prefer documenting `railway variable list` (or dashboard) for secret presence; `variable get` is not available on this CLI version.
2. **Section 1:** If any secret was ever exposed in logs, rotate again in Railway and update dependent systems.
3. **Section 3:** Add `STRIPE_SECRET_KEY` / `STRIPE_WEBHOOK_SECRET` on Railway and Vercel when billing webhooks are ready.
4. **Section 3:** Configure `SMTP_*` (or provider-specific vars) for outbound email.
5. **Section 5:** SMTP and R2 warnings until those integrations are enabled.
6. **Section 7:** `STRIPE_SECRET_KEY` on Vercel when needed for server routes.
7. **Section 8:** Clients and monitors should follow redirects (`curl -L` / `-sSL`) when hitting apex `https://precisionoptionsignals.com` if it returns **307**.

---

## Next Steps

1. Keep Railway and Vercel URLs aligned (`API_URL`, `VITE_API_URL`, `vercel.json` destination).
2. When going to full production hardening, consider `APP_ENV=production` on Railway and satisfy backend production checks (Stripe, R2, etc.) per `backend/src/config.rs`.
3. Add Stripe and SMTP, then re-run this checklist.
4. Optionally add monitoring that uses redirect-following for the apex domain health check.
