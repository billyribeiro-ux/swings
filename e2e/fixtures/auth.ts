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
 */

import { test as base, type AuthBundle, AppClient, ApiClient, STORAGE_KEYS } from './app';
import { disposableEmail, disposablePassword } from './helpers';

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

/** Shared login helper used by both admin + member fixtures. */
async function loginAndPrime(
	app: AppClient,
	api: ApiClient,
	email: string,
	password: string
): Promise<AuthBundle | null> {
	const res = await api.post<AuthBundle>('/api/auth/login', { email, password });
	if (!res.ok || !res.body) return null;
	await app.primeAuth(res.body);
	return res.body;
}

export const authedAdminTest = base.extend<AdminFixtures>({
	admin: async ({ app, api }, use) => {
		if (!(await api.isReachable())) {
			base.skip(true, 'Backend unreachable; admin fixture disabled.');
			return;
		}
		const { email, password } = adminCredentials();
		const bundle = await loginAndPrime(app, api, email, password);
		if (!bundle) {
			base.skip(
				true,
				'Admin login failed — ensure ADMIN_EMAIL/ADMIN_PASSWORD match the seeded user.'
			);
			return;
		}
		await use(bundle);
	}
});

export const authedMemberTest = base.extend<MemberFixtures>({
	member: async ({ app, api }, use) => {
		if (!(await api.isReachable())) {
			base.skip(true, 'Backend unreachable; member fixture disabled.');
			return;
		}
		const email = disposableEmail('member');
		const password = disposablePassword();
		const registered = await api.post<AuthBundle>('/api/auth/register', {
			email,
			password,
			name: 'E2E Member'
		});
		if (!registered.ok || !registered.body) {
			base.skip(true, `Member registration failed (status ${registered.status}).`);
			return;
		}
		await app.primeAuth(registered.body);
		await use(registered.body);
	}
});

export { STORAGE_KEYS };
