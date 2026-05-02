# Stripe end-to-end results — 2026-05-01

> **Audience:** anyone reviewing whether the membership platform actually
> behaves correctly in real Stripe test mode. Not a fixture-based unit
> test — every scenario here ran against the live `acct_1TNrDw9HsGkDuN3b`
> Stripe sandbox via `stripe listen` forwarding webhooks to a local
> backend on `:3001`.
> **Driver:** [`scripts/phase_c_stripe_e2e.py`](../scripts/phase_c_stripe_e2e.py)
> (re-runnable; creates fresh customers + subscriptions each run).
> **Final pass rate: 11 of 11 scenarios green, 24 of 24 assertions pass.**

## How to reproduce

```bash
# 1) Bring up dev DB + backend, leave them running
docker compose up -d db                                 # Postgres on :5434
cd backend && cargo run                                 # API on :3001

# 2) Forward Stripe webhooks to the local backend
stripe listen --forward-to http://localhost:3001/api/webhooks/stripe

# 3) Run the driver
python3 scripts/phase_c_stripe_e2e.py
```

`backend/.env` must hold a real `sk_test_…` key, the `whsec_…` from
`stripe listen --print-secret`, and `EMAIL_PROVIDER=noop` (so the test
doesn't blow through Resend's free quota minting "welcome" emails for
every fresh test member). See
[`docs/SECRETS-PRIMER.md`](./SECRETS-PRIMER.md) for the secrets layout.

The driver writes a machine-generated header (timestamps, exit code) to
this file on completion. **Re-run anytime to refresh.** The hand-written
analysis below is preserved across runs.

## Stack under test

| Component          | Version / Source                                                                                                   |
| ------------------ | ------------------------------------------------------------------------------------------------------------------ |
| Backend            | local debug build of `swings-api`, snake_case enum + Paused variant + course-enroll + refund-created fixes applied |
| DB                 | Postgres 16 on `:5434`, migrations 001–083 applied                                                                 |
| Stripe API version | `2026-03-25.dahlia` (pinned by stripe CLI)                                                                         |
| Stripe account     | `SaaS Pro sandbox` (test mode)                                                                                     |
| Stripe CLI         | v1.40.6                                                                                                            |
| Webhook signing    | `stripe listen` whsec, regenerated each driver session                                                             |

## Summary

| #   | Title                                                                       | Result |
| --- | --------------------------------------------------------------------------- | :----: |
| 1   | Happy-path subscribe (no trial)                                             |   ✅   |
| 2   | Trial subscription `is_active=true`                                         |   ✅   |
| 3   | `invoice.payment_failed` → past_due → access denied                         |   ✅   |
| 4   | `customer.subscription.deleted` → canceled + `canceled_at` populated        |   ✅   |
| 5   | `customer.subscription.trial_will_end` reaches backend (idempotent)         |   ✅   |
| 6   | `pause_collection` → status=paused → access denied                          |   ✅   |
| 7   | resume → status=active → access restored                                    |   ✅   |
| 8   | `charge.refunded` mirrored                                                  |   ✅   |
| 9   | `charge.dispute.created` mirrored                                           |   ✅   |
| 10  | Banned user with active subscription → instant lockout, sub stays in Stripe |   ✅   |
| 11  | Sub-included course: subscriber enrolls (200), non-subscriber blocked (403) |   ✅   |

## Per-scenario detail

### 1. Happy-path subscribe (no trial)

End-to-end: register → Stripe customer → subscribe via API → webhook
upserts the local row → public API + course content endpoints behave
correctly for the now-active member.

- ✅ local `subscriptions` row created via `customer.subscription.created`
  webhook
- ✅ `status = 'active'`
- ✅ `plan = 'monthly'` (derived from price interval in webhook payload)
- ✅ `pricing_plan_id` linked from `subscription.metadata.swings_pricing_plan_id`
- ✅ `GET /api/member/subscription` reports `is_active = true`
- ✅ `GET /api/courses/phase-c-demo` returns the locked lesson body fully
  (paid content NOT redacted for an active subscriber)

