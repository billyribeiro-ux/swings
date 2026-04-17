/**
 * ConsentBanner — browser test coverage.
 *
 * Scope:
 *   1. Banner appears as a labelled region when no decision is recorded.
 *   2. "Accept all" dismisses the banner and sets every category on.
 *   3. Popup variant delegates to <Dialog>, providing focus-trap semantics
 *      (role=dialog + aria-modal=true).
 */
import { afterEach, describe, expect, it } from 'vitest';
import { page } from 'vitest/browser';
import { render } from 'vitest-browser-svelte';
import { consent } from '$lib/stores/consent.svelte';
import Harness from './_fixtures/ConsentBannerHarness.svelte';

afterEach(() => {
	// Reset shared store state between tests so one test's grant doesn't
	// paper over another test's banner visibility assertion.
	consent.revokeAll();
});

describe('ConsentBanner', () => {
	it('appears when hasDecided is false', async () => {
		render(Harness, { layout: 'bar' });
		const region = page.getByRole('region', { name: /cookie consent/i });
		await expect.element(region).toBeInTheDocument();
	});

	it('dismisses after Accept all and flips every category on', async () => {
		render(Harness, { layout: 'bar' });
		const accept = page.getByRole('button', { name: /accept all/i });
		await accept.click();
		await new Promise((r) => setTimeout(r, 30));
		const region = document.querySelector('[aria-labelledby="consent-banner-title"]');
		expect(region).toBeNull();
		expect(consent.state.hasDecided).toBe(true);
		for (const v of Object.values(consent.state.categories)) {
			expect(v).toBe(true);
		}
	});

	it('popup variant renders inside a <Dialog> (aria-modal + focus trap)', async () => {
		render(Harness, { layout: 'popup' });
		const dialog = page.getByRole('dialog');
		await expect.element(dialog).toBeInTheDocument();
		await expect.element(dialog).toHaveAttribute('aria-modal', 'true');
	});

	it('Reject all dismisses the banner and leaves only necessary granted', async () => {
		render(Harness, { layout: 'bar' });
		const reject = page.getByRole('button', { name: /reject all/i });
		await reject.click();
		await new Promise((r) => setTimeout(r, 30));
		expect(consent.state.hasDecided).toBe(true);
		expect(consent.state.categories.necessary).toBe(true);
		expect(consent.state.categories.analytics).toBe(false);
		expect(consent.state.categories.marketing).toBe(false);
	});
});
