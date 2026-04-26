# Stripe Checkout — End-to-End QA Runbook

> **Last revised:** 2026-04-25
> **Audience:** Operator running a Stripe TEST-mode regression on the local
> stack (`backend` on `:3001`, frontend on `:5177`, Postgres at `:5434`).
> **Spec source:** [`docs/REMAINING-WORK.md`](./REMAINING-WORK.md) Phase 4.
> **Companion:** [`docs/stripe-local-testing.md`](./stripe-local-testing.md)
> for the day-to-day "how do I run Stripe locally" tutorial.
>
> This document is the **release-day go-live drill** for the EC-13 webhook
> expansion (migrations 077 + 078). It assumes the backend has just shipped
> the post-purchase event family (`invoice.payment_failed`, `invoice.paid`,
> `charge.refunded`, `payment_intent.payment_failed`,
> `charge.dispute.created`, `customer.subscription.{trial_will_end,paused,
> resumed}`) and we have not yet driven a single live TEST charge through
> the full pipe.
>
> **Goal:** at the end of this checklist, every webhook arm has been
> exercised end-to-end against TEST Stripe, every DB mutation verified
> with a copy-paste `psql` query, and every notification/outbox row that
> the code expects has been observed.

---

## TL;DR — what works today, what is blocked

| Area | Status (audited 2026-04-25) | Notes |
|---|---|---|
| Webhook handler dispatch (12 event types) | ready | `backend/src/handlers/webhooks.rs:128-156` |
| Migration 078 tables present | confirmed | `subscription_invoices`, `payment_failures`, `payment_refunds`, `payment_disputes`, `subscription_trial_events`, `stripe_webhook_audit` — all exist |
| Notification templates seeded | confirmed | `subscription.payment_failed` / `payment_recovered` / `trial_ending` (078) + `subscription.confirmed` / `cancelled` (020) |
| `pricing_plans` seeded | partial | 2 rows present (`Monthly $49 / Annual $399`) but `stripe_price_id IS NULL` on both — checkout will run via `price_data` mode (correct, supported path) |
| `backend/.env` `STRIPE_SECRET_KEY` | **placeholder** (`sk_test_xxx`) | Operator must paste a real `sk_test_*` from the Stripe Dashboard |
| `backend/.env` `STRIPE_WEBHOOK_SECRET` | **placeholder** (`whsec_xxx`) | Set per-session from `pnpm stripe:listen` output |
| Root `.env` `STRIPE_SECRET_KEY` | **MISSING** | SvelteKit `createCheckoutSession` will `error(500, 'Stripe is not configured')` until present (`src/routes/api/checkout.remote.ts:22-24`) |
| Stripe CLI installed | confirmed | `stripe 1.40.6` |
| Local services running | confirmed | API on :3001, web on :5177, Postgres `swings-db-1` healthy on :5434 |
| Existing webhook integration tests | confirmed | `backend/tests/stripe_webhooks.rs` covers every new arm offline |
| Live E2E (real Stripe TEST API) | **not yet executed** | This runbook is what closes that gap |

---

## Section 1 — Local configuration audit (mandatory pre-flight)

### 1.1 What lives where

| Process | Reads | Variables it needs | File |
|---|---|---|---|
| **SvelteKit** (`/api/checkout.remote.ts`) | `env.STRIPE_SECRET_KEY` (private), `publicEnv.PUBLIC_APP_URL` (public) | `STRIPE_SECRET_KEY=sk_test_…`, `PUBLIC_APP_URL=http://localhost:5177` | repo root `.env` |
| **Rust API** (`webhooks.rs`, `member.rs::post_billing_portal`) | `Config::stripe_secret_key`, `Config::stripe_webhook_secret` | `STRIPE_SECRET_KEY=sk_test_…` (same value), `STRIPE_WEBHOOK_SECRET=whsec_…` (rotates per `stripe listen` session) | `backend/.env` |
| **Stripe CLI** (`pnpm stripe:listen`) | `~/.config/stripe/config.toml` | TEST-mode API key from `stripe login` | host config |

### 1.2 Audit checklist — copy/paste each command

```bash
# 1. Backend STRIPE_SECRET_KEY non-placeholder?
grep -E "^STRIPE_SECRET_KEY=" /Users/billyribeiro/Desktop/my-websites/swings/backend/.env
#    Expected: STRIPE_SECRET_KEY=sk_test_<at least 24 chars>
#    AUDIT FINDING: currently `sk_test_xxx` (PLACEHOLDER). Fix below.

# 2. Backend STRIPE_WEBHOOK_SECRET non-placeholder?
grep -E "^STRIPE_WEBHOOK_SECRET=" /Users/billyribeiro/Desktop/my-websites/swings/backend/.env
#    Expected: STRIPE_WEBHOOK_SECRET=whsec_<long base64ish>
#    AUDIT FINDING: currently `whsec_xxx` (PLACEHOLDER). Fix in step 4 below.

# 3. Root .env has matching STRIPE_SECRET_KEY?
grep -E "^STRIPE_SECRET_KEY=" /Users/billyribeiro/Desktop/my-websites/swings/.env
#    Expected: STRIPE_SECRET_KEY=sk_test_<same value as backend/.env>
#    AUDIT FINDING: currently MISSING. Add it.

# 4. pnpm script wired correctly?
grep stripe:listen /Users/billyribeiro/Desktop/my-websites/swings/package.json
#    Expected: "stripe:listen": "stripe listen --forward-to http://127.0.0.1:3001/api/webhooks/stripe"
#    AUDIT FINDING: confirmed correct (package.json:28).

# 5. pricing_plans has at least one active row?
PGPASSWORD=swings_secret psql -h localhost -p 5434 -U swings -d swings -c \
  "SELECT id, name, slug, stripe_price_id, interval, amount_cents, currency, is_active FROM pricing_plans WHERE is_active ORDER BY sort_order;"
#    AUDIT FINDING: 2 rows present — Monthly ($49/mo) and Annual ($399/yr).
#    Both have stripe_price_id = NULL. Checkout still works via inline
#    `price_data` (supported path; see src/routes/api/checkout.remote.ts:48-69).

# 6. Stripe CLI installed and authenticated?
stripe --version
stripe config --list 2>&1 | grep -E "test_mode|account_id"
#    Expected: stripe v1.x.x; account_id present and test_mode = true
#    If missing: stripe login   (opens browser; pair the CLI to your account)
```

### 1.3 Fix-up commands (do these once, not every session)

