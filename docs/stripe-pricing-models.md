# Stripe pricing models in Swings

> **Last revised:** 2026-04-24  
> **Audience:** engineers, RevOps, and anyone deciding how catalog prices relate to Stripe  
> **Related:** [`stripe-local-testing.md`](./stripe-local-testing.md) (test keys + webhooks), [`DEPLOYMENT.md`](./DEPLOYMENT.md) (production secrets)

Swings reads **subscription list prices** from Postgres (`pricing_plans`) and exposes them at **`GET /api/pricing/plans`**. When a customer checks out, SvelteKit’s **`createCheckoutSession`** remote command (`src/routes/api/checkout.remote.ts`) builds a Stripe Checkout Session in one of two ways:

1. **Stripe Price ID path** — line item uses a pre-created Stripe **`price_…`** stored on the plan row (`stripe_price_id`).
2. **Inline `price_data` path** — `stripe_price_id` is **null**; amounts, currency, and billing interval are taken from the plan row and sent as **`line_items[].price_data`** (Stripe still creates what it needs for that Checkout Session; see [Create a Checkout Session](https://docs.stripe.com/api/checkout/sessions/create)).

This document compares both models, who typically uses which, and **how to configure each** in this repository.

---

## Model A — Stripe Price as canonical (`stripe_price_id` set)

### What it means

The **amount customers pay** for that catalog line item is defined by a **Stripe Product + Price** in your Stripe account. Your database stores a **pointer** (`stripe_price_id`, and optionally `stripe_product_id`) plus merchandising fields (name, slug, features, sort order, “active” flags). **Changing the number customers pay** means creating or selecting a **new** Stripe Price (or updating Stripe according to Stripe’s rules for your product), then updating the DB to point at it.

### Pros

- **Single billing truth** in Stripe: invoices, tax, proration, coupons, and the Dashboard all align with one `price_…` object.
- **Easier finance / audit**: “What did we charge?” maps cleanly to Stripe objects and export pipelines.
- **Fewer surprises** in production: no drift between “number shown in admin” and “number Stripe charged” if process is followed.
- **Enterprise default** for core retail subscription SKUs (see [What enterprises usually do](#what-enterprises-usually-do)).

### Cons

- **Operational overhead**: every material price change touches Stripe (Dashboard or API) **and** your DB.
- **Slower iteration** for experiments unless you automate Price creation (script, Terraform, Stripe API in CI).
- **Stale pointer risk**: if someone edits `amount_cents` in the DB but forgets to update `stripe_price_id`, the **UI can lie** until you fix the link.

### How to set up (Swings)

1. **Stripe (Test or Live mode)**
   - [Products](https://dashboard.stripe.com/products) → create or pick a **Product**.
   - Add a recurring **Price** (monthly / yearly). Copy the Price id (`price_xxxxxxxx`).

2. **Database / Admin**
   - Open **Admin → Subscriptions → Plans** (`/admin/subscriptions/plans`).
   - Edit the plan (or create one) so that:
     - **`amount_cents` / interval / currency** match the Stripe Price (keeps your public `/api/pricing/plans` accurate).
     - **`stripe_price_id`** is set to that `price_…` value.
   - Save. The public API now returns the same merchandising data **and** checkout will use `line_items: [{ price: stripe_price_id, quantity: 1 }]`.

3. **Environment (unchanged from other docs)**
   - **Root `.env`:** `STRIPE_SECRET_KEY`, `PUBLIC_APP_URL` — see [`stripe-local-testing.md`](./stripe-local-testing.md).
   - **`backend/.env`:** same `STRIPE_SECRET_KEY`, `STRIPE_WEBHOOK_SECRET` for webhooks.

4. **Verify**
   - `GET /api/pricing/plans` shows the plan with `stripe_price_id` populated.
   - Run through Checkout; in the Stripe Dashboard, the Payment / Subscription line item should reference the **named Price**, not only inline metadata.

---

## Model B — App database as canonical (`stripe_price_id` null, `price_data`)

### What it means

**`amount_cents`, `currency`, `interval`,** and display fields live in **`pricing_plans`**. Checkout **does not** require a pre-created Stripe Price: if `stripe_price_id` is **empty / null**, `createCheckoutSession` sends **`price_data`** built from the row (see `lineItemsForPlan` in `src/routes/api/checkout.remote.ts`). Stripe still processes a normal subscription Checkout Session; you are not bypassing Stripe— you are choosing **not** to maintain a parallel Price catalog in the Dashboard for every tweak.

### Pros

- **Fast iteration**: change amounts in **Admin → Subscriptions → Plans** (or SQL) and test Checkout without creating new Dashboard Prices each time.
- **Great for dev / staging / demos** and for teams that want the **marketing site** driven entirely from Postgres until the catalog stabilizes.
- **No duplicate entry** if you are not yet ready to mirror every SKU in Stripe.

### Cons

- **Dashboard hygiene**: Stripe may show more **ad-hoc** line items / Prices created from Checkout than a tightly curated Price book.
- **Governance**: easier for **DB and Stripe to diverge** if someone later sets a `stripe_price_id` that does not match `amount_cents` (human error).
- **Enterprise core catalogs** rarely use this as the **only** long-term strategy for production money (see below)—they often graduate to Model A for go-live SKUs.

### How to set up (Swings)

1. **Stripe**
   - You only need a **Stripe account** and **test (or live) API keys**—no requirement to pre-create a Product/Price for each plan **for checkout to work**.

2. **Database / Admin**
   - In **Admin → Subscriptions → Plans**, ensure **`stripe_price_id`** is **blank** for the plans you want to drive dynamically.
   - Set **`amount_cents`**, **`currency`**, **`interval`** (`month` / `year`), **`name`**, **`slug`**, etc., to what you want customers to see and pay.
   - Mark the plan **active** if your UI filters on that flag.

3. **Environment**
   - Same as Model A: root `.env` + `backend/.env` keys as in [`stripe-local-testing.md`](./stripe-local-testing.md).

4. **Verify**
   - `GET /api/pricing/plans` shows `stripe_price_id: null` for those rows.
   - Checkout from the site uses **`planSlug`**; the server resolves the plan and uses **`price_data`**.

---

## Side-by-side summary

| Dimension                                  | Model A (`stripe_price_id`)                 | Model B (`price_data`, id null)         |
| ------------------------------------------ | ------------------------------------------- | --------------------------------------- |
| **Source of truth for the charged amount** | Stripe Price object                         | Postgres `pricing_plans` row            |
| **Typical Stripe Dashboard**               | Clean Product/Price catalog                 | More Checkout-driven objects            |
| **Speed of price experiments**             | Slower unless automated                     | Faster                                  |
| **Mismatch risk**                          | DB display vs Stripe if pointer not updated | Lower until you introduce Price IDs     |
| **Enterprise norm for core revenue**       | **Common**                                  | **Uncommon as sole long-term approach** |

---

## What enterprises usually do

Large companies and regulated teams typically:

- Use **Model A** for **production** subscription SKUs that real money touches: **Stripe (or a billing platform)** owns the **billable price**, with **change control** (new Price, migrate subscribers per policy, finance sign-off).
- Use **Model B** (or automation that creates Prices from the same spec) for **engineering environments**, **sandboxes**, **short-lived experiments**, or **internal** tools—then **promote** to Model A when a price becomes “real.”

They also invest in **webhook reliability**, **idempotency**, **audit logs** for who changed prices, and **monitoring** on checkout failures—regardless of which line-item shape Checkout uses.

---

## Pushing catalog edits to **existing** Stripe subscriptions

Saving a plan in **Admin → Subscriptions → Plans** always updates Postgres and the internal change log. **By default**, Stripe subscriptions that were created earlier keep their previous line item until you explicitly roll them forward.

**Operator control (since 2026-04-24):**

1. **Checkout** (`createCheckoutSession` in `src/routes/api/checkout.remote.ts`) sets `subscription_data.metadata.swings_pricing_plan_id` to the catalog row’s UUID whenever checkout is started with a `planSlug`. That metadata is copied onto the Stripe Subscription object.
2. **Webhooks** (`customer.subscription.updated` in `backend/src/handlers/webhooks.rs`) read `metadata.swings_pricing_plan_id` and persist it on `subscriptions.pricing_plan_id` (migration `076_subscriptions_pricing_plan_id.sql`).
3. **Admin save** — enable “Also update existing Stripe subscriptions after save” on the plan editor. The request body may include:
   - `stripe_rollout.push_to_stripe_subscriptions: true`
   - `stripe_rollout.audience`: `linked_subscriptions_only` (only rows linked to this catalog plan) **or** `linked_and_unlinked_legacy_same_cadence` (adds legacy members whose `pricing_plan_id` is still null but whose `subscriptions.plan` enum matches the plan’s cadence — dangerous if you operate multiple monthly SKUs).
4. The browser must send an **`Idempotency-Key`** header on that save (the admin UI generates a UUID automatically when the checkbox is on). The API rejects rollout requests without it.
5. **Proration** — the rollout uses Stripe’s default proration behaviour for subscription updates (`create_prorations`). Finer-grained proration switches are not wired yet because of duplicate enum paths in `async-stripe`; operators can still adjust behaviour from the Stripe Dashboard if needed.
6. **Model A** (`stripe_price_id` set) — each targeted subscription’s first line item is switched to that `price_…`.
7. **Model B** (`stripe_price_id` null) — each line item is updated with inline `price_data`; the catalog row **must** carry a non-empty **`stripe_product_id`** so Stripe knows which Product to attach the generated Price to.

---

## Which should you use here?

| Situation                      | Recommendation                                                                                                                                                    |
| ------------------------------ | ----------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Local / staging / MVP**      | Model B is fine; iterate quickly.                                                                                                                                 |
| **Production, stable catalog** | Prefer Model A for each paid plan you care to reconcile in Stripe.                                                                                                |
| **Hybrid**                     | Some plans with `stripe_price_id` (enterprise SKUs), others null (experiments)—**document** which is which so operators do not “fix” amounts in the wrong system. |

---

## Code references (for maintainers)

- Checkout resolution: `src/routes/api/checkout.remote.ts` — `fetchActivePlans`, `lineItemsForPlan`, `createCheckoutSession`.
- Public plan payload: `GET /api/pricing/plans` — `backend/src/handlers/pricing.rs`.
- Admin plan CRUD + Stripe rollout UI: **Admin → Subscriptions → Plans** — `src/routes/admin/subscriptions/plans/+page.svelte` calling `PUT /api/admin/pricing/plans/{id}`.
- Rollout worker: `backend/src/services/pricing_rollout.rs`, invoked from `backend/src/handlers/pricing.rs`.

---

## See also

- [`docs/stripe-local-testing.md`](./stripe-local-testing.md) — test keys, `stripe listen`, webhook secret, test cards.
- [`docs/DEPLOYMENT.md`](./DEPLOYMENT.md) — live keys and production webhook endpoints.
- Stripe docs: [Checkout Session `line_items`](https://docs.stripe.com/api/checkout/sessions/create#create_checkout_session-line_items), [Prices](https://docs.stripe.com/api/prices).
