# Backend audit report

**Scope:** Rust API in `backend/` (Axum + SQLx + PostgreSQL).  
**Date:** Generated from repository snapshot (Precision Options Signals / `swings-api`).

This document describes what the backend **does**, what it **stores**, and what it **depends on**. It is independent of any single host (Render, Hetzner, etc.).

---

## 1. Executive summary

- **Role:** Single HTTP API for auth, members, subscriptions, watchlists, courses, blog/CMS, media uploads, pricing/coupons, popups, analytics ingest, and Stripe webhooks.
- **Data:** Almost everything is in **PostgreSQL** (`DATABASE_URL`). **Binary uploads** live on **local disk** (`UPLOAD_DIR`, default `./uploads`) and are served at **`/uploads/*`**.
- **External services:** **PostgreSQL** (required), **Stripe** (billing + webhooks), **SMTP** (optional, for email).
- **Frontend:** Not part of this crate; the SvelteKit app calls this API (often via same-origin `/api` rewrites on Vercel).

---

## 2. Runtime behavior (`main.rs`)

1. Load env (`.env` / `backend/.env` fallback for `DATABASE_URL`).
2. Connect to Postgres with SQLx pool options (max 10 connections, acquire/idle/lifetime tuned for Neon-style Postgres).
3. Run **SQLx migrations** from `backend/migrations/`.
4. If `ADMIN_EMAIL` + `ADMIN_PASSWORD` are set → seed admin user. In **`APP_ENV=production`**, both are **required** or the process panics.
5. Create **`UPLOAD_DIR`** if missing.
6. Optionally initialize **SMTP** (`EmailService`); if `SMTP_USER` is empty, email is disabled (warning logged).
7. Build **CORS** allowlist **only** from `CORS_ALLOWED_ORIGINS` (comma-separated) or, if unset, `FRONTEND_URL` as a single origin. Operators must list apex and `www` explicitly when both are used.
8. **Preflight:** `allow_headers(Any)` so browser extensions / extra headers do not break `OPTIONS`.
9. Bind **`0.0.0.0:PORT`** (default `3001`) and serve with `into_make_service_with_connect_info::<SocketAddr>()` so rate limiting can read client IPs (and optional forwarded headers safely behind a trusted proxy).

---

## 3. HTTP API surface

All routes below are mounted by `main.rs`. **Auth:** `Authorization: Bearer <JWT>`. **Admin** routes require JWT with role `admin` (`AdminUser` extractor).

### 3.1 Auth — `/api/auth`

| Method | Path | Purpose |
|--------|------|---------|
| POST | `/register` | Register |
| POST | `/login` | Login |
| POST | `/refresh` | Refresh tokens |
| GET | `/me` | Current user |
| POST | `/logout` | Logout |
| POST | `/forgot-password` | Request reset |
| POST | `/reset-password` | Complete reset |

### 3.2 Analytics (public ingest) — `/api/analytics`

| Method | Path | Purpose |
|--------|------|---------|
| POST | `/events` | Batch ingest SPA events |

### 3.3 Admin — `/api/admin`

| Area | Routes (summary) |
|------|-------------------|
| Dashboard | `GET /stats`, `GET /analytics/summary` |
| Members | List/get/delete; `PUT /members/{id}/role`; subscription helpers: billing portal, cancel, resume |
| Watchlists | CRUD watchlists; CRUD alerts under watchlist + global alert update/delete |

### 3.4 Admin blog — `/api/admin/blog`

- Posts: CRUD, restore from trash, status, autosave, revisions, post meta.
- Categories & tags: CRUD.
- Media: list, **multipart upload**, update, delete (writes files under `UPLOAD_DIR`).

### 3.5 Admin courses — `/api/admin/courses`

- Courses CRUD, publish toggle, modules CRUD, lessons CRUD.

### 3.6 Admin pricing — `/api/admin/pricing`

- Plans CRUD, toggle, history.

### 3.7 Admin coupons — `/api/admin/coupons`

- Coupons CRUD, bulk create, usages, toggle.

### 3.8 Admin popups — `/api/admin/popups`

- Popups CRUD, toggle, duplicate, submissions list, analytics.

### 3.9 Public blog — `/api/blog`

- List/get posts (by slug), password unlock, categories, tags, posts by category/tag, slugs listing.

### 3.10 Public courses — `/api/courses`

- `GET /courses`, `GET /courses/{slug}`.

### 3.11 Public pricing — `/api/pricing`

- `GET /plans`.

### 3.12 Public coupons — `/api/coupons`

- `POST /coupons/validate`, `POST /coupons/apply`.

### 3.13 Public popups — `/api/popups`

| Method | Path | Purpose |
|--------|------|---------|
| GET | `/popups/active` | Active popups (query: page, device, user_status) |
| POST | `/popups/event` | Track popup event |
| POST | `/popups/submit` | Submit popup form |

### 3.14 Member (authenticated) — `/api/member`

- Profile: `GET`/`PUT /profile`
- Subscription: `GET /subscription`, `POST /billing-portal`, cancel/resume
- Watchlists: `GET /watchlists`, `GET /watchlists/{id}`
- Courses (legacy list/progress): `GET /courses`, `PUT /courses/{course_id}/progress`
- **Also:** `courses::member_router()` — enroll, course progress, lesson progress (`/courses/{id}/enroll`, etc.)

### 3.15 Webhooks — `/api/webhooks`

| Method | Path | Purpose |
|--------|------|---------|
| POST | `/stripe` | Stripe signed webhooks (subscriptions, checkout completed) |

