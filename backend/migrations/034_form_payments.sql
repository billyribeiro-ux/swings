-- FORM-08: Stripe-backed payment intents raised from form submissions.
--
-- A `form_payment_intents` row is inserted when the public
-- `POST /api/forms/{slug}/payment-intent` endpoint mints a Stripe
-- PaymentIntent / Subscription. The row pins a draft (`partial_id`) or
-- finalised submission (`submission_id`) to the `stripe_payment_intent_id`;
-- the `payment_intent.succeeded` webhook updates `status` and backfills
-- `submission_id` when the submission is committed.
--
--   * `kind = 'one_time'`    — standard PaymentIntent.
--   * `kind = 'donation'`    — same shape, but the amount may come from
--                              the donor (allowed range driven by the
--                              field's `suggested_amounts` + `allow_custom`
--                              schema config).
--   * `kind = 'subscription'`— PaymentIntent created via
--                              `Stripe Customer + Subscription`; we record
--                              the canonical `pi_*` id from the first
--                              invoice so webhook reconciliation is uniform.
--
-- `idempotency_key` mirrors the `Idempotency-Key` header the client
-- supplied; UNIQUE guarantees replayed requests reuse the first row + its
-- stored client_secret without round-tripping Stripe a second time.

CREATE TABLE IF NOT EXISTS form_payment_intents (
    id                       UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    form_id                  UUID NOT NULL REFERENCES forms(id) ON DELETE CASCADE,
    submission_id            UUID NULL REFERENCES form_submissions(id) ON DELETE SET NULL,
    partial_id               UUID NULL REFERENCES form_partials(id) ON DELETE SET NULL,
    field_key                TEXT NOT NULL,
    stripe_payment_intent_id TEXT NOT NULL UNIQUE,
    stripe_client_secret     TEXT NOT NULL,
    stripe_customer_id       TEXT,
    stripe_subscription_id   TEXT,
    amount_cents             BIGINT NOT NULL CHECK (amount_cents >= 0),
    currency                 TEXT NOT NULL,
    kind                     TEXT NOT NULL CHECK (kind IN ('one_time','donation','subscription')),
    status                   TEXT NOT NULL,
    idempotency_key          TEXT NOT NULL UNIQUE,
    created_at               TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at               TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS form_payment_intents_form_id_idx
    ON form_payment_intents (form_id);

CREATE INDEX IF NOT EXISTS form_payment_intents_submission_id_idx
    ON form_payment_intents (submission_id) WHERE submission_id IS NOT NULL;

CREATE INDEX IF NOT EXISTS form_payment_intents_partial_id_idx
    ON form_payment_intents (partial_id) WHERE partial_id IS NOT NULL;
