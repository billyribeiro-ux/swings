/**
 * Register page (`/register`).
 *
 * Posts to `/api/auth/register`. On success the store goes to `/dashboard`.
 * Failed validation renders an inline error block but no toast — assert on the
 * rendered text plus the Problem response body for redundancy.
 */

import { expect, type Page, type Response as PlaywrightResponse } from '@playwright/test';
import { authForm } from '../fixtures/selectors';

export interface RegisterInput {
	email: string;
	password: string;
	name: string;
}

export class RegisterPage {
	constructor(readonly page: Page) {}

	async open(): Promise<void> {
		await this.page.goto('/register');
		await expect(
			this.page.getByRole('heading', { level: 1, name: /create your account/i })
		).toBeVisible();
	}

	async enterCredentials(input: RegisterInput): Promise<void> {
		await authForm.name(this.page).fill(input.name);
		await authForm.email(this.page).fill(input.email);
		await authForm.password(this.page).fill(input.password);
		await authForm.confirmPassword(this.page).fill(input.password);
	}

	async submit(): Promise<PlaywrightResponse | null> {
		const responsePromise = this.page.waitForResponse(
			(r) => r.url().endsWith('/api/auth/register'),
			{ timeout: 10_000 }
		);
		await authForm.submit(this.page).click();
		try {
			return await responsePromise;
		} catch {
			return null;
		}
	}
}
