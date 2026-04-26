# AUDIT — PHASE 2: DOMAIN GAP ANALYSIS

**Date:** 2026-04-17
**Scope:** Parity assessment against WooCommerce + WC Subscriptions + WC Memberships (Domain 1), FluentForms Pro (Domain 2), OptinMonster/Convert Pro/PopupMaker-class (Domain 3), Consent Magic Pro/CookieYes/Complianz (Domain 4), and cross-cutting notifications (Domain 5) — all at April 2026 feature level.

**Rubric.** Each row is graded:

- **Exists?** — Yes / Partial / No / Stub (DB-only, no handler; or handler-only, no UI).
- **Completeness %** — 0 (nothing), 1–25 (scaffolding), 26–50 (core present, many gaps), 51–75 (functional but missing parity features), 76–99 (parity minus polish), 100 (parity met or exceeded).
- **Production-Ready?** — "Yes", "No", or "Partial" (functional but lacking at least one of: authz, audit, idempotency, i18n, observability, tests, a11y).

Paths relative to repo root. `migrations/NNN` abbreviates `backend/migrations/NNN_*.sql`.

---

## DOMAIN 1 — E-COMMERCE / SUBSCRIPTIONS / MEMBERSHIPS

Ruthless assessment: the app today is a **subscription + LMS SaaS**, not a storefront. Almost every product/order/tax/shipping/inventory dimension of WooCommerce is absent. Subscriptions cover only the narrow monthly/annual path Stripe drives directly.

### 1.1 Products

| Feature                                       | Exists? | Location                                 | Completeness | Prod-Ready | Gap Notes                                                                                                   |
| --------------------------------------------- | ------- | ---------------------------------------- | ------------ | ---------- | ----------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------- |
| Simple product                                | No      | —                                        | 0            | No         | No `products` table. Courses and pricing plans are hand-rolled separate tables, not a unified product type. |
| Variable product (attributes × variations)    | No      | —                                        | 0            | No         | —                                                                                                           |
| Grouped product                               | No      | —                                        | 0            | No         | —                                                                                                           |
| External / affiliate product                  | No      | —                                        | 0            | No         | —                                                                                                           |
| Virtual product                               | Partial | `courses`, `pricing_plans`               | 20           | No         | Courses/plans behave as virtual goods but share no common interface.                                        |
| Downloadable product (digital file delivery)  | No      | —                                        | 0            | No         | No `downloads`, no expiring signed URLs, no download-limit enforcement. R2 upload exists for media only.    |
| Subscription product                          | Partial | `subscriptions`, `pricing_plans`         | 45           | Partial    | Hardcoded `subscription_plan` enum `monthly                                                                 | annual`. No `product_id` FK; plan selection is Stripe-price-driven. |
| Bundle / composite product                    | No      | —                                        | 0            | No         | —                                                                                                           |
| SKU, barcode, weight, dimensions              | No      | —                                        | 0            | No         | —                                                                                                           |
| Inventory / stock / thresholds / backorders   | No      | —                                        | 0            | No         | —                                                                                                           |
| Tax class / shipping class per product        | No      | —                                        | 0            | No         | —                                                                                                           |
| Global attributes + per-product attributes    | No      | —                                        | 0            | No         | —                                                                                                           |
| Variation matrix                              | No      | —                                        | 0            | No         | —                                                                                                           |
| Upsells / cross-sells / related               | No      | —                                        | 0            | No         | —                                                                                                           |
| Product image gallery                         | Partial | `media` + `blog_posts.featured_image_id` | 15           | No         | Media library exists; no product→gallery join.                                                              |
| Digital file delivery w/ expiring signed URLs | No      | —                                        | 0            | No         | R2 public URLs today; no `GetObject` presign path.                                                          |

### 1.2 Catalog

| Feature                           | Exists? | Location                            | Completeness | Prod-Ready | Gap Notes                                                                                                                                |
| --------------------------------- | ------- | ----------------------------------- | ------------ | ---------- | ---------------------------------------------------------------------------------------------------------------------------------------- |
| Categories (nested, hierarchical) | Partial | `blog_categories` (has `parent_id`) | 30           | No         | Blog-scoped only. No product-scoped hierarchy.                                                                                           |
| Tags (flat)                       | Partial | `blog_tags`                         | 30           | No         | Blog-scoped only.                                                                                                                        |
| Brands                            | No      | —                                   | 0            | No         | —                                                                                                                                        |
| Attributes (global + per-product) | No      | —                                   | 0            | No         | —                                                                                                                                        |
| Search with faceting              | No      | —                                   | 0            | No         | Only `ILIKE` text search on blog posts / coupons / members. No full-text index, no facets, no Elasticsearch/Meili/Typesense integration. |
| Filters / sorting / pagination    | Partial | per-handler query params            | 40           | Partial    | Pagination consistent via `PaginatedResponse<T>`; filtering is ad-hoc per handler; no unified facet contract.                            |

### 1.3 Cart

| Feature                          | Exists? | Location | Completeness | Prod-Ready | Gap Notes                                                                                              |
| -------------------------------- | ------- | -------- | ------------ | ---------- | ------------------------------------------------------------------------------------------------------ |
| Persistent cart (guest + authed) | No      | —        | 0            | No         | No `carts` table. Stripe Checkout is the cart today.                                                   |
| Merge-on-login                   | No      | —        | 0            | No         | —                                                                                                      |
| Line-item metadata               | No      | —        | 0            | No         | —                                                                                                      |
| Applied coupons at cart level    | No      | —        | 0            | No         | `coupon/validate` + `coupon/apply` endpoints exist but operate on a single `amount_cents`, not a cart. |
| Fees (e.g. processing fee)       | No      | —        | 0            | No         | —                                                                                                      |
| Tax calculation                  | No      | —        | 0            | No         | —                                                                                                      |
| Shipping calculation             | No      | —        | 0            | No         | —                                                                                                      |
| Abandoned-cart recovery          | No      | —        | 0            | No         | —                                                                                                      |

### 1.4 Checkout

