import { loadStripe, type Stripe } from '@stripe/stripe-js';
import { env } from '$env/dynamic/public';

let stripePromise: Promise<Stripe | null>;

export function getStripe() {
	if (!stripePromise) {
		stripePromise = loadStripe(env.PUBLIC_STRIPE_PUBLISHABLE_KEY || '');
	}
	return stripePromise;
}

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
