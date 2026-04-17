/**
 * Admin UI Kit showcase (`/admin/_ui-kit`) — dev-only gallery of PE7 primitives.
 *
 * Used by the popups bucket (to trigger a Dialog) and as a ready-made visual
 * regression surface. When the admin layout doesn't render (unauthenticated,
 * or the `_ui-kit` route disabled in prod), methods return `false` so callers
 * can `test.skip()`.
 */

import { expect, type Locator, type Page } from '@playwright/test';

export class AdminUiKitPage {
	constructor(readonly page: Page) {}

	async open(): Promise<boolean> {
		await this.page.goto('/admin/_ui-kit');
		// A signed-out visit hits the admin login instead of the gallery.
		const loginHeading = this.page.getByRole('heading', { level: 1, name: /admin login/i });
		if (await loginHeading.isVisible().catch(() => false)) {
			return false;
		}
		await expect(this.page).toHaveURL(/\/admin\/_ui-kit/);
		return true;
	}

	buttonSection(): Locator {
		return this.page.getByRole('region', { name: /buttons?/i }).or(
			this.page.locator('section').filter({ hasText: /buttons?/i }).first()
		);
	}

	dialogSection(): Locator {
		return this.page.getByRole('region', { name: /dialog/i }).or(
			this.page.locator('section').filter({ hasText: /dialog/i }).first()
		);
	}

	toastSection(): Locator {
		return this.page.getByRole('region', { name: /toast/i }).or(
			this.page.locator('section').filter({ hasText: /toast/i }).first()
		);
	}

	async openDialogShowcase(): Promise<void> {
		const trigger = this.dialogSection().getByRole('button', { name: /open dialog|open/i }).first();
		await trigger.click();
	}

	async pushToast(kind: 'info' | 'success' | 'warning' = 'info'): Promise<void> {
		const trigger = this.toastSection()
			.getByRole('button', { name: new RegExp(kind, 'i') })
			.first();
		await trigger.click();
	}
}
