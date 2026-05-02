<script lang="ts" module>
	/**
	 * Module-level coordination so only one ActionMenu is open at a time.
	 * Mirrors the pattern in `Tooltip.svelte` — opening a new menu closes
	 * whichever one is currently open. Plain `let` (not `$state`) since
	 * this is a cross-instance coordination signal, never read inside a
	 * reactive context.
	 */
	let activeCloser: (() => void) | null = null;

	function claimActive(closer: () => void) {
		if (activeCloser && activeCloser !== closer) {
			activeCloser();
		}
		activeCloser = closer;
	}

	function releaseActive(closer: () => void) {
		if (activeCloser === closer) {
			activeCloser = null;
		}
	}

	let nextId = 0;
	function uid(): string {
		nextId += 1;
		return `action-menu-${nextId}`;
	}
</script>

<script lang="ts">
	import { browser } from '$app/environment';
	import type { Attachment } from 'svelte/attachments';
	import type { Snippet } from 'svelte';

	/**
	 * Move the menu surface to `document.body` so `position: fixed` is relative
	 * to the viewport and isn't clipped by ancestors with `overflow-*`,
	 * `backdrop-filter`, `filter`, or `transform` (common in admin tables and
	 * glass panels). Mirrors the same pattern used by `Tooltip.svelte`.
	 */
	const portalToBody: Attachment = (node) => {
		if (!browser) return;
		document.body.appendChild(node);
		return () => {
			node.remove();
		};
	};

	type Placement =
		| 'bottom-start'
		| 'bottom-end'
		| 'top-start'
		| 'top-end';

	interface TriggerProps {
		'aria-haspopup': 'menu';
		'aria-expanded': boolean;
		'aria-controls': string;
		onclick: (event: MouseEvent) => void;
		onkeydown: (event: KeyboardEvent) => void;
	}

	interface Props {
		placement?: Placement | undefined;
		label?: string | undefined;
		disabled?: boolean | undefined;
		trigger: Snippet<[TriggerProps]>;
		items: Snippet;
	}

	let {
		placement = 'bottom-end',
		label = 'Actions',
		disabled = false,
		trigger,
		items
	}: Props = $props();

	const id = uid();

	let anchorEl: HTMLSpanElement | undefined = $state();
	let menuEl: HTMLDivElement | undefined = $state();
	let triggerButtonEl: HTMLElement | null = null;
	let open = $state(false);
	let resolvedPlacement = $state<Placement>('bottom-end');
	let coords = $state<{ top: number; left: number }>({ top: 0, left: 0 });
	/** Set to true when the menu was opened via keyboard so we can focus the first item. */
	let focusFirstOnOpen = $state(false);

	const closeFn = () => {
		open = false;
	};

	function getTriggerEl(): HTMLElement | null {
		if (!anchorEl) return null;
		// `display: contents` means the anchor span itself has no box; query its
		// first element child instead. Fallback to anchor for edge cases.
		return (anchorEl.firstElementChild as HTMLElement | null) ?? anchorEl;
	}

	function getTriggerRect(): DOMRect | null {
		const el = getTriggerEl();
		return el ? el.getBoundingClientRect() : null;
	}

	function doOpen(viaKeyboard: boolean) {
		if (disabled) return;
		focusFirstOnOpen = viaKeyboard;
		// Seed resolvedPlacement from the prop so the first paint uses the
		// requested side; computePosition() may flip it once measured.
		resolvedPlacement = placement;
		claimActive(closeFn);
		open = true;
	}

	function close(returnFocus = true) {
		if (!open) return;
		open = false;
		focusFirstOnOpen = false;
		releaseActive(closeFn);
		if (returnFocus) {
			// Restore focus to the trigger so keyboard users keep their place.
			const el = triggerButtonEl ?? getTriggerEl();
			el?.focus();
		}
	}

	function toggle(viaKeyboard: boolean) {
		if (open) {
			close(viaKeyboard);
		} else {
			doOpen(viaKeyboard);
		}
	}

	function handleTriggerClick(e: MouseEvent) {
		// Track the trigger element so we can return focus on close.
		triggerButtonEl = e.currentTarget as HTMLElement;
		// `detail === 0` means a synthetic click (e.g. Enter/Space on a
		// button) — treat that as keyboard activation so focus lands on the
		// first item.
		toggle(e.detail === 0);
	}

	function handleTriggerKeyDown(e: KeyboardEvent) {
		triggerButtonEl = e.currentTarget as HTMLElement;
		if (e.key === 'ArrowDown' || e.key === 'ArrowUp') {
			e.preventDefault();
			if (!open) doOpen(true);
		} else if ((e.key === 'Enter' || e.key === ' ') && !open) {
			// Native button behaviour fires click on Enter/Space — let that
			// happen so toggle() runs once. Nothing to do here.
		} else if (e.key === 'Escape' && open) {
			e.preventDefault();
			close();
		}
	}

	const MARGIN = 6;
	const GAP = 6;

	function computePosition() {
		if (!menuEl) return;
		const rect = getTriggerRect();
		if (!rect) return;

		const menuRect = menuEl.getBoundingClientRect();
		const vw = window.innerWidth;
		const vh = window.innerHeight;

		const fitsBelow = rect.bottom + menuRect.height + GAP <= vh - MARGIN;
		const fitsAbove = rect.top - menuRect.height - GAP >= MARGIN;

		let next: Placement = placement;
		if (placement.startsWith('bottom') && !fitsBelow && fitsAbove) {
			next = (placement === 'bottom-start' ? 'top-start' : 'top-end') as Placement;
		} else if (placement.startsWith('top') && !fitsAbove && fitsBelow) {
			next = (placement === 'top-start' ? 'bottom-start' : 'bottom-end') as Placement;
		}
		resolvedPlacement = next;

		let top: number;
		let left: number;
		if (next === 'bottom-start') {
			top = rect.bottom + GAP;
			left = rect.left;
		} else if (next === 'bottom-end') {
			top = rect.bottom + GAP;
			left = rect.right - menuRect.width;
		} else if (next === 'top-start') {
			top = rect.top - menuRect.height - GAP;
			left = rect.left;
		} else {
			top = rect.top - menuRect.height - GAP;
			left = rect.right - menuRect.width;
		}

		// Clamp to viewport so the menu never clips off-screen.
		const clampedLeft = Math.max(MARGIN, Math.min(left, vw - menuRect.width - MARGIN));
		const clampedTop = Math.max(MARGIN, Math.min(top, vh - menuRect.height - MARGIN));
		coords = { top: clampedTop, left: clampedLeft };
	}

	function focusItem(index: number) {
		if (!menuEl) return;
		const itemList = menuEl.querySelectorAll<HTMLElement>('[role="menuitem"]:not([aria-disabled="true"])');
		if (itemList.length === 0) return;
		const wrapped = ((index % itemList.length) + itemList.length) % itemList.length;
		itemList[wrapped]?.focus();
	}

	function focusableItems(): HTMLElement[] {
		if (!menuEl) return [];
		return Array.from(
			menuEl.querySelectorAll<HTMLElement>('[role="menuitem"]:not([aria-disabled="true"])')
		);
	}

	function currentItemIndex(): number {
		const list = focusableItems();
		const active = document.activeElement as HTMLElement | null;
		if (!active) return -1;
		return list.indexOf(active);
	}

	function handleMenuKeyDown(e: KeyboardEvent) {
		if (e.key === 'Escape') {
			e.preventDefault();
			close();
			return;
		}
		if (e.key === 'ArrowDown') {
			e.preventDefault();
			const idx = currentItemIndex();
			focusItem(idx + 1);
			return;
		}
		if (e.key === 'ArrowUp') {
			e.preventDefault();
			const idx = currentItemIndex();
			focusItem(idx <= 0 ? focusableItems().length - 1 : idx - 1);
			return;
		}
		if (e.key === 'Home') {
			e.preventDefault();
			focusItem(0);
			return;
		}
		if (e.key === 'End') {
			e.preventDefault();
			focusItem(focusableItems().length - 1);
			return;
		}
		if (e.key === 'Tab') {
			// Tab/Shift-Tab cycles within the menu — preserve a contained focus
			// trap so admin keyboard users never lose context to the page.
			const list = focusableItems();
			if (list.length === 0) return;
			const idx = currentItemIndex();
			e.preventDefault();
			if (e.shiftKey) {
				focusItem(idx <= 0 ? list.length - 1 : idx - 1);
			} else {
				focusItem(idx + 1);
			}
		}
	}

	function handleMenuClick(e: MouseEvent) {
		// Item handlers are wired by the consumer via ActionMenuItem; the menu
		// auto-closes whenever an item is activated. We use bubble-phase click
		// so the consumer's onclick has already run by the time we close.
		const target = e.target as HTMLElement | null;
		const item = target?.closest<HTMLElement>('[role="menuitem"]');
		if (item && item.getAttribute('aria-disabled') !== 'true') {
			close();
		}
	}

	function handleOutsidePointerDown(e: PointerEvent) {
		const target = e.target as Node | null;
		if (!target) return;
		if (menuEl?.contains(target)) return;
		if (anchorEl?.contains(target)) return;
		close(false);
	}

	$effect(() => {
		if (!open) return;
		// Re-measure after mount so menuRect is real.
		computePosition();
		if (focusFirstOnOpen) {
			// Wait one frame so the portaled menu is in the DOM.
			requestAnimationFrame(() => focusItem(0));
		}

		const onScroll = () => computePosition();
		const onResize = () => computePosition();
		window.addEventListener('scroll', onScroll, true);
		window.addEventListener('resize', onResize);
		document.addEventListener('pointerdown', handleOutsidePointerDown, true);

		return () => {
			window.removeEventListener('scroll', onScroll, true);
			window.removeEventListener('resize', onResize);
			document.removeEventListener('pointerdown', handleOutsidePointerDown, true);
		};
	});

	$effect(() => {
		if (disabled && open) close(false);
	});

	$effect(() => {
		return () => {
			releaseActive(closeFn);
		};
	});

	const triggerProps: TriggerProps = $derived({
		'aria-haspopup': 'menu',
		'aria-expanded': open,
		'aria-controls': id,
		onclick: handleTriggerClick,
		onkeydown: handleTriggerKeyDown
	});
