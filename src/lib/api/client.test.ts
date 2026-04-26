/**
 * `lib/api/client.ts` — Vitest unit coverage.
 *
 * Scope:
 *   - request() happy path: parses JSON, includes credentials.
 *   - 401 → refresh → retry: refresh succeeds, original retry returns body.
 *   - 401 → refresh fails: throws ApiError(401), calls auth.logout.
 *   - parseApiError (RFC 7807 — covers Phase 4.1 fix): prefers `detail`,
 *     falls back to `title`, then to legacy `error`/`message`, else default.
 *   - getBlob: returns `{ blob, filename }` parsed from Content-Disposition.
 *   - Auto Idempotency-Key: injected for non-GET admin requests, NOT for GETs,
 *     NOT for non-admin endpoints, and NOT overridden when caller supplies one.
 *
 * Strategy: stub `globalThis.fetch` per test. The auth store's `logout()` is
 * spied to assert the 401-final branch.
 */
import { describe, expect, it, beforeEach, afterEach, vi } from 'vitest';

// Mock `$lib/stores/auth.svelte` (used by `client.ts`) BEFORE importing the
// client so the module-load `import { auth }` line picks up our stub.
vi.mock('$lib/stores/auth.svelte', () => ({
	auth: { logout: vi.fn(async () => undefined) }
}));

// `getPublicApiBase` reads `import.meta.env`; lock it to an empty string so
// fetch URLs are deterministic in assertions.
vi.mock('$lib/api/publicApiBase', () => ({
	getPublicApiBase: () => ''
}));

// Re-import the dependencies after mocks are registered.
import { api, ApiError } from './client';
import { auth } from '$lib/stores/auth.svelte';

type FetchArgs = [input: RequestInfo | URL, init?: RequestInit];

function jsonResponse(body: unknown, init: ResponseInit = {}): Response {
	return new Response(JSON.stringify(body), {
		status: 200,
		headers: { 'content-type': 'application/json' },
		...init
	});
}

function problemResponse(status: number, body: unknown): Response {
	return new Response(JSON.stringify(body), {
		status,
		headers: { 'content-type': 'application/problem+json' }
	});
}

let fetchSpy: ReturnType<typeof vi.fn>;

beforeEach(() => {
	fetchSpy = vi.fn();
	globalThis.fetch = fetchSpy as unknown as typeof fetch;
	(auth.logout as unknown as ReturnType<typeof vi.fn>).mockClear();
});

afterEach(() => {
	vi.restoreAllMocks();
});

describe('ApiClient.request', () => {
	it('GET parses JSON body and includes credentials', async () => {
		fetchSpy.mockResolvedValueOnce(jsonResponse({ hello: 'world' }));
		const result = await api.get<{ hello: string }>('/api/health');
		expect(result.hello).toBe('world');
		const [, init] = fetchSpy.mock.calls[0] as FetchArgs;
		expect(init?.credentials).toBe('include');
		expect(init?.method).toBe('GET');
	});

	it('attaches Content-Type for JSON bodies on POST', async () => {
		fetchSpy.mockResolvedValueOnce(jsonResponse({ ok: true }));
		await api.post('/api/auth/login', { email: 'x@y', password: 'p' });
		const [, init] = fetchSpy.mock.calls[0] as FetchArgs;
		const headers = new Headers(init?.headers);
		expect(headers.get('Content-Type')).toBe('application/json');
	});

	it('does NOT inject Idempotency-Key on GET requests', async () => {
		fetchSpy.mockResolvedValueOnce(jsonResponse([]));
		await api.get('/api/admin/coupons');
		const [, init] = fetchSpy.mock.calls[0] as FetchArgs;
		const headers = new Headers(init?.headers);
		expect(headers.get('Idempotency-Key')).toBeNull();
	});

	it('does NOT inject Idempotency-Key on non-admin POSTs', async () => {
		fetchSpy.mockResolvedValueOnce(jsonResponse({ ok: true }));
		await api.post('/api/auth/login', { email: 'a', password: 'b' });
		const [, init] = fetchSpy.mock.calls[0] as FetchArgs;
		const headers = new Headers(init?.headers);
		expect(headers.get('Idempotency-Key')).toBeNull();
	});

	it('auto-injects Idempotency-Key on /api/admin/* mutating calls', async () => {
		fetchSpy.mockResolvedValueOnce(jsonResponse({ ok: true }));
		await api.post('/api/admin/coupons', { code: 'X' });
		const [, init] = fetchSpy.mock.calls[0] as FetchArgs;
		const headers = new Headers(init?.headers);
		const key = headers.get('Idempotency-Key');
		expect(key).toBeTruthy();
		// UUID-ish — at least 16 chars and contains a hyphen.
		expect(key!.length).toBeGreaterThanOrEqual(16);
	});

	it('caller-supplied Idempotency-Key wins over auto-injected one', async () => {
		fetchSpy.mockResolvedValueOnce(jsonResponse({ ok: true }));
		await api.post(
			'/api/admin/refunds',
			{ amount: 100 },
			{
				headers: { 'Idempotency-Key': 'my-stable-key' }
			}
		);
		const [, init] = fetchSpy.mock.calls[0] as FetchArgs;
		const headers = new Headers(init?.headers);
		expect(headers.get('Idempotency-Key')).toBe('my-stable-key');
	});

	it('on 401 → refresh ok → retry returns body', async () => {
		fetchSpy
			.mockResolvedValueOnce(new Response('', { status: 401 })) // initial
			.mockResolvedValueOnce(jsonResponse({ refreshed: true })) // refresh
			.mockResolvedValueOnce(jsonResponse({ retried: 'value' })); // retry

		const result = await api.get<{ retried: string }>('/api/me');
		expect(result.retried).toBe('value');
		expect(fetchSpy).toHaveBeenCalledTimes(3);
		// Refresh fired against /api/auth/refresh.
		const [refreshUrl] = fetchSpy.mock.calls[1] as FetchArgs;
		expect(String(refreshUrl)).toContain('/api/auth/refresh');
	});

	it('on 401 → refresh fails → throws ApiError(401) and calls logout', async () => {
		fetchSpy
			.mockResolvedValueOnce(new Response('', { status: 401 })) // initial
			.mockResolvedValueOnce(new Response('', { status: 401 })); // refresh fails

		await expect(api.get('/api/me')).rejects.toMatchObject({
			status: 401,
			message: 'Session expired'
		});
		expect(auth.logout).toHaveBeenCalledOnce();
	});
});

