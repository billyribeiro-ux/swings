/**
 * Pre-authenticated test variants.
 *
 *   - `authedAdminTest` — logs in as the bootstrap admin seeded from
 *     `ADMIN_EMAIL` / `ADMIN_PASSWORD` and exposes `admin: AuthBundle`.
 *   - `authedMemberTest` — registers a fresh disposable member, then logs
 *     that user in. A new account per test guarantees parallel workers do
 *     not stomp on each other's subscription state. Exposes `member: AuthBundle`.
 *
 * When the backend is unreachable the fixture calls `test.skip()` with a
 * descriptive reason — prevents the entire auth bucket from hard-failing on a
 * cold dev box.
 *
 * BFF cookie carry (Phase 8.2): the backend now issues `Set-Cookie:
 * swings_access=...` + `swings_refresh=...` on `/api/auth/login` and the
 * SPA never sees a bearer token (`client.ts` uses `credentials: 'include'`).
 * The fixture therefore performs the login via `page.context().request`,
 * which auto-persists the response cookies into the browser context's
 * cookie jar. The first navigation by the spec then carries them to the
 * Rust API as `Cookie: swings_access=...`. We still expose the JSON
 * payload (`AuthBundle`) so specs can assert on user metadata, but the
 * legacy `localStorage` priming remains as a belt-and-braces compatibility
 * shim for any client code that still reads the cached `user` blob.
 */

import {
	test as base,
	type AuthBundle,
	AppClient,
	ApiClient,
	STORAGE_KEYS,
	type AppFixtures
} from './app';
import { disposableEmail, disposablePassword } from './helpers';
import { apiBaseUrl } from './helpers';
import type { BrowserContext, APIRequestContext } from '@playwright/test';

export interface AdminFixtures {
	admin: AuthBundle;
}

export interface MemberFixtures {
	member: AuthBundle;
}

/** Credential source for the bootstrap admin — must match backend `main.rs`. */
function adminCredentials(): { email: string; password: string } {
	return {
		email: process.env.ADMIN_EMAIL ?? 'admin@swings.test',
		password: process.env.ADMIN_PASSWORD ?? 'admin-password-1234'
	};
}

/**
 * Cookie-aware login: use the **page's** APIRequestContext so the
 * `Set-Cookie` headers from `/api/auth/login` land in the browser
 * context's cookie jar that subsequent `page.goto(...)` will carry.
 * Returns `null` on any failure so the caller can `test.skip()`.
 */
async function loginViaCookie(
	context: BrowserContext,
	pageRequest: APIRequestContext,
	email: string,
	password: string
): Promise<AuthBundle | null> {
	try {
		const res = await pageRequest.post(`${apiBaseUrl()}/api/auth/login`, {
			headers: { 'content-type': 'application/json' },
			data: JSON.stringify({ email, password })
		});
		if (!res.ok()) return null;
		const body = (await res.json().catch(() => null)) as AuthBundle | null;
		if (!body) return null;

		// Extra safety net: when the API origin and the browser origin differ
		// (dev: backend on :3001, app on :5173), Playwright's auto-cookie-carry
		// still puts the cookies on the API host. Spec navigations to the SPA
		// hit the SvelteKit dev server which proxies `/api/*`, so requests are
		// same-origin against the SPA — meaning the cookie also needs to live
		// on the SPA host. Mirror it explicitly.
		const apiCookies = await context.cookies(apiBaseUrl());
		const spaUrl = process.env.PLAYWRIGHT_BASE_URL ?? 'http://127.0.0.1:5173';
		const spaHost = new URL(spaUrl).hostname;
		const mirrored = apiCookies
			.filter((c) => c.name === 'swings_access' || c.name === 'swings_refresh')
			.map((c) => ({ ...c, domain: spaHost }));
		if (mirrored.length > 0) {
			await context.addCookies(mirrored);
		}
		return body;
	} catch {
		return null;
	}
}

/** Shared post-login priming — wires localStorage (legacy `user` blob). */
async function primeFromBundle(app: AppClient, bundle: AuthBundle): Promise<void> {
	await app.primeAuth(bundle);
}

export const authedAdminTest = base.extend<AdminFixtures & AppFixtures>({
	admin: async ({ app, api, page }, use) => {
		if (!(await api.isReachable())) {
			base.skip(true, 'Backend unreachable; admin fixture disabled.');
			return;
		}
		const { email, password } = adminCredentials();
		const bundle = await loginViaCookie(
			page.context(),
			page.context().request,
			email,
			password
		);
		if (!bundle) {
			base.skip(
				true,
				'Admin login failed — ensure ADMIN_EMAIL/ADMIN_PASSWORD match the seeded user.'
			);
			return;
		}
		await primeFromBundle(app, bundle);
		await use(bundle);
	}
});

export const authedMemberTest = base.extend<MemberFixtures & AppFixtures>({
	member: async ({ app, api, page }, use) => {
		if (!(await api.isReachable())) {
			base.skip(true, 'Backend unreachable; member fixture disabled.');
			return;
		}
		const email = disposableEmail('member');
		const password = disposablePassword();
		// Registration also issues `Set-Cookie` for the fresh session, mirroring
		// the login flow; reuse the cookie-aware path for both halves.
		try {
			const reg = await page.context().request.post(`${apiBaseUrl()}/api/auth/register`, {
				headers: { 'content-type': 'application/json' },
				data: JSON.stringify({ email, password, name: 'E2E Member' })
			});
			if (!reg.ok()) {
				base.skip(true, `Member registration failed (status ${reg.status()}).`);
				return;
			}
			const bundle = (await reg.json().catch(() => null)) as AuthBundle | null;
			if (!bundle) {
				base.skip(true, 'Member registration returned malformed body.');
				return;
			}
			// Mirror cookies onto the SPA host (see `loginViaCookie` rationale).
			const spaUrl = process.env.PLAYWRIGHT_BASE_URL ?? 'http://127.0.0.1:5173';
			const spaHost = new URL(spaUrl).hostname;
			const apiCookies = await page.context().cookies(apiBaseUrl());
			const mirrored = apiCookies
				.filter((c) => c.name === 'swings_access' || c.name === 'swings_refresh')
				.map((c) => ({ ...c, domain: spaHost }));
			if (mirrored.length > 0) {
				await page.context().addCookies(mirrored);
			}
			await primeFromBundle(app, bundle);
			await use(bundle);
		} catch {
			base.skip(true, 'Member registration threw unexpectedly.');
			return;
		}
		// `api` and `ApiClient` are still consumed by parent fixtures even when
		// not used directly here — keep the dependency to preserve the chain.
		void api;
		void ApiClient;
	}
});

export { STORAGE_KEYS };
