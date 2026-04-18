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

export async function createCheckoutSession(priceId: string): Promise<void> {
	try {
		const { url } = await createCheckoutSessionRemote({ priceId });
		if (url) {
			window.location.href = url;
		}
	} catch (error) {
		console.error('Checkout error:', error);
		throw error;
	}
}