describe('parseApiError (RFC 7807 / Phase 4.1)', () => {
	it('prefers `detail` over `title`/`error`', async () => {
		fetchSpy.mockResolvedValueOnce(
			problemResponse(422, {
				type: 'about:blank',
				title: 'Validation Failed',
				status: 422,
				detail: 'Email is invalid',
				error: 'Should not show'
			})
		);
		await expect(api.post('/api/auth/login', { email: 'bad' })).rejects.toMatchObject({
			status: 422,
			message: 'Email is invalid'
		});
	});

	it('falls back to `title` when `detail` is missing', async () => {
		fetchSpy.mockResolvedValueOnce(
			problemResponse(500, { type: 'about:blank', title: 'Internal Error', status: 500 })
		);
		await expect(api.get('/api/health')).rejects.toMatchObject({
			status: 500,
			message: 'Internal Error'
		});
	});

	it('falls back to legacy `error` field', async () => {
		fetchSpy.mockResolvedValueOnce(
			new Response(JSON.stringify({ error: 'Legacy boom' }), {
				status: 400,
				headers: { 'content-type': 'application/json' }
			})
		);
		await expect(api.get('/api/legacy')).rejects.toMatchObject({
			status: 400,
			message: 'Legacy boom'
		});
	});

	it('falls back to default when body is not JSON', async () => {
		fetchSpy.mockResolvedValueOnce(
			new Response('not json', {
				status: 503,
				headers: { 'content-type': 'text/plain' }
			})
		);
		await expect(api.get('/api/oops')).rejects.toMatchObject({
			status: 503,
			message: 'Request failed'
		});
	});

	it('exposes RFC 7807 errors map via ApiError.details', async () => {
		fetchSpy.mockResolvedValueOnce(
			problemResponse(422, {
				type: 'about:blank',
				title: 'Validation Failed',
				status: 422,
				detail: 'See errors',
				errors: { email: ['must be valid'] }
			})
		);
		try {
			await api.post('/api/x', {});
			throw new Error('should have thrown');
		} catch (e) {
			expect(e).toBeInstanceOf(ApiError);
			const err = e as ApiError;
			expect(err.details?.errors).toEqual({ email: ['must be valid'] });
		}
	});
});

describe('ApiClient.getBlob', () => {
	it('returns the blob and filename parsed from Content-Disposition', async () => {
		const blob = new Blob(['csv,data'], { type: 'text/csv' });
		fetchSpy.mockResolvedValueOnce(
			new Response(blob, {
				status: 200,
				headers: {
					'content-type': 'text/csv',
					'content-disposition': 'attachment; filename="export.csv"'
				}
			})
		);
		const result = await api.getBlob('/api/admin/audit/export.csv');
		expect(result.filename).toBe('export.csv');
		expect(result.blob).toBeInstanceOf(Blob);
	});

	it('handles RFC 5987 UTF-8 filename', async () => {
		fetchSpy.mockResolvedValueOnce(
			new Response('', {
				status: 200,
				headers: {
					'content-disposition': "attachment; filename*=UTF-8''r%C3%A9sum%C3%A9.pdf"
				}
			})
		);
		const result = await api.getBlob('/api/admin/dsar/jobs/x/artifact');
		expect(result.filename).toBe('résumé.pdf');
	});

	it('returns null filename when no Content-Disposition is present', async () => {
		fetchSpy.mockResolvedValueOnce(new Response('', { status: 200 }));
		const result = await api.getBlob('/api/x');
		expect(result.filename).toBeNull();
	});

	it('throws on non-OK and surfaces the parsed error', async () => {
		fetchSpy.mockResolvedValueOnce(
			problemResponse(404, {
				type: 'about:blank',
				title: 'Not Found',
				status: 404,
				detail: 'Artifact missing'
			})
		);
		await expect(api.getBlob('/api/missing')).rejects.toMatchObject({
			status: 404,
			message: 'Artifact missing'
		});
	});
});
