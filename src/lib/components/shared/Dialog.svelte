<!--
  Dialog — modal with backdrop, focus trap, ESC to close.

  A11y:
  - `role="dialog"` + `aria-modal="true"`.
  - `aria-labelledby` / `aria-describedby` wired to generated ids.
  - Focus trap via `{@attach}`: first focusable element in the panel takes
    focus on open; Tab / Shift+Tab cycle inside; Escape closes.
  - Rest of document receives `inert` attribute while open (keeps background
    content out of the focus order and reading order for AT).
  - Reduced motion: backdrop fade / panel slide disabled via media query.
  - Backdrop click closes by default (`closeOnBackdrop = true`).
-->
<script lang="ts" module>
	export type { DialogProps, DialogSize } from './Dialog.types';

	const FOCUSABLE_SELECTOR = [
		'a[href]',
		'button:not([disabled])',
		'input:not([disabled])',
		'select:not([disabled])',
		'textarea:not([disabled])',
		'[tabindex]:not([tabindex="-1"])',
		'[contenteditable="true"]'
	].join(', ');

	let idCounter = 0;
	function nextId(prefix: string): string {
		idCounter += 1;
		return `${prefix}-${idCounter}`;
	}
</script>

<script lang="ts">
	import type { Attachment } from 'svelte/attachments';
	import type { DialogProps as Props } from './Dialog.types';

	let {
		open = $bindable(false),
		onclose,
		title,
		description,
		size = 'md',
		closeOnBackdrop = true,
		closeOnEscape = true,
		children,
		footer
	}: Props = $props();

	const titleId = nextId('dialog-title');
	const descId = nextId('dialog-desc');

	function requestClose() {
		if (!open) return;
		open = false;
		onclose?.();
	}

	/**
	 * Focus-trap attachment: runs after the panel is mounted.
	 * - Marks siblings of the dialog container as `inert` (restores on cleanup).
	 * - Puts focus on the first focusable descendant.
	 * - Wraps Tab / Shift+Tab inside the panel.
	 * - Listens for Escape at document level (capture) while mounted.
	 * - Restores focus to the previously-focused element on unmount.
	 */
	const focusTrap: Attachment<HTMLElement> = (node) => {
		const previouslyFocused = document.activeElement as HTMLElement | null;

		const host = node.parentElement; // .dialog-root
		const inertTargets: Element[] = [];
		if (host?.parentElement) {
			for (const sibling of host.parentElement.children) {
				if (sibling !== host && !sibling.hasAttribute('inert')) {
					sibling.setAttribute('inert', '');
					inertTargets.push(sibling);
				}
			}
		}

		function focusables(): HTMLElement[] {
			return Array.from(node.querySelectorAll<HTMLElement>(FOCUSABLE_SELECTOR)).filter(
				(el) =>
					!el.hasAttribute('disabled') && el.tabIndex !== -1 && el.offsetParent !== null
			);
		}

		// Defer focus to next frame so the panel transition doesn't steal it.
		const rafId = requestAnimationFrame(() => {
			const list = focusables();
			const head = list[0];
			if (head) {
				head.focus();
			} else {
				node.focus();
			}
		});

		function handleKey(e: KeyboardEvent) {
			if (e.key === 'Escape' && closeOnEscape) {
				e.preventDefault();
				requestClose();
				return;
			}
			if (e.key !== 'Tab') return;

			const list = focusables();
			if (list.length === 0) {
				e.preventDefault();
				node.focus();
				return;
			}
			const first = list[0]!;
			const last = list[list.length - 1]!;
			const active = document.activeElement as HTMLElement | null;

			if (e.shiftKey) {
				if (active === first || !node.contains(active)) {
					e.preventDefault();
					last.focus();
				}
			} else {
				if (active === last || !node.contains(active)) {
					e.preventDefault();
					first.focus();
				}
			}
		}

		document.addEventListener('keydown', handleKey, true);

		return () => {
			cancelAnimationFrame(rafId);
			document.removeEventListener('keydown', handleKey, true);
			for (const target of inertTargets) {
				target.removeAttribute('inert');
			}
			// Restore focus only if it's safe to do so (element still in DOM).
			if (previouslyFocused && document.contains(previouslyFocused)) {
				previouslyFocused.focus();
			}
		};
	};

	function handleBackdropClick() {
		if (closeOnBackdrop) requestClose();
	}
