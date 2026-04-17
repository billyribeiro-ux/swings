import type { Handle, HandleServerError } from '@sveltejs/kit';

/**
 * FDN-08: Per-request nonce for the Content Security Policy.
 *
 * 16 cryptographically random bytes → base64 (no padding). The token is:
 *   - emitted in the `Content-Security-Policy` header (`script-src 'nonce-…'`)
 *   - substituted into `src/app.html` in place of the `%swings.nonce%`
 *     placeholder via `resolve(..., { transformPageChunk })`
 *
 * Using 16 bytes comfortably exceeds the 128-bit unpredictability floor the
 * CSP spec recommends for nonces.
 */
function generateNonce(): string {
	const bytes = new Uint8Array(16);
	crypto.getRandomValues(bytes);
	// btoa expects a binary string; build one char-at-a-time (cheaper than
	// spreading into an array for a 16-byte input).
	let bin = '';
	for (const b of bytes) bin += String.fromCharCode(b);
	return btoa(bin).replace(/=+$/, '');
}

/**
 * FDN-08: Content Security Policy spec — §12 of AUDIT_PHASE3_PLAN.md.
 *
 * Exported so unit tests can assert the directives without reimplementing
 * the concat logic. The nonce is injected at callsites.
 */
export function buildCsp(nonce: string): string {
	const directives: Record<string, readonly string[]> = {
		'default-src': ["'self'"],
		'script-src': [
			"'self'",
			`'nonce-${nonce}'`,
			'https://js.stripe.com',
			'https://challenges.cloudflare.com'
		],
		// `'unsafe-inline'` is retained for styles per the plan's temporary
		// allowance (Svelte transitions emit inline <style> fragments and the
		// Google Fonts stylesheet also ships inline @font-face). Google Fonts
		// origins are explicit so we don't have to drop `'self'`.
		'style-src': ["'self'", "'unsafe-inline'", 'https://fonts.googleapis.com'],
		'img-src': ["'self'", 'data:', 'https://*.r2.cloudflarestorage.com', 'https://*.r2.dev'],
		'connect-src': ["'self'", 'https://api.stripe.com', 'https://api.resend.com'],
		'font-src': ["'self'", 'data:', 'https://fonts.gstatic.com'],
		'frame-src': ['https://js.stripe.com', 'https://challenges.cloudflare.com'],
		'frame-ancestors': ["'none'"],
		'base-uri': ["'self'"],
		'form-action': ["'self'"],
		'report-uri': ['/api/csp-report']
	};
	return Object.entries(directives)
		.map(([k, v]) => `${k} ${v.join(' ')}`)
		.join('; ');
}

const NONCE_PLACEHOLDER = '%swings.nonce%';

// Match `<script …>` opening tags that don't already carry a `nonce=` attribute.
// Capture group 1 is the existing attribute string (may be empty) so we can
// append the nonce without breaking ordering. Case-insensitive so the pattern
// survives upstream tooling that may uppercase tag names.
const SCRIPT_OPEN_WITHOUT_NONCE = /<script((?:\s+[^>]*)?)>/gi;

/**
 * FDN-08: ensure every `<script>` tag in the rendered HTML carries our
 * per-request nonce. Covers the inline hydration scripts SvelteKit emits for
 * client/runtime bootstrap — they would otherwise be blocked by a strict CSP
 * that has no `'unsafe-inline'` in `script-src`.
 */
function injectNonce(html: string, nonce: string): string {
	const withPlaceholder = html.includes(NONCE_PLACEHOLDER)
		? html.replaceAll(NONCE_PLACEHOLDER, nonce)
		: html;
	return withPlaceholder.replace(SCRIPT_OPEN_WITHOUT_NONCE, (match, attrs) => {
		if (/\snonce\s*=/i.test(attrs)) return match;
		return `<script${attrs} nonce="${nonce}">`;
	});
}

export const handle: Handle = async ({ event, resolve }) => {
	const nonce = generateNonce();

	const response = await resolve(event, {
		// `js` is preloaded as modulepreload by SvelteKit by default; we only need
		// to opt fonts and CSS in explicitly.
		preload: ({ type }) => type === 'font' || type === 'css',
		// Swap our `%swings.nonce%` placeholder in `src/app.html` with the
		// per-request nonce, then pepper the same nonce onto any remaining
		// `<script>` tags SvelteKit emits for hydration/module bootstrap.
		// (SvelteKit's own `%sveltekit.nonce%` is only generated when
		// `kit.csp` is configured; we roll our own CSP here.)
		transformPageChunk: ({ html }) => injectNonce(html, nonce)
	});

	// Pre-existing security headers (preserved).
	response.headers.set('X-Frame-Options', 'SAMEORIGIN');
	response.headers.set('X-Content-Type-Options', 'nosniff');
	response.headers.set('Referrer-Policy', 'strict-origin-when-cross-origin');
	response.headers.set(
		'Permissions-Policy',
		'camera=(), microphone=(), geolocation=(), interest-cohort=()'
	);

	// FDN-08: CSP + transport + cross-origin isolation hardening.
	response.headers.set('Content-Security-Policy', buildCsp(nonce));
	response.headers.set(
		'Strict-Transport-Security',
		'max-age=63072000; includeSubDomains; preload'
	);
	response.headers.set('Cross-Origin-Opener-Policy', 'same-origin');
	response.headers.set('Cross-Origin-Resource-Policy', 'same-site');

	// Cache immutable assets (Vite hashed files)
	const { pathname } = event.url;
	if (pathname.startsWith('/_app/immutable/')) {
		response.headers.set('Cache-Control', 'public, max-age=31536000, immutable');
	}

	return response;
};

export const handleError: HandleServerError = ({ error, event, status, message }) => {
	const errorId = crypto.randomUUID();
	console.error(`[server-error ${errorId}]`, status, message, event.url.pathname, error);
	return {
		message: status >= 500 ? 'An unexpected error occurred. Please try again.' : message,
		id: errorId
	};
};
