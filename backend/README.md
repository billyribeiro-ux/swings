# Swings API - Rust Backend

Axum + Tokio + SQLx + PostgreSQL backend for the Precision Options Signals membership platform.

## Prerequisites

- **Rust** (1.75+): https://rustup.rs
- **PostgreSQL** (15+): running locally or via Docker
- **sqlx-cli** (for migrations): `cargo install sqlx-cli --no-default-features --features postgres`

## Setup

```bash
# 1. Copy env file and fill in your values
cp .env.example .env

# 2. Create the database
createdb swings

# 3. Run migrations
sqlx migrate run

# 4. Start the server
cargo run
```

The API will start on `http://localhost:3001` by default.

## Environment Variables

| Variable | Required | Default | Description |
|---|---|---|---|
| `DATABASE_URL` | Yes | - | PostgreSQL connection string. For Neon, include `sslmode=require`. |
| `JWT_SECRET` | Yes | - | Secret key for JWT signing |
| `JWT_EXPIRATION_HOURS` | No | `24` | Access token lifetime |
| `REFRESH_TOKEN_EXPIRATION_DAYS` | No | `30` | Refresh token lifetime |
| `PORT` | No | `3001` | Server port |
| `FRONTEND_URL` | No | `http://localhost:5173` | Frontend URL for CORS |
| `CORS_ALLOWED_ORIGINS` | No | `FRONTEND_URL` | Comma-separated **exact** browser origins (scheme + host + port). No trailing `/`. List every origin browsers use (e.g. apex and `www` separately if both are live). |
| `API_URL` | No | `http://localhost:3001` | Public base URL of this API (used for absolute media URLs in local upload mode). |
| `UPLOAD_DIR` | No | `./uploads` | Local media directory when R2 is not configured. |
| `STRIPE_SECRET_KEY` | No | - | Stripe secret key |
| `STRIPE_WEBHOOK_SECRET` | No | - | Stripe webhook signing secret |
| `APP_ENV` | No | `development` | Set to `production` to enforce production-only guards (admin seed, R2, Stripe, JWT, URLs, etc.). |
| `ADMIN_EMAIL` | Dev optional / Prod required | - | Seed admin email |
| `ADMIN_PASSWORD` | Dev optional / Prod required | - | Seed admin password |
| `ADMIN_NAME` | No | `Admin` | Seed admin display name |
| `R2_ACCOUNT_ID` | Prod required | - | Cloudflare account id for S3-compatible endpoint |
| `R2_ACCESS_KEY_ID` | Prod required | - | R2 API token access key |
| `R2_SECRET_ACCESS_KEY` | Prod required | - | R2 API token secret |
| `R2_BUCKET_NAME` | Prod required | - | Bucket name |
| `R2_PUBLIC_URL` | Prod required | - | Public base URL for objects (no trailing `/`) |

### CORS

Allowed origins come **only** from `CORS_ALLOWED_ORIGINS` and/or the default of `FRONTEND_URL`. Preflight accepts **any** request header so browser extensions (Sentry, etc.) cannot break `OPTIONS`.

### Media (R2 vs local)

When all `R2_*` variables are set, uploads go to Cloudflare R2 and `/uploads` static serving is skipped in **production**. If any R2 variable is missing, the API logs a warning and stores files under `UPLOAD_DIR` (and serves them at `/uploads/...`).

### Rate limiting

`POST /api/auth/login`, `POST /api/auth/register`, and `POST /api/auth/forgot-password` are rate-limited per client IP using `tower-governor`. The stack uses **`SmartIpKeyExtractor`**, which reads `X-Forwarded-For` / `Forwarded` when present. Deploy behind a trusted reverse proxy (e.g. Railway) that sets those headers from the real client; otherwise clients could spoof IPs.

### Production checklist

With `APP_ENV=production`, the process **panics on startup** unless `DATABASE_URL`, `JWT_SECRET`, `ADMIN_EMAIL`, `ADMIN_PASSWORD`, `API_URL`, `FRONTEND_URL`, `STRIPE_SECRET_KEY`, `STRIPE_WEBHOOK_SECRET`, and **all** `R2_*` variables are set and non-empty, and R2 client initialization succeeds.

### Database pool (Neon)

The API uses SQLx pool options suited to serverless Postgres: bounded acquire time, idle timeout, and max connection lifetime. Keep `sslmode=require` in `DATABASE_URL` for Neon.

## API Endpoints

### Auth
- `POST /api/auth/register` - Create account
- `POST /api/auth/login` - Sign in
- `POST /api/auth/refresh` - Refresh tokens
- `GET /api/auth/me` - Get current user
- `POST /api/auth/logout` - Sign out (invalidates refresh tokens)

### Member (requires auth)
- `GET /api/member/profile` - Get profile
- `PUT /api/member/profile` - Update profile
- `GET /api/member/subscription` - Get subscription status
- `GET /api/member/watchlists` - List published watchlists
- `GET /api/member/watchlists/:id` - Get watchlist with alerts
- `GET /api/member/courses` - Get enrolled courses
- `PUT /api/member/courses/:id/progress` - Update course progress

### Admin (requires admin role)
- `GET /api/admin/stats` - Dashboard statistics
- `GET /api/admin/members` - List all members (paginated)
- `GET /api/admin/members/:id` - Get member details
- `PUT /api/admin/members/:id/role` - Update member role
- `DELETE /api/admin/members/:id` - Delete member
- `GET /api/admin/watchlists` - List all watchlists (paginated)
- `POST /api/admin/watchlists` - Create watchlist
- `GET /api/admin/watchlists/:id` - Get watchlist with alerts
- `PUT /api/admin/watchlists/:id` - Update watchlist
- `DELETE /api/admin/watchlists/:id` - Delete watchlist
- `GET /api/admin/watchlists/:id/alerts` - List alerts
- `POST /api/admin/watchlists/:id/alerts` - Create alert
- `PUT /api/admin/alerts/:id` - Update alert
- `DELETE /api/admin/alerts/:id` - Delete alert

### Webhooks
- `POST /api/webhooks/stripe` - Stripe webhook handler

## Database Schema

- `users` - Members and admins with argon2 password hashing
- `refresh_tokens` - JWT refresh token storage with rotation
- `subscriptions` - Stripe subscription sync
- `watchlists` - Weekly watchlists with publish control
- `watchlist_alerts` - Trade alerts (ticker, entry, invalidation, targets)
- `course_enrollments` - Course progress tracking

## Creating an Admin User

After registering via the frontend, promote a user to admin via psql:

```sql
UPDATE users SET role = 'admin' WHERE email = 'your@email.com';
```