</script>

<span bind:this={anchorEl} class="action-menu-anchor">
	{@render trigger(triggerProps)}
</span>

{#if open && !disabled}
	<div
		bind:this={menuEl}
		{@attach portalToBody}
		{id}
		role="menu"
		tabindex="-1"
		aria-label={label}
		aria-orientation="vertical"
		class="action-menu action-menu--{resolvedPlacement}"
		style:top="{coords.top}px"
		style:left="{coords.left}px"
		onkeydown={handleMenuKeyDown}
		onclick={handleMenuClick}
	>
		{@render items()}
	</div>
{/if}

<style>
	.action-menu-anchor {
		display: contents;
	}

	.action-menu {
		/* Local design tokens — keep one-off oklch values discoverable in one
		   place. Sibling palette of the post-polish Tooltip surface. */
		--menu-bg: oklch(16% 0.02 252);
		--menu-border: oklch(32% 0.02 252);
		--menu-shadow: 0 8px 24px rgba(0, 0, 0, 0.4);

		position: fixed;
		/* Above admin chrome, alongside Tooltip's z-index band. */
		z-index: var(--z-tooltip, 11000);
		min-width: 12rem;
		padding: 0.35rem;
		background: var(--menu-bg);
		border: 1px solid var(--menu-border);
		border-radius: var(--radius-lg);
		box-shadow: var(--menu-shadow);
		font-family: var(--font-ui);
		opacity: 0;
		transform: translate3d(0, 0, 0);
		animation: action-menu-in 120ms var(--ease-out) forwards;
		display: flex;
		flex-direction: column;
		gap: 0.1rem;
	}

	.action-menu--bottom-start,
	.action-menu--bottom-end {
		animation-name: action-menu-in-bottom;
	}

	.action-menu--top-start,
	.action-menu--top-end {
		animation-name: action-menu-in-top;
	}

	@keyframes action-menu-in-bottom {
		from {
			opacity: 0;
			transform: translate3d(0, -4px, 0);
		}
		to {
			opacity: 1;
			transform: translate3d(0, 0, 0);
		}
	}

	@keyframes action-menu-in-top {
		from {
			opacity: 0;
			transform: translate3d(0, 4px, 0);
		}
		to {
			opacity: 1;
			transform: translate3d(0, 0, 0);
		}
	}

	@media (prefers-reduced-motion: reduce) {
		.action-menu {
			animation: action-menu-in-reduced 1ms linear forwards;
		}
		@keyframes action-menu-in-reduced {
			to {
				opacity: 1;
				transform: none;
			}
		}
	}
</style>
