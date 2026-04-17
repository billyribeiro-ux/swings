/**
 * Base Playwright fixtures — the entrypoint every spec imports `test` from.
 *
 * Two collaborators are exposed:
 *
 *   - `app: AppClient` — page-level actions (navigation, login, logout, assert
 *     the last JSON response is an RFC 7807 Problem). Thin wrapper around the
 *     raw `page` that bakes in our storage conventions (see `stores/auth.svelte.ts`
 *     for the shape — {TOKEN_KEY, REFRESH_KEY, USER_KEY} in localStorage).
 *
 *   - `api: ApiClient` — Node-side fetch client scoped to the current
 *     `APIRequestContext`. Used from fixtures (`authedAdminTest`) and specs
 *     that need to seed backend state without going through the UI.
 *
 * Graceful degradation: every network-touching method swallows failure to a
 * typed `{ ok: false, status, problem? }` result so callers can `test.skip()`
 * when the backend is unreachable rather than false-failing the suite.
 */

import {
	test as base,
	expect,
	type APIRequestContext,
	type Page,
	type Response as PlaywrightResponse
} from '@playwright/test';
import { apiBaseUrl, isProblem, type ProblemDocument } from './helpers';

export { expect };

/* -------------------------------------------------------------------------- */
/*  Auth payload shapes — mirror `src/lib/api/types.ts` / `schema.d.ts`.      */
/* -------------------------------------------------------------------------- */

export interface StoredUser {
	id: string;
	email: string;
	name: string;
	role: 'member' | 'admin';
	avatar_url: string | null;
	created_at: string;
}

export interface AuthBundle {
	user: StoredUser;
	access_token: string;
	refresh_token: string;
}

/** localStorage keys set by `$lib/stores/auth.svelte.ts`. Kept in sync here. */
export const STORAGE_KEYS = {
	access: 'swings_access_token',
	refresh: 'swings_refresh_token',
	user: 'swings_user'
} as const;

/* -------------------------------------------------------------------------- */
/*  API client — direct calls to the Rust backend for seeding + assertions.  */
/* -------------------------------------------------------------------------- */

export interface ApiResult<T> {
	ok: boolean;
	status: number;
	body: T | null;
	problem: ProblemDocument | null;
}

export class ApiClient {
	constructor(
		private readonly request: APIRequestContext,
		private readonly base = apiBaseUrl()
	) {}

	private url(path: string): string {
		const p = path.startsWith('/') ? path : `/${path}`;
		return `${this.base}${p}`;
	}

	async get<T>(path: string, headers: Record<string, string> = {}): Promise<ApiResult<T>> {
		const res = await this.request.get(this.url(path), { headers });
		return this.coerce<T>(res);
	}

	async post<T>(
		path: string,
		body?: unknown,
		headers: Record<string, string> = {}
	): Promise<ApiResult<T>> {
		const res = await this.request.post(this.url(path), {
			headers: { 'content-type': 'application/json', ...headers },
			data: body === undefined ? undefined : JSON.stringify(body)
		});
		return this.coerce<T>(res);
	}

	/** Health probe for `test.skip(...)` guards. */
	async isReachable(): Promise<boolean> {
		try {
			const res = await this.request.get(this.url('/health'), { timeout: 2_000 });
			return res.ok();
		} catch {
			return false;
		}
	}

	/** `/api/popups/active` returns `[]` when empty — use to gate popup specs. */
	async popupsExist(): Promise<boolean> {
		const result = await this.get<unknown[]>('/api/popups/active');
		return result.ok && Array.isArray(result.body) && result.body.length > 0;
	}

	private async coerce<T>(res: PlaywrightResponse): Promise<ApiResult<T>> {
		const contentType = res.headers()['content-type'] ?? '';
		let body: unknown = null;
		if (contentType.includes('json')) {
			try {
				body = await res.json();
			} catch {
				body = null;
			}
		}
		const problem = isProblem(body) ? body : null;
		return {
			ok: res.ok(),
			status: res.status(),
			body: problem ? null : (body as T | null),
			problem
		};
	}
}

/* -------------------------------------------------------------------------- */
/*  Page-level wrapper                                                        */
/* -------------------------------------------------------------------------- */

export class AppClient {
	constructor(
		readonly page: Page,
		readonly api: ApiClient
	) {}

