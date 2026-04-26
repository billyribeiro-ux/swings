# Changelog

All notable changes described in this file are grouped by release date.  
Timestamps use the operator-facing calendar date attached to the change list.

---

## 2024-04-24 — Subscription price rollout & catalog ↔ Stripe control

> **Note on the date:** this entry was authored under the heading **April 24, 2024** per project request. The engineering work ships with repository state current as of **2026-04-24** (migrations, OpenAPI snapshot, and coordinated frontend/backend commits).

### Summary

Operators can **choose** what happens to **existing Stripe subscriptions** when they change a row in `pricing_plans`: either leave Stripe alone (historical default — new checkouts only) or **push** the updated commercial terms to every targeted member subscription after the database save.

### Database

- **`076_subscriptions_pricing_plan_id.sql`**
  - Adds nullable `subscriptions.pricing_plan_id` (`UUID`, FK → `pricing_plans.id`, `ON DELETE SET NULL`).
  - Partial index `idx_subscriptions_pricing_plan_id` for fast rollout candidate queries.

### Stripe ↔ Swings linking

- **Checkout (`src/routes/api/checkout.remote.ts`)**
  - When checkout is created from a **`planSlug`**, the session now includes  
    `subscription_data.metadata.swings_pricing_plan_id = <catalog plan UUID>`.
  - `priceId`-only checkouts intentionally omit this metadata (no catalog row is implied).

- **Webhooks (`backend/src/handlers/webhooks.rs`)**
  - `customer.subscription.updated` parses `metadata.swings_pricing_plan_id` and passes it into `db::upsert_subscription`, so the local mirror learns which catalog plan a subscription was purchased from.
  - `checkout.session.completed` continues to upsert with `NULL` catalog link until the subscription webhook arrives.
  - `customer.subscription.deleted` preserves `pricing_plan_id` while flipping status.

### HTTP API

- **`PUT /api/admin/pricing/plans/{id}`** (`backend/src/handlers/pricing.rs`)
  - Request body may include optional **`stripe_rollout`** (`PricingStripeRollout` in `backend/src/models.rs`):
    - `push_to_stripe_subscriptions` (`bool`) — when `true`, after the plan row + change log are written, the server walks matching `subscriptions` rows and calls Stripe for each.
    - `audience` (`linked_subscriptions_only` | `linked_and_unlinked_legacy_same_cadence`) — controls whether only linked rows are touched or legacy unlinked rows with the same monthly/annual cadence are included.
  - If `push_to_stripe_subscriptions` is `true`, the request **must** include an **`Idempotency-Key`** HTTP header (8–255 characters). Missing/short keys return **400**.
  - Rollout is only allowed when at least one **billing** field changed in the same request (`amount_cents`, `stripe_price_id`, `currency`, `interval`, or `interval_count`).
  - Response shape is now **`AdminUpdatePricingPlanResponse`**: all `PricingPlan` fields at the top level **plus** optional `stripe_rollout` (`targeted`, `succeeded`, `failed[]` with per-subscription error strings). HTTP **200** is returned even when some Stripe calls fail; inspect `stripe_rollout.failed`.

- **`GET /api/admin/pricing/plans/price-log`**
  - Returns the latest `amount_cents` adjustments (joins `pricing_change_log`, `pricing_plans`, `users`) for the admin price history panel.

### Stripe rollout implementation

- New module **`backend/src/services/pricing_rollout.rs`**
  - Retrieves each Stripe subscription with `expand[]=items.data`.
  - **Exactly one** subscription item is supported; multi-item subscriptions return a descriptive **400** for that member.
  - **Model A** — updates the item to the catalog’s `stripe_price_id`.
  - **Model B** — sends inline `SubscriptionPriceData` using `stripe_product_id`, `amount_cents`, currency, and interval from the catalog row. Missing product id is rejected with a clear **400**.
  - Each Stripe `Subscription::update` uses a fresh **Stripe idempotency key** derived from the admin key + plan id + stable index so transport retries never double-apply.
  - **Proration:** the code intentionally omits `proration_behavior` and relies on **Stripe’s default** for subscription updates (`create_prorations`). Documented rationale: `async-stripe` exposes two incompatible `SubscriptionProrationBehavior` enums (subscription vs subscription_item); wiring the wrong one silently fails type-check or mis-serialises.

### Auditing

- Successful rollouts emit an additional **`pricing_plan.stripe_rollout`** row in `admin_actions` with targeted / succeeded / failed counts.
- The existing **`pricing_plan.update`** audit row remains unchanged.

### Frontend

- **`src/routes/admin/subscriptions/plans/+page.svelte`**
  - Fixes API paths to the real backend routes: **`/api/admin/pricing/plans`**, **`/api/admin/pricing/plans/price-log`**, **`PUT /api/admin/pricing/plans/{id}`**.
  - Edit form adds **“Also update existing Stripe subscriptions after save”** with audience radios.
  - When enabled, the client sends `stripe_rollout` in the JSON body and an `Idempotency-Key` header (UUID).
  - Displays a short post-save status line summarising Stripe outcomes.

- **`src/lib/api/types.ts`** — documents `PricingStripeRollout`, `AdminUpdatePricingPlanResponse`, and related DTOs for hand-written clients.

### OpenAPI / TypeScript

- `backend/tests/snapshots/openapi.json` regenerated; `pnpm gen:types` refreshes `src/lib/api/schema.d.ts`.

### Documentation

- **`docs/stripe-pricing-models.md`** — new section _Pushing catalog edits to existing Stripe subscriptions_ plus code pointers.

### Operational caveats

1. **`linked_and_unlinked_legacy_same_cadence`** can touch the wrong people if you run **multiple** active monthly (or annual) catalog plans — unlinked rows are only disambiguated by `subscription_plan` enum, not SKU.
2. Legacy subscribers created **before** this release will have **`pricing_plan_id = NULL`** until the next `customer.subscription.updated` webhook refreshes metadata from Stripe — they only appear in the legacy audience bucket.
3. Stripe must be configured (`STRIPE_SECRET_KEY`); otherwise rollout errors surface in `stripe_rollout.failed`.

---
