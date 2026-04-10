/**
 * Pure resolver for tests and build-time behavior (no Svelte imports).
 *
 * Architecture (repo evidence):
 * - Frontend: `@sveltejs/adapter-vercel` (svelte.config.js)
 * - API: separate host (e.g. Render `backend/render.yaml`, `FRONTEND_URL` → Vercel)
 *
 * Production browser bundles must not fall back to `localhost` — that breaks Vercel users.
 * Set `VITE_API_URL` to the public API origin in the Vercel project (Production + Preview as needed).
 */
export function resolvePublicApiBase(params: {
	viteApiUrl: string | undefined;
	dev: boolean;
	browser: boolean;
}): string {
	const raw = params.viteApiUrl;
	if (raw != null && String(raw).trim() !== '') {
		return String(raw).trim().replace(/\/$/, '');
	}
	if (params.dev) {
		return params.browser ? '' : 'http://127.0.0.1:3001';
	}
	// Production / preview build without VITE_API_URL: same-origin `/api/...` (only valid if you add edge rewrites).
	return '';
}
