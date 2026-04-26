import process from 'node:process';
import adapter from '@sveltejs/adapter-vercel';

/**
 * Service worker registration policy (must stay aligned with `src/hooks.client.ts` + `$lib/client/service-worker-dev-policy`).
 * @see https://svelte.dev/docs/kit/configuration#serviceWorker
 */
const allowServiceWorkerInDev =
	process.env.PUBLIC_SERVICE_WORKER_IN_DEV === '1' ||
	process.env.PUBLIC_SERVICE_WORKER_IN_DEV === 'true';

const registerServiceWorker = process.env.NODE_ENV === 'production' || allowServiceWorkerInDev;

/** @type {import('@sveltejs/kit').Config} */
const config = {
	kit: {
		adapter: adapter({
			runtime: 'nodejs22.x'
		}),
		serviceWorker: {
			register: registerServiceWorker
		},
		prerender: {
			handleHttpError: 'warn',
			handleMissingId: 'warn',
			crawl: true,
			entries: [
				'/',
				'/about',
				'/courses',
				'/blog',
				'/pricing',
				'/pricing/monthly',
				'/pricing/annual'
			]
		},
		// SvelteKit 2.27+ experimental: type-safe server↔client RPC via
		// `.remote.ts` files exporting `query` / `form` / `command` / `prerender`
		// helpers from `$app/server`. Still flagged experimental upstream; opt-in
		// is required. See https://svelte.dev/docs/kit/remote-functions
		experimental: {
			remoteFunctions: true
		}
	},
	// Paired with the Kit flag: enables top-level `await` inside `<script>` and
	// markup `{#await}` shortcut via bare `await` expressions (e.g.
	// `{await getPosts()}`) resolved during SSR. Required for remote query
	// consumption from `.svelte` files.
	compilerOptions: {
		experimental: {
			async: true
		}
	}
};

export default config;