| Feature                               | Exists?                | Location                                            | Completeness | Prod-Ready | Gap Notes                                                    |
| ------------------------------------- | ---------------------- | --------------------------------------------------- | ------------ | ---------- | ------------------------------------------------------------ |
| Multi-step checkout                   | No                     | —                                                   | 0            | No         | Stripe Checkout hosted page only.                            |
| Single-page checkout                  | No (hosted)            | `src/routes/api/create-checkout-session/+server.ts` | 25           | Partial    | Redirects to Stripe-hosted checkout; no on-site custom flow. |
| Address book                          | No                     | —                                                   | 0            | No         | No `addresses` table, no country/state ref data.             |
| Saved payment methods                 | No (Stripe vault only) | —                                                   | 10           | No         | Relies on Stripe customer/portal; not reflected in our DB.   |
| Guest checkout                        | No (requires account)  | —                                                   | 0            | No         | Register-then-checkout only.                                 |
| Order notes (customer + internal)     | No                     | —                                                   | 0            | No         | No `orders` table.                                           |
| Terms acceptance capture              | No                     | —                                                   | 0            | No         | —                                                            |
| Tax/VAT ID capture                    | No                     | —                                                   | 0            | No         | —                                                            |
| B2B workflows (company accounts, POs) | No                     | —                                                   | 0            | No         | —                                                            |

### 1.5 Orders

| Feature                                                                             | Exists? | Location | Completeness | Prod-Ready | Gap Notes                                                                                                         |
| ----------------------------------------------------------------------------------- | ------- | -------- | ------------ | ---------- | ----------------------------------------------------------------------------------------------------------------- |
| Order entity                                                                        | No      | —        | 0            | No         | **Entire concept absent.** Revenue is inferred from `sales_events` + `subscriptions`.                             |
| Full state machine (pending→processing→on-hold→completed→refunded→cancelled→failed) | No      | —        | 0            | No         | —                                                                                                                 |
| Partial refunds                                                                     | No      | —        | 0            | No         | —                                                                                                                 |
| Order notes (customer + internal)                                                   | No      | —        | 0            | No         | —                                                                                                                 |
| Order emails                                                                        | No      | —        | 0            | No         | Only subscription-confirmation/cancelled templates exist in `backend/src/email.rs`; not wired to any order event. |
| Invoice PDF                                                                         | No      | —        | 0            | No         | `src/routes/api/greeks-pdf/+server.ts` generates an options-Greeks PDF — unrelated.                               |
| Packing slips                                                                       | No      | —        | 0            | No         | —                                                                                                                 |
| Post-creation order editing                                                         | No      | —        | 0            | No         | —                                                                                                                 |

### 1.6 Payments

| Feature                                                   | Exists?               | Location                                                            | Completeness | Prod-Ready | Gap Notes                                                                                                                                                                           |
| --------------------------------------------------------- | --------------------- | ------------------------------------------------------------------- | ------------ | ---------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Stripe Payment Intents / SCA / 3DS2                       | Partial               | Stripe Checkout delegates SCA; no direct PaymentIntent flow in Rust | 30           | Partial    | Only subscription-via-Checkout; no cards-on-file PaymentIntent path.                                                                                                                |
| Stripe Connect (marketplace)                              | No                    | —                                                                   | 0            | No         | —                                                                                                                                                                                   |
| PayPal                                                    | No                    | —                                                                   | 0            | No         | —                                                                                                                                                                                   |
| Apple Pay / Google Pay                                    | Indirect              | via Stripe Checkout                                                 | 25           | Partial    | Inherited from Stripe Checkout; not explicitly configured.                                                                                                                          |
| ACH / SEPA                                                | No                    | —                                                                   | 0            | No         | —                                                                                                                                                                                   |
| BNPL (Klarna / Afterpay / Affirm)                         | No                    | —                                                                   | 0            | No         | —                                                                                                                                                                                   |
| Manual / offline payment methods                          | No                    | —                                                                   | 0            | No         | —                                                                                                                                                                                   |
| Payment method vault (tokenized)                          | No (Stripe-side only) | —                                                                   | 10           | No         | —                                                                                                                                                                                   |
| Webhook handlers with idempotency                         | Partial               | `backend/src/handlers/webhooks.rs`, `migrations/017`                | 55           | Partial    | HMAC verify + idempotency table + 5-min tolerance + cleanup. Only handles 4 Stripe event types; no invoice._, payment_intent._, charge.\*, dispute, dunning. Other providers: none. |
| Idempotency-Key HTTP header on mutating payment endpoints | No                    | —                                                                   | 0            | No         | Not implemented on server or client.                                                                                                                                                |

### 1.7 Subscriptions (WC Subscriptions parity)

| Feature                                     | Exists? | Location                                                                                                      | Completeness | Prod-Ready | Gap Notes                                                                                                                         |
| ------------------------------------------- | ------- | ------------------------------------------------------------------------------------------------------------- | ------------ | ---------- | --------------------------------------------------------------------------------------------------------------------------------- |
| Billing intervals (month / year)            | Partial | enum `subscription_plan` ∈ {monthly, annual}                                                                  | 45           | Partial    | Hardcoded 2 intervals; no week/quarter/custom N-units.                                                                            |
| Sign-up fees                                | No      | —                                                                                                             | 0            | No         | —                                                                                                                                 |
| Trial periods                               | Partial | `pricing_plans.trial_days` field                                                                              | 20           | No         | Column exists but no server logic honors it.                                                                                      |
| Synchronized renewals (same day each month) | No      | —                                                                                                             | 0            | No         | —                                                                                                                                 |
| Prorations on mid-cycle changes             | No      | —                                                                                                             | 0            | No         | Delegated to Stripe if set up; no proration logic in our code.                                                                    |
| Upgrades / downgrades                       | No      | —                                                                                                             | 0            | No         | Plan change endpoint absent.                                                                                                      |
| Pause / resume                              | Partial | `handlers/member.rs::post_subscription_{cancel,resume}` + `stripe_api::set_subscription_cancel_at_period_end` | 40           | Partial    | Only supports cancel-at-period-end toggle. No true "pause" (skip N cycles) or resume-from-paused.                                 |
| Retry logic (smart dunning)                 | No      | —                                                                                                             | 0            | No         | Not tracking `past_due`-triggered retries beyond the webhook status update.                                                       |
| Failed payment handling                     | Partial | webhook status `past_due`                                                                                     | 15           | No         | Status is captured; no email, no retry scheduler, no grace period, no auto-cancel after N failures.                               |
| Subscription switching                      | No      | —                                                                                                             | 0            | No         | —                                                                                                                                 |
| Manual renewal                              | No      | —                                                                                                             | 0            | No         | —                                                                                                                                 |
| Early renewal                               | No      | —                                                                                                             | 0            | No         | —                                                                                                                                 |
| Recurring vs one-time coupons               | Partial | Stripe coupon id field exists; no wiring                                                                      | 20           | No         | `coupons.stripe_coupon_id` / `stripe_promotion_code_id` columns are present but no code creates the Stripe-side coupon to mirror. |

### 1.8 Memberships (WC Memberships parity)

