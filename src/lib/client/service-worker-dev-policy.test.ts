import { describe, it, expect } from 'vitest';
import { shouldUnregisterServiceWorkersInDev } from './service-worker-dev-policy';

describe('shouldUnregisterServiceWorkersInDev', () => {
	it('is false in production', () => {
		expect(
			shouldUnregisterServiceWorkersInDev({
				dev: false,
				optedIntoServiceWorkerInDev: false
			})
		).toBe(false);
	});

	it('is false when opted into SW in dev', () => {
		expect(
			shouldUnregisterServiceWorkersInDev({
				dev: true,
				optedIntoServiceWorkerInDev: true
			})
		).toBe(false);
	});

	it('is true in dev without opt-in', () => {
		expect(
			shouldUnregisterServiceWorkersInDev({
				dev: true,
				optedIntoServiceWorkerInDev: false
			})
		).toBe(true);
	});
});
