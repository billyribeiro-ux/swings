/**
 * `checkout.ts` — Vitest unit coverage.
 *
 * Scope:
 *   - planSlug input → routes through `createCheckoutSession({ planSlug })`.
 *   - `price_*` input → routes through `{ priceId }` shape.
 *   - When the remote returns `{ url }`, the helper sets `window.location.href`.
 *   - When the remote returns `{ url: null }`, the helper does NOT navigate.
 *   - When the remote throws, the helper rethrows for caller-side handling.
 *
 * @vitest-environment jsdom
 */
import { describe, expect, it, vi, beforeEach, afterEach } from 'vitest';

const remoteSpy = vi.fn();
vi.mock('../../routes/api/checkout.remote', () => ({
	createCheckoutSession: (...args: unknown[]) => remoteSpy(...args)
}));

import { createCheckoutSession } from './checkout';

let hrefSetter: (v: string) => void;

beforeEach(() => {
	remoteSpy.mockReset();
	hrefSetter = vi.fn();
	// Replace window.location with a stub that captures `href` writes.
	// jsdom marks `window.location` as non-configurable in newer versions —
	// `delete` clears the protected setter so we can install our own.
	// eslint-disable-next-line @typescript-eslint/no-explicit-any
	delete (window as any).location;
	Object.defineProperty(window, 'location', {
		configurable: true,
		value: {
			assign: vi.fn(),
			set href(v: string) {
				hrefSetter(v);
			},
			get href() {
				return 'http://localhost/';
			}
		}
	});
});

afterEach(() => {
	vi.restoreAllMocks();
});

describe('createCheckoutSession', () => {
	it('routes plan slugs to { planSlug }', async () => {
		remoteSpy.mockResolvedValueOnce({ sessionId: 'cs_1', url: 'https://stripe.test/x' });
		await createCheckoutSession('monthly');
		expect(remoteSpy).toHaveBeenCalledWith({ planSlug: 'monthly' });
	});

	it('routes Stripe price IDs to { priceId }', async () => {
		remoteSpy.mockResolvedValueOnce({ sessionId: 'cs_2', url: 'https://stripe.test/y' });
		await createCheckoutSession('price_abc123');
		expect(remoteSpy).toHaveBeenCalledWith({ priceId: 'price_abc123' });
	});

	it('navigates the browser to the returned Checkout URL', async () => {
		remoteSpy.mockResolvedValueOnce({ sessionId: 'cs_3', url: 'https://stripe.test/go' });
		await createCheckoutSession('annual');
		expect(hrefSetter).toHaveBeenCalledWith('https://stripe.test/go');
	});

	it('does not navigate when the URL is null', async () => {
		remoteSpy.mockResolvedValueOnce({ sessionId: 'cs_4', url: null });
		await createCheckoutSession('monthly');
		expect(hrefSetter).not.toHaveBeenCalled();
	});

	it('rethrows when the remote command rejects', async () => {
		const boom = new Error('stripe down');
		remoteSpy.mockRejectedValueOnce(boom);
		await expect(createCheckoutSession('monthly')).rejects.toBe(boom);
	});
});