| Feature                                                             | Exists? | Location                                                | Completeness | Prod-Ready | Gap Notes                                                                                                                                           |
| ------------------------------------------------------------------- | ------- | ------------------------------------------------------- | ------------ | ---------- | --------------------------------------------------------------------------------------------------------------------------------------------------- |
| Membership plans (distinct from products)                           | No      | —                                                       | 0            | No         | **Entire concept absent.** "Member" = has active subscription.                                                                                      |
| Content restriction on posts / pages / products / categories / URLs | Partial | client-side `auth.isAuthenticated` + admin layout guard | 10           | No         | No server-side restriction engine; blog posts expose `visibility: public/members_only` via `blog_posts.visibility TEXT` but no handler enforces it. |
| Product-granted memberships                                         | No      | —                                                       | 0            | No         | —                                                                                                                                                   |
| Time-based access (drip content)                                    | No      | —                                                       | 0            | No         | —                                                                                                                                                   |
| Member discounts / member-only pricing                              | No      | —                                                       | 0            | No         | —                                                                                                                                                   |
| Access scheduling                                                   | No      | —                                                       | 0            | No         | —                                                                                                                                                   |
| Expiration + renewal (membership-level, not subscription)           | No      | —                                                       | 0            | No         | —                                                                                                                                                   |
| Email drip sequences                                                | No      | —                                                       | 0            | No         | —                                                                                                                                                   |
| Membership-tied forums / courses hooks                              | Partial | `courses.is_included_in_subscription`                   | 15           | No         | Boolean flag; no policy engine.                                                                                                                     |

### 1.9 Coupons

| Feature                              | Exists? | Location                                                | Completeness | Prod-Ready | Gap Notes                                                                                                                      |
| ------------------------------------ | ------- | ------------------------------------------------------- | ------------ | ---------- | ------------------------------------------------------------------------------------------------------------------------------ |
| Percentage                           | Yes     | `migrations/013` + `handlers/coupons.rs`                | 85           | Partial    | Money-as-`f64` at API boundary — **constraint violation**.                                                                     |
| Fixed cart                           | Partial | `DiscountType::FixedAmount`                             | 60           | Partial    | Exists but no cart; applied against a single `amount_cents`.                                                                   |
| Fixed product                        | No      | —                                                       | 0            | No         | Applies-to "plan" / "course" lists are primitive.                                                                              |
| BOGO                                 | No      | —                                                       | 0            | No         | —                                                                                                                              |
| Free shipping                        | No      | —                                                       | 0            | No         | No shipping.                                                                                                                   |
| First-order-only                     | Yes     | `coupons.first_purchase_only`                           | 90           | Partial    | Verified by `coupon_usages` row count, not by actual "has completed order" state.                                              |
| Usage limits (global)                | Yes     | `usage_limit` + `usage_count` atomic increment          | 95           | Partial    | Race-safe (UPDATE + check). No reservation on in-flight checkouts.                                                             |
| Usage limits (per-user)              | Yes     | `per_user_limit`                                        | 90           | Partial    | Enforced via `COUNT(coupon_usages)`.                                                                                           |
| Date ranges                          | Yes     | `starts_at` / `expires_at`                              | 95           | Partial    | UTC-stored.                                                                                                                    |
| Minimum spend                        | Yes     | `min_purchase_cents`                                    | 90           | Yes        | Enforced server-side.                                                                                                          |
| Product/category inclusion/exclusion | Partial | `applicable_plan_ids`, `applicable_course_ids` UUID[]   | 50           | Partial    | Inclusion only, no exclusion; no category-wide scope. Logic in `validate_coupon_inner` checks `applies_to` string enum ad hoc. |
| Individual use (not stackable)       | Partial | `stackable` bool                                        | 40           | Partial    | Column exists; no engine enforces stacking rules — there is no cart.                                                           |
| Recurring subscription coupons       | Stub    | `stripe_coupon_id` / `stripe_promotion_code_id` columns | 20           | No         | Mirror to Stripe not implemented.                                                                                              |
| Free trial coupon                    | Stub    | `DiscountType::FreeTrial` enum                          | 15           | No         | Discount calc returns `None`; no trial-days grant logic.                                                                       |
| Bulk code generation                 | Yes     | `admin_bulk_create_coupons`                             | 80           | Partial    | 1–1000 codes; thread-local RNG; no cryptographic-grade uniqueness check.                                                       |

### 1.10 Taxes

| Feature                            | Exists? | Location | Completeness | Prod-Ready | Gap Notes |
| ---------------------------------- | ------- | -------- | ------------ | ---------- | --------- |
| Tax rates by region                | No      | —        | 0            | No         | —         |
| Compound taxes                     | No      | —        | 0            | No         | —         |
| Tax classes                        | No      | —        | 0            | No         | —         |
| VAT / GST / HST                    | No      | —        | 0            | No         | —         |
| Tax-inclusive vs exclusive pricing | No      | —        | 0            | No         | —         |
| EU VAT MOSS                        | No      | —        | 0            | No         | —         |
| TaxJar / Avalara integration hooks | No      | —        | 0            | No         | —         |
| Tax-exempt customers               | No      | —        | 0            | No         | —         |

### 1.11 Shipping

| Feature               | Exists? | Location | Completeness | Prod-Ready | Gap Notes                         |
| --------------------- | ------- | -------- | ------------ | ---------- | --------------------------------- |
| All shipping concepts | No      | —        | 0            | No         | Digital-goods-only product space. |

### 1.12 Inventory

| Feature                | Exists? | Location | Completeness | Prod-Ready | Gap Notes         |
| ---------------------- | ------- | -------- | ------------ | ---------- | ----------------- |
| All inventory concepts | No      | —        | 0            | No         | No physical SKUs. |

### 1.13 Customers

| Feature                           | Exists? | Location                                                                  | Completeness | Prod-Ready | Gap Notes                                  |
| --------------------------------- | ------- | ------------------------------------------------------------------------- | ------------ | ---------- | ------------------------------------------ |
| Profile                           | Yes     | `users` + `migrations/005`                                                | 80           | Yes        | Bio/social URLs present.                   |
| Address book                      | No      | —                                                                         | 0            | No         | —                                          |
| Order history                     | No      | —                                                                         | 0            | No         | No orders.                                 |
| Download library                  | No      | —                                                                         | 0            | No         | —                                          |
| Subscription dashboard            | Yes     | `src/routes/dashboard/account/+page.svelte` + `/api/member/subscription*` | 75           | Partial    | View, cancel, resume, billing-portal-link. |
| Membership dashboard              | No      | —                                                                         | 0            | No         | —                                          |
| Wishlist                          | No      | —                                                                         | 0            | No         | —                                          |
| B2B company accounts w/ sub-users | No      | —                                                                         | 0            | No         | —                                          |

