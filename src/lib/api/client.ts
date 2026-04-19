import { auth } from '$lib/stores/auth.svelte';
import { getPublicApiBase } from '$lib/api/publicApiBase';
import type { components } from '$lib/api/schema';

/**
 * FDN-02 migration demonstration: the refresh flow uses the OpenAPI-derived
 * `TokenResponse` type sourced from `schema.d.ts` (generated from the committed
 * backend snapshot). New call sites should follow this pattern.
 */
type TokenResponse = components['schemas']['TokenResponse'];

const API_BASE = getPublicApiBase();

interface FetchOptions extends RequestInit {
	skipAuth?: boolean;
}

class ApiClient {
	private baseUrl: string;
	/** Ensures parallel 401s share one refresh (avoids races invalidating the refresh token). */
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

		if (!skipAuth && auth.accessToken) {
			headers.set('Authorization', `Bearer ${auth.accessToken}`);
		}

		const response = await fetch(`${this.baseUrl}${endpoint}`, {
			...fetchOptions,
			headers
		});

		if (response.status === 401 && !skipAuth && auth.refreshToken) {
			const refreshed = await this.refreshTokens();
			if (refreshed) {
				headers.set('Authorization', `Bearer ${auth.accessToken}`);
				const retry = await fetch(`${this.baseUrl}${endpoint}`, {
					...fetchOptions,
					headers
				});
				if (!retry.ok) {
					const err = await retry.json().catch(() => ({ error: 'Request failed' }));
					throw new ApiError(retry.status, err.error || 'Request failed');
				}
				return retry.json();
			} else {
				auth.logout();
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
			const res = await fetch(`${this.baseUrl}/api/auth/refresh`, {
				method: 'POST',
				headers: { 'Content-Type': 'application/json' },
				body: JSON.stringify({ refresh_token: auth.refreshToken })
			});

			if (!res.ok) return false;

			const data: TokenResponse = await res.json();
			auth.setTokens(data.access_token, data.refresh_token);
			return true;
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
	 * artefact composes.
	 */
	async getBlob(
		endpoint: string,
		options?: FetchOptions
	): Promise<{ blob: Blob; filename: string | null }> {
		const { skipAuth, ...fetchOptions } = options ?? {};
		const headers = new Headers(fetchOptions.headers);
		if (!skipAuth && auth.accessToken) {
			headers.set('Authorization', `Bearer ${auth.accessToken}`);
		}
		let response = await fetch(`${this.baseUrl}${endpoint}`, {
			...fetchOptions,
			method: 'GET',
			headers
		});
		if (response.status === 401 && !skipAuth && auth.refreshToken) {
			const refreshed = await this.refreshTokens();
			if (!refreshed) {
				auth.logout();
				throw new ApiError(401, 'Session expired');
			}
			headers.set('Authorization', `Bearer ${auth.accessToken}`);
			response = await fetch(`${this.baseUrl}${endpoint}`, {
				...fetchOptions,
				method: 'GET',
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