> NEVER commit real keys. `backend/.env` and `.env` are both gitignored.
> Get your TEST keys from <https://dashboard.stripe.com/test/apikeys>
> ("Reveal test key").

```bash
# Edit backend/.env in place — replace placeholders.
# (Use your editor; do NOT paste keys into a chat.)
$EDITOR /Users/billyribeiro/Desktop/my-websites/swings/backend/.env
#    Set:  STRIPE_SECRET_KEY=sk_test_<your real test secret>
#    Leave STRIPE_WEBHOOK_SECRET=whsec_xxx for now — the next section
#    rotates it from the CLI.

# Add the same key to the SvelteKit-side .env so checkout.remote.ts works.
$EDITOR /Users/billyribeiro/Desktop/my-websites/swings/.env
#    Add line:  STRIPE_SECRET_KEY=sk_test_<same value>

# Authenticate the Stripe CLI to TEST mode.
stripe login
#    Opens browser. Confirm the device code matches.
```

### 1.4 Why both processes need the same secret

Two distinct Stripe API client paths exist:

1. **SvelteKit checkout** (`src/routes/api/checkout.remote.ts:25`) constructs a
   `new Stripe(env.STRIPE_SECRET_KEY)` and calls
   `stripe.checkout.sessions.create(...)` to mint the hosted Checkout URL.
2. **Rust billing portal + future server-driven mutations**
   (`backend/src/handlers/member.rs:165` invokes
   `stripe_api::create_billing_portal_session`).

Both must point at **the same Stripe account** or webhooks fired against
the SvelteKit-created Customer will never resolve to a Customer the Rust
API can read. Use one `sk_test_*` value everywhere.

---

## Section 2 — End-to-end checkout flow trace (code map)

This is the audited path for a single subscription checkout, anchored to
specific files and lines so the operator knows what to expect at each hop.

```
┌──────────────────────────────────────────────────────────────────────────┐
│ 1. User clicks "Start Monthly Plan" on /pricing/monthly/+page.svelte     │
│    (handleCheckout → createCheckoutSession('monthly'))                   │
│    files: src/routes/pricing/monthly/+page.svelte:108-117                │
│           src/routes/pricing/annual/+page.svelte (same shape)            │
└──────────────────────────────────────────────────────────────────────────┘
                                  │
                                  ▼
┌──────────────────────────────────────────────────────────────────────────┐
│ 2. Browser invokes the SvelteKit remote function                         │
│    src/lib/utils/checkout.ts:11 → ../../routes/api/checkout.remote.ts    │
│    Sends `{ planSlug: 'monthly' }` (validated by checkoutSchema)         │
└──────────────────────────────────────────────────────────────────────────┘
                                  │
                                  ▼
┌──────────────────────────────────────────────────────────────────────────┐
│ 3. Server-side handler (Node)                                            │
│    src/routes/api/checkout.remote.ts:152-207 createCheckoutSession       │
│       a. fetchActivePlans() → GET /api/pricing/plans (Rust API)          │
│          backend/src/handlers/pricing.rs:690-706 public_list_plans       │
│       b. lineItemsForPlan(plan):                                         │
│            - if plan.stripe_price_id → { price: 'price_…', quantity: 1 } │
│            - else → inline price_data with amount_cents/currency/interval│
│       c. stripe.checkout.sessions.create({                               │
│            mode: 'subscription',                                         │
│            line_items: …,                                                │
│            subscription_data.metadata.swings_pricing_plan_id: plan.id,   │
│            success_url: ${PUBLIC_APP_URL}/success?session_id={CSID}      │
│            cancel_url:  ${PUBLIC_APP_URL}/pricing?canceled=true,         │
│            allow_promotion_codes: true,                                  │
│            billing_address_collection: 'required'                        │
│          })                                                              │
│       d. returns { sessionId, url } → browser redirects to Stripe        │
└──────────────────────────────────────────────────────────────────────────┘
                                  │
                                  ▼
┌──────────────────────────────────────────────────────────────────────────┐
│ 4. Stripe-hosted Checkout collects card + email + billing address.       │
│    On success Stripe → 303 to PUBLIC_APP_URL/success?session_id=cs_…     │
│    src/routes/success/+page.svelte:14-19 reads ?session_id from URL.     │
└──────────────────────────────────────────────────────────────────────────┘
                                  │
                                  ▼
┌──────────────────────────────────────────────────────────────────────────┐
│ 5. Stripe fires webhooks (in this canonical order):                      │
│      a. checkout.session.completed                                       │
│      b. customer.subscription.created                                    │
│      c. invoice.paid       ← only on positive-amount checkouts           │
│    All three forwarded by `pnpm stripe:listen` to                        │
│    POST http://127.0.0.1:3001/api/webhooks/stripe                        │
└──────────────────────────────────────────────────────────────────────────┘
                                  │
                                  ▼
┌──────────────────────────────────────────────────────────────────────────┐
│ 6. Rust webhook entry point                                              │
│    backend/src/handlers/webhooks.rs:56-164 stripe_webhook                │
│      - verify_stripe_signature (HMAC-SHA256 v1, 5 min skew tolerance)    │
│      - try_claim_stripe_webhook_event (idempotency on event_id)          │
│      - dispatch on event_type → focused handler                          │
│                                                                          │
│   Handler map (webhooks.rs:128-156):                                     │
│      checkout.session.completed         → handle_checkout_completed (374)│
│      customer.subscription.{created,updated} → handle_subscription_update│
│      customer.subscription.deleted      → handle_subscription_deleted    │
│      customer.subscription.paused       → handle_subscription_paused     │
│      customer.subscription.resumed      → handle_subscription_resumed    │
│      customer.subscription.trial_will_end → handle_subscription_trial_…  │
│      invoice.payment_failed             → handle_invoice_payment_failed  │
│      invoice.paid                       → handle_invoice_paid            │
│      charge.refunded                    → handle_charge_refunded         │
│      payment_intent.payment_failed      → handle_payment_intent_failed   │
│      charge.dispute.created             → handle_charge_dispute_created  │
└──────────────────────────────────────────────────────────────────────────┘
                                  │
                                  ▼
┌──────────────────────────────────────────────────────────────────────────┐
│ 7. DB writes per event                                                   │
│   checkout.session.completed (handler at webhooks.rs:374-449):           │
│      - find_user_by_email(customer_details.email)                        │
│      - db::upsert_subscription(...) writes `subscriptions` row           │
│        (Monthly default; corrected by subscription.updated)              │
│      - users.stripe_customer_id is set inside upsert_subscription        │
│      - send_notification('subscription.confirmed', ...) → outbox row     │
│      - record_webhook_audit_best_effort → stripe_webhook_audit           │
│   customer.subscription.created/updated (webhooks.rs:247-310):           │
│      - infers plan from items.data[0].price.recurring.interval           │
│      - reads metadata.swings_pricing_plan_id (set by step 3c)            │
│      - upsert_subscription updates plan + status + period + plan id      │
│   invoice.paid (webhooks.rs:590-671):                                    │
│      - billing::upsert_invoice → subscription_invoices row (status=paid) │
│      - if previously past_due/unpaid → flips to active + emits           │
│        subscription.payment_recovered email                              │
└──────────────────────────────────────────────────────────────────────────┘
                                  │
                                  ▼
┌──────────────────────────────────────────────────────────────────────────┐
│ 8. Outbox dispatcher (separate worker, started in main.rs alongside the  │
│    HTTP server) picks up `outbox_events` rows and sends via Resend/SMTP. │
│    See backend/src/events/worker.rs.                                     │
└──────────────────────────────────────────────────────────────────────────┘
                                  │
                                  ▼
┌──────────────────────────────────────────────────────────────────────────┐
│ 9. Operator verifies in admin UI                                         │
│      /admin/subscriptions      → src/routes/admin/subscriptions/+page    │
│      /admin/orders             → src/routes/admin/orders/+page           │
│      /admin/audit              → admin actions (FTS, post-Phase 3)       │
│    And via psql (queries in Section 5 below).                            │
└──────────────────────────────────────────────────────────────────────────┘
```

