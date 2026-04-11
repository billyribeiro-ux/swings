import devtoolsJson from 'vite-plugin-devtools-json';
import { defineConfig } from 'vitest/config';
import { sveltekit } from '@sveltejs/kit/vite';

export default defineConfig({
	plugins: [sveltekit(), devtoolsJson()],
	server: {
		proxy: {
			// Rust API (pnpm dev + cargo run in backend). Browser uses same-origin /api via getPublicApiBase().
			'/api': {
				target: 'http://127.0.0.1:3001',
				changeOrigin: true
			}
		}
	},
	// Vitest 4: files using `vitest/browser` must run in browser mode (`pnpm exec playwright install` + dedicated project).
	// Default `pnpm test:unit` runs Node tests only; e2e/ is Playwright (`pnpm exec playwright test`).
	test: {
		include: ['src/**/*.{test,spec}.ts'],
		exclude: ['src/**/*.svelte.spec.ts']
	}
});
