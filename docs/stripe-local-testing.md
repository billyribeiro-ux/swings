# Stripe test mode — local signup + checkout (end to end)

> **Last revised:** 2026-04-24
> **Audience:** developers on `pnpm dev:all` + Postgres

**Pricing is already dynamic:** the site reads **active plans** from `GET /api/pricing/plans` (Postgres). The marketing UI and `createCheckoutSession` both use that API — there are **no** `PUBLIC_STRIPE_*_PRICE_ID` environment variables for checkout.

Checkout runs in SvelteKit (`src/routes/api/checkout.remote.ts`):

- You pass a **plan slug** (e.g. `monthly`, `annual`) from the client.
- The server **fetches the same public plans** (server-side) and:
  - uses `stripe_price_id` from the DB when your team has linked a canonical Stripe **Price** in the admin, **or**
  - if `stripe_price_id` is null, uses Stripe’s **`line_items.price_data`** for subscriptions, as documented in [Create a Checkout Session](https://docs.stripe.com/api/checkout/sessions/create) (API version in code: `2026-04-22.dahlia`) — so local testing does not require pre-creating Prices in the Dashboard for amounts that already match the API.

**Secrets:** you still use **Test mode** API keys from your Stripe account: [API keys (test mode)](https://docs.stripe.com/keys#test-live-modes).

**Test cards** (official reference): [Testing](https://docs.stripe.com/testing) — e.g. success `4242 4242 4242 4242`, 3DS flows, decline scenarios.

**Two processes talk to Stripe:**

1. **SvelteKit** — `STRIPE_SECRET_KEY` in the **repo root** `.env` (creates Checkout sessions).
2. **Rust API** — `STRIPE_SECRET_KEY` in **`backend/.env`** (webhooks, billing portal, etc.). Use the **same** test secret for one account. Webhooks need **`STRIPE_WEBHOOK_SECRET`** (see below).

---

## 1. Environment

### Root `.env` (SvelteKit)

| Variable | Example |
| --- | --- |
| `STRIPE_SECRET_KEY` | `sk_test_...` |
| `PUBLIC_APP_URL` | `http://localhost:5173` |

`PUBLIC_STRIPE_PUBLISHABLE_KEY` is only for embedded **Elements**-style UIs, not for hosted Checkout.

### `backend/.env`

| Variable | Example |
| --- | --- |
| `STRIPE_SECRET_KEY` | same `sk_test_...` |
| `STRIPE_WEBHOOK_SECRET` | from Stripe CLI in dev (`whsec_...`) — see section 3 |

---

## 2. Run the app

```bash
pnpm dev:all
```

---

## 3. Webhooks (so subscriptions link to the user in Postgres)

The handler is **Rust:** `POST /api/webhooks/stripe`. In test mode, forward events with the [Stripe CLI](https://docs.stripe.com/stripe-cli):

```bash
pnpm stripe:listen
```

Paste the printed **`whsec_...`** into `backend/.env` as `STRIPE_WEBHOOK_SECRET` and **restart the API**.

**Email match:** `checkout.session.completed` matches Stripe’s customer email to a **registered** user. Use the **same email** in Checkout that you used at `/register`.

---

## 4. Pay with a test card

On the Stripe-hosted Checkout page, use any future expiry, any CVC, and the official [test card numbers](https://docs.stripe.com/testing#cards) (e.g. `4242`… for success in most cases).

---

## 5. Optional: link real Price IDs in admin

For stable reporting in the Stripe Dashboard, you can still set **`stripe_price_id`** on a plan in **Admin → Subscriptions → Plans** — but it is **not** required for checkout to work; amounts and intervals come from the API/DB and Stripe’s API accepts `price` or `price_data` per the docs above.

---

## 6. Production

Use **live** keys on your hosts, register a real webhook endpoint in the Stripe Dashboard, and set `STRIPE_WEBHOOK_SECRET` to that endpoint’s signing secret. See [`docs/DEPLOYMENT.md`](./DEPLOYMENT.md).