### 1.14 Reports / Analytics

| Feature            | Exists? | Location                                          | Completeness | Prod-Ready | Gap Notes                                                                                |
| ------------------ | ------- | ------------------------------------------------- | ------------ | ---------- | ---------------------------------------------------------------------------------------- |
| Sales totals       | Partial | `sales_events` + `admin.rs::analytics_revenue`    | 40           | Partial    | `sales_events` table exists but has no inserters (not populated by webhooks).            |
| MRR / ARR          | Partial | `db::admin_estimated_mrr_arr_cents`               | 60           | Partial    | Computed at read time as `count × price`, not event-sourced.                             |
| Churn              | No      | —                                                 | 0            | No         | `monthly_revenue_snapshots` table has `churned_subscribers` column but no writer.        |
| LTV                | No      | —                                                 | 0            | No         | —                                                                                        |
| Cohort analysis    | No      | —                                                 | 0            | No         | —                                                                                        |
| Coupon performance | Partial | `admin_coupon_stats` + `admin_list_coupon_usages` | 70           | Partial    | Active count, total usages, total discount. No per-coupon ROI, no redeemed-vs-attempted. |
| Tax reports        | No      | —                                                 | 0            | No         | —                                                                                        |
| Inventory reports  | No      | —                                                 | 0            | No         | —                                                                                        |
| Membership reports | No      | —                                                 | 0            | No         | —                                                                                        |

### 1.15 Admin UI

| Feature                   | Exists? | Location                                            | Completeness | Prod-Ready | Gap Notes                                                                      |
| ------------------------- | ------- | --------------------------------------------------- | ------------ | ---------- | ------------------------------------------------------------------------------ |
| Members admin             | Yes     | `src/routes/admin/members/**`                       | 70           | Partial    | List, detail, role change, delete, cancel/resume sub.                          |
| Pricing plans admin       | Yes     | `src/routes/admin/subscriptions/plans/+page.svelte` | 75           | Partial    | CRUD + change log.                                                             |
| Coupons admin             | Yes     | `src/routes/admin/coupons/**`                       | 80           | Partial    | CRUD, toggle, bulk, usages.                                                    |
| Subscriptions admin list  | Yes     | `handlers/subscriptions.rs` (untracked)             | 70           | Partial    | List + stats; no per-sub edit screen wired.                                    |
| Orders admin              | No      | —                                                   | 0            | No         | —                                                                              |
| Products admin            | No      | —                                                   | 0            | No         | —                                                                              |
| Reports admin             | Partial | `src/routes/admin/analytics/**`                     | 50           | Partial    | Revenue + analytics charts; missing cohorts, LTV, etc.                         |
| Watchlists + alerts admin | Yes     | `src/routes/admin/watchlists/**`                    | 80           | Yes        | Domain-specific (options trades).                                              |
| Blog admin                | Yes     | `src/routes/admin/blog/**`                          | 80           | Partial    | Full CRUD; trash/restore; TipTap editor; media library.                        |
| Courses admin             | Yes     | `src/routes/admin/courses/**`                       | 75           | Partial    | CRUD + modules + lessons + publish toggle.                                     |
| Popups admin              | Yes     | `src/routes/admin/popups/**`                        | 60           | Partial    | List + new + [id] edit. Missing template library, live device preview, A/B UI. |

---

## DOMAIN 2 — FORMS (FluentForms Pro parity)

Crushing verdict: **there is no general-purpose forms subsystem.** What exists is a popup that stores arbitrary `form_data JSONB`. No builder, no field types, no validation schema, no integrations. Everything below that isn't the popup path is greenfield.

### 2.1 Form builder

| Feature                                 | Exists? | Completeness | Prod-Ready | Gap Notes                         |
| --------------------------------------- | ------- | ------------ | ---------- | --------------------------------- |
| Drag-and-drop admin UI                  | No      | 0            | No         | No `forms` table, no admin route. |
| Live preview                            | No      | 0            | No         | —                                 |
| Conditional logic (show/hide/skip/calc) | No      | 0            | No         | —                                 |
| Multi-column layouts                    | No      | 0            | No         | —                                 |
| Multi-step with progress bar            | No      | 0            | No         | —                                 |
| Save-and-resume                         | No      | 0            | No         | —                                 |
| Repeaters / nested fields               | No      | 0            | No         | —                                 |

### 2.2 Field types

| Field type                                        | Exists? | Gap Notes                                                                               |
| ------------------------------------------------- | ------- | --------------------------------------------------------------------------------------- |
| text / email / phone / URL / textarea / number    | Partial | Only as popup-content `PopupElement.type` string enum (12 types, no server validation). |
| slider / rating                                   | No      | —                                                                                       |
| date / time / datetime                            | No      | —                                                                                       |
| select / multi-select / radio / checkbox          | Partial | Enum exists in `PopupElement.type`; no option schema storage.                           |
| file upload (multi, chunked, restrictions, S3/R2) | No      | Media upload exists only for admin blog media, not forms.                               |
| image upload w/ preview                           | No      | —                                                                                       |
| signature pad                                     | No      | —                                                                                       |
| rich-text editor field                            | No      | TipTap integrated for blog posts only.                                                  |
| hidden / HTML block / section break / page break  | No      | —                                                                                       |
| address (Google/Mapbox autocomplete)              | No      | —                                                                                       |
| GDPR consent / terms                              | No      | —                                                                                       |
| payment field (Stripe/PayPal)                     | No      | —                                                                                       |
| subscription field                                | No      | —                                                                                       |
| quiz / survey / NPS / Likert / matrix / ranking   | No      | —                                                                                       |
| calculation field                                 | No      | —                                                                                       |
| dynamic dropdown (API-sourced)                    | No      | —                                                                                       |
| country / state                                   | No      | —                                                                                       |
| post / product selector                           | No      | —                                                                                       |

### 2.3 Validation

| Feature                               | Exists? | Completeness | Prod-Ready | Gap Notes                                                                                                           |
| ------------------------------------- | ------- | ------------ | ---------- | ------------------------------------------------------------------------------------------------------------------- |
| Client-side validation                | Partial | 15           | No         | Per-Svelte-component ad hoc; no schema.                                                                             |
| Server-side validation                | Partial | 10           | No         | `validator` crate used on Rust DTOs (coupons, auth, popups), not on form submissions (which are free-form `JSONB`). |
| Built-in rules                        | Partial | 15           | No         | Only auth-adjacent validators.                                                                                      |
| Custom regex                          | No      | 0            | No         | —                                                                                                                   |
| Cross-field validation                | No      | 0            | No         | —                                                                                                                   |
| Async validation (unique email, etc.) | Partial | 20           | No         | Only in `handlers/auth.rs::register`.                                                                               |
| Conditional validation                | No      | 0            | No         | —                                                                                                                   |

