import { browser } from '$app/environment';
import { resolvePublicApiBase } from '$lib/api/resolvePublicApiBase';

/**
 * Base URL for the Rust HTTP API (no trailing slash).
 *
 * - **Dev (browser):** `""` — Vite proxies `/api` → `vite.config.ts` target.
 * - **Dev (SSR):** `http://127.0.0.1:3001` — Node cannot use the browser proxy.
 * - **Production browser:** same-origin `/api` (Vercel rewrites to the Rust API). No CORS for page navigations.
 * - **Production SSR / build:** set `VITE_API_URL` so server-side loaders can reach the API host directly.
 */
export function getPublicApiBase(): string {
	return resolvePublicApiBase({
		viteApiUrl: import.meta.env.VITE_API_URL,
		dev: import.meta.env.DEV,
		browser
	});
}
