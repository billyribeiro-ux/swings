/**
 * Hosted Stripe Checkout via SvelteKit `/api/create-checkout-session`.
 * Intentionally does not import `@stripe/stripe-js` so marketing pages avoid loading
 * `js.stripe.com` (and its deprecated `unload` listeners) until real Elements usage exists.
 */
export async function createCheckoutSession(priceId: string) {
	try {
		const response = await fetch('/api/create-checkout-session', {
			method: 'POST',
			headers: {
				'Content-Type': 'application/json'
			},
			body: JSON.stringify({ priceId })
		});

		if (!response.ok) {
			const error = await response.json();
			throw new Error(error.error || 'Failed to create checkout session');
		}

		const { url } = await response.json();

		if (url) {
			window.location.href = url;
		}
	} catch (error) {
		console.error('Checkout error:', error);
		throw error;
	}
}
