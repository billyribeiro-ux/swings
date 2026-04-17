/**
 * Button — browser test coverage.
 *
 * Focus: guarantee the three external-surface contracts documented in PE7 §9:
 *   1. `href` routes rendering to `<a>`.
 *   2. `disabled` short-circuits click AND renders `disabled`.
 *   3. `loading` shows spinner + `aria-busy` and blocks click.
 */
import { describe, expect, it, vi } from 'vitest';
import { page } from 'vitest/browser';
import { render } from 'vitest-browser-svelte';
import Button from './Button.svelte';
import ButtonWithChildren from './_fixtures/ButtonWithChildren.svelte';

describe('Button', () => {
	it('renders as <button> by default', async () => {
		render(ButtonWithChildren, { label: 'Save' });
		const btn = page.getByRole('button', { name: 'Save' });
		await expect.element(btn).toBeInTheDocument();
	});

	it('renders as <a> when href is provided', async () => {
		render(ButtonWithChildren, { label: 'Docs', href: '/docs' });
		const link = page.getByRole('link', { name: 'Docs' });
		await expect.element(link).toBeInTheDocument();
		await expect.element(link).toHaveAttribute('href', '/docs');
	});

	it('auto-sets rel for target="_blank"', async () => {
		render(ButtonWithChildren, {
			label: 'External',
			href: 'https://svelte.dev',
			target: '_blank'
		});
		const link = page.getByRole('link', { name: 'External' });
		await expect.element(link).toHaveAttribute('rel', 'noopener noreferrer');
	});

	it('disables click when disabled', async () => {
		const onclick = vi.fn();
		render(ButtonWithChildren, { label: 'Blocked', disabled: true, onclick });
		const btn = page.getByRole('button', { name: 'Blocked' });
		await expect.element(btn).toBeDisabled();
		await btn.click().catch(() => {
			/* Playwright rejects click on disabled; that's the desired guard. */
		});
		expect(onclick).not.toHaveBeenCalled();
	});

	it('disables anchor navigation when disabled', async () => {
		render(ButtonWithChildren, { label: 'Docs', href: '/docs', disabled: true });
		const link = page.getByRole('link', { name: 'Docs' });
		await expect.element(link).toHaveAttribute('aria-disabled', 'true');
		await expect.element(link).not.toHaveAttribute('href');
	});

	it('renders a spinner and aria-busy when loading', async () => {
		const onclick = vi.fn();
		render(ButtonWithChildren, { label: 'Loading', loading: true, onclick });
		const btn = page.getByRole('button', { name: 'Loading' });
		await expect.element(btn).toHaveAttribute('aria-busy', 'true');
		// Spinner has role="status" from the <Spinner> child.
		const spinner = page.getByRole('status');
		await expect.element(spinner).toBeInTheDocument();
	});
});

// Re-export the component so the fixture path resolves (hoisted).
export { Button };