**Important:** the user's email entered into Stripe Checkout MUST match an
existing `users.email`. `handle_checkout_completed` at `webhooks.rs:395`
silently no-ops if no user row matches. Always **register first** at
`http://localhost:5177/register` and use the same email at checkout.

---

## Section 3 — Test card matrix

Reference: <https://docs.stripe.com/testing#cards>. Each row below pairs a
canonical Stripe TEST PAN with the webhook → DB → admin UI → email
fan-out you should observe.

> **Conventions:**
> - Expiry: any **future** month/year (e.g. `12/34`).
> - CVC: any 3 digits (`123`).
> - ZIP: any (`12345`).
> - "Webhooks fired" lists the **events the operator should see in the
>   `stripe listen` terminal**, in order.

| # | Card | Scenario | Webhooks fired (in order) | Tables touched | Admin UI shows | Email queued |
|---|------|----------|---------------------------|----------------|----------------|--------------|
| 1 | `4242 4242 4242 4242` | Happy path, no 3DS | `checkout.session.completed` → `customer.subscription.created` → `invoice.paid` | `subscriptions` (active), `users.stripe_customer_id` set, `subscription_invoices` (paid), `stripe_webhook_audit` ×3, `outbox_events` (1) | `/admin/subscriptions`: row `status=active`, plan, MRR ticks up | `subscription.confirmed` |
| 2 | `4000 0027 6000 3184` | 3DS required, success after challenge | (Stripe pauses for 3DS) → `checkout.session.completed` → `customer.subscription.created` → `invoice.paid` | same as #1 | same as #1 | `subscription.confirmed` |
| 3 | `4000 0000 0000 9995` | Insufficient funds — declines at checkout | `checkout.session.completed` does **not** fire if Stripe rejects card on the hosted page; user stays on Checkout. If card is attached then first invoice fails: `invoice.payment_failed`, `payment_intent.payment_failed` | `payment_failures` row (final=true if no retry) | No new sub row; if checkout was abandoned, nothing new | None (the Checkout-page error is rendered inline; no template fires) |
| 4 | `4000 0000 0000 0341` | Attaches successfully but **first charge fails** — used to simulate post-checkout dunning | `checkout.session.completed` → `customer.subscription.created` (status=`incomplete`/`past_due`) → `invoice.payment_failed` | `subscriptions` (status flips to `past_due`), `subscription_invoices` (status=open), `payment_failures` row, audit row | `/admin/subscriptions`: row with `Past due` badge | `subscription.payment_failed` |
| 5 | `4000 0000 0000 0259` | Always declines on auth (generic decline) | No `checkout.session.completed`. Stripe Checkout shows decline inline | None (no row created) | Nothing new | None |
| 6 | `4000 0000 0000 0069` | Expired card | Same as #5 — declined inline | None | Nothing new | None |
| 7 | `4000 0000 0000 0127` | CVC fails | Same as #5 — declined inline | None | Nothing new | None |
| 8 | `4242…` then **refund from Stripe Dashboard** | Refund flow | `charge.refunded` (and `charge.refund.updated` if available) | `payment_refunds` row, `stripe_webhook_audit` row. If the original charge was an `orders.payment_intent_id` → `orders.status='refunded'` and `disputed_at` untouched | (no admin orders surface for sub-only refunds today; refund visible via psql) | None today (no `subscription.refunded` template; this is a Phase 1.1 follow-up — note in your QA report) |
| 9 | trigger via `stripe trigger charge.dispute.created` | Dispute / chargeback | `charge.dispute.created` | `payment_disputes` row, `orders.disputed_at = now()` (when an order is linked), `outbox_events` row with `event_type='ops.dispute_opened'` | If linked to an order, the order shows `disputed_at` (no UI badge yet) | None for end-customer; outbox event fires `ops.dispute_opened` for ops alerting |
| 10 | `stripe subscriptions cancel sub_xxx` (CLI) **or** Stripe Dashboard | Subscription cancellation | `customer.subscription.deleted` | `subscriptions.status='canceled'`, audit row | `/admin/subscriptions`: badge flips to `Canceled` | `subscription.cancelled` |
| 11 | `stripe trigger customer.subscription.trial_will_end` | Trial about to end | `customer.subscription.trial_will_end` | `subscription_trial_events` row (PK on `(subscription_id, trial_end)`), audit row | (no surfacing in UI; verify via psql) | `subscription.trial_ending` |
| 12 | `stripe trigger customer.subscription.paused` then `…resumed` | Pause / resume | `customer.subscription.paused` then `customer.subscription.resumed` | `subscriptions.status='paused'` then back to `active`, `paused_at` set then nulled | Status badge flips through `Paused` (rendered as default chip) → `Active` | None |

### Notes on cards 3, 5–7

