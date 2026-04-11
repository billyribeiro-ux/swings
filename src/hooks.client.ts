import { dev } from '$app/environment';
import type { HandleClientError } from '@sveltejs/kit';
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

export const handleError: HandleClientError = ({ error, event, status, message }) => {
	const errorId = crypto.randomUUID();
	console.error(`[client-error ${errorId}]`, status, message, event.url.pathname, error);
	return {
		message: status >= 500 ? 'An unexpected error occurred. Please try again.' : message,
		id: errorId
	};
};
