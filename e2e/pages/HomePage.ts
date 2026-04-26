/**
 * Home page (`/`) — landing hero, marketing nav, featured pricing section.
 *
 * Public, prerendered. Tests here are cheap because SSR output is static.
 */

import { expect, type Locator, type Page } from '@playwright/test';
import { landmarks } from '../fixtures/selectors';

export class HomePage {
	constructor(readonly page: Page) {}

	async open(): Promise<void> {
		await this.page.goto('/');
		await this.page.waitForLoadState('domcontentloaded');
	}

	/** `<h1>` in Hero.svelte — the marketing headline. */
	hero(): Locator {
		return this.page.getByRole('heading', { level: 1 });
	}

	/** Top-of-page "Get Started" style CTA — matches the first visible primary button. */
	ctaPrimary(): Locator {
		return this.page.getByRole('link', { name: /get started|join|sign up/i }).first();
	}

	async visitPricingFromNav(): Promise<void> {
		await landmarks
			.primaryNav(this.page)
			.getByRole('link', { name: /pricing/i })
			.first()
			.click();
		await expect(this.page).toHaveURL(/\/pricing/);
	}
}
