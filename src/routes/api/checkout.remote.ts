/**
 * SvelteKit Remote Functions — Stripe Checkout.
 *
 * Plan resolution is **server-side** (never trust client amounts):
 * fetches public `/api/pricing/plans` (Postgres) and either uses the stored
 * `stripe_price_id` or, if unset, inline `line_items.price_data` per
 * [Create a Checkout Session](https://docs.stripe.com/api/checkout/sessions/create)
 * (2026-04-22.dahlia) — so admin/env never needs hardcoded `price_` IDs for
 * the marketing site: amounts and intervals are whatever the API returns.
 */
import { command } from '$app/server';
import { error } from '@sveltejs/kit';
import Stripe from 'stripe';
import { env } from '$env/dynamic/private';
import { env as publicEnv } from '$env/dynamic/public';
import type { PricingPlan } from '$lib/api/types';

/** Hoisted Stripe client — reused across requests so we don't pay TLS/handshake costs per call. */
let stripeClient: Stripe | null = null;
function getStripe(): Stripe {
	if (stripeClient) return stripeClient;
	if (!env.STRIPE_SECRET_KEY) {
		error(500, 'Stripe is not configured');
	}
	stripeClient = new Stripe(env.STRIPE_SECRET_KEY, { apiVersion: '2026-04-22.dahlia' });
	return stripeClient;
}

/**
 * Resolve the public origin for Stripe success/cancel callbacks.
 *
 * In production we refuse to fall back: if `PUBLIC_APP_URL` is unset on a
 * Vercel/Netlify deploy the checkout would silently route customers back
 * to `http://localhost:5173`, which 100% breaks the post-payment flow on
 * every device that isn't the operator's own dev box. Failing the
 * checkout call early surfaces the misconfig as a 500 the operator can
 * see, not as silent customer loss.
 *
 * In dev we keep the convenience fallback so `vite dev` works without
 * extra env wiring.
 */
function siteOrigin(): string {
	const configured = publicEnv.PUBLIC_APP_URL?.trim();
	if (configured) return configured.replace(/\/$/, '');
	if (env.NODE_ENV === 'production' || env.VERCEL_ENV === 'production') {
		error(
			500,
			'Checkout is not configured: PUBLIC_APP_URL must be set in production'
		);
	}
	return 'http://localhost:5173';
}

/** Load active plans the same way the browser does, via same-origin rewrites to the Rust API. */
async function fetchActivePlans(): Promise<PricingPlan[]> {
	const res = await fetch(new URL('/api/pricing/plans', siteOrigin()).href, {
		headers: { accept: 'application/json' }
	});
	if (!res.ok) {
		console.error(`[stripe-checkout] pricing plans HTTP ${res.status}`);
		error(502, 'Pricing is temporarily unavailable');
	}
	return res.json() as Promise<PricingPlan[]>;
}

function lineItemsForPlan(plan: PricingPlan): Stripe.Checkout.SessionCreateParams['line_items'] {
	if (plan.stripe_price_id) {
		return [{ price: plan.stripe_price_id, quantity: 1 }];
	}

	// No Price object in DB yet: Stripe allows inline `price_data` (creates a Price per checkout for reporting).
	// @see line_items — one of `price` or `price_data` (subscription + recurring)
	const interval: Stripe.Price.Recurring.Interval = plan.interval === 'year' ? 'year' : 'month';
	if (plan.interval === 'one_time') {
		error(400, 'This plan is not a subscription; use a one-time payment flow');
	}
	return [
		{
			quantity: 1,
			price_data: {
				currency: plan.currency,
				unit_amount: plan.amount_cents,
				product_data: { name: plan.name },
				recurring: { interval, interval_count: plan.interval_count || 1 }
			}
		}
	];
}

/**
 * SECURITY: input schema for the checkout command.
 *
 * SvelteKit remote `command(schema, handler)` accepts any
 * [Standard Schema](https://standardschema.dev) v1 validator, so we can
 * declare this inline without pulling in zod / valibot just for two
 * fields. The validator enforces:
 *
 *   - body is a plain object (rejects arrays, nulls, primitives)
 *   - `planSlug`, when present, is a non-empty, lowercase-slug string
 *     (`[a-z0-9-]{1,64}`). Rejects path traversal / SQL wildcards.
 *   - `priceId`, when present, is a Stripe price id (`price_...` with
 *     only Stripe's published alphabet).
 *   - at least one of `planSlug` / `priceId` is present.
 *
 * Dropping `'unchecked'` means the handler body no longer has to reassert
 * types; the validator is the contract.
 */
interface CheckoutPayload {
	planSlug?: string | undefined;
	priceId?: string | undefined;
}

const SLUG_RE = /^[a-z0-9](?:[a-z0-9-]{0,62}[a-z0-9])?$/;
const PRICE_ID_RE = /^price_[A-Za-z0-9]{1,64}$/;

