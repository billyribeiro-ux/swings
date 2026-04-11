/**
 * Service worker policy for local development.
 *
 * Production builds always register the worker (see `svelte.config.js`). During `vite dev` we
 * default to **not** registering, because stale workers + HMR cause FetchEvent / cache bugs that
 * look like app bugs.
 *
 * To test the real SW while running `pnpm dev`, set `PUBLIC_SERVICE_WORKER_IN_DEV=1` in `.env`
 * (must match `svelte.config.js` so Kit injects registration and we skip unregister here).
 */

export type ServiceWorkerDevPolicyInput = {
	dev: boolean;
	/** True when `PUBLIC_SERVICE_WORKER_IN_DEV` is set — intentional SW testing in dev. */
	optedIntoServiceWorkerInDev: boolean;
};

/**
 * Unregisters every service worker for this origin in dev, unless the team opted in to SW
 * testing. Idempotent; safe to run on every client boot.
 */
export function applyServiceWorkerDevPolicy(input: ServiceWorkerDevPolicyInput): void {
	if (!input.dev || input.optedIntoServiceWorkerInDev) return;
	if (typeof navigator === 'undefined' || !('serviceWorker' in navigator)) return;

	void navigator.serviceWorker
		.getRegistrations()
		.then((registrations) => Promise.all(registrations.map((r) => r.unregister())));
}

/** Pure helper for tests — mirrors the boolean gate above. */
export function shouldUnregisterServiceWorkersInDev(input: ServiceWorkerDevPolicyInput): boolean {
	return input.dev && !input.optedIntoServiceWorkerInDev;
}