For cards that decline on the **Stripe-hosted Checkout page itself** (insufficient funds, generic decline, expired, CVC), the user never gets redirected back to your app and **no webhook fires for that attempt**. Stripe surfaces the error inline and lets the user retry with a different card. To exercise the post-checkout failure pipeline (which is what `invoice.payment_failed` / `payment_intent.payment_failed` actually cover), use card #4 (`4000 0000 0000 0341`) — it attaches cleanly and only fails when Stripe tries to charge it for the first invoice.

---

## Section 4 — Manual QA checklist (the runbook)

### Pre-flight (do these once per session)

```
[ ] 1. docker postgres up
       docker ps --format "table {{.Names}}\t{{.Status}}\t{{.Ports}}" | grep swings-db
       Expected: "swings-db-1   Up X hours (healthy)   0.0.0.0:5434->5432/tcp"

[ ] 2. backend running on :3001
       lsof -nP -iTCP:3001 -sTCP:LISTEN
       (one row, COMMAND swings-api or cargo)
       If not running: cd backend && cargo run

[ ] 3. frontend running on :5177
       lsof -nP -iTCP:5177 -sTCP:LISTEN
       (one row, COMMAND node)
       If not running: pnpm dev

[ ] 4. Stripe CLI installed
       stripe --version
       Expected: "stripe version 1.x.x" (currently 1.40.6 — confirmed)

[ ] 5. Stripe CLI authenticated to TEST mode
       stripe config --list | grep -E "test_mode|account"
       If missing: stripe login   (opens browser, pair device, choose test)

[ ] 6. Real sk_test_... in BOTH .env files (Section 1.3 above)
       grep -E "^STRIPE_SECRET_KEY=sk_test_(?!xxx$)" backend/.env
       grep -E "^STRIPE_SECRET_KEY=sk_test_" .env

[ ] 7. Webhook listener running in a SEPARATE terminal
       pnpm stripe:listen
       The CLI prints:
         > Ready! You are using Stripe API version [...].
         > Your webhook signing secret is whsec_AbCd1234... (^C to quit)
       COPY that whsec_ value.

[ ] 8. Paste the whsec_ into backend/.env
       $EDITOR backend/.env
       Set STRIPE_WEBHOOK_SECRET=whsec_AbCd1234…

[ ] 9. Restart the API so it picks up the new secret
       (in the cargo run terminal: Ctrl-C, then `cargo run` again)

[ ] 10. Sanity-check that the API accepts webhooks
        In the stripe listen terminal:
          stripe trigger ping
        Expected in the listen terminal: "200 OK"
        If 401: secret mismatch. If 500: STRIPE_WEBHOOK_SECRET still empty.

[ ] 11. Register a test user (if you don't already have one)
        Open http://localhost:5177/register
        Email: stripe-qa-{date}@example.com  (must be unique per run)
        Note down the email — Stripe Checkout MUST use the SAME email
        for handle_checkout_completed to find the user.
```

### Test 1 — Successful subscription checkout (`4242 4242 4242 4242`)

```
[ ] 1. Open http://localhost:5177/pricing
[ ] 2. Click "Get Started" on the Monthly card → routes to /pricing/monthly
[ ] 3. Click "Start Monthly Plan -- $49/mo" — page shows "Processing..."
[ ] 4. Browser redirects to Stripe Checkout (https://checkout.stripe.com/...)
[ ] 5. Enter:
         Email:     <SAME email you registered with above>
         Card:      4242 4242 4242 4242
         Expiry:    12 / 34
         CVC:       123
         Name:      Stripe QA
         Country:   United States, ZIP 12345
[ ] 6. Click "Subscribe". Stripe processes (~2-3 s).
[ ] 7. Browser lands on http://localhost:5177/success?session_id=cs_test_...
       Page renders: green checkmark + "Welcome to Precision Options Signals!"
[ ] 8. Switch to the `pnpm stripe:listen` terminal. Within ~5 s you should see:
         --> checkout.session.completed [evt_…]
         <-- [200] POST http://127.0.0.1:3001/api/webhooks/stripe [evt_…]
         --> customer.subscription.created [evt_…]
         <-- [200] POST http://127.0.0.1:3001/api/webhooks/stripe [evt_…]
         --> invoice.paid [evt_…]
         <-- [200] POST http://127.0.0.1:3001/api/webhooks/stripe [evt_…]
       (Order may vary; all three should be 200.)
[ ] 9. Switch to the cargo terminal. You should see structured logs like:
         INFO Stripe webhook received: checkout.session.completed (evt_…)
         INFO stripe checkout.session.completed received customer_id=… subscription_id=…
         INFO Stripe webhook received: customer.subscription.created (evt_…)
         INFO Stripe webhook received: invoice.paid (evt_…)
[ ] 10. Run the verification queries (Section 5 — "After Test 1") below.
[ ] 11. Open http://localhost:5177/admin/subscriptions in the browser.
        Log in as admin (billy.ribeiro@icloud.com / password from backend/.env).
        Expected: a row for stripe-qa-{date}@example.com, status badge "Active",
        plan "Monthly", amount $49.00/mo.
[ ] 12. Verify the email was queued (check the outbox in the same terminal):
         tail -n 50 backend's stdout for "outbox_event published … subscription.confirmed"
        OR (if EMAIL_PROVIDER=resend with a real key) check the Resend dashboard
        OR (if EMAIL_PROVIDER=smtp/noop) only the queue row matters; verify via psql.
```

### Test 2 — 3DS challenge success (`4000 0027 6000 3184`)

```
[ ] 1. Same as Test 1 steps 1-5, but use card 4000 0027 6000 3184.
[ ] 2. After clicking "Subscribe" Stripe shows a 3D Secure modal.
       Click "Complete authentication" (TEST 3DS auto-approves).
[ ] 3. Continue from Test 1 step 7 onward — webhook + DB sequence is identical.
       The only difference is wall-clock latency (1-2 s longer for the 3DS handoff).
[ ] 4. Verification: same psql queries as Test 1 should return one MORE row
       in `subscriptions` (so 2 total if you're running the suite top-to-bottom).
```

### Test 3 — Card that fails on first charge (`4000 0000 0000 0341`)

This is the only realistic way to exercise `invoice.payment_failed` on
the local stack — the card attaches, but Stripe deliberately fails the
first invoice charge. The subscription enters dunning.