const checkoutSchema = {
	'~standard': {
		version: 1 as const,
		vendor: 'swings',
		validate(
			value: unknown
		):
			| { value: CheckoutPayload }
			| { issues: { message: string; path?: (string | number)[] }[] } {
			const issues: { message: string; path?: (string | number)[] }[] = [];
			if (!value || typeof value !== 'object' || Array.isArray(value)) {
				return { issues: [{ message: 'payload must be a plain object' }] };
			}
			const body = value as Record<string, unknown>;

			let planSlug: string | undefined;
			if (body.planSlug !== undefined) {
				if (typeof body.planSlug !== 'string' || !SLUG_RE.test(body.planSlug)) {
					issues.push({
						message: 'planSlug must be a lowercase slug',
						path: ['planSlug']
					});
				} else {
					planSlug = body.planSlug;
				}
			}

			let priceId: string | undefined;
			if (body.priceId !== undefined) {
				if (typeof body.priceId !== 'string' || !PRICE_ID_RE.test(body.priceId)) {
					issues.push({
						message: "priceId must look like 'price_XXXX'",
						path: ['priceId']
					});
				} else {
					priceId = body.priceId;
				}
			}

			if (!planSlug && !priceId) {
				issues.push({ message: 'provide planSlug or priceId' });
			}

			return issues.length ? { issues } : { value: { planSlug, priceId } };
		}
	}
} as const;

/**
 * Create a hosted Stripe Checkout Session for a subscription.
 *
 * Pass `{ planSlug: "monthly" }` (preferred) or `{ priceId: "price_..." }` (escape hatch / tests).
 *
 * The `checkoutSchema` validator runs before the handler body; the handler
 * only ever sees a shape that has already been shape-checked. We still
 * receive `unknown` at the type level because SvelteKit's inference over
 * inline Standard-Schema objects is not narrow; the runtime cast inside
 * is safe because the validator already rejected everything else.
 */
export const createCheckoutSession = command(
	checkoutSchema,
	async (payload: unknown): Promise<{ sessionId: string; url: string | null }> => {
		const { planSlug, priceId } = payload as CheckoutPayload;

		let line_items: Stripe.Checkout.SessionCreateParams['line_items'];
		let swingsPricingPlanId: string | undefined;
		let trialPeriodDays: number | undefined;
		let collectPaymentMethod = true; // default — collect a card up front

		if (priceId) {
			line_items = [{ price: priceId, quantity: 1 }];
		} else if (planSlug) {
			const plans = await fetchActivePlans();
			const plan = plans.find((p) => p.is_active && p.slug === planSlug) ?? null;
			if (!plan) {
				error(400, 'Unknown or inactive plan');
			}
			line_items = lineItemsForPlan(plan);
			swingsPricingPlanId = plan.id;
			// Honour the catalog's trial offer. `trial_days = 0` (the default)
			// produces a regular paid subscription with no trial.
			if (plan.trial_days && plan.trial_days > 0) {
				trialPeriodDays = plan.trial_days;
			}
			// Honour the "trial without credit card" toggle. When the catalog
			// row has `collect_payment_method_at_checkout = false`, Stripe
			// Checkout skips the card form (`payment_method_collection =
			// if_required`) — typically paired with a non-zero `trial_days`
			// so the member starts the trial without paying.
			if (plan.collect_payment_method_at_checkout === false) {
				collectPaymentMethod = false;
			}
		} else {
			error(400, 'Pass planSlug (e.g. monthly) or priceId (price_...)');
		}

		if (!line_items?.length) {
			error(500, 'No line items for checkout');
		}

		const stripe = getStripe();
		const appUrl = siteOrigin();

		try {
			const session = await stripe.checkout.sessions.create({
				mode: 'subscription',
				payment_method_types: ['card'],
				line_items,
				// `subscription_data` carries both the catalog-link metadata
				// (used by `customer.subscription.*` webhooks to populate
				// `subscriptions.pricing_plan_id`) and the trial window when
				// the plan offers one.
				subscription_data: {
					...(swingsPricingPlanId
						? { metadata: { swings_pricing_plan_id: swingsPricingPlanId } }
						: {}),
					...(trialPeriodDays !== undefined ? { trial_period_days: trialPeriodDays } : {})
				},
				// `if_required` lets the member skip the card form during the
				// hosted Checkout when no payment is due immediately (i.e.
				// they're starting a free trial). Stripe will email them
				// when the trial is about to end and ask for a card; if they
				// don't add one the subscription auto-cancels at trial end.
				payment_method_collection: collectPaymentMethod ? 'always' : 'if_required',
				success_url: `${appUrl}/success?session_id={CHECKOUT_SESSION_ID}`,
				cancel_url: `${appUrl}/pricing?canceled=true`,
				allow_promotion_codes: true,
				billing_address_collection: 'required'
			});
			return { sessionId: session.id, url: session.url };
		} catch (err) {
			const errorId = crypto.randomUUID();
			console.error(`[stripe-checkout ${errorId}]`, err);
			error(500, 'Failed to create checkout session');
		}
	}
);
