/**
 * Pricing page smoke coverage.
 *
 * Pricing lives at `/pricing`, with pinned billing-cycle variants at
 * `/pricing/monthly` and `/pricing/annual`. All three are prerendered but the
 * plan cards rely on `/api/pricing/plans` — when the backend isn't reachable
 * we still want the page shell + CTAs to render, so the test only asserts on
 * statically-rendered chrome.
 *
 * Each route has its own h1 copy (compare `/pricing`'s "Choose the Plan…" vs
 * `/pricing/monthly`'s "Weekly Watchlists…"), so the assertion pairs the
 * route with an expected regex instead of using a single shared pattern.
 */

import { test, expect } from '../fixtures/app';

const ROUTES: ReadonlyArray<{ path: string; heading: RegExp }> = [
	{ path: '/pricing', heading: /choose the plan|trading workflow/i },
	{ path: '/pricing/monthly', heading: /month-to-month|weekly watchlists/i },
	{ path: '/pricing/annual', heading: /annual plan|save\s+\d/i }
];

for (const { path, heading } of ROUTES) {
	test(`renders ${path}`, async ({ page }) => {
		const response = await page.goto(path);
		expect(response?.status(), `HTTP status for ${path}`).toBeLessThan(400);
		await expect(page.getByRole('heading', { level: 1, name: heading })).toBeVisible();
	});
}

test('pricing CTAs trigger checkout flow', async ({ page }) => {
	await page.goto('/pricing');
	// The CTA is now a `<button>` (see `Pricing.svelte`) that calls
	// `createCheckoutSession(planSlug)` which proxies to the SvelteKit
	// remote command in `routes/api/checkout.remote.ts`. The remote either
	// (a) returns `{ url }` and the SPA does `window.location.href = url`
	//     (Stripe Checkout — only works with `STRIPE_SECRET_KEY` set), or
	// (b) errors (no Stripe key, no plans seeded), surfacing as an `alert`
	//     and leaving the page on `/pricing`.
	// We assert the network call fires; the destination depends on the env.
	const cta = page
		.getByRole('button', { name: /get started|start now|sign up|subscribe|select|choose/i })
		.first();
	if (!(await cta.isVisible().catch(() => false))) {
		test.skip(true, 'No pricing CTA visible (plans not seeded).');
		return;
	}
	// Watch for the remote command — the SvelteKit RPC convention is
	// `POST /<route>/<command>` against the page that owns the remote.
	const checkoutCall = page
		.waitForRequest((req) => /checkout/i.test(req.url()) && req.method() === 'POST', {
			timeout: 5_000
		})
		.catch(() => null);

	// Stripe Checkout would full-page-redirect; suppress so the test can
	// finish even when keys are wired up. We also trap `window.alert`
	// raised by the failure path so the spec doesn't dialog-stall.
	await page.addInitScript(() => {
		// eslint-disable-next-line @typescript-eslint/no-explicit-any
		(window as any).alert = () => {};
	});

	await cta.click();
	const req = await checkoutCall;
	// Either the network call fired (preferred), or the page navigated to
	// Stripe / a sign-in route. Both prove the CTA wired through.
	const navigatedAway = !page.url().endsWith('/pricing');
	expect(
		req !== null || navigatedAway,
		`expected checkout request or navigation; current url: ${page.url()}`
	).toBe(true);
});
