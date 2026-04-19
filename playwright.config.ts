import { defineConfig, devices } from '@playwright/test';

/**
 * End-to-end test configuration.
 *
 * - `webServer` boots `pnpm build && pnpm preview` (deterministic SSR build) so CI
 *   does not exercise the Vite dev-server HMR path. Locally the server is reused
 *   across runs to keep the inner loop fast.
 * - Specs live under `e2e/`. The smoke suite runs in all three engines to catch
 *   browser-specific regressions; heavier flows (auth, popups) pin to Chromium to
 *   keep wall-clock under a few minutes.
 * - Traces/screenshots/videos are captured only on failure — `trace: 'on-first-retry'`
 *   avoids the ~3x slowdown of always-on tracing while still giving post-mortem
 *   visibility for flaky cases.
 */
const PORT = Number(process.env.PLAYWRIGHT_PORT ?? 4173);
const BASE_URL = process.env.PLAYWRIGHT_BASE_URL ?? `http://localhost:${PORT}`;

export default defineConfig({
	testDir: 'e2e',
	fullyParallel: true,
	retries: process.env.CI ? 2 : 0,
	workers: process.env.CI ? 2 : undefined,
	reporter: [['list'], ['html', { open: 'never' }]],
	forbidOnly: !!process.env.CI,

	use: {
		baseURL: BASE_URL,
		trace: 'on-first-retry',
		screenshot: 'only-on-failure',
		video: 'retain-on-failure',
		actionTimeout: 10_000,
		navigationTimeout: 30_000
	},

	webServer: {
		command: process.env.PLAYWRIGHT_WEB_SERVER_COMMAND ?? 'pnpm build && pnpm preview',
		url: BASE_URL,
		reuseExistingServer: !process.env.CI,
		stdout: 'pipe',
		stderr: 'pipe',
		timeout: 120_000
	},

	projects: [
		// Smoke suite — exercised in all three engines for cross-browser coverage.
		{
			name: 'smoke-chromium',
			testDir: 'e2e/smoke',
			use: { ...devices['Desktop Chrome'] }
		},
		{
			name: 'smoke-firefox',
			testDir: 'e2e/smoke',
			use: { ...devices['Desktop Firefox'] }
		},
		{
			name: 'smoke-webkit',
			testDir: 'e2e/smoke',
			use: { ...devices['Desktop Safari'] }
		},

		// Heavier flows — Chromium only to bound CI minutes. These tests touch the
		// backend (login/register/rate-limit) and cross-browser matters less than
		// the speed of the feedback loop.
		{
			name: 'auth',
			testDir: 'e2e/auth',
			use: { ...devices['Desktop Chrome'] }
		},
		{
			name: 'popups',
			testDir: 'e2e/popups',
			use: { ...devices['Desktop Chrome'] }
		},

		// Admin panel flows — back-office UI guarded by a privileged session.
		// Each spec opts into the `authedAdminTest` fixture which skips when
		// the backend is unreachable, so this project is safe to enable in
		// pure-frontend CI as well.
		{
			name: 'admin',
			testDir: 'e2e/admin',
			use: { ...devices['Desktop Chrome'] }
		}
	]
});
