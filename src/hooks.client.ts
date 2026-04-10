import { dev } from '$app/environment';
import { applyServiceWorkerDevPolicy } from '$lib/client/service-worker-dev-policy';

/**
 * Client bootstrap (runs once per app load in the browser).
 * @see https://svelte.dev/docs/kit/hooks
 */
applyServiceWorkerDevPolicy({
	dev,
	optedIntoServiceWorkerInDev:
		import.meta.env.PUBLIC_SERVICE_WORKER_IN_DEV === '1' ||
		import.meta.env.PUBLIC_SERVICE_WORKER_IN_DEV === 'true'
});
