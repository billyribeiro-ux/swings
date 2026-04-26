/**
 * Happy-path: register -> login -> dashboard -> logout.
 *
 * Every run uses a disposable email so the test is idempotent. The backend
 * returns JWT tokens that the auth store persists to localStorage; after
 * logout both tokens and the cached `user` blob should be gone.
 */

import { test, expect, STORAGE_KEYS } from '../fixtures/app';
import { disposableEmail, disposablePassword } from '../fixtures/helpers';
import { RegisterPage } from '../pages/RegisterPage';
import { LoginPage } from '../pages/LoginPage';
import { DashboardPage } from '../pages/DashboardPage';

test.describe('account lifecycle', () => {
	// Phase 8.5: the cookie-consent banner stacks above the dashboard chrome
	// (z-40) and intercepts pointer events on the "Sign Out" button. Dismiss
	// it once at the start so subsequent logout clicks land on the button.
	test.beforeEach(async ({ page }) => {
		await page.goto('/');
		const banner = page.getByTestId('consent-banner');
		// `isVisible` resolves quickly when the element is missing (server
		// render w/o consent state); short timeout to keep the cold start fast.
		const visible = await banner.isVisible().catch(() => false);
		if (visible) {
			const accept = page.getByRole('button', { name: /accept all|reject all/i }).first();
			if (await accept.isVisible().catch(() => false)) {
				await accept.click();
			}
		}
	});

	test('register then re-login then logout purges tokens', async ({ page, api }) => {
		if (!(await api.isReachable())) {
			test.skip(true, 'Backend unreachable.');
			return;
		}

		const email = disposableEmail('signup');
		const password = disposablePassword();

		// 1. Register
		const register = new RegisterPage(page);
		await register.open();
		await register.enterCredentials({ email, password, name: 'E2E Signup' });
		const registerResp = await register.submit();
		expect(registerResp?.status(), 'register should succeed').toBe(200);

		// 2. Redirect to dashboard — the store may flash `loading` first.
		const dashboard = new DashboardPage(page);
		await page.waitForURL(/\/dashboard/, { timeout: 10_000 });
		await dashboard.expectLoaded();

		// 3. Logout and re-login with the same credentials.
		await dashboard.logout();
		await expect(page).toHaveURL(/\/login/);

		const login = new LoginPage(page);
		await login.open();
		await login.enterCredentials(email, password);
		const loginResp = await login.submit();
		expect(loginResp?.status(), 'login should succeed').toBe(200);
		await page.waitForURL(/\/dashboard/, { timeout: 10_000 });
		await dashboard.expectLoaded();

		// 4. Final logout clears localStorage.
		await dashboard.logout();
		const stored = await page.evaluate(
			(keys) => ({
				access: localStorage.getItem(keys.access),
				refresh: localStorage.getItem(keys.refresh),
				user: localStorage.getItem(keys.user)
			}),
			STORAGE_KEYS
		);
		expect(stored.access).toBeNull();
		expect(stored.refresh).toBeNull();
		expect(stored.user).toBeNull();
	});
});
