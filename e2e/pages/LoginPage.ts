/**
 * Login page (`/login`).
 *
 * Submits to `POST /api/auth/login`. On success the store fires `goto(...)` to
 * `/dashboard` (member) or `/admin` (admin); tests wait for the navigation
 * rather than polling `localStorage` so the assertions line up with real UX.
 */

import { expect, type Page, type Response as PlaywrightResponse } from '@playwright/test';
import { authForm } from '../fixtures/selectors';
import type { ProblemDocument } from '../fixtures/helpers';
import { isProblem } from '../fixtures/helpers';

export class LoginPage {
	constructor(readonly page: Page) {}

	async open(): Promise<void> {
		await this.page.goto('/login');
		await expect(
			this.page.getByRole('heading', { level: 1, name: /welcome back/i })
		).toBeVisible();
	}

	async enterCredentials(email: string, password: string): Promise<void> {
		await authForm.email(this.page).fill(email);
		await authForm.password(this.page).fill(password);
	}

	/** Click submit and resolve with the login-endpoint response for assertions. */
	async submit(): Promise<PlaywrightResponse | null> {
		const responsePromise = this.page.waitForResponse(
			(r) => r.url().endsWith('/api/auth/login'),
			{ timeout: 10_000 }
		);
		await authForm.submit(this.page).click();
		try {
			return await responsePromise;
		} catch {
			return null;
		}
	}

	async expectProblem(
		response: PlaywrightResponse | null,
		status: number,
		typeSuffix?: string
	): Promise<ProblemDocument> {
		expect(response, 'login response missing').not.toBeNull();
		const res = response!;
		expect(res.status()).toBe(status);
		const body = await res.json().catch(() => null);
		expect(isProblem(body)).toBe(true);
		const problem = body as ProblemDocument;
		if (typeSuffix) {
			expect(problem.type.endsWith(typeSuffix)).toBe(true);
		}
		return problem;
	}
}
