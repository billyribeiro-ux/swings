/**
 * SvelteKit Remote Functions — Stripe Checkout.
 *
 * Experimental (SvelteKit 2.27+); opt-in via `kit.experimental.remoteFunctions`
 * in `svelte.config.js`. Replaces the `+server.ts` POST handler that previously
 * served the same endpoint; consumers import `createCheckoutSession` directly
 * and call it like a regular async function. On the wire, SvelteKit routes the
 * call through a generated POST endpoint with devalue (de)serialization and
 * client↔server type parity.
 *
 * @see https://svelte.dev/docs/kit/remote-functions
 */
import { command } from '$app/server';
import { error } from '@sveltejs/kit';
import Stripe from 'stripe';
import { env } from '$env/dynamic/private';
import { env as publicEnv } from '$env/dynamic/public';

/** Hoisted Stripe client — reused across requests so we don't pay TLS/handshake costs per call. */
let stripeClient: Stripe | null = null;
function getStripe(): Stripe {
	if (stripeClient) return stripeClient;
	if (!env.STRIPE_SECRET_KEY) {
		error(500, 'Stripe is not configured');
	}
	stripeClient = new Stripe(env.STRIPE_SECRET_KEY, { apiVersion: '2026-03-25.dahlia' });
	return stripeClient;
}

/**
 * Create a hosted Stripe Checkout Session for a subscription price.
 *
 * Uses `'unchecked'` so we bypass the Standard Schema requirement (no validation
 * library installed); we validate `priceId` inline instead. When a schema lib
 * like Valibot is added, switch this to `v.pipe(v.string(), v.startsWith('price_'))`.
 */
export const createCheckoutSession = command(
	'unchecked',
	async (input: unknown): Promise<{ sessionId: string; url: string | null }> => {
		if (!input || typeof input !== 'object' || !('priceId' in input)) {
			error(400, 'priceId is required');
		}
		const priceId = (input as { priceId: unknown }).priceId;
		if (typeof priceId !== 'string' || !priceId.startsWith('price_')) {
			error(400, 'priceId must be a Stripe price identifier');
		}

		const stripe = getStripe();
		const appUrl = publicEnv.PUBLIC_APP_URL || 'http://localhost:5173';

		try {
			const session = await stripe.checkout.sessions.create({
				mode: 'subscription',
				payment_method_types: ['card'],
				line_items: [{ price: priceId, quantity: 1 }],
				success_url: `${appUrl}/success?session_id={CHECKOUT_SESSION_ID}`,
				cancel_url: `${appUrl}/pricing?canceled=true`,
				allow_promotion_codes: true,
				billing_address_collection: 'required'
			});
			return { sessionId: session.id, url: session.url };
		} catch (err) {
			// Log full details server-side, return a generic error to the client.
			const errorId = crypto.randomUUID();
			console.error(`[stripe-checkout ${errorId}]`, err);
			error(500, 'Failed to create checkout session');
		}
	}
);
