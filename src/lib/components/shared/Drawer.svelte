<!--
  Drawer — slide-in panel variant of Dialog.

  Logical positioning:
  - `start` / `end` are writing-mode aware (LTR: left/right, RTL: right/left).
  - `top` / `bottom` are block-axis.

  A11y: identical machinery to Dialog (role=dialog, aria-modal, focus trap,
  ESC close, inert background, aria-labelledby/describedby).
-->
<script lang="ts" module>
	export type { DrawerPosition, DrawerProps, DrawerSize } from './Drawer.types';

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
	import type { DrawerProps as Props } from './Drawer.types';

	let {
		open = $bindable(false),
		onclose,
		title,
		description,
		size = 'md',
		position = 'end',
		closeOnBackdrop = true,
		closeOnEscape = true,
		children,
		footer
	}: Props = $props();

	const titleId = nextId('drawer-title');
	const descId = nextId('drawer-desc');

	function requestClose() {
		if (!open) return;
		open = false;
		onclose?.();
	}

	const focusTrap: Attachment<HTMLElement> = (node) => {
		const previouslyFocused = document.activeElement as HTMLElement | null;

		const host = node.parentElement;
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

		const rafId = requestAnimationFrame(() => {
			const list = focusables();
			if (list.length > 0) {
				list[0].focus();
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
			const first = list[0];
			const last = list[list.length - 1];
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
	<div class="drawer-root" data-position={position} data-size={size}>
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
	.drawer-root {
		position: fixed;
		inset: 0;
		z-index: var(--z-50);
		display: flex;
	}
	.backdrop {
		position: absolute;
		inset: 0;
		background-color: oklch(0.15 0.03 252 / 0.55);
		animation: backdrop-fade var(--duration-200) var(--ease-out);
	}
	.panel {
		position: relative;
		background-color: var(--surface-bg-canvas);
		color: var(--surface-fg-default);
		box-shadow: var(--shadow-2xl);
		display: flex;
		flex-direction: column;
		outline: none;
	}

	/* Inline-axis: start / end — RTL-aware via writing-mode. */
	.drawer-root[data-position='start'] {
		justify-content: flex-start;
		align-items: stretch;
	}
	.drawer-root[data-position='end'] {
		justify-content: flex-end;
		align-items: stretch;
	}
	.drawer-root[data-position='start'] .panel,
	.drawer-root[data-position='end'] .panel {
		block-size: 100dvh;
		inline-size: min(var(--panel-inline-size), 100vw);
	}
	.drawer-root[data-position='start'] .panel {
		animation: slide-from-start var(--duration-300) var(--ease-spring);
		border-start-end-radius: var(--radius-xl);
		border-end-end-radius: var(--radius-xl);
	}
	.drawer-root[data-position='end'] .panel {
		animation: slide-from-end var(--duration-300) var(--ease-spring);
		border-start-start-radius: var(--radius-xl);
		border-end-start-radius: var(--radius-xl);
	}

	/* Block-axis: top / bottom. */
	.drawer-root[data-position='top'] {
		align-items: flex-start;
	}
	.drawer-root[data-position='bottom'] {
		align-items: flex-end;
	}
	.drawer-root[data-position='top'] .panel,
	.drawer-root[data-position='bottom'] .panel {
		inline-size: 100vw;
		block-size: min(var(--panel-block-size), 100dvh);
	}
	.drawer-root[data-position='top'] .panel {
		animation: slide-from-top var(--duration-300) var(--ease-spring);
		border-end-start-radius: var(--radius-xl);
		border-end-end-radius: var(--radius-xl);
	}
	.drawer-root[data-position='bottom'] .panel {
		animation: slide-from-bottom var(--duration-300) var(--ease-spring);
		border-start-start-radius: var(--radius-xl);
		border-start-end-radius: var(--radius-xl);
	}

	/* Sizes — inline for start/end drawers. */
	.drawer-root[data-size='sm'] .panel {
		--panel-inline-size: 20rem;
		--panel-block-size: 16rem;
	}
	.drawer-root[data-size='md'] .panel {
		--panel-inline-size: 28rem;
		--panel-block-size: 22rem;
	}
	.drawer-root[data-size='lg'] .panel {
		--panel-inline-size: 36rem;
		--panel-block-size: 28rem;
	}
	.drawer-root[data-size='xl'] .panel {
		--panel-inline-size: 48rem;
		--panel-block-size: 36rem;
	}
	.drawer-root[data-size='full'] .panel {
		--panel-inline-size: 100vw;
		--panel-block-size: 100dvh;
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
	@keyframes slide-from-start {
		from {
			transform: translateX(-100%);
		}
		to {
			transform: translateX(0);
		}
	}
	@keyframes slide-from-end {
		from {
			transform: translateX(100%);
		}
		to {
			transform: translateX(0);
		}
	}
	@keyframes slide-from-top {
		from {
			transform: translateY(-100%);
		}
		to {
			transform: translateY(0);
		}
	}
	@keyframes slide-from-bottom {
		from {
			transform: translateY(100%);
		}
		to {
			transform: translateY(0);
		}
	}

	/* RTL: start/end animations flip horizontally. */
	:global([dir='rtl']) .drawer-root[data-position='start'] .panel {
		animation-name: slide-from-end;
	}
	:global([dir='rtl']) .drawer-root[data-position='end'] .panel {
		animation-name: slide-from-start;
	}

	@media (prefers-reduced-motion: reduce) {
		.backdrop,
		.panel {
			animation: none;
		}
	}
</style>