### 3.16 Static files

- **`GET /uploads/*`** — `ServeDir` over `UPLOAD_DIR` (uploaded media).

---

## 4. Data storage

### 4.1 PostgreSQL

All relational data lives in **one** database. Migrations live in `backend/migrations/`:

| File | Domain |
|------|--------|
| `001_initial.sql` | Users, refresh tokens, subscriptions, watchlists, alerts, course_enrollments (legacy) |
| `002_blog.sql` | Media metadata, blog categories/tags/posts, junctions, revisions |
| `003_password_resets.sql` | Password reset tokens |
| `004_media_title.sql` | Media title |
| `005_user_author_profile.sql` | Author-style profile fields on users |
| `006_post_format.sql` | Post format |
| `007_post_meta.sql` | Post meta KV |
| `008_media_focal.sql` | Media focal point |
| `009_analytics.sql` | Analytics sessions/events |
| `010_normalize_user_emails.sql` | Email normalization |
| `011_courses.sql` | Courses, modules, lessons, lesson_progress; enrollment extensions |
| `012_pricing_plans.sql` | Pricing plans |
| `013_coupons.sql` | Coupons + usages |
| `014_analytics_enhanced.sql` | Analytics enhancements |
| `015_popups.sql` | Popups + submissions/events |
| `016_blog_trash_meta.sql` | Blog trash metadata |

### 4.2 Filesystem

- **`UPLOAD_DIR`** (default `./uploads`): uploaded blog/media **files**.
- **`media.storage_path`** / **`media.url`** in DB reference these files; public URLs are built using **`API_URL`** at upload time.

**Important:** If the API runs on ephemeral disk (typical PaaS without a volume), **uploads can be lost on redeploy** unless you use persistent disk or object storage.

### 4.3 Stripe (external)

- Payment method and subscription **source of truth** in Stripe.
- Local **`subscriptions`** (and related user fields) are **synced** via webhooks / API usage.

---

## 5. External integrations

| System | Required? | Code |
|--------|-----------|------|
| PostgreSQL | Yes | `sqlx`, `DATABASE_URL` |
| Stripe | For paid features | `stripe_api.rs`, `webhooks.rs`, `async-stripe` |
| SMTP | No | `email.rs`, `lettre`, `tera` |

No Redis, S3, Elasticsearch, or other backends are wired in this repository.

---

## 6. Rust dependencies (`Cargo.toml`)

| Crate area | Purpose |
|------------|---------|
| axum, axum-extra, tokio, tower, tower-http | HTTP server, CORS, trace, static `/uploads` |
| sqlx | Postgres + migrations + queries |
| jsonwebtoken, argon2 | JWT, passwords |
| serde, serde_json, uuid, chrono | Serialization / types |
| async-stripe | Stripe API |
| axum_typed_multipart, tempfile, sanitize-filename | Multipart uploads |
| lettre, tera | Email |
| validator | Request validation |
| hmac, sha2 | Webhook signature verification |
| rust_decimal | Decimal types |
| tracing | Logging |

---

## 7. Configuration (environment variables)

### Required

- `DATABASE_URL`
- `JWT_SECRET`

### Production constraints

- `APP_ENV=production` → **`ADMIN_EMAIL` and `ADMIN_PASSWORD` required** (admin seed).

### Commonly required for a live product

- `STRIPE_SECRET_KEY`, `STRIPE_WEBHOOK_SECRET`
- `API_URL` — public base URL of this API (media URLs)
- `FRONTEND_URL` / `CORS_ALLOWED_ORIGINS` — CORS (env-only; include apex and `www` in the list when needed)
- `APP_URL` — app links (defaults exist)

### Optional

- `PORT`, `UPLOAD_DIR`, `JWT_EXPIRATION_HOURS`, `REFRESH_TOKEN_EXPIRATION_DAYS`
- `SMTP_HOST`, `SMTP_PORT`, `SMTP_USER`, `SMTP_PASSWORD`, `SMTP_FROM`

See `backend/.env.example` and `backend/README.md` for field-level documentation.

---

## 8. Auth model

- **Extractors** (`extractors.rs`): `AuthUser` (any logged-in user), `AdminUser` (role `admin`), optional auth helper for public routes.
- **Middleware module** (`middleware.rs`): placeholder only; auth is per-route via extractors.

---

## 9. Deployment artifacts in repo

- **`backend/Dockerfile`** — container image for the API.
- **`backend/render.yaml`** — example Render blueprint (web + Postgres). **Update `FRONTEND_URL` and all secrets** before reuse; it may not match current production domains.

---

## 10. Redeploy checklist (host-agnostic)

1. Provision **PostgreSQL** and set `DATABASE_URL`.
2. Run migrations (happens automatically on API startup, or use `sqlx migrate run`).
3. Set `JWT_SECRET`, `APP_ENV`, admin seed vars for production.
4. Set `API_URL` to the **public** API origin (for media URLs).
5. Configure **Stripe** keys and **webhook** endpoint: `https://<api-host>/api/webhooks/stripe`.
6. Ensure **`UPLOAD_DIR` is persistent** or migrate to object storage (not implemented in-repo).
7. Point the **frontend** (`VITE_API_URL` for SSR + rewrites for browser) at the new API base.

---

## 11. What this report is not

- It does not audit the **SvelteKit** app (`src/`).
- It does not list **production secrets** or your current hosting account state.
- It does not guarantee **Render**, **Hetzner**, or any vendor is configured; only what the **code** expects.

---

*End of report.*
