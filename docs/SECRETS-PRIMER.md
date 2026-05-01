# Secrets primer — JWT, AEAD keys, and the `.env` files

> **Last revised:** 2026-05-01
> **Audience:** anyone setting up `swings` for local dev or coordinating the
> production deploy. If you've never minted a JWT secret before, this is the
> doc that should make you confident.
> **Companion files:** [`backend/.env.example`](../backend/.env.example),
> [`.env.example`](../.env.example), [`backend/README.md`](../backend/README.md) §
> Environment variables.

---

## TL;DR cheat sheet

| Variable | Where you get it | Where it goes | Why it exists |
| --- | --- | --- | --- |
| `JWT_SECRET` | `openssl rand -base64 64` (you generate it locally) | `backend/.env` only | Signs access tokens. Anyone who knows it can mint valid JWTs. |
| `SETTINGS_ENCRYPTION_KEY` | `openssl rand -base64 32` | `backend/.env` only | AEAD key for `app_settings.value_type='secret'` rows. 32 bytes exactly. |
| `STRIPE_SECRET_KEY` (test) | https://dashboard.stripe.com/test/apikeys → "Secret key" (`sk_test_…`). Or `stripe config --list` if you've already run `stripe login`. | **Both** `backend/.env` AND root `.env` with the **same** value | Backend uses it for webhook verification + portal sessions; SvelteKit BFF uses it to mint Checkout Sessions. Same Stripe account = same key. |
| `STRIPE_WEBHOOK_SECRET` (test) | First-line stdout of `stripe listen --print-secret …` | `backend/.env` only | Verifies that incoming webhook bodies were actually signed by Stripe. |
| `RESEND_API_KEY` | https://resend.com/api-keys (or set `EMAIL_PROVIDER=noop` for dev) | `backend/.env` only | Outbound email. `noop` logs to stdout instead of sending. |
| `ADMIN_EMAIL` / `ADMIN_PASSWORD` | You choose them | `backend/.env` only | Seeded into the `users` table on first boot. Argon2-hashed at rest. |
| `DATABASE_URL` | Docker-compose default for local dev | `backend/.env` | Postgres DSN. |

`.env.example` files in this repo carry **placeholder values, never real
secrets**. They are committed to git and tell you the shape of each value
(prefix, format) so you know what to look for. The real values live in the
gitignored `.env` files on your machine, and in your hosting provider's
secrets store in production.

---

## What is a JWT secret, really?

When a user logs in, the backend mints a **JSON Web Token** (JWT) — a
short-lived bearer token the SPA presents on subsequent requests to prove
"I'm user X with role Y." The token is just signed JSON; anyone who reads it
can decode the claims, but they can't forge new ones because the signature
requires a secret key.

```
header.payload.HMAC-SHA256(header.payload, JWT_SECRET)
```

The secret is symmetric — it's used both to sign and to verify. So:

- **Anyone who knows your `JWT_SECRET` can mint valid tokens for any user.**
  They can claim to be the admin, your CEO, anyone. Treat it like a master
  password.
- It is **not** something you fetch from a third party. There is no "JWT
  dashboard." You generate it locally with `openssl rand` and never share it.
- Different environments use **different** secrets. Dev gets one,
  staging gets one, prod gets one. Compromising your dev key never gives
  anyone a path into prod.

### Generating the dev `JWT_SECRET`

In your terminal:

```bash
openssl rand -base64 64 | tr -d '\n'
```

Copy the output (a single line of base64). Open `backend/.env` and replace
the placeholder:

```diff
- JWT_SECRET=your-super-secret-jwt-key-change-this
+ JWT_SECRET=ZmFr…(your real 64-byte base64 string here)
```

That's it. Restart the backend (`cd backend && cargo run`). You should see
it boot to "Swings API listening on port 3001" without panicking. If
anything is wrong with the value, the boot log will tell you.

### Why 64 bytes (= 88 base64 chars)?

`JWT_SECRET` is checked for non-emptiness at startup but the algorithm we
use (`HS256`) accepts any length. Long is better:

- 32 bytes is the floor of "this is genuinely strong against brute force
  for the next 50 years."
- 64 bytes is a common "comfortable margin" for HMAC-SHA256 keys (the
  algorithm operates on 64-byte blocks internally).

You will never see the secret in any user-facing surface. It's only ever
read by the Rust process on boot.

### When to rotate

Rotate `JWT_SECRET` when:

1. You think it has been exposed (committed to git, posted in chat,
   leaked via a logging bug).
2. Quarterly, as a hygiene practice in production.
3. After any security incident that might have read the env.

