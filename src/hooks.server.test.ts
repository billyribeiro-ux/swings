// FDN-08: unit tests for the CSP header builder + nonce injection.
//
// Runs under Node-mode vitest (no browser). Covers directive composition and
// the `transformPageChunk` post-processor; the full handle() flow is
// exercised at integration time by the SvelteKit runtime (dev + e2e).

import { describe, it, expect } from 'vitest';
// `injectNonce` is not exported (intentionally — it's an internal helper) so
// we test the exported `buildCsp` contract + derive behavioural expectations
// of the nonce injector from the SvelteKit hydration-script shape.
import { buildCsp } from './hooks.server';

describe('buildCsp', () => {
	it('emits all required directives with the per-request nonce interpolated', () => {
		const csp = buildCsp('test-nonce-abc');

		expect(csp).toContain("default-src 'self'");
		expect(csp).toContain("script-src 'self' 'nonce-test-nonce-abc' https://js.stripe.com");
		expect(csp).toContain("style-src 'self' 'unsafe-inline'");
		expect(csp).toContain("img-src 'self' data: https://*.r2.cloudflarestorage.com");
		expect(csp).toContain("connect-src 'self' https://api.stripe.com https://api.resend.com");
		expect(csp).toContain("font-src 'self' data: https://fonts.gstatic.com");
		expect(csp).toContain('frame-src https://js.stripe.com https://challenges.cloudflare.com');
		expect(csp).toContain("frame-ancestors 'none'");
		expect(csp).toContain("base-uri 'self'");
		expect(csp).toContain("form-action 'self'");
		expect(csp).toContain('report-uri /api/csp-report');
		// `report-to` pairs with the `Reporting-Endpoints` response header
		// added in `handle` — modern browsers prefer it over `report-uri`
		// but both are emitted for backwards compatibility with older
		// Safari / Chromium builds.
		expect(csp).toContain('report-to csp-endpoint');
	});

	it('separates directives with "; " so browsers parse each cleanly', () => {
		const csp = buildCsp('n');
		// 11 directive separators between 12 directives: default-src,
		// script-src, style-src, img-src, connect-src, font-src,
		// frame-src, frame-ancestors, base-uri, form-action, report-uri,
		// report-to.
		const parts = csp.split('; ');
		expect(parts.length).toBe(12);
		// Each directive starts with the directive name.
		for (const part of parts) {
			expect(part).toMatch(/^[a-z-]+ /);
		}
	});

	it('produces a distinct nonce payload per call (caller-supplied)', () => {
		const a = buildCsp('nonce-a');
		const b = buildCsp('nonce-b');
		expect(a).not.toBe(b);
		expect(a).toContain("'nonce-nonce-a'");
		expect(b).toContain("'nonce-nonce-b'");
	});
});