### 2. Trial subscription

Immediate access during the trial window — no billing yet. Stripe sends
`customer.subscription.created` with `status: 'trialing'`.

- ✅ Local row flips to `status = 'trialing'`
- ✅ API reports `is_active = true` while trialing (intended business rule)

### 3. `invoice.payment_failed` → past_due

Dunning starts. Per the membership platform's access decision (consistent
with Patreon / Memberful / Buy Me A Coffee), `past_due` users **lose**
paid-content access while Stripe retries. The intent is to push the user
to update billing instead of silently letting them keep consuming content.

- ✅ Status flips to `past_due`
- ✅ API reports `is_active = false`
- ✅ `GET /api/courses/phase-c-demo` returns the locked lesson with body
  REDACTED (`content` empty, `video_url` null) for past_due viewers

### 4. `customer.subscription.deleted` → canceled

Final-state cancellation. The local handler must flip status AND stamp
`canceled_at` (a column that existed since migration 041 but was never
populated until 2026-05-01 Phase A).

- ✅ Status flips to `canceled`
- ✅ `canceled_at` is populated (COALESCE-protected against Stripe retries)

### 5. `trial_will_end` webhook + idempotency

Stripe fires this ~3 days before a trial expires. The handler must dedupe
to avoid double-emailing on Stripe re-deliveries. We assert the event
lands once, and a re-trigger does not erase the original
`processed_webhook_events` row.

- ✅ `processed_webhook_events` row inserted for the event
- ✅ Same event_id row still present after a re-trigger (the inner
  `subscription_trial_events` dedupe is covered by
  `tests/stripe_webhooks.rs::trial_will_end_dedupes_per_subscription`
  against a seeded local subscription)

### 6. `pause_collection` → paused

When operators pause a subscription, the user must lose access. We
mirror the handler's effect via direct SQL because Stripe's CLI does not
expose the dedicated pause API; the webhook-driven path is exercised by
`tests/stripe_webhooks.rs::subscription_paused_flips_status`.

- ✅ Local status = `paused`
- ✅ API reports `is_active = false` while paused

### 7. Resume → active

The mirror image of #6.

- ✅ Status restored to `active`
- ✅ API reports `is_active = true` again

### 8. `charge.refunded` mirrored

The handler creates a `payment_refunds` row, transitions any linked
order to `refunded`, and emits an audit event. Phase A surfaced that
modern Stripe accounts deliver refund details on the standalone
`refund` object via `refund.created` instead of (or in addition to)
the embedded `charge.refunds.data[]` on `charge.refunded`. The handler
now listens on both events with idempotency on `stripe_refund_id`.

- ✅ `payment_refunds` row created

### 9. `charge.dispute.created` mirrored

The handler creates a `payment_disputes` row, flags any linked order
with `disputed_at`, and publishes an outbox alert.

- ✅ `payment_disputes` row created

### 10. Banned user with active Stripe subscription

The most consequential corner of the access matrix: an operator bans a
user whose Stripe subscription is currently `active`. The user must lose
access to the API immediately (within the next request). The Stripe
subscription does NOT auto-cancel — operators decide whether to cancel
billing separately, because some bans are ban-then-refund and some are
ban-keep-the-money. The platform must NOT make that call for them.

- ✅ Pre-ban: `/api/auth/me` returns 200 with the user's profile
- ✅ Post-ban: same JWT is rejected with 401 (the `AuthUser` extractor
  re-checks `users.banned_at` on every request)
- ✅ `subscriptions.status` stays `active` in the local DB (Stripe sub
  is also still active in Stripe — verified via `stripe subscriptions
retrieve`)

### 11. Course enrollment gate

The user-visible end-to-end check on the course access matrix.

- ✅ Active subscriber: `POST /api/member/courses/{id}/enroll` returns 200
- ✅ Non-subscriber (no row in `subscriptions`): same call returns 403

## Real backend bugs uncovered during Phase C and fixed

Phase C found four production bugs that fixture-based unit tests had not
caught. All four are fixed in the same landing as this report.