**Side effect of rotation: every active session is invalidated.** Every
logged-in user has to log in again. Refresh tokens stored in the DB are
not affected (they're hashed independently with `crypto.hash_token`), so
the next refresh round-trip mints a new pair signed by the new secret.

---

## What is `SETTINGS_ENCRYPTION_KEY`?

Some rows in the `app_settings` table hold actual secrets — webhook
signing tokens for outbound forms integrations, third-party API
credentials, etc. Those values are stored encrypted at rest using AES-GCM
("AEAD" = authenticated encryption with associated data). The key for
that cipher is `SETTINGS_ENCRYPTION_KEY`.

It must be **exactly 32 bytes**, base64-encoded:

```bash
openssl rand -base64 32 | tr -d '\n'
```

If you set a value that isn't a valid base64 32-byte key, the backend
panics at first decrypt with `"not a valid base64 32-byte key"`. (The
panic happens lazily — at first use, not at boot — because the key is
only needed when there's at least one secret-typed setting to read.)

### Why a separate key from `JWT_SECRET`?

Different threat model, different rotation cadence, different blast radius:

- `JWT_SECRET` rotation = log every user out (~minor inconvenience).
- `SETTINGS_ENCRYPTION_KEY` rotation = every encrypted setting becomes
  unreadable until you re-encrypt them with the new key (much more
  involved).

Keeping them separate means an incident scoped to one doesn't force the
operational pain of the other.

---

## What goes in which `.env`?

There are **two** `.env` files in this repo, and they hold partially
overlapping values for a reason:

```
swings/
├── .env                  ← SvelteKit frontend / BFF (gitignored)
├── .env.example          ← committed template
└── backend/
    ├── .env              ← Rust API (gitignored)
    └── .env.example      ← committed template
```

### Root `.env` (SvelteKit / BFF)

Read by SvelteKit's `$env/dynamic/public` and `$env/dynamic/private`
machinery during SSR + the Vercel runtime. Contains:

| Var | Purpose |
| --- | --- |
| `PUBLIC_APP_URL` | Where Stripe should redirect users back after Checkout. |
| `STRIPE_SECRET_KEY` | The BFF mints Checkout Sessions; needs Stripe API access. **Same value as `backend/.env`'s copy.** |
| `PUBLIC_STRIPE_PUBLISHABLE_KEY` | Optional. Only needed if you embed Stripe Elements (card fields) directly in a form. Prefix `PUBLIC_` is required by SvelteKit so the value is exposed to the browser. |
| `VITE_API_URL` | Optional. Backend origin for SSR fetches in non-Vercel deploys (Vercel rewrites `/api/*` to Railway directly via `vercel.json`). |

### `backend/.env` (Rust API)

Read by `swings_api::config::Config::from_env` at boot. Contains
**everything** the Axum binary needs:

| Bucket | Vars |
| --- | --- |
| Core | `DATABASE_URL`, `JWT_SECRET`, `JWT_EXPIRATION_HOURS`, `REFRESH_TOKEN_EXPIRATION_DAYS`, `PORT`, `APP_ENV`, `FRONTEND_URL`, `API_URL`, `APP_URL`, `CORS_ALLOWED_ORIGINS` |
| Crypto | `SETTINGS_ENCRYPTION_KEY` (and optional `APP_DATA_KEY`) |
| Admin seeding | `ADMIN_EMAIL`, `ADMIN_PASSWORD`, `ADMIN_NAME` |
| Stripe | `STRIPE_SECRET_KEY`, `STRIPE_WEBHOOK_SECRET` |
| Email | `EMAIL_PROVIDER` (`resend` / `smtp` / `noop`), `RESEND_API_KEY`, `RESEND_WEBHOOK_SECRET`, `RESEND_FROM`, `SMTP_*` |
| Object storage | `R2_*` |
| Anti-abuse (optional) | `TURNSTILE_SECRET`, `AKISMET_*`, `MAXMIND_DB_PATH`, `CONSENT_IP_SALT` |
| Worker tuning | `OUTBOX_*`, `AUDIT_RETENTION_INTERVAL_SECS`, `DSAR_*_INTERVAL_SECS`, `IDEMPOTENCY_GC_INTERVAL_SECS` |
| Pool tuning | `PGPOOL_*` |
| Misc | `RATE_LIMIT_BACKEND`, `LOG_FORMAT`, `SWINGS_ALLOW_HTTP_WEBHOOKS` |

Full reference with defaults: see [`backend/.env.example`](../backend/.env.example).

### Why is `STRIPE_SECRET_KEY` in BOTH files?

Two processes use it:

1. **SvelteKit BFF** (root `.env`) — when a user clicks "Subscribe," the
   BFF endpoint at `src/routes/api/checkout.remote.ts` calls
   `stripe.checkout.sessions.create()` to mint a hosted Checkout URL.
2. **Axum API** (`backend/.env`) — when Stripe POSTs back webhooks, the
   API verifies the signature and calls Stripe to retrieve / update
   subscriptions, refunds, customer portals, etc.

Both processes must point at the **same Stripe account**. If they don't,
the subscription Stripe creates won't match the customer the API expects
to mirror, and weird "ghost subscription" bugs surface.

The duplication is enforced by convention, not code. Both `.env` files
have a comment block explaining this. Don't drift them.

---

## Adding a new env var (process)

Whenever you introduce a new env var the Rust binary reads:

1. Read the var in `backend/src/config.rs` with a **typed default** if
   it's optional, or `Result<_>` if it's required.
2. Add a matching entry to **`backend/.env.example`** with:
   - a comment explaining what it does, when it's required, and the
     expected format,
   - a placeholder value (never a real secret),
   - if it's a frontend-relevant or duplicate-by-design var, add it to
     **`.env.example`** too.
3. If it's required-in-production, extend
   `Config::assert_production_ready()` so the API panics on boot when
   `APP_ENV=production` and the var is missing/empty. **Hard fail beats
   subtle misbehavior in prod.**
4. Add a sentence to [`backend/README.md`](../backend/README.md) §
   "Environment variables" so the table stays accurate.
5. Note the new var in [`CHANGELOG.md`](../CHANGELOG.md).

A var that the Rust code reads but the example template doesn't list is a
silent setup trap. The CI parity job catches missing-default panics, but
the social contract is: every new env read = one example-file entry +
one runtime guard.

---

## Production deploy

1. Generate fresh values for `JWT_SECRET` and `SETTINGS_ENCRYPTION_KEY`
   on a trusted machine (NOT your dev laptop).
2. Paste them into Railway's **Variables** UI (or your secret manager of
   choice). Never into a file checked into git.
3. Use `STRIPE_SECRET_KEY` from https://dashboard.stripe.com/apikeys —
   prefixed `sk_live_…`, NOT `sk_test_…`. Same value in the Vercel
   project's environment variables for the BFF copy.
4. Use `STRIPE_WEBHOOK_SECRET` from the **deployed** webhook endpoint
   in the Stripe dashboard, NOT from `stripe listen`. The dashboard
   endpoint and `stripe listen` endpoint each have their own signing
   secret — using the wrong one drops every webhook on the floor.
5. Set `APP_ENV=production`. The backend's
   `Config::assert_production_ready()` will panic on boot if any
   required var is missing or empty — that's the failsafe.

---

## Rotation runbook

| Variable | Trigger | Steps |
| --- | --- | --- |
| `JWT_SECRET` | Quarterly OR after suspected exposure | 1) Generate new value. 2) Set in env (rolling deploy). 3) Every user re-logs in on next request — expected. 4) Revoke any long-lived API tokens that were minted by hand. |
| `SETTINGS_ENCRYPTION_KEY` | Suspected exposure ONLY (rotation breaks reads) | 1) Generate new value. 2) Run a one-off migration that reads each `app_settings` secret with the **old** key and re-encrypts with the **new** key. 3) Then swap env. (Do NOT just rotate the env var or all secret-typed settings become unreadable.) |
| `STRIPE_SECRET_KEY` | If leaked: Stripe dashboard "Roll keys" generates new pair instantly. | 1) Roll in dashboard. 2) Paste new value into Railway + Vercel. 3) Stripe deactivates the old one immediately. |
| `STRIPE_WEBHOOK_SECRET` | If leaked: regenerate in dashboard. | 1) Click "Roll" next to the webhook endpoint in Stripe dashboard. 2) Paste new `whsec_…` into Railway. 3) Restart backend. |