```
[ ] 1. Same as Test 1 steps 1-7, with card 4000 0000 0000 0341.
[ ] 2. You WILL still land on /success — Stripe lets the checkout
       complete (the subscription is created `incomplete` / `past_due`).
[ ] 3. In the stripe listen terminal expect:
         --> checkout.session.completed       [200]
         --> customer.subscription.created    [200]
         --> invoice.payment_failed           [200]    ← key event
[ ] 4. Backend log line should include:
         INFO Stripe webhook received: invoice.payment_failed (evt_…)
[ ] 5. Verification queries (Section 5 — "After Test 3"):
         - subscriptions: status='past_due'
         - payment_failures: row with attempt_count=1, final=false
         - subscription_invoices: row with status='open', amount_due_cents=4900
         - outbox_events: row for 'subscription.payment_failed'
[ ] 6. Admin UI: /admin/subscriptions shows a "Past due" badge (orange).
```

### Test 4 — Decline on Stripe Checkout page (`4000 0000 0000 0259`)

```
[ ] 1. Same as Test 1 steps 1-5, with card 4000 0000 0000 0259.
[ ] 2. Stripe Checkout shows: "Your card was declined." Stay on Checkout.
[ ] 3. Stripe listen terminal: NO webhook fires. (Stripe does not bill
       a Customer until the user's card succeeds.)
[ ] 4. Backend log: silent — no new event.
[ ] 5. Verification: row counts in `subscriptions`, `subscription_invoices`,
       `payment_failures` are unchanged from before this test.
[ ] 6. Repeat with cards 4000 0000 0000 0069 (expired) and
       4000 0000 0000 0127 (CVC fail). Same expected behaviour.
```

### Test 5 — Refund from Stripe Dashboard

```
[ ] 1. Open https://dashboard.stripe.com/test/payments
[ ] 2. Find the most recent successful payment from Test 1 (~$49.00).
[ ] 3. Click into it → "Refund" → "Refund $49.00" → "Refund".
[ ] 4. Within ~5 s, stripe listen shows:
         --> charge.refunded   [200]
[ ] 5. Verification (Section 5 — "After Test 5"):
         - payment_refunds: 1 row, amount_cents=4900, status='succeeded'
         - stripe_webhook_audit: row with event_type='charge.refunded'
       (orders.refund_amount_cents is unchanged because subscription
        invoices skip the orders table — this is by design, see
        commerce/refunds.rs:5-10.)
[ ] 6. Admin UI: no refund surface today for sub-only refunds. Phase 5.1
       calls for a dedicated UI; for now log this as observed.
```

### Test 6 — Dispute / chargeback

```
[ ] 1. In a free terminal:
         stripe trigger charge.dispute.created
[ ] 2. CLI prints "Trigger succeeded! Check dashboard for events." and the
       listen terminal shows:
         --> charge.dispute.created   [200]
[ ] 3. Verification (Section 5 — "After Test 6"):
         - payment_disputes: 1 row, status (varies by trigger fixture),
           is_charge_refundable = true|false
         - stripe_webhook_audit: row event_type='charge.dispute.created'
         - outbox_events: row with event_type='ops.dispute_opened',
           idempotency_key=dispute:dp_…
[ ] 4. (orders.disputed_at is set ONLY when the disputed charge maps to an
        existing orders row via stripe_payment_intent_id. The trigger
        fixture uses a synthetic charge unconnected to any local order,
        so expect orders.disputed_at to remain unchanged for this test.)
```

### Test 7 — Trial-will-end reminder

```
[ ] 1. Run:
         stripe trigger customer.subscription.trial_will_end
[ ] 2. listen terminal: 200 OK.
[ ] 3. Backend log: INFO Stripe webhook received: customer.subscription.trial_will_end
       Likely DEBUG: "trial_will_end for unknown subscription; skipping"
       — the synthetic trigger fires for a Stripe sub_… that doesn't
       map to a local subscriptions row. This is expected.
[ ] 4. To exercise the happy path: the operator needs to either
       (a) create a real subscription with a trial via Stripe Dashboard,
           wait for the 3-day reminder (impractical for QA), OR
       (b) directly insert a subscriptions row with the matching
           stripe_subscription_id then re-run the trigger:
             stripe trigger customer.subscription.trial_will_end \
               --override 'subscription:id=sub_real_id_from_db'
[ ] 5. When matched, verification:
         - subscription_trial_events: 1 row, PK (subscription_id, trial_end)
         - outbox_events: row for 'subscription.trial_ending'
         - audit row: customer.subscription.trial_will_end
```

### Test 8 — Pause / resume

```
[ ] 1. From your live test sub (Test 1 result), grab its sub_… id:
         PGPASSWORD=swings_secret psql -h localhost -p 5434 -U swings -d swings -t -c \
           "SELECT stripe_subscription_id FROM subscriptions ORDER BY created_at DESC LIMIT 1;"
[ ] 2. Pause it via Stripe CLI:
         stripe subscriptions update sub_REPLACE \
           --pause-collection.behavior=mark_uncollectible
       (Stripe fires customer.subscription.updated → the local handler
        flips status. A separate customer.subscription.paused fires when
        collection moves to that explicit state.)
[ ] 3. Verification:
         - subscriptions.status = 'paused' (or 'unpaid' depending on behaviour)
         - subscriptions.paused_at IS NOT NULL
[ ] 4. Resume:
         stripe subscriptions update sub_REPLACE --pause-collection=
       Verification:
         - subscriptions.status = 'active'
         - subscriptions.paused_at IS NULL
```

### Test 9 — Cancellation (clean teardown of the test sub)

```
[ ] 1. Cancel via Stripe CLI:
         stripe subscriptions cancel sub_REPLACE
[ ] 2. listen terminal: --> customer.subscription.deleted  [200]
[ ] 3. Verification:
         - subscriptions.status = 'canceled'
         - admin UI: badge flips to "Canceled"
         - outbox_events: row for 'subscription.cancelled'
```

---

## Section 5 — Verification queries (psql copy-paste)

Connection string is constant for every query: Postgres at
`localhost:5434`, user `swings`, db `swings`, password `swings_secret`.

```bash
# helper alias for the rest of this section
alias swings-psql='PGPASSWORD=swings_secret psql -h localhost -p 5434 -U swings -d swings'
```

### Baseline snapshot (run once before starting the suite)