### 2.4 Submissions

| Feature                                        | Exists? | Location                                                                 | Completeness | Prod-Ready | Gap Notes                                                                                                                                                                                                            |
| ---------------------------------------------- | ------- | ------------------------------------------------------------------------ | ------------ | ---------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Persist to Postgres                            | Partial | `popup_submissions`                                                      | 40           | Partial    | Free-form JSONB; no per-form schema typing; no GDPR retention policy.                                                                                                                                                |
| Full audit (IP, UA, referrer, UTM, timestamps) | Partial | `popup_submissions.ip_address`, `user_agent`, `page_url`, `submitted_at` | 50           | Partial    | `ip_address` and `user_agent` columns exist but **handler never populates them** (see `public_submit_form` — only writes `popup_id`, `user_id`, `session_id`, `form_data`, `page_url`). No UTM capture, no referrer. |
| Partial submission capture (autosave)          | No      | 0                                                                        | No           | —          |
| Search / filter submissions                    | Partial | admin submissions list per popup                                         | 40           | No         | List-by-popup only; no search within `form_data`.                                                                                                                                                                    |
| Export CSV / XLSX                              | No      | 0                                                                        | No           | —          |
| Bulk actions                                   | No      | 0                                                                        | No           | —          |

### 2.5 Notifications (form-level)

| Channel                                                   | Exists? | Gap Notes                        |
| --------------------------------------------------------- | ------- | -------------------------------- |
| Email (multiple recipients, conditional routing, dynamic) | No      | No per-form notification config. |
| SMS (Twilio)                                              | No      | No Twilio crate in `Cargo.toml`. |
| Slack / Discord / webhook                                 | No      | —                                |
| In-app admin notification                                 | No      | No in-app notification table.    |

### 2.6 Integrations (third-party)

| Integration                                                    | Exists? | Gap Notes                       |
| -------------------------------------------------------------- | ------- | ------------------------------- |
| Mailchimp / ActiveCampaign / ConvertKit / HubSpot / Salesforce | No      | No outbound HTTP client wiring. |
| Zapier / Make / webhook-out                                    | No      | —                               |
| Google Sheets / Notion / Airtable / Zoho                       | No      | —                               |

### 2.7 Payment forms

| Feature                               | Exists? | Gap Notes                                                    |
| ------------------------------------- | ------- | ------------------------------------------------------------ |
| Stripe / PayPal field                 | No      | Stripe integration is subscription-only via hosted Checkout. |
| Subscription signup via form          | No      | —                                                            |
| Donation (fixed / custom / suggested) | No      | —                                                            |
| Product purchase via form             | No      | —                                                            |

### 2.8 Advanced

| Feature                        | Exists? | Gap Notes                             |
| ------------------------------ | ------- | ------------------------------------- |
| Conversational forms           | No      | —                                     |
| Quiz with scoring              | No      | —                                     |
| Calculators                    | No      | —                                     |
| Surveys with branching         | No      | —                                     |
| Akismet / hCaptcha / Turnstile | No      | —                                     |
| Honeypot                       | No      | —                                     |
| Rate limiting (form-level)     | No      | Only auth-route rate limiting exists. |
| Duplicate prevention           | No      | —                                     |

### 2.9 Layouts / Styling

| Feature                                     | Exists? | Gap Notes                                                  |
| ------------------------------------------- | ------- | ---------------------------------------------------------- |
| Per-form theme override (PE7 tokens, OKLCH) | No      | No forms; tokens today are hex.                            |
| 9-breakpoint responsive                     | No      | Current token file has only 768px/1024px breakpoints.      |
| Dark/light theme aware                      | Partial | Root styles are single-theme (dark navy); no theme switch. |

### 2.10 Logic / post-submit actions

| Feature                                         | Exists? | Gap Notes                   |
| ----------------------------------------------- | ------- | --------------------------- |
| Conditional routing / confirmations / redirects | Partial | `popups.redirect_url` only. |
| Webhook triggers                                | No      | —                           |
| Post-submit action chain                        | No      | —                           |

### 2.11 Accessibility

| Feature                      | Exists?    | Gap Notes                                   |
| ---------------------------- | ---------- | ------------------------------------------- |
| WCAG 2.2 AA                  | Unverified | No audit report; no automated a11y CI step. |
| Full keyboard nav            | Unverified | —                                           |
| Screen reader tested         | Unverified | —                                           |
| ARIA live regions for errors | Unverified | —                                           |

---

## DOMAIN 3 — POPUPS (OptinMonster / Convert Pro / PopupMaker-class)

Best-covered of the non-trivial domains. There is a working popup engine with 6 types × 7 triggers and basic targeting, admin CRUD, duplicate, submissions viewer. But targeting is shallow, no A/B, no gamified formats, no templates, frequency rules are coarse.

### 3.1 Popup types

| Type                                 | Exists? | Location                                                          | Completeness | Prod-Ready | Gap Notes                                                 |
| ------------------------------------ | ------- | ----------------------------------------------------------------- | ------------ | ---------- | --------------------------------------------------------- |
| Modal                                | Yes     | `migrations/015` CHECK constraint `modal`; `PopupRenderer.svelte` | 75           | Partial    | —                                                         |
| Slide-in                             | Yes     | enum `slide_in`                                                   | 65           | Partial    | Renderer coverage not verified per-type.                  |
| Floating bar                         | Yes     | enum `floating_bar`                                               | 55           | Partial    | —                                                         |
| Fullscreen takeover                  | Yes     | enum `fullscreen`                                                 | 55           | Partial    | —                                                         |
| Inline                               | Yes     | enum `inline`                                                     | 45           | Partial    | No `inline` mount API exposed for page authors.           |
| Banner                               | Yes     | enum `banner`                                                     | 50           | Partial    | Not in target spec; treat as "bar" variant.               |
| Content locker                       | No      | —                                                                 | 0            | No         | —                                                         |
| Countdown bar                        | No      | —                                                                 | 0            | No         | —                                                         |
| Notification-style                   | No      | —                                                                 | 0            | No         | —                                                         |
| Exit-intent modal                    | Yes     | trigger `exit_intent`                                             | 60           | Partial    | Desktop only (`getDevice() !== 'desktop'` short-circuit). |
| Scroll-triggered                     | Yes     | trigger `scroll_percentage`                                       | 70           | Partial    | —                                                         |
| Time-delayed                         | Yes     | trigger `time_delay`                                              | 85           | Yes        | Default 3000ms.                                           |
| Gamified (spin-to-win, scratch card) | No      | —                                                                 | 0            | No         | —                                                         |

