// Svelte 5 reactive auth state class
//
// BFF (Phase 1.3 / `docs/REMAINING-WORK.md`):
//   The access + refresh tokens NO LONGER live in `localStorage`. They are
//   minted by the backend as `HttpOnly`, `Secure` (in prod), `SameSite=Lax`
//   cookies (`swings_access` / `swings_refresh`) that the browser attaches
//   automatically on every same-origin `/api/*` request. JS literally cannot
//   read them — an XSS sink can no longer exfiltrate the bearer token.
//
//   What this store still owns:
//     * `user` — the *non-sensitive* identity record (id, name, role, email)
//       that the UI renders. Cached in `localStorage` under `swings_user` so
//       the SSR-hydrated shell doesn't flash "logged out" before the
//       `/api/auth/me` round-trip resolves.
//     * `loading` — one-shot resolution flag.
//     * `logout` — calls the server logout endpoint so the cookies get
//       cleared, then drops the cached user.
//
//   What this store no longer owns:
//     * `accessToken` / `refreshToken` — gone. The browser cookie jar is the
//       source of truth.
//     * `setAuth` / `setTokens` — gone. Login pages call `setUser` only.
//
// See AUDIT.md §3.1 #1 for the regression that motivated this migration.

import { browser } from '$app/environment';

export interface AuthUser {
	id: string;
	email: string;
	name: string;
	role: 'member' | 'admin';
	avatar_url: string | null;
	created_at: string;
}

const USER_KEY = 'swings_user';
// BFF migration cleanup: the previous build wrote raw JWTs under these keys.
// Hydration explicitly removes them so a session that started pre-rollout
// does not leave dormant tokens lying around in `localStorage`.
const LEGACY_TOKEN_KEY = 'swings_access_token';
const LEGACY_REFRESH_KEY = 'swings_refresh_token';

class AuthState {
	user = $state<AuthUser | null>(null);
	loading = $state(true);

	// `isAuthenticated` no longer references a JS-readable access token (we
	// can't see the cookie). Presence of a cached `user` plus a successful
	// `/api/auth/me` round-trip on mount is the gate the SPA actually needs.
	isAuthenticated = $derived(!!this.user);
	isAdmin = $derived(this.user?.role?.toLowerCase() === 'admin');
	isMember = $derived(this.user?.role?.toLowerCase() === 'member');

	constructor() {
		if (browser) {
			// One-time migration: scrub any pre-cookie tokens lingering in
			// localStorage. Safe to run unconditionally — `removeItem` is a
			// no-op when the key is absent.
			try {
				localStorage.removeItem(LEGACY_TOKEN_KEY);
				localStorage.removeItem(LEGACY_REFRESH_KEY);
			} catch {
				// Private-mode browsers throw on localStorage access; ignore.
			}

			const stored = (() => {
				try {
					return localStorage.getItem(USER_KEY);
				} catch {
					return null;
				}
			})();
			if (stored) {
				try {
					this.user = JSON.parse(stored);
				} catch {
					this.user = null;
				}
			}
		}
		// Resolve loading on both server and client so SSR-rendered UI doesn't
		// get stuck in a "loading" state forever.
		this.loading = false;
	}

	setUser = (user: AuthUser) => {
		this.user = user;
		if (browser) {
			try {
				localStorage.setItem(USER_KEY, JSON.stringify(user));
			} catch {
				// Quota / private-mode failures must never break the auth
				// flow — the in-memory `$state` is still authoritative.
			}
		}
	};

	logout = async () => {
		// Clear server-side cookies first. We send `credentials: 'include'`
		// so the existing session cookie travels with the POST, letting the
		// extractor identify the caller; the response carries deletion
		// `Set-Cookie` headers (Max-Age=0) for both halves.
		if (browser) {
			try {
				// Same-origin in production (Vercel rewrites `/api/*` →
				// Railway). In dev the Vite proxy forwards the cookie just
				// as well. We deliberately do not surface failures here —
				// even if the server is unreachable, dropping the local
				// user state makes the UI behave as logged out.
				await fetch('/api/auth/logout', {
					method: 'POST',
					credentials: 'include',
					headers: { 'Content-Type': 'application/json' },
					body: '{}'
				});
			} catch {
				// Network failure should not trap the user in a "logged in"
				// shell — fall through and clear local state regardless.
			}
		}

		this.user = null;

		if (browser) {
			try {
				localStorage.removeItem(USER_KEY);
				// Belt-and-braces: scrub legacy keys again in case some
				// other tab wrote them while this tab was alive.
				localStorage.removeItem(LEGACY_TOKEN_KEY);
				localStorage.removeItem(LEGACY_REFRESH_KEY);
			} catch {
				// Private mode — already gone or inaccessible.
			}
		}
	};
}

export const auth = new AuthState();
