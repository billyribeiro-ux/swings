import { auth } from '$lib/stores/auth.svelte';
import { getPublicApiBase } from '$lib/api/publicApiBase';

const API_BASE = getPublicApiBase();

interface FetchOptions extends RequestInit {
	skipAuth?: boolean;
}

/**
 * BFF (Phase 1.3, `docs/REMAINING-WORK.md`):
 *
 * Every authenticated request now carries `credentials: 'include'` so the
 * httpOnly `swings_access` cookie travels automatically. We deliberately
 * STOP attaching `Authorization: Bearer ...` from `auth.accessToken` —
 * that field no longer exists. The cookie is the source of truth.
 *
 * The 401 → refresh → retry flow is preserved: `POST /api/auth/refresh`
 * with `credentials: 'include'` lets the backend read the
 * `swings_refresh` cookie and issue a rotated pair via fresh
 * `Set-Cookie` headers. The browser swallows them; the SPA never needs
 * to handle the raw token.
 */
class ApiClient {
	private baseUrl: string;
	/** Ensures parallel 401s share one refresh (avoids races invalidating the refresh cookie). */
	private refreshInFlight: Promise<boolean> | null = null;

	constructor(baseUrl: string) {
		this.baseUrl = baseUrl;
	}

	private async request<T>(endpoint: string, options: FetchOptions = {}): Promise<T> {
		const { skipAuth, ...fetchOptions } = options;
		const headers = new Headers(fetchOptions.headers);

		if (
			!headers.has('Content-Type') &&
			fetchOptions.body &&
			!(fetchOptions.body instanceof FormData)
		) {
			headers.set('Content-Type', 'application/json');
		}

		// `credentials: 'include'` is the BFF contract: the browser
		// attaches `Cookie: swings_access=...` to every request. Same-
		// origin in production (Vercel rewrites `/api/*` → Railway) so
		// the cookie is naturally in scope; cross-origin in dev when the
		// SPA hits Vite-proxied `/api` over `http://localhost:5173`.
		const response = await fetch(`${this.baseUrl}${endpoint}`, {
			...fetchOptions,
			credentials: 'include',
			headers
		});

		if (response.status === 401 && !skipAuth) {
			const refreshed = await this.refreshTokens();
			if (refreshed) {
				const retry = await fetch(`${this.baseUrl}${endpoint}`, {
					...fetchOptions,
					credentials: 'include',
					headers
				});
				if (!retry.ok) {
					const err = await retry.json().catch(() => ({ error: 'Request failed' }));
					throw new ApiError(retry.status, err.error || 'Request failed');
				}
				return retry.json();
			} else {
				// Don't await the logout fetch — the request that failed has
				// already returned 401, the cookies are already invalid; we
				// just need to drop in-memory `user` so the UI re-routes.
				void auth.logout();
				throw new ApiError(401, 'Session expired');
			}
		}

		if (!response.ok) {
			const err = await response.json().catch(() => ({ error: 'Request failed' }));
			throw new ApiError(response.status, err.error || 'Request failed');
		}

		return response.json();
	}

	private async refreshTokens(): Promise<boolean> {
		if (this.refreshInFlight) {
			return this.refreshInFlight;
		}
		this.refreshInFlight = this.performRefresh();
		try {
			return await this.refreshInFlight;
		} finally {
			this.refreshInFlight = null;
		}
	}

	private async performRefresh(): Promise<boolean> {
		try {
			// `credentials: 'include'` ships `Cookie: swings_refresh=...`
			// to the backend. The handler reads it from the cookie jar
			// (no JSON body required during Phase A). The response sets
			// rotated `Set-Cookie` headers for both halves; the browser
			// swallows them and we never see the new tokens.
			//
			// IMPORTANT: do NOT send `Content-Type: application/json` with
			// an empty body — earlier we sent `'{}'` and the typed extractor
			// `Json<RefreshRequest>` rejected it as malformed (missing
			// `refresh_token`) with 422 before our cookie-fallback ran.
			const res = await fetch(`${this.baseUrl}/api/auth/refresh`, {
				method: 'POST',
				credentials: 'include'
			});

			return res.ok;
		} catch {
			return false;
		}
	}

	async get<T>(endpoint: string, options?: FetchOptions): Promise<T> {
		return this.request<T>(endpoint, { ...options, method: 'GET' });
	}

	async post<T>(endpoint: string, body?: unknown, options?: FetchOptions): Promise<T> {
		return this.request<T>(endpoint, {
			...options,
			method: 'POST',
			body: body ? JSON.stringify(body) : undefined
		});
	}

	async put<T>(endpoint: string, body?: unknown, options?: FetchOptions): Promise<T> {
		return this.request<T>(endpoint, {
			...options,
			method: 'PUT',
			body: body ? JSON.stringify(body) : undefined
		});
	}

	async del<T>(endpoint: string, options?: FetchOptions): Promise<T> {
		return this.request<T>(endpoint, { ...options, method: 'DELETE' });
	}

	async delete<T>(endpoint: string, options?: FetchOptions): Promise<T> {
		return this.del<T>(endpoint, options);
	}

	async upload<T>(endpoint: string, formData: FormData, options?: FetchOptions): Promise<T> {
		return this.request<T>(endpoint, {
			...options,
			method: 'POST',
			body: formData
		});
	}

	/**
	 * Authenticated GET that returns the raw response body as a `Blob`
	 * along with the suggested filename parsed from `Content-Disposition`
	 * (when present). Used by file-download endpoints — DSAR
	 * `/jobs/{id}/artifact`, audit `/export.csv`, etc. — that the JSON
	 * helpers can't service because the body isn't JSON.
	 *
	 * Mirrors the auth + 401 → refresh → retry behaviour of
	 * [`request`] so the operator's session stays alive across long
	 * artefact composes. `credentials: 'include'` ships the
	 * `swings_access` cookie automatically.
	 */
	async getBlob(
		endpoint: string,
		options?: FetchOptions
	): Promise<{ blob: Blob; filename: string | null }> {
		const { skipAuth, ...fetchOptions } = options ?? {};
		const headers = new Headers(fetchOptions.headers);
		let response = await fetch(`${this.baseUrl}${endpoint}`, {
			...fetchOptions,
			method: 'GET',
			credentials: 'include',
			headers
		});
		if (response.status === 401 && !skipAuth) {
			const refreshed = await this.refreshTokens();
			if (!refreshed) {
				void auth.logout();
				throw new ApiError(401, 'Session expired');
			}
			response = await fetch(`${this.baseUrl}${endpoint}`, {
				...fetchOptions,
				method: 'GET',
				credentials: 'include',
				headers
			});
		}
		if (!response.ok) {
			// Best-effort error JSON parse; falls back to a status-only
			// message if the body isn't JSON (likely for artefact streams).
			const err = await response.json().catch(() => ({ error: 'Download failed' }));
			throw new ApiError(response.status, err.error || 'Download failed');
		}
		const blob = await response.blob();
		const disposition = response.headers.get('Content-Disposition') ?? '';
		// Match `filename="…"` or `filename=…` after the standard
		// `attachment;` prefix that [`stream_artifact`] sets.
		const m = /filename\*?=(?:UTF-8''|"?)([^";]+)"?/i.exec(disposition);
		return { blob, filename: m ? decodeURIComponent(m[1]) : null };
	}
}

export class ApiError extends Error {
	status: number;

	constructor(status: number, message: string) {
		super(message);
		this.status = status;
	}
}

export const api = new ApiClient(API_BASE);