### 3.2 Triggers

| Trigger                 | Exists? | Location                  | Completeness | Prod-Ready | Gap Notes                                      |
| ----------------------- | ------- | ------------------------- | ------------ | ---------- | ---------------------------------------------- |
| Page load               | Yes     | `on_load`                 | 90           | Yes        | —                                              |
| Time on page            | Yes     | `time_delay`              | 90           | Yes        | —                                              |
| Scroll depth (%)        | Yes     | `scroll_percentage`       | 85           | Yes        | —                                              |
| Exit intent             | Yes     | `exit_intent`             | 65           | Partial    | Desktop only; no mobile-back-button heuristic. |
| Inactivity              | Yes     | `inactivity`              | 70           | Yes        | —                                              |
| Click element           | Yes     | `click` (CSS selector)    | 70           | Partial    | Selector-based; no click-count threshold.      |
| After form submit       | No      | —                         | 0            | No         | —                                              |
| After N page views      | No      | —                         | 0            | No         | —                                              |
| After adding to cart    | No      | —                         | 0            | No         | No cart.                                       |
| Cart abandonment signal | No      | —                         | 0            | No         | —                                              |
| Cookie-based            | No      | —                         | 0            | No         | —                                              |
| URL parameter           | No      | —                         | 0            | No         | —                                              |
| Device type             | Partial | as targeting, not trigger | 30           | Partial    | —                                              |
| Returning visitor       | No      | —                         | 0            | No         | —                                              |

### 3.3 Targeting

| Feature                            | Exists? | Location                                           | Completeness | Prod-Ready | Gap Notes                                                                                     |
| ---------------------------------- | ------- | -------------------------------------------------- | ------------ | ---------- | --------------------------------------------------------------------------------------------- |
| URL rules (include/exclude, regex) | Partial | `matches_page_pattern` in `handlers/popups.rs`     | 35           | Partial    | Glob-only (`*` prefix or suffix or exact); no exclude list, no regex.                         |
| Referrer                           | No      | —                                                  | 0            | No         | —                                                                                             |
| UTM source/campaign/medium         | No      | —                                                  | 0            | No         | —                                                                                             |
| Geolocation                        | No      | —                                                  | 0            | No         | —                                                                                             |
| Device (mobile/desktop/tablet)     | Yes     | `targeting_rules.devices`, `getDevice()` in engine | 80           | Yes        | Width-based classification.                                                                   |
| Browser detection                  | No      | —                                                  | 0            | No         | —                                                                                             |
| New vs returning visitor           | No      | —                                                  | 0            | No         | —                                                                                             |
| Logged-in status                   | Partial | `userStatus` array, `getUserStatus()`              | 55           | Partial    | Maps `auth.isAuthenticated`+`isAdmin` to `logged_in/logged_out/member`; no tier-based status. |
| Membership tier                    | No      | —                                                  | 0            | No         | —                                                                                             |
| Cart value / cart contents         | No      | —                                                  | 0            | No         | —                                                                                             |
| Day / time                         | No      | —                                                  | 0            | No         | —                                                                                             |
| A/B test variant                   | No      | —                                                  | 0            | No         | —                                                                                             |

### 3.4 Frequency

| Feature                 | Exists? | Location                           | Completeness | Prod-Ready | Gap Notes                                    |
| ----------------------- | ------- | ---------------------------------- | ------------ | ---------- | -------------------------------------------- |
| Once per session        | Yes     | `once_per_session`, sessionStorage | 90           | Yes        | Client-side.                                 |
| Once ever               | Yes     | `once_ever`, localStorage          | 85           | Partial    | Client-side only — cleared by privacy tools. |
| Every time              | Yes     | `every_time`                       | 95           | Yes        | —                                            |
| Every N days            | No      | `custom` enum exists but unwired   | 10           | No         | —                                            |
| Until converted         | No      | —                                  | 0            | No         | —                                            |
| Until dismissed N times | No      | —                                  | 0            | No         | —                                            |

### 3.5 Content

| Feature                              | Exists? | Gap Notes                                                                        |
| ------------------------------------ | ------- | -------------------------------------------------------------------------------- |
| WYSIWYG                              | Partial | TipTap exists for blog; popup content is an `elements` array in JSON.            |
| Embedded forms (Domain 2 injectable) | No      | No form system to inject.                                                        |
| Countdown timers                     | No      | —                                                                                |
| Video embeds                         | No      | —                                                                                |
| Product cards                        | No      | —                                                                                |
| Custom HTML                          | Partial | `content_json.elements` can include type-tagged blocks; sanitization unverified. |

### 3.6 Builder

| Feature                        | Exists?    | Gap Notes                                                          |
| ------------------------------ | ---------- | ------------------------------------------------------------------ |
| Drag-and-drop admin            | Partial    | `src/routes/admin/popups/[id]` exists but not inspected in detail. |
| Live preview                   | Unverified | —                                                                  |
| Template library               | No         | —                                                                  |
| Device preview (9 breakpoints) | No         | Only 2 CSS breakpoints globally.                                   |

### 3.7 Analytics

| Feature                                | Exists? | Location                             | Completeness | Prod-Ready        | Gap Notes                            |
| -------------------------------------- | ------- | ------------------------------------ | ------------ | ----------------- | ------------------------------------ |
| Impressions / conversions / dismissals | Yes     | `popup_events` + `admin_*_analytics` | 80           | Partial           | 4 event types tracked.               |
| Conversion rate                        | Yes     | computed in `build_popup_analytics`  | 85           | Yes               | `(submissions / impressions) × 100`. |
| A/B variant performance                | No      | 0                                    | No           | No variant model. |
| Revenue attribution                    | No      | 0                                    | No           | —                 |

### 3.8 A/B testing

| Feature                 | Exists? | Gap Notes |
| ----------------------- | ------- | --------- |
| Split traffic           | No      | —         |
| Significance calculator | No      | —         |
| Winner auto-promotion   | No      | —         |

### 3.9 Accessibility

| Feature                | Exists?    | Gap Notes                                                                              |
| ---------------------- | ---------- | -------------------------------------------------------------------------------------- |
| Focus trap             | Unverified | PopupRenderer not read.                                                                |
| ESC to close           | Unverified | —                                                                                      |
| ARIA dialog semantics  | Unverified | —                                                                                      |
| Reduced-motion respect | Partial    | Global `@media (prefers-reduced-motion: reduce)` in `global.css` applies blanket rule. |

---

## DOMAIN 4 — CONSENT MANAGEMENT (Consent Magic / CookieYes / Complianz)

