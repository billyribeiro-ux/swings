import path from 'node:path';
import { fileURLToPath } from 'node:url';

import devtoolsJson from 'vite-plugin-devtools-json';
import { defineConfig } from 'vitest/config';
import { sveltekit } from '@sveltejs/kit/vite';

const repoRoot = path.dirname(fileURLToPath(import.meta.url));

export default defineConfig({
	plugins: [sveltekit(), devtoolsJson()],
	build: {
		// Large, intentional vendor chunks in this app make Vite's default 500k warning too noisy.
		chunkSizeWarningLimit: 10000
	},
	server: {
		// i18n catalogues live at repo root (`messages/*.json`) and are imported from
		// `src/lib/i18n/paraglide.ts`. SvelteKit’s dev allow-list otherwise excludes
		// that tree — Vite refuses to read the files and the browser sees `/messages/*.json` 404s.
		fs: {
			allow: [path.join(repoRoot, 'messages')]
		},
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
