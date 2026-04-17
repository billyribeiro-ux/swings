/**
 * Login with an unknown user returns HTTP 401 and an RFC 7807 Problem body
 * (`type: /problems/unauthorized`, `title: "Unauthorized"`). We assert both
 * the HTTP surface + the rendered inline error copy.
 */

import { test, expect } from '../fixtures/app';
import { LoginPage } from '../pages/LoginPage';
import { disposableEmail } from '../fixtures/helpers';

test('login with wrong password surfaces RFC 7807 401', async ({ page, api }) => {
	if (!(await api.isReachable())) {
		test.skip(true, 'Backend unreachable.');
		return;
	}

	const login = new LoginPage(page);
	await login.open();
	await login.enterCredentials(disposableEmail('noone'), 'not-the-real-password');
	const response = await login.submit();
	const problem = await login.expectProblem(response, 401, '/unauthorized');
	expect(problem.title).toBe('Unauthorized');

	// The UI surfaces a "Invalid email or password" message.
	await expect(page.getByText(/invalid email or password/i)).toBeVisible();
});
