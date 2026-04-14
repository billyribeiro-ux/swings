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
			entries: ['/', '/about', '/courses', '/blog', '/pricing', '/pricing/monthly', '/pricing/annual']
		}
	}
};

export default config;
