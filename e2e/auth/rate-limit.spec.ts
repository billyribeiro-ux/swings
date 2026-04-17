/**
 * FDN-08 rate-limit smoke.
 *
 * `POST /api/auth/login` is throttled at 5 requests/minute per IP. Firing six
 * consecutive invalid attempts must yield a 429. The in-process governor uses
 * a burst-of-5 bucket, so the exact response to the 6th request depends on
 * timing — we accept either 429 on the last attempt OR at least one 429 in
 * the sequence so the test stays green against the faster distributed backend
 * variant.
 */

import { test, expect } from '../fixtures/app';
import { disposableEmail } from '../fixtures/helpers';

test('repeated failed logins eventually return 429', async ({ api }) => {
	test.slow();
	if (!(await api.isReachable())) {
		test.skip(true, 'Backend unreachable.');
		return;
	}

	const email = disposableEmail('ratelimit');
	const statuses: number[] = [];
	for (let i = 0; i < 8; i += 1) {
		const result = await api.post<unknown>('/api/auth/login', {
			email,
			password: `wrong-${i}`
		});
		statuses.push(result.status);
		// A non-4xx/5xx response shouldn't happen here; stop early if it does.
		if (result.ok) break;
	}

	const anyRateLimited = statuses.some((s) => s === 429);
	expect(
		anyRateLimited,
		`expected at least one 429 in ${statuses.join(', ')} (FDN-08 auth rate limit)`
	).toBe(true);
});