**Nothing exists.** No banner, no category model, no log, no DSAR, no Consent Mode v2, no TCF v2.2, no GPC honoring, no geo, no i18n. `src/routes/privacy/` and `terms/` are static marketing pages. This is the largest greenfield block in the audit.

### 4.1 Regulations supported

| Regulation                             | Exists? | Gap Notes |
| -------------------------------------- | ------- | --------- |
| GDPR                                   | No      | —         |
| CCPA / CPRA                            | No      | —         |
| LGPD                                   | No      | —         |
| PIPEDA                                 | No      | —         |
| ePrivacy                               | No      | —         |
| UK GDPR                                | No      | —         |
| Quebec Law 25                          | No      | —         |
| CO / VA / CT / UT / TX / FL state laws | No      | —         |
| Google Consent Mode v2                 | No      | —         |
| IAB TCF v2.2                           | No      | —         |
| Global Privacy Control (GPC)           | No      | —         |

### 4.2 Banner

| Feature                                         | Exists? | Gap Notes                    |
| ----------------------------------------------- | ------- | ---------------------------- |
| Customizable (PE7 tokens, OKLCH, 9 breakpoints) | No      | Tokens hex today; no banner. |
| Position (bottom/top/center/corner)             | No      | —                            |
| Layout (bar/box/popup)                          | No      | —                            |
| Per-geo auto-config                             | No      | —                            |

### 4.3 Consent categories

| Feature                                                                   | Exists? | Gap Notes              |
| ------------------------------------------------------------------------- | ------- | ---------------------- |
| Strictly necessary / functional / analytics / marketing / personalization | No      | No `consent_*` tables. |
| Custom categories                                                         | No      | —                      |
| Granular per-service toggles                                              | No      | —                      |

### 4.4 Scripts manager / cookie scanner / log / DSAR

| Feature                                              | Exists? | Gap Notes                                      |
| ---------------------------------------------------- | ------- | ---------------------------------------------- |
| Scan-and-block third-party scripts pre-consent       | No      | —                                              |
| Conditional loading based on consent                 | No      | —                                              |
| Auto-blocking of known trackers (GA, FB, Hotjar…)    | No      | `AnalyticsBeacon.svelte` runs unconditionally. |
| Automated site scan                                  | No      | —                                              |
| Cookie inventory + auto-categorization               | No      | —                                              |
| Declaration generation                               | No      | `/privacy` is hand-written.                    |
| Immutable consent log                                | No      | —                                              |
| DSAR (access / delete / portability / rectification) | No      | —                                              |
| Exportable log for DSAR                              | No      | —                                              |

### 4.5 Consent Mode v2 / Geo / i18n / Admin

| Feature                                                                      | Exists? | Gap Notes                                    |
| ---------------------------------------------------------------------------- | ------- | -------------------------------------------- |
| `ad_storage / analytics_storage / ad_user_data / ad_personalization` signals | No      | —                                            |
| IP-based geo detection                                                       | No      | Nothing ingests IP beyond rate-limit keying. |
| Region-specific banner variants                                              | No      | —                                            |
| Multi-language framework + translation UI                                    | No      | No i18n in codebase.                         |
| Policy version control                                                       | No      | —                                            |
| Banner A/B testing                                                           | No      | —                                            |
| Consent-rate analytics per region                                            | No      | —                                            |

---

## DOMAIN 5 — NOTIFICATIONS (cross-cutting)

Email-only today, 4 inline Tera templates, only **one** template (`password_reset`) is actually invoked from a handler. No queue, no retry, no DLQ, no preferences, no unsubscribe, no open/click tracking, no bounce handling. Multi-channel (SMS, push, Slack, Discord, webhook-out) entirely absent.

### 5.1 Channels

| Channel                                        | Exists? | Location                                | Completeness | Prod-Ready | Gap Notes                                      |
| ---------------------------------------------- | ------- | --------------------------------------- | ------------ | ---------- | ---------------------------------------------- |
| Transactional email (Postmark/Resend/SES/SMTP) | Partial | `backend/src/email.rs`, Lettre STARTTLS | 35           | Partial    | SMTP only; no API-based provider; no fallback. |
| Marketing email                                | No      | —                                       | 0            | No         | —                                              |
| SMS (Twilio)                                   | No      | —                                       | 0            | No         | —                                              |
| Push (web push via VAPID)                      | No      | —                                       | 0            | No         | Service worker exists but no VAPID key mgmt.   |
| iOS/Android push (FCM/APNs)                    | No      | —                                       | 0            | No         | —                                              |
| In-app inbox                                   | No      | —                                       | 0            | No         | No `in_app_notifications` table.               |
| Slack / Discord                                | No      | —                                       | 0            | No         | —                                              |
| Webhook-out                                    | No      | —                                       | 0            | No         | —                                              |

### 5.2 Templates

| Feature                            | Exists? | Location            | Completeness | Prod-Ready | Gap Notes                              |
| ---------------------------------- | ------- | ------------------- | ------------ | ---------- | -------------------------------------- |
| Template language (MJML or equiv.) | Partial | Tera inline strings | 30           | No         | Not MJML; no responsive email builder. |
| Per-template variants              | No      | —                   | 0            | No         | —                                      |
| i18n                               | No      | —                   | 0            | No         | —                                      |
| Preview in admin                   | No      | —                   | 0            | No         | —                                      |
| Test-send                          | No      | —                   | 0            | No         | —                                      |
| Version control                    | No      | —                   | 0            | No         | —                                      |

### 5.3 Orchestration

| Feature                               | Exists? | Location | Completeness | Prod-Ready | Gap Notes                                                                  |
| ------------------------------------- | ------- | -------- | ------------ | ---------- | -------------------------------------------------------------------------- |
| Event-driven bus (Rust)               | No      | —        | 0            | No         | No `tokio::sync::broadcast`, no external queue.                            |
| Retry with exponential backoff        | No      | —        | 0            | No         | —                                                                          |
| Dead-letter queue                     | No      | —        | 0            | No         | —                                                                          |
| Rate limiting (per-user, per-channel) | No      | —        | 0            | No         | Rate limiter exists for HTTP ingress only.                                 |
| Per-user quiet hours                  | No      | —        | 0            | No         | —                                                                          |
| Per-user channel preferences          | No      | —        | 0            | No         | —                                                                          |
| Unsubscribe handling per category     | No      | —        | 0            | No         | Template footer mentions `/settings` link; route not wired to preferences. |

### 5.4 Audit / deliverability

