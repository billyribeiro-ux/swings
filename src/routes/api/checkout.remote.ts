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

function siteOrigin(): string {
	return (publicEnv.PUBLIC_APP_URL || 'http://localhost:5173').replace(/\/$/, '');
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

function lineItemsForPlan(
	plan: PricingPlan
): Stripe.Checkout.SessionCreateParams['line_items'] {
	if (plan.stripe_price_id) {
		return [{ price: plan.stripe_price_id, quantity: 1 }];
	}

	// No Price object in DB yet: Stripe allows inline `price_data` (creates a Price per checkout for reporting).
	// @see line_items — one of `price` or `price_data` (subscription + recurring)
	const interval: Stripe.Price.Recurring.Interval =
		plan.interval === 'year' ? 'year' : 'month';
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
 * Create a hosted Stripe Checkout Session for a subscription.
 *
 * Pass `{ planSlug: "monthly" }` (preferred) or `{ priceId: "price_..." }` (escape hatch / tests).
 */
export const createCheckoutSession = command(
	'unchecked',
	async (input: unknown): Promise<{ sessionId: string; url: string | null }> => {
		if (!input || typeof input !== 'object') {
			error(400, 'Invalid payload');
		}
		const body = input as Record<string, unknown>;
		const planSlug = typeof body.planSlug === 'string' ? body.planSlug : undefined;
		const directPrice = typeof body.priceId === 'string' ? body.priceId : undefined;

		let line_items: Stripe.Checkout.SessionCreateParams['line_items'];

		if (directPrice?.startsWith('price_')) {
			line_items = [{ price: directPrice, quantity: 1 }];
		} else if (planSlug) {
			const plans = await fetchActivePlans();
			const plan = plans.find((p) => p.is_active && p.slug === planSlug) ?? null;
			if (!plan) {
				error(400, 'Unknown or inactive plan');
			}
			line_items = lineItemsForPlan(plan);
		} else {
			error(400, 'Pass planSlug (e.g. monthly) or priceId (price_...)');
		}

		if (!line_items?.length) {
			error(500, 'No line items for checkout');
		}

		const stripe = getStripe();
		const appUrl = publicEnv.PUBLIC_APP_URL || 'http://localhost:5173';

		try {
			const session = await stripe.checkout.sessions.create({
				mode: 'subscription',
				payment_method_types: ['card'],
				line_items,
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
