/**
 * Hosted Stripe Checkout — thin client wrapper over the `createCheckoutSession`
 * remote command (`$routes/api/checkout.remote.ts`).
 *
 * Intentionally does not import `@stripe/stripe-js` so marketing pages avoid
 * loading `js.stripe.com` (and its deprecated `unload` listeners) until real
 * Elements usage exists. The remote call returns the Session URL and this
 * helper navigates the browser to it.
 */
import { createCheckoutSession as createCheckoutSessionRemote } from '../../routes/api/checkout.remote';
import { getPricingPlanBySlug } from '$lib/api/publicPricing';

async function resolveStripePriceId(identifier: string): Promise<string> {
	if (identifier.startsWith('price_')) return identifier;
	const plan = await getPricingPlanBySlug(identifier);
	if (!plan?.stripe_price_id) {
		throw new Error(`Stripe price is not configured for plan "${identifier}"`);
	}
	return plan.stripe_price_id;
}

export async function createCheckoutSession(identifier: string): Promise<void> {
	try {
		const priceId = await resolveStripePriceId(identifier);
		const { url } = await createCheckoutSessionRemote({ priceId });
		if (url) {
			window.location.href = url;
		}
	} catch (error) {
		console.error('Checkout error:', error);
		throw error;
	}
}
