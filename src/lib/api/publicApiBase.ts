import { browser } from '$app/environment';
import { resolvePublicApiBase } from '$lib/api/resolvePublicApiBase';

/**
 * Base URL for the Rust HTTP API (no trailing slash).
 *
 * - **Dev (browser):** `""` — Vite proxies `/api` → `vite.config.ts` target.
 * - **Dev (SSR):** `http://127.0.0.1:3001` — Node cannot use the browser proxy.
 * - **Production (Vercel + API on Render, etc.):** set `VITE_API_URL` at build time to the API origin.
 *   Do not rely on localhost; this app is not same-origin with the API unless you add rewrites.
 */
export function getPublicApiBase(): string {
	return resolvePublicApiBase({
		viteApiUrl: import.meta.env.VITE_API_URL,
		dev: import.meta.env.DEV,
		browser
	});
}