```sql
swings-psql <<'EOF'
SELECT 'users'                  AS t, COUNT(*) FROM users
UNION ALL SELECT 'subscriptions',           COUNT(*) FROM subscriptions
UNION ALL SELECT 'subscription_invoices',   COUNT(*) FROM subscription_invoices
UNION ALL SELECT 'payment_failures',        COUNT(*) FROM payment_failures
UNION ALL SELECT 'payment_refunds',         COUNT(*) FROM payment_refunds
UNION ALL SELECT 'payment_disputes',        COUNT(*) FROM payment_disputes
UNION ALL SELECT 'subscription_trial_events', COUNT(*) FROM subscription_trial_events
UNION ALL SELECT 'stripe_webhook_audit',    COUNT(*) FROM stripe_webhook_audit
UNION ALL SELECT 'outbox_events',           COUNT(*) FROM outbox_events
UNION ALL SELECT 'processed_webhook_events', COUNT(*) FROM processed_webhook_events
ORDER BY t;
EOF
```

### After Test 1 (happy-path subscription)

```sql
swings-psql <<'EOF'
-- The new subscription row.
SELECT id, status, plan, stripe_subscription_id, stripe_customer_id,
       pricing_plan_id, current_period_start, current_period_end
  FROM subscriptions
 ORDER BY created_at DESC
 LIMIT 1;

-- The matching invoice (after invoice.paid lands).
SELECT stripe_invoice_id, status, amount_due_cents, amount_paid_cents,
       currency, attempt_count, paid_at
  FROM subscription_invoices
 ORDER BY created_at DESC
 LIMIT 1;

-- Audit ledger of the three webhooks we expect.
SELECT event_type, target_kind, target_id, created_at
  FROM stripe_webhook_audit
 ORDER BY created_at DESC
 LIMIT 5;

-- Confirmation email queued in outbox (event_type from FDN-04).
SELECT event_type, aggregate_type, aggregate_id, dispatched_at, attempts, last_error
  FROM outbox_events
 WHERE event_type LIKE 'subscription.%'
 ORDER BY created_at DESC
 LIMIT 3;

-- The user row should now carry the stripe_customer_id.
SELECT id, email, stripe_customer_id
  FROM users
 WHERE email = 'stripe-qa-{date}@example.com';
EOF
```

Expected: 1 subscription row (`status=active`), 1 invoice row
(`status=paid`, `amount_paid_cents=4900`), 3 audit rows
(`checkout.session.completed`, `customer.subscription.updated`,
`invoice.paid`), 1 outbox row for `subscription.confirmed`,
`users.stripe_customer_id` populated.

### After Test 3 (failed first charge — `4000 0000 0000 0341`)

```sql
swings-psql <<'EOF'
SELECT id, status, plan, stripe_subscription_id
  FROM subscriptions
 ORDER BY created_at DESC
 LIMIT 1;
-- Expected: status='past_due'

SELECT stripe_event_id, stripe_invoice_id, attempt_count, final,
       failure_code, failure_message, next_payment_attempt
  FROM payment_failures
 ORDER BY created_at DESC
 LIMIT 1;
-- Expected: 1 row, attempt_count=1, final=false (Stripe will retry),
-- failure_code likely 'card_declined'

SELECT event_type, target_id
  FROM stripe_webhook_audit
 WHERE event_type = 'invoice.payment_failed'
 ORDER BY created_at DESC LIMIT 1;

SELECT event_type, attempts, last_error
  FROM outbox_events
 WHERE event_type = 'subscription.payment_failed'
 ORDER BY created_at DESC LIMIT 1;
EOF
```

### After Test 5 (refund)

```sql
swings-psql <<'EOF'
SELECT stripe_refund_id, stripe_charge_id, stripe_invoice_id,
       order_id, subscription_id, user_id, amount_cents, currency,
       reason, status
  FROM payment_refunds
 ORDER BY created_at DESC LIMIT 1;
-- Expected: 1 row, amount_cents=4900, status='succeeded'

SELECT event_type
  FROM stripe_webhook_audit
 WHERE event_type = 'charge.refunded'
 ORDER BY created_at DESC LIMIT 1;
EOF
```

### After Test 6 (dispute)

```sql
swings-psql <<'EOF'
SELECT stripe_dispute_id, stripe_charge_id, status, reason,
       amount_cents, currency, evidence_due_by, is_charge_refundable
  FROM payment_disputes
 ORDER BY created_at DESC LIMIT 1;

SELECT event_type, aggregate_id, payload->>'order_id' AS order_id,
       payload->>'stripe_dispute_id' AS dispute_id
  FROM outbox_events
 WHERE event_type = 'ops.dispute_opened'
 ORDER BY created_at DESC LIMIT 1;
EOF
```

### After Test 9 (cancellation)

```sql
swings-psql <<'EOF'
SELECT id, status, stripe_subscription_id
  FROM subscriptions
 ORDER BY updated_at DESC LIMIT 1;
-- Expected: status='canceled'

SELECT event_type FROM outbox_events
 WHERE event_type = 'subscription.cancelled'
 ORDER BY created_at DESC LIMIT 1;
EOF
```

### Idempotency proof

Every `stripe trigger ...` invocation generates a fresh event id, but
Stripe **also retries delivery** if the listener errors. To prove the
idempotency layer works, deliberately replay an event:

```bash
# In the listen terminal, copy an event id (e.g. evt_3ABC...) from a
# previously-processed event. Then in another terminal:
stripe events resend evt_3ABC...
```

Verification:

```sql
swings-psql <<'EOF'
SELECT event_id, event_type, processed_at
  FROM processed_webhook_events
 WHERE event_id = 'evt_3ABC…';
-- Should return exactly 1 row (the second delivery was dedup'd).

SELECT COUNT(*) FROM payment_failures WHERE stripe_event_id = 'evt_3ABC…';
-- 1 if the original was a payment_failed event; 0 otherwise.
EOF
```

The `try_claim_stripe_webhook_event` call at `webhooks.rs:98` returns
`Ok(false)` on the second delivery and the handler short-circuits to a
200 — no DB delta should be observed.

---

## Section 6 — Test harness recommendation

Per Phase 4d of the spec, three options were considered:

