/**
 * Shared pure helpers for E2E tests.
 *
 * Keep this file free of Playwright-specific imports so it can be consumed from
 * fixtures, page objects, and tests without creating accidental cycles.
 */

/**
 * Generate a disposable email address guaranteed unique across parallel workers
 * and retry attempts. `crypto.randomUUID` is available in Node 24+.
 */
export function disposableEmail(prefix = 'e2e'): string {
	const id = crypto.randomUUID().replace(/-/g, '').slice(0, 12);
	return `${prefix}+${id}@example.test`;
}

/**
 * Generate a password that satisfies the FDN-01 validator constraints
 * (>= 8 chars). We append randomness so that repeated runs don't share one
 * value — useful when we later want to assert token rotation.
 */
export function disposablePassword(): string {
	const id = crypto.randomUUID().replace(/-/g, '').slice(0, 16);
	return `Aa1!${id}`;
}

/**
 * Expected RFC 7807 Problem Details shape. Handlers return this as
 * `Content-Type: application/problem+json` per FDN-01.
 */
export interface ProblemDocument {
	type: string;
	title: string;
	status: number;
	detail: string;
	instance?: string;
	correlation_id?: string;
}

/**
 * Type guard: the body looks like an RFC 7807 Problem. We only check the
 * required fields; extension fields are permitted.
 */
export function isProblem(value: unknown): value is ProblemDocument {
	if (!value || typeof value !== 'object') return false;
	const v = value as Record<string, unknown>;
	return (
		typeof v.type === 'string' &&
		typeof v.title === 'string' &&
		typeof v.status === 'number' &&
		typeof v.detail === 'string'
	);
}

/**
 * Resolve the backend API origin used for E2E seeding / assertions.
 *
 * In local dev the Rust API listens on :3001 while the SvelteKit preview
 * server terminates on :4173 and proxies `/api` only in dev mode. For E2E
 * preview runs we talk to the backend directly.
 */
export function apiBaseUrl(): string {
	return process.env.VITE_API_URL ?? process.env.API_BASE_URL ?? 'http://127.0.0.1:3001';
}