</script>

{#if open}
	<div class="dialog-root" data-size={size}>
		<!-- Backdrop is not a button; it's a click surface. Screen readers see the
		     modal; sighted users can click-to-dismiss. Escape is the a11y path. -->
		<div
			class="backdrop"
			role="presentation"
			onclick={handleBackdropClick}
			aria-hidden="true"
		></div>
		<div
			class="panel"
			role="dialog"
			aria-modal="true"
			aria-labelledby={titleId}
			aria-describedby={description ? descId : undefined}
			tabindex="-1"
			{@attach focusTrap}
		>
			<header class="panel-header">
				<h2 id={titleId} class="panel-title">{title}</h2>
				{#if description}
					<p id={descId} class="panel-description">{description}</p>
				{/if}
			</header>
			<div class="panel-body">
				{@render children()}
			</div>
			{#if footer}
				<footer class="panel-footer">
					{@render footer()}
				</footer>
			{/if}
		</div>
	</div>
{/if}

<style>
	.dialog-root {
		position: fixed;
		inset: 0;
		z-index: var(--z-50);
		display: flex;
		align-items: center;
		justify-content: center;
		padding: var(--space-4);
	}

	.backdrop {
		position: absolute;
		inset: 0;
		background-color: oklch(0.15 0.03 252 / 0.55);
		animation: backdrop-fade var(--duration-200) var(--ease-out);
	}

	.panel {
		position: relative;
		inline-size: min(var(--panel-width), calc(100vw - var(--space-8)));
		max-block-size: calc(100dvh - var(--space-10));
		display: flex;
		flex-direction: column;
		background-color: var(--surface-bg-canvas);
		color: var(--surface-fg-default);
		border-radius: var(--radius-xl);
		box-shadow: var(--shadow-2xl);
		animation: panel-enter var(--duration-200) var(--ease-spring);
		outline: none;
	}

	.dialog-root[data-size='sm'] .panel {
		--panel-width: 24rem;
	}
	.dialog-root[data-size='md'] .panel {
		--panel-width: 32rem;
	}
	.dialog-root[data-size='lg'] .panel {
		--panel-width: 48rem;
	}
	.dialog-root[data-size='xl'] .panel {
		--panel-width: 64rem;
	}
	.dialog-root[data-size='full'] .panel {
		--panel-width: 100vw;
		max-block-size: 100dvh;
		block-size: 100dvh;
		border-radius: 0;
	}

	.panel-header {
		padding-block: var(--space-5);
		padding-inline: var(--space-6);
		border-block-end: 1px solid var(--surface-border-subtle);
	}

	.panel-title {
		font-family: var(--font-heading);
		font-size: var(--fs-lg);
		font-weight: var(--w-bold);
		margin: 0;
	}

	.panel-description {
		margin-block-start: var(--space-1);
		margin-block-end: 0;
		font-size: var(--fs-sm);
		color: var(--surface-fg-muted);
	}

	.panel-body {
		padding-block: var(--space-5);
		padding-inline: var(--space-6);
		overflow-y: auto;
		flex: 1 1 auto;
	}

	.panel-footer {
		display: flex;
		gap: var(--space-3);
		justify-content: flex-end;
		padding-block: var(--space-4);
		padding-inline: var(--space-6);
		border-block-start: 1px solid var(--surface-border-subtle);
	}

	@keyframes backdrop-fade {
		from {
			opacity: 0;
		}
		to {
			opacity: 1;
		}
	}

	@keyframes panel-enter {
		from {
			opacity: 0;
			transform: translateY(8px) scale(0.98);
		}
		to {
			opacity: 1;
			transform: translateY(0) scale(1);
		}
	}

	@media (prefers-reduced-motion: reduce) {
		.backdrop,
		.panel {
			animation: none;
		}
	}
</style>
