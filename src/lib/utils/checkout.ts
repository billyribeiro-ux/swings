/**
 * Hosted Stripe Checkout — thin client wrapper over the `createCheckoutSession`
 * remote command (`$routes/api/checkout.remote.ts`).
 *
 * Pass a **plan slug** (e.g. `monthly`, `annual`); the server loads active plans
 * from the Rust API and builds the Stripe session (reuses `stripe_price_id` from
 * the DB or inline `price_data` when unset — no env `price_` wiring required).
 */
import { createCheckoutSession as createCheckoutSessionRemote } from '../../routes/api/checkout.remote';

export async function createCheckoutSession(planSlugOrPriceId: string): Promise<void> {
	try {
		const payload = planSlugOrPriceId.startsWith('price_')
			? { priceId: planSlugOrPriceId }
			: { planSlug: planSlugOrPriceId };
		const { url } = await createCheckoutSessionRemote(payload);
		if (url) {
			window.location.href = url;
		}
	} catch (error) {
		console.error('Checkout error:', error);
		throw error;
	}
}
