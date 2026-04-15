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
| `DATABASE_URL` | Yes | - | PostgreSQL connection string |
| `JWT_SECRET` | Yes | - | Secret key for JWT signing |
| `JWT_EXPIRATION_HOURS` | No | `24` | Access token lifetime |
| `REFRESH_TOKEN_EXPIRATION_DAYS` | No | `30` | Refresh token lifetime |
| `PORT` | No | `3001` | Server port |
| `FRONTEND_URL` | No | `http://localhost:5173` | Frontend URL for CORS |
| `CORS_ALLOWED_ORIGINS` | No | `FRONTEND_URL` | Comma-separated **exact** browser origins (scheme + host + port). No trailing `/`. |

### CORS in production

Browsers send the page’s origin on every cross-site API call (e.g. `https://www.precisionoptionsignals.com`). Your API must list that **exact** string in `CORS_ALLOWED_ORIGINS`, or the preflight `OPTIONS` response will omit `Access-Control-Allow-Origin` and the browser blocks the request.

- If visitors use **`https://www.…`**, allow `https://www.precisionoptionsignals.com`.
- **`https://precisionoptionsignals.com`** (apex) is a **different** origin; include both if you use both.
- After changing env vars on Render, **redeploy** (or restart) the API service.

Example:

```env
CORS_ALLOWED_ORIGINS=https://precisionoptionsignals.com,https://www.precisionoptionsignals.com
```
| `STRIPE_SECRET_KEY` | No | - | Stripe secret key |
| `STRIPE_WEBHOOK_SECRET` | No | - | Stripe webhook signing secret |
| `APP_ENV` | No | `development` | Use `production` to enforce production-only guards |
| `ADMIN_EMAIL` | Dev optional / Prod required | - | Seed admin email |
| `ADMIN_PASSWORD` | Dev optional / Prod required | - | Seed admin password |
| `ADMIN_NAME` | No | `Admin` | Seed admin display name |

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
