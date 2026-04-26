/**
 * `auth` store — browser-mode runes coverage.
 *
 * Scope:
 *   - setUser hydrates state and persists to localStorage.
 *   - isAuthenticated, isAdmin, isMember derive correctly.
 *   - logout calls /api/auth/logout, clears state, scrubs USER_KEY.
 *   - logout tolerates network failure (still clears in-memory state).
 *   - Constructor scrubs legacy token keys from localStorage.
 *
 * Module loads only once per worker; we reset state via the public API
 * between tests rather than reinstantiating the singleton.
 */
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { auth, type AuthUser } from './auth.svelte';

const USER_KEY = 'swings_user';
const LEGACY_TOKEN_KEY = 'swings_access_token';
const LEGACY_REFRESH_KEY = 'swings_refresh_token';

const sampleAdmin: AuthUser = {
	id: 'u1',
	email: 'admin@swings.test',
	name: 'Admin',
	role: 'admin',
	avatar_url: null,
	created_at: '2026-01-01T00:00:00Z'
};
const sampleMember: AuthUser = { ...sampleAdmin, id: 'u2', role: 'member' };

beforeEach(() => {
	localStorage.removeItem(USER_KEY);
	localStorage.removeItem(LEGACY_TOKEN_KEY);
	localStorage.removeItem(LEGACY_REFRESH_KEY);
	void auth.logout(); // best-effort drain
});

afterEach(() => {
	vi.restoreAllMocks();
});

describe('auth store', () => {
	it('setUser writes to localStorage and updates state', () => {
		auth.setUser(sampleAdmin);
		expect(auth.user?.id).toBe('u1');
		const stored = JSON.parse(localStorage.getItem(USER_KEY) ?? 'null');
		expect(stored?.id).toBe('u1');
	});

	it('isAuthenticated reflects whether `user` is set', async () => {
		await auth.logout();
		expect(auth.isAuthenticated).toBe(false);
		auth.setUser(sampleAdmin);
		expect(auth.isAuthenticated).toBe(true);
	});

	it('isAdmin / isMember derive from role', () => {
		auth.setUser(sampleAdmin);
		expect(auth.isAdmin).toBe(true);
		expect(auth.isMember).toBe(false);
		auth.setUser(sampleMember);
		expect(auth.isAdmin).toBe(false);
		expect(auth.isMember).toBe(true);
	});

	it('logout posts to /api/auth/logout with credentials', async () => {
		auth.setUser(sampleAdmin);
		const fetchSpy = vi
			.spyOn(globalThis, 'fetch')
			.mockResolvedValue(new Response('', { status: 200 }));
		await auth.logout();
		expect(fetchSpy).toHaveBeenCalledOnce();
		const [url, init] = fetchSpy.mock.calls[0] as [string, RequestInit];
		expect(String(url)).toContain('/api/auth/logout');
		expect(init.method).toBe('POST');
		expect(init.credentials).toBe('include');
		expect(auth.user).toBeNull();
		expect(localStorage.getItem(USER_KEY)).toBeNull();
	});

	it('logout tolerates a network failure and still clears state', async () => {
		auth.setUser(sampleAdmin);
		vi.spyOn(globalThis, 'fetch').mockRejectedValue(new Error('offline'));
		await auth.logout();
		expect(auth.user).toBeNull();
		expect(localStorage.getItem(USER_KEY)).toBeNull();
	});

	it('logout scrubs legacy token keys', async () => {
		localStorage.setItem(LEGACY_TOKEN_KEY, 'jwt-junk');
		localStorage.setItem(LEGACY_REFRESH_KEY, 'refresh-junk');
		vi.spyOn(globalThis, 'fetch').mockResolvedValue(new Response('', { status: 200 }));
		await auth.logout();
		expect(localStorage.getItem(LEGACY_TOKEN_KEY)).toBeNull();
		expect(localStorage.getItem(LEGACY_REFRESH_KEY)).toBeNull();
	});
});
