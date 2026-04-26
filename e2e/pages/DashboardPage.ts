/**
 * Dashboard (`/dashboard`) — member area behind auth gate.
 *
 * Relies on `authedMemberTest` having primed localStorage. The top-level
 * `onMount` in `dashboard/+layout.svelte` redirects to `/login` when the store
 * reports `!isAuthenticated`, so tests that don't prime will naturally fall
 * back and can assert on that behaviour.
 */

import { expect, type Locator, type Page } from '@playwright/test';
import { dashboardNavLink } from '../fixtures/selectors';

export class DashboardPage {
	constructor(readonly page: Page) {}

	async open(): Promise<void> {
		await this.page.goto('/dashboard');
		await this.page.waitForLoadState('domcontentloaded');
	}

	async expectLoaded(): Promise<void> {
		// The dashboard shell renders a greeting ("Welcome back, {firstName}") in
		// the header once the auth store resolves. Waiting on the greeting also
		// confirms the /dashboard layout rendered rather than the login redirect.
		await expect(this.page.getByRole('heading', { name: /welcome back/i })).toBeVisible({
			timeout: 10_000
		});
	}

	signOutButton(): Locator {
		return this.page.getByRole('button', { name: /sign out/i });
	}

	async logout(): Promise<void> {
		await this.signOutButton().click();
		await this.page.waitForURL(/\/login/, { timeout: 10_000 });
	}

	/**
	 * Navigate to notification preferences if the FDN-05 route exists. When
	 * the backend hasn't exposed the settings page yet we resolve falsy so the
	 * caller can `test.skip()` cleanly.
	 */
	async openNotificationPrefs(): Promise<boolean> {
		const link = this.page.getByRole('link', { name: /notification/i }).first();
		if (!(await link.isVisible().catch(() => false))) return false;
		await link.click();
		await this.page.waitForLoadState('domcontentloaded');
		const notFound = await this.page
			.getByText(/404|not found/i)
			.isVisible()
			.catch(() => false);
		return !notFound;
	}

	/** Quick nav helper — used by auth specs to bounce between sections. */
	async navTo(label: RegExp): Promise<void> {
		await dashboardNavLink(this.page, label).click();
	}
}
