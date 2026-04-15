import { expect, test } from '@playwright/test';

test('homepage loads with brand copy', async ({ page }) => {
	await page.goto('/');
	await expect(page).toHaveTitle(/Precision Options Signals/i);
	await expect(page.getByText('Precision', { exact: false }).first()).toBeVisible();
});

test('pricing page is reachable', async ({ page }) => {
	await page.goto('/pricing');
	await expect(page).toHaveURL(/\/pricing/);
	await expect(page.getByText('Precision', { exact: false }).first()).toBeVisible();
});

test('login page renders sign-in form', async ({ page }) => {
	await page.goto('/login');
	await expect(page.getByRole('heading', { level: 1, name: /Welcome Back/i })).toBeVisible();
	await expect(page.getByLabel('Email')).toBeVisible();
	await expect(page.getByLabel('Password')).toBeVisible();
});

test('admin route requires authentication', async ({ page }) => {
	await page.goto('/admin');
	await expect(page.getByRole('heading', { level: 1, name: /Admin Login/i })).toBeVisible();
});