	/** Same as `page.goto` but resolves relative to `baseURL` and waits for the
	 *  network to settle so spec code can stop sprinkling `waitForLoadState`. */
	async goto(path: string): Promise<void> {
		await this.page.goto(path);
		await this.page.waitForLoadState('domcontentloaded');
	}

	/** Primes `localStorage` with a pre-authenticated bundle, then reloads so the
	 *  auth store reads the values on construction (see `AuthState.constructor`). */
	async primeAuth(bundle: AuthBundle): Promise<void> {
		await this.page.addInitScript(
			([keys, payload]) => {
				try {
					localStorage.setItem(keys.access, payload.access_token);
					localStorage.setItem(keys.refresh, payload.refresh_token);
					localStorage.setItem(keys.user, JSON.stringify(payload.user));
				} catch {
					// Storage can be disabled under certain testing modes; don't crash.
				}
			},
			[STORAGE_KEYS, bundle] as const
		);
	}

	/** Drive the UI login form. Returns the `/api/auth/login` response for assertions. */
	async login(email: string, password: string): Promise<PlaywrightResponse | null> {
		await this.goto('/login');
		const responsePromise = this.page.waitForResponse(
			(r) => r.url().endsWith('/api/auth/login'),
			{ timeout: 10_000 }
		);
		await this.page.getByLabel(/email/i).fill(email);
		await this.page.getByLabel(/^password$/i).fill(password);
		await this.page.getByRole('button', { name: /sign in/i }).click();
		try {
			return await responsePromise;
		} catch {
			return null;
		}
	}

	async register(input: { email: string; password: string; name: string }): Promise<PlaywrightResponse | null> {
		await this.goto('/register');
		const responsePromise = this.page.waitForResponse(
			(r) => r.url().endsWith('/api/auth/register'),
			{ timeout: 10_000 }
		);
		await this.page.getByLabel(/full name/i).fill(input.name);
		await this.page.getByLabel(/email/i).fill(input.email);
		await this.page.getByLabel(/^password$/i).fill(input.password);
		await this.page.getByLabel(/confirm password/i).fill(input.password);
		await this.page.getByRole('button', { name: /create account/i }).click();
		try {
			return await responsePromise;
		} catch {
			return null;
		}
	}

	async logout(): Promise<void> {
		// Dashboard ships a "Sign Out" button. Admin has the same CTA in the sidebar.
		const signOut = this.page.getByRole('button', { name: /sign out/i });
		if (await signOut.isVisible().catch(() => false)) {
			await signOut.click();
		}
		// Hard purge — ensures a clean slate even if the UI button was hidden.
		await this.page.evaluate((keys) => {
			try {
				localStorage.removeItem(keys.access);
				localStorage.removeItem(keys.refresh);
				localStorage.removeItem(keys.user);
			} catch {
				/* storage disabled */
			}
		}, STORAGE_KEYS);
	}

	/**
	 * Assert the given response is an RFC 7807 Problem with the expected status
	 * and (optionally) a type URI ending with `typeSuffix`. The body may be
	 * `application/problem+json` (preferred) or fall back to plain JSON with a
	 * matching shape — older endpoints still return the latter.
	 */
	async expectProblem(
		response: PlaywrightResponse | null,
		status: number,
		typeSuffix?: string
	): Promise<ProblemDocument> {
		expect(response, 'response missing').not.toBeNull();
		const res = response!;
		expect(res.status(), `expected HTTP ${status}`).toBe(status);
		const body = await res.json().catch(() => null);
		expect(body, 'response body is JSON').not.toBeNull();
		expect(isProblem(body), `body is RFC 7807 Problem: ${JSON.stringify(body)}`).toBe(true);
		const problem = body as ProblemDocument;
		expect(problem.status).toBe(status);
		if (typeSuffix) {
			expect(problem.type.endsWith(typeSuffix)).toBe(true);
		}
		return problem;
	}
}

/* -------------------------------------------------------------------------- */
/*  Fixture wiring                                                            */
/* -------------------------------------------------------------------------- */

export interface AppFixtures {
	app: AppClient;
	api: ApiClient;
}

export const test = base.extend<AppFixtures>({
	api: async ({ request }, use) => {
		const client = new ApiClient(request);
		await use(client);
	},
	app: async ({ page, api }, use) => {
		await use(new AppClient(page, api));
	}
});