| Option | Pros | Cons | Verdict |
|---|---|---|---|
| (A) Extend `backend/tests/stripe_webhooks.rs` with multi-event flows | Already exists (`backend/tests/stripe_webhooks.rs`); zero CI cost; deterministic; uses real `verify_stripe_signature` + `try_claim_stripe_webhook_event` paths; no live Stripe dependency | Cannot validate the SvelteKit-side `createCheckoutSession` Node code path; cannot prove Stripe accepts our params | **Recommended primary** — extends what we already have, ships in CI tomorrow |
| (B) New Playwright `e2e/stripe-checkout.spec.ts` driving the live Stripe TEST hosted Checkout | Validates the full pipe (UI click → Stripe → webhook → admin UI); catches Stripe API drift | 30-90 s per test; flaky against external service; requires CI secret; 3DS challenge requires special Stripe cards/iframe-sandbox dance | **Recommended nightly** — too slow for every PR but valuable as a daily smoke |
| (C) Bash script `scripts/stripe-e2e.sh` orchestrating `stripe trigger` + DB polling | Minimal infra; great for ad-hoc local QA | Duplicates option (A) without the type/contract guarantees; no CI integration story | Not recommended |

### Recommended path

1. **Now:** extend `backend/tests/stripe_webhooks.rs` with a single
   `subscription_lifecycle_end_to_end` test that drives, in order:
   `checkout.session.completed` → `customer.subscription.updated` →
   `invoice.paid` → `invoice.payment_failed` → `invoice.paid` (recovery)
   → `customer.subscription.deleted`, asserting DB state after each.
   Existing per-arm tests cover correctness; the new test proves the
   sequence interleaves correctly (recovery-from-dunning is the most
   likely regression hot-spot).

2. **Next sprint:** add `e2e/stripe-checkout.spec.ts` (Playwright) driving
   card #1 (`4242…`) and card #4 (`4000 0000 0000 0341`) end-to-end. Run
   on a **nightly** CI cron, not every PR. Use Playwright's `expect.poll`
   to wait for the admin-UI row to appear (the webhook → DB write is
   asynchronous from the redirect-to-success-page perspective).

3. **No need for the bash script** — option (A) supersedes it for CI and
   this runbook supersedes it for ad-hoc QA.

---

## Section 7 — Pricing-plan seeding (already done, but documented)

The local DB already has 2 active rows (audit confirmed):

| name | slug | amount_cents | currency | interval | stripe_price_id | is_active |
|---|---|---|---|---|---|---|
| Monthly | monthly | 4900 | usd | month | NULL | true |
| Annual | annual | 39900 | usd | year | NULL | true |

`stripe_price_id` is NULL on both — checkout uses the `price_data`
inline-amount path (`src/routes/api/checkout.remote.ts:48-69`). This is
fully supported and is the path most local-QA setups use.

### Optional — link to real Stripe Price IDs

For "what-the-Stripe-Dashboard-sees" reporting fidelity, you can link
each plan to a Stripe-hosted Price object. Steps:

1. Stripe Dashboard → **Products** → "Add product"
   - Name: `Precision Options Signals — Monthly`
   - Pricing: `$49.00 USD` recurring monthly. Save.
   - Copy the `price_…` id.
2. Repeat for `Annual` ($399/yr).
3. Update the local DB:
   ```sql
   swings-psql <<'EOF'
   UPDATE pricing_plans
      SET stripe_price_id = 'price_REPLACE_MONTHLY'
    WHERE slug = 'monthly';
   UPDATE pricing_plans
      SET stripe_price_id = 'price_REPLACE_ANNUAL'
    WHERE slug = 'annual';
   EOF
   ```

   **OR** via the admin UI at `http://localhost:5177/admin/subscriptions/plans`
   (uses `PUT /api/admin/pricing/plans/{id}` — handler at
   `backend/src/handlers/pricing.rs:217-506`).

4. Subsequent checkouts will pass `{ price: 'price_…' }` instead of the
   inline `price_data` object.

### If you ever start with an empty `pricing_plans` table

The `012_pricing_plans.sql` migration auto-seeds the two rows above on a
fresh DB. If the table is somehow empty (a manual truncate), seed via
the admin UI → `New Plan`, or via SQL:

```sql
INSERT INTO pricing_plans (name, slug, description, amount_cents, currency, interval, interval_count, features, is_popular, is_active, sort_order)
VALUES
  ('Monthly', 'monthly', 'Weekly watchlists & trade alerts',  4900, 'usd', 'month', 1,
   '["Weekly watchlists & trade alerts","Full course library access","Members-only community","Mobile app access"]'::jsonb,
   FALSE, TRUE, 1),
  ('Annual',  'annual',  'Save vs monthly',                  39900, 'usd', 'year',  1,
   '["Everything in Monthly","Save $189/year vs monthly","Priority support","Exclusive annual member content"]'::jsonb,
   TRUE,  TRUE, 2)
ON CONFLICT (slug) DO NOTHING;
```

---

## Section 8 — Known gaps the runbook surfaces

These are deliberate scope cuts the operator should be aware of when
filing QA tickets — they are **not** failures of this runbook, they are
known trailing work:

1. **No end-customer "subscription.refunded" email.** `charge.refunded`
   writes the `payment_refunds` row, audit row, and (when an order
   exists) flips `orders.status` — but does not enqueue an email to the
   customer. Tracked under Phase 1.1 follow-up (the spec lists
   `order.refund.issued` as a registered template name in
   `backend/src/notifications/templates.rs` but `handle_charge_refunded`
   at `webhooks.rs:676-762` does not call `send_notification` today).

2. **No admin UI surface for refunds against subscription invoices.**
   Refunds tied to an `orders` row appear via the orders admin
   (`/admin/orders`); refunds tied **only** to a Stripe invoice (no
   local order row, which is how subscription refunds normally arrive)
   are visible only via psql. Phase 5.1 in the spec calls for a sub-page
   under `/admin/subscriptions/{id}` with a `payment_refunds` panel.

3. **No admin UI badge for `disputed_at`.** `payment_disputes` writes
   the row; `orders.disputed_at` is set; nothing renders a chip in the
   subs list. Operator must `psql` to see it. Phase 5.1.

4. **`customer.subscription.trial_will_end` happy path requires a real
   subscription with a trial.** The Stripe `trigger` fixture uses a
   synthetic `sub_` id that doesn't match any local row, so the handler
   logs `"trial_will_end for unknown subscription; skipping"` and the
   `subscription.trial_ending` email is not enqueued. To validate the
   email template, create a real trial sub (or temporarily insert a
   matching `subscriptions` row) before triggering.

5. **`/admin/audit` does not yet render webhook events.** Webhook
   actions land in the new `stripe_webhook_audit` table (separate from
   `admin_actions`, which has a NOT NULL FK to `users(id)` and so cannot
   carry a `system:stripe-webhook` actor — see comment in
   `078_stripe_webhook_expansion.sql:197-203`). The admin audit page
   queries `admin_actions` only. A future "Webhooks" tab on the audit
   page should `UNION ALL` both tables.