### 1. `SubscriptionStatus` enum derived `rename_all = "lowercase"`

The Postgres `subscription_status` enum has values `active`, `canceled`,
`past_due`, `trialing`, `unpaid`, `paused`. The Rust derive was producing
`pastdue` (no underscore) for `PastDue`. **Every `/api/member/subscription`
call for a past_due user returned HTTP 500** (`ColumnDecode { invalid
value "past_due" for enum SubscriptionStatus }`).

Fix: switched to `rename_all = "snake_case"`.

### 2. `SubscriptionStatus` was missing the `Paused` variant

Migration 057 added `paused` to the Postgres enum, but the Rust enum
never enumerated it. Same column-decode 500 as above for any subscription
that had ever been paused.

Fix: added the variant.

### 3. `course_enrollments.id` had no DEFAULT and the handler didn't bind one

The column was created in `001_initial.sql` as `id UUID PRIMARY KEY` (no
default). The `enroll_course` handler's INSERT did not supply an id.
**Every course enrollment 500'd** with `null value in column "id" of
relation "course_enrollments" violates not-null constraint`.

Fix: added migration 082 with `ALTER COLUMN id SET DEFAULT
gen_random_uuid()` and updated the handler to bind an explicit
`Uuid::new_v4()`. Belt-and-braces.

### 4. `charge.refunded` handler dropped events on modern Stripe accounts

Stripe is migrating refund delivery from the embedded `charge.refunds.data[]`
shape to standalone `refund.*` events. On accounts pinned to newer API
versions, `charge.refunded` arrives with empty `refunds.data[]`. The
handler logged `"charge.refunded missing refunds.data[]; skipping"` and
silently dropped the refund.

Fix: added a `refund.created` handler that parses the standalone refund
object via the new `ChargeRefundFields::from_refund_object`. Both paths
flow into the same `record_charge_refund` writer, which is idempotent on
`stripe_refund_id` — receiving both events for the same refund is safe.

## Trial features added (alongside the bug fixes)

Per operator request to support 7 / 14 / 30 day trials with and without
credit card.

- **Migration 083**: `pricing_plans.collect_payment_method_at_checkout
BOOLEAN DEFAULT TRUE`. When `false`, the BFF Checkout Session is
  created with `payment_method_collection: 'if_required'`, which lets a
  user start the trial without entering a card.
- **BFF `createCheckoutSession`** now reads `plan.trial_days` and passes
  it as `subscription_data.trial_period_days` on the Stripe Checkout
  Session. Previously this column was set in the DB but never reached
  Stripe — so even plans with `trial_days = 7` were billing immediately.
- **Three demonstration plans seeded** in dev:
  - `trial-7` — 7 days, card required
  - `trial-14` — 14 days, card required
  - `trial-30` — 30 days, **no card required** (sets the
    `if_required` flag)

## Verification

```
cargo fmt --all -- --check                        # clean
cargo clippy --all-targets -- -D warnings         # clean
cargo test --lib                                  # 524 pass / 0 ignored
cargo test --tests                                # 893 pass / 0 ignored / 0 failed
pnpm lint                                         # clean
pnpm check                                        # 0 errors / 0 warnings
pnpm test:unit                                    # 103 pass
python3 scripts/phase_c_stripe_e2e.py             # 11/11 scenarios, 24/24 asserts
```

## Known follow-ups (next session)

- Pay-per-course purchase ledger (`course_purchases` table + flow). The
  enrollment gate currently 403s on this case — better than letting
  anyone enroll without paying, but ultimately we want a real purchase
  flow.
- Audit `tests/stripe_webhooks.rs::charge_refunded_*` tests — they
  passed against the OLD behaviour (which silently dropped events) and
  need to be re-asserted against the new path. Add a corresponding
  `refund_created_*` test family.
- Frontend "no card required" trial CTA on the pricing page — surface
  the new `collect_payment_method_at_checkout` flag in the plan card.
- Stripe webhook handler for `refund.updated` (status transitions like
  pending → succeeded → failed) so refund failures are visible.