---

## Common questions

**Q: Why does my backend panic on boot with "JWT_SECRET must be set"?**
A: Either the file isn't being read, or the value is empty. Verify
`backend/.env` exists and the line is `JWT_SECRET=…` with no trailing
spaces. The backend reads `.env` from the directory it's run in
(`backend/.env` when you run from the `backend/` directory).

**Q: Can I share `JWT_SECRET` across staging and prod?**
A: No. If staging is compromised, the attacker can mint prod tokens.
One env, one secret.

**Q: Where are my dev secrets stored once I paste them into `.env`?**
A: In the gitignored file on your laptop's disk. They are never
transmitted, committed, or echoed by the backend. They live in the Rust
process's memory while the binary runs and are dropped when it exits.

**Q: Should I check my secrets into a password manager?**
A: For prod values, yes — the prod values are usually generated once and
held by ops. For dev values, no — anyone can regenerate them in 5
seconds with `openssl rand`.

**Q: I rotated `STRIPE_SECRET_KEY` but not `STRIPE_WEBHOOK_SECRET`. Will
webhooks still work?**
A: Yes. They're independent. The webhook secret only signs webhook
bodies; the API secret authenticates outbound calls TO Stripe. Rolling
one does not require rolling the other.

**Q: Is it OK that `.env.example` gets committed?**
A: Yes — that's its purpose. It documents the shape of each variable so
new contributors know what to put where. As long as no real values land
in the example, it's safe.

---

## See also

- [`backend/.env.example`](../backend/.env.example) — reference template.
- [`.env.example`](../.env.example) — root template.
- [`backend/src/config.rs`](../backend/src/config.rs) — the function
  that reads each var (`Config::from_env`) and the prod-readiness guard
  (`Config::assert_production_ready`).
- [`docs/DEPLOYMENT.md`](./DEPLOYMENT.md) — full Vercel + Railway
  go-live walk-through.
- [`docs/stripe-local-testing.md`](./stripe-local-testing.md) — how to
  use `stripe listen` and `stripe trigger` for end-to-end webhook QA.
- [`SECURITY.md`](../SECURITY.md) — repo security policy.