6. **Resend webhook secret is unset by default.** The Resend
   delivery-status webhook (`/api/webhooks/email/resend`) is wired
   (`webhooks.rs:1154-1219`) but rejects requests with a 500 unless
   `RESEND_WEBHOOK_SECRET` is set. Out of scope for Stripe QA but worth
   noting if you observe email status events that never land.

---

## Section 9 — Cleanup / teardown after the suite

```bash
# 1. Cancel any test subscription you created (Stripe-side):
stripe subscriptions list --limit 5
# (note any sub_… that's still active)
stripe subscriptions cancel sub_REPLACE

# 2. (Optional) wipe the QA rows from the local DB so the next run starts clean:
swings-psql <<'EOF'
DELETE FROM payment_failures   WHERE created_at > NOW() - INTERVAL '1 day';
DELETE FROM payment_refunds    WHERE created_at > NOW() - INTERVAL '1 day';
DELETE FROM payment_disputes   WHERE created_at > NOW() - INTERVAL '1 day';
DELETE FROM subscription_invoices WHERE created_at > NOW() - INTERVAL '1 day';
DELETE FROM subscription_trial_events; -- always safe; populated only by happy-path trial
DELETE FROM stripe_webhook_audit  WHERE created_at > NOW() - INTERVAL '1 day';
DELETE FROM processed_webhook_events WHERE processed_at > NOW() - INTERVAL '1 day';
DELETE FROM subscriptions      WHERE created_at > NOW() - INTERVAL '1 day';
DELETE FROM users              WHERE email LIKE 'stripe-qa-%@example.com';
EOF
# (Outbox events are kept — the dispatcher will mark them dispatched_at
#  on next tick and they harmlessly age out per the audit retention policy.)

# 3. Stop the listener:
#    Ctrl-C in the `pnpm stripe:listen` terminal.

# 4. Remove the rotating webhook secret from backend/.env so the next
#    session is forced to repaste a fresh one:
$EDITOR /Users/billyribeiro/Desktop/my-websites/swings/backend/.env
#    Reset STRIPE_WEBHOOK_SECRET=whsec_xxx
```

---

## Appendix A — Stripe CLI cheatsheet

```bash
# Authenticate this machine to your Stripe TEST account.
stripe login

# Forward TEST-mode webhook events to the local API.
pnpm stripe:listen
# (= stripe listen --forward-to http://127.0.0.1:3001/api/webhooks/stripe)

# Forward only specific events (filters noise during focused QA):
stripe listen \
  --forward-to http://127.0.0.1:3001/api/webhooks/stripe \
  --events checkout.session.completed,customer.subscription.created,invoice.paid,invoice.payment_failed,charge.refunded,charge.dispute.created

# Generate a synthetic event (signature is valid):
stripe trigger checkout.session.completed
stripe trigger invoice.payment_failed
stripe trigger charge.refunded
stripe trigger charge.dispute.created
stripe trigger customer.subscription.trial_will_end
stripe trigger customer.subscription.paused
stripe trigger customer.subscription.resumed
stripe trigger payment_intent.payment_failed

# Inspect a live test-mode subscription:
stripe subscriptions retrieve sub_…
stripe subscriptions update sub_… --pause-collection.behavior=mark_uncollectible
stripe subscriptions update sub_… --pause-collection=
stripe subscriptions cancel sub_…

# Tail the Stripe API request log for your test mode account:
stripe logs tail
```

---

## Appendix B — Files audited (citations)

| Concern | File | Lines |
|---|---|---|
| Webhook entry + dispatch table | `backend/src/handlers/webhooks.rs` | 56-164 |
| `checkout.session.completed` handler | `backend/src/handlers/webhooks.rs` | 374-449 |
| `customer.subscription.{created,updated}` | `backend/src/handlers/webhooks.rs` | 247-310 |
| `customer.subscription.deleted` | `backend/src/handlers/webhooks.rs` | 312-372 |
| `customer.subscription.{paused,resumed}` | `backend/src/handlers/webhooks.rs` | 1039-1123 |
| `customer.subscription.trial_will_end` | `backend/src/handlers/webhooks.rs` | 952-1034 |
| `invoice.payment_failed` | `backend/src/handlers/webhooks.rs` | 456-586 |
| `invoice.paid` | `backend/src/handlers/webhooks.rs` | 590-671 |
| `charge.refunded` | `backend/src/handlers/webhooks.rs` | 676-762 |
| `payment_intent.payment_failed` | `backend/src/handlers/webhooks.rs` | 769-859 |
| `charge.dispute.created` | `backend/src/handlers/webhooks.rs` | 864-946 |
| Stripe HMAC verify | `backend/src/handlers/webhooks.rs` | 166-210 |
| Idempotency claim | `backend/src/handlers/webhooks.rs` | 98-117 |
| Invoice persistence | `backend/src/commerce/billing.rs` | 1-285 |
| Refund persistence | `backend/src/commerce/refunds.rs` | 1-182 |
| Dispute persistence | `backend/src/commerce/disputes.rs` | 1-185 |
| Webhook audit best-effort writer | `backend/src/commerce/webhook_audit.rs` | (whole file) |
| SvelteKit checkout session creator | `src/routes/api/checkout.remote.ts` | 1-208 |
| Browser-side checkout helper | `src/lib/utils/checkout.ts` | (whole file) |
| Pricing page (entry) | `src/routes/pricing/+page.svelte` | 1-275 |
| Monthly checkout page | `src/routes/pricing/monthly/+page.svelte` | 108-117 |
| Success page | `src/routes/success/+page.svelte` | 14-19 |
| Public pricing endpoint | `backend/src/handlers/pricing.rs` | 690-706 |
| Admin subscriptions list | `src/routes/admin/subscriptions/+page.svelte` | (whole file) |
| Migration — pricing plans schema | `backend/migrations/012_pricing_plans.sql` | 1-52 |
| Migration — webhook expansion | `backend/migrations/078_stripe_webhook_expansion.sql` | 1-282 |
| Existing webhook integration tests | `backend/tests/stripe_webhooks.rs` | 1-80 (header) |
| `pnpm stripe:listen` script | `package.json` | 28 |
| Local-testing tutorial (companion doc) | `docs/stripe-local-testing.md` | 1-85 |
| Spec — Phase 4 | `docs/REMAINING-WORK.md` | 421-559 |

---

*End of QA runbook.*
