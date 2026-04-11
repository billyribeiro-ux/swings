import devtoolsJson from 'vite-plugin-devtools-json';
import { defineConfig } from 'vite';
import { sveltekit } from '@sveltejs/kit/vite';

export default defineConfig({
	plugins: [sveltekit(), devtoolsJson()],
	server: {
		// Stable port so backend `FRONTEND_URL` / `PUBLIC_APP_URL` can match (see `.env.example`).
		port: 5180,
		strictPort: true,
		proxy: {
			// Rust API (pnpm dev + cargo run in backend). Browser uses same-origin /api via getPublicApiBase().
			'/api': {
				target: 'http://127.0.0.1:3001',
				changeOrigin: true
			}
		}
	}
});