| Feature                             | Exists? | Gap Notes                        |
| ----------------------------------- | ------- | -------------------------------- |
| Delivery log                        | No      | —                                |
| Open/click tracking (consent-gated) | No      | —                                |
| Bounce handling                     | No      | No SES/Postmark webhook inbound. |
| Complaint handling                  | No      | —                                |

### 5.5 Admin UI

| Feature                    | Exists? | Gap Notes                               |
| -------------------------- | ------- | --------------------------------------- |
| Template editor            | No      | Templates live in Rust `const` strings. |
| Send history               | No      | —                                       |
| User preference management | No      | —                                       |
| Broadcast composer         | No      | —                                       |

### 5.6 Handler invocations (dead-code check)

| Template / sender                | Wired from                          | Status                                                              |
| -------------------------------- | ----------------------------------- | ------------------------------------------------------------------- |
| `send_password_reset`            | `handlers/auth.rs::forgot_password` | **Live**                                                            |
| `send_welcome`                   | —                                   | **Dead** (never called; register handler does not invoke it)        |
| `send_subscription_confirmation` | —                                   | **Dead** (webhook `handle_checkout_completed` does not invoke it)   |
| `send_subscription_cancelled`    | —                                   | **Dead** (webhook `handle_subscription_deleted` does not invoke it) |

---

## APPENDIX — CROSS-CUTTING OBSERVATIONS

### A. Infrastructure gaps preventing parity

1. **No event bus / outbox pattern.** Every stateful change today is executed inline with the request. For WooCommerce-class reliability we need an outbox (or Redis Streams / PGMQ / RabbitMQ) so email sends, tax calculations, audit writes, and third-party fan-out survive restarts.
2. **No shared cache / rate-limit backend.** `tower_governor` is in-process; any horizontal scale requires Redis or DB-backed quotas. Popup-frequency `once_ever` is browser-localStorage-only — server has no notion.
3. **No geo / IP database.** Required by consent (region detection), popups (UTM+geo targeting), and tax (region-based rates). MaxMind GeoLite2 or Cloudflare `CF-IPCountry` header + fallback.
4. **No UA parser.** Needed for consent log, form audit trail, popup analytics. `woothee` or `uap-core`.
5. **No HTML sanitizer in Rust.** `blog_posts.content` is inserted directly from the admin with no server-side sanitization — acceptable today (admin-only) but unacceptable when (a) user-generated form content lands, or (b) popup `content_json` blocks include `html` elements. `ammonia` is the standard choice.
6. **No PDF generator.** Invoices / packing slips / tax reports need this. Candidates: `printpdf`, `weasyprint` via sidecar, `wkhtmltopdf`.
7. **No MJML / email template compiler.** Tera inline templates don't scale past 4. `mrml` crate is the Rust MJML implementation.
8. **No OpenAPI / codegen.** `src/lib/api/types.ts` is hand-maintained parallel to Rust DTOs — drift is a real risk. Phase 3 must pick one: `utoipa`, `aide`, `poem-openapi`, or a schema-first OpenAPI 3.1 document.

### B. Constraint-violation catalogue (code-level)

| Violation                                                          | Where                                                                                                                                                            | Fix class                                           |
| ------------------------------------------------------------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------- |
| Tokens are hex not OKLCH                                           | `src/styles/tokens.css`                                                                                                                                          | CSS migration                                       |
| No `@layer` cascade                                                | `src/styles/*.css`                                                                                                                                               | CSS refactor                                        |
| Only 2 breakpoints (768, 1024) vs. mandated 9                      | `src/styles/*.css`, `src/styles/global.css:32/40`                                                                                                                | CSS refactor                                        |
| Historical Tailwind comments                                       | `reset.css:3`, `tokens.css:3,59`, `global.css:128`, `layout.css:1`                                                                                               | Cosmetic; also delete empty `src/routes/layout.css` |
| Money as `f64` at API boundary                                     | `CreateCouponRequest.discount_value: f64`, `UpdateCouponRequest.discount_value: Option<f64>` (models.rs), `admin_create_coupon` (coupons.rs:453)                 | Change to integer minor units or decimal string     |
| `.unwrap()` / `.expect()` in prod paths                            | `main.rs:70, 75, 84, 191-198`; `middleware/rate_limit.rs:24`; `models.rs:633`; `handlers/coupons.rs:110,118,776,780` (decimal→f64 via `to_f64().unwrap_or(0.0)`) | Replace with typed errors                           |
| `#[allow(dead_code)]` on revenue structs                           | `models.rs` (SalesEvent, MonthlyRevenueSnapshot, RevenueAnalytics, …)                                                                                            | Wire up or delete                                   |
| No `#![deny(warnings)]` / `#![forbid(unsafe_code)]`                | `backend/src/main.rs`                                                                                                                                            | Lint-gate on crate root                             |
| No CSP header                                                      | `src/hooks.server.ts`                                                                                                                                            | Add per-route CSP                                   |
| `discount_value` uses `NUMERIC(10,2)` in schema but API uses `f64` | `migrations/013` + models.rs                                                                                                                                     | Align types                                         |

### C. Untrusted input paths that expand with new domains

- Popup `content_json` blocks will render on public pages (today, admin-authored; Phase 4 may expose editor to members).
- Form submissions will carry arbitrary strings that feed into email, webhook-out, and storage layers — sanitization + CSP + MIME-sniff protection all needed.
- Consent category signals must be normalized before being echoed to Google Consent Mode + TCF IAB string.
- Webhook-out must sign payloads we emit (HMAC) for downstream partners, mirroring Stripe's pattern.

---

## CHECKPOINT

Phase 2 deliverable complete. **Awaiting your review/approval before starting Phase 3** (dependency-ordered implementation plan: migrations, Rust module tree, OpenAPI contract, admin UI routes, public component inventory, test plan, observability, security review).

Key decisions I will ask for at the start of Phase 3:

1. **Scope:** full WooCommerce parity (products/cart/orders/tax/shipping) — or trim to digital-goods only (subscriptions + courses + downloadable + memberships)?
2. **Type contract:** `utoipa` (annotation-first) vs. `ts-rs` (codegen from Rust) vs. schema-first OpenAPI 3.1 `.yaml`?
3. **Queue / event bus:** Redis Streams, Postgres-based (`pgmq` or custom outbox), or external (NATS/RabbitMQ)?
4. **Email provider:** stay on SMTP (Lettre) for transactional or cut over to Resend/Postmark API?
5. **Geo DB:** self-hosted MaxMind GeoLite2 or Cloudflare `CF-IPCountry` header with MaxMind fallback?
6. **CSS rebuild sequencing:** PE7 token migration in parallel with Phase 4 subsystems, or as Phase 3.5 prerequisite?
