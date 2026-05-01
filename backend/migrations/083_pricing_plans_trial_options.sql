-- Trial without credit card: opt-in flag on pricing_plans.
--
-- Stripe Checkout Sessions in `mode: subscription` collect a payment
-- method by default. To run a "trial without CC" we have to set
-- `payment_method_collection: 'if_required'` on the Checkout Session
-- — Stripe then SKIPS the card form and lets the user start the trial
-- with no payment method on file. Stripe will (correctly) refuse to
-- bill the subscription when the trial converts unless a payment method
-- is added, so the member experience is "trial → at trial-end Stripe
-- emails them to add a card; if they don't, the subscription
-- auto-cancels."
--
-- The flag lives on `pricing_plans` so each plan picks the trial mode
-- independently. Defaults to TRUE (matches existing behaviour:
-- collect a card up-front, charge after the trial). Set FALSE on the
-- "no-CC trial" plans to enable the if_required path.
ALTER TABLE pricing_plans
    ADD COLUMN IF NOT EXISTS collect_payment_method_at_checkout
        BOOLEAN NOT NULL DEFAULT TRUE;

COMMENT ON COLUMN pricing_plans.collect_payment_method_at_checkout IS
    'When TRUE (default), Stripe Checkout requires a payment method up-front. '
    'When FALSE, the BFF passes payment_method_collection=if_required so the '
    'user can start a trial without entering a card.';
