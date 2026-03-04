import { json } from '@sveltejs/kit';
import type { RequestHandler } from './$types';
import Stripe from 'stripe';
import { env } from '$env/dynamic/private';
import { env as publicEnv } from '$env/dynamic/public';

export const POST: RequestHandler = async ({ request }) => {
	try {
		const { priceId } = await request.json();

		if (!priceId) {
			return json({ error: 'Price ID is required' }, { status: 400 });
		}

		if (!env.STRIPE_SECRET_KEY) {
			return json({ error: 'Stripe is not configured' }, { status: 500 });
		}

		const stripe = new Stripe(env.STRIPE_SECRET_KEY, {
			apiVersion: '2026-02-25.clover'
		});

		const session = await stripe.checkout.sessions.create({
			mode: 'subscription',
			payment_method_types: ['card'],
			line_items: [
				{
					price: priceId,
					quantity: 1
				}
			],
			success_url: `${publicEnv.PUBLIC_APP_URL || 'http://localhost:5173'}/success?session_id={CHECKOUT_SESSION_ID}`,
			cancel_url: `${publicEnv.PUBLIC_APP_URL || 'http://localhost:5173'}/pricing?canceled=true`,
			allow_promotion_codes: true,
			billing_address_collection: 'required'
		});

		return json({ sessionId: session.id, url: session.url });
	} catch (error) {
		console.error('Stripe checkout error:', error);
		return json(
			{ error: error instanceof Error ? error.message : 'Failed to create checkout session' },
			{ status: 500 }
		);
	}
};
