import { auth } from '$lib/stores/auth.svelte';
import { getPublicApiBase } from '$lib/api/publicApiBase';

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

			const data: { access_token: string; refresh_token: string } = await res.json();
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
}

export class ApiError extends Error {
	status: number;

	constructor(status: number, message: string) {
		super(message);
		this.status = status;
	}
}

export const api = new ApiClient(API_BASE);
