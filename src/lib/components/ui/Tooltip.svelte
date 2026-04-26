<script lang="ts" module>
	/**
	 * Module-level coordination so only one tooltip is visible at any time.
	 * Opening a new tooltip closes whichever one is currently open.
	 *
	 * Plain `let` (not `$state`) — this is just a coordination signal across
	 * instances, never read inside a reactive context.
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
		return `tooltip-${nextId}`;
	}
</script>

<script lang="ts">
	import type { Snippet } from 'svelte';

	type Placement = 'top' | 'right' | 'bottom' | 'left';

	interface TriggerProps {
		'aria-describedby': string | undefined;
	}

	interface Props {
		label: string;
		placement?: Placement | undefined;
		hotkey?: string | undefined;
		delay?: number | undefined;
		disabled?: boolean | undefined;
		children?: Snippet | undefined;
		trigger?: Snippet<[TriggerProps]> | undefined;
	}

	let {
		label,
		placement = 'top',
		hotkey,
		delay = 400,
		disabled = false,
		children,
		trigger
	}: Props = $props();

	const id = uid();

	let anchorEl: HTMLSpanElement | undefined = $state();
	let tooltipEl: HTMLDivElement | undefined = $state();
	let open = $state(false);
	let openedByKeyboard = $state(false);
	// Initial value is a literal so the rune doesn't capture the `placement`
	// prop at construction time (svelte/state_referenced_locally). The first
	// `computePosition()` call (fired in the open `$effect`) reassigns it
	// from the current `placement` prop, optionally flipped to fit the viewport.
	let resolvedPlacement = $state<Placement>('top');
	let coords = $state<{ top: number; left: number }>({ top: 0, left: 0 });

	let openTimer: ReturnType<typeof setTimeout> | null = null;
	let isTouchInteraction = false;

	const closeFn = () => {
		open = false;
	};

	function clearOpenTimer() {
		if (openTimer !== null) {
			clearTimeout(openTimer);
			openTimer = null;
		}
	}

	function scheduleOpen(immediate = false) {
		if (disabled || !label) return;
		if (isTouchInteraction) return;
		clearOpenTimer();
		if (immediate || delay <= 0) {
			doOpen();
			return;
		}
		openTimer = setTimeout(() => {
			openTimer = null;
			doOpen();
		}, delay);
	}

	function doOpen() {
		if (disabled || !label) return;
		claimActive(closeFn);
		// Seed resolvedPlacement from the prop so the first paint uses the
		// requested side; computePosition() may flip it once measured.
		resolvedPlacement = placement;
		open = true;
	}

	function close() {
		clearOpenTimer();
		open = false;
		openedByKeyboard = false;
		releaseActive(closeFn);
	}

	function getTriggerRect(): DOMRect | null {
		if (!anchorEl) return null;
		// `display: contents` means the anchor span itself has no box; query its
		// first element child instead. Fallback to anchor for edge cases.
		const triggerEl =
			(anchorEl.firstElementChild as HTMLElement | null) ?? (anchorEl as HTMLElement);
		return triggerEl.getBoundingClientRect();
	}

	function computePosition() {
		if (!tooltipEl) return;
		const rect = getTriggerRect();
		if (!rect) return;

		const tipRect = tooltipEl.getBoundingClientRect();
		const vw = window.innerWidth;
		const vh = window.innerHeight;
		const gap = 8;
		const margin = 6;

		const fitsTop = rect.top - tipRect.height - gap >= margin;
		const fitsBottom = rect.bottom + tipRect.height + gap <= vh - margin;
		const fitsLeft = rect.left - tipRect.width - gap >= margin;
		const fitsRight = rect.right + tipRect.width + gap <= vw - margin;

		let next: Placement = placement;
		if (placement === 'top' && !fitsTop && fitsBottom) next = 'bottom';
		else if (placement === 'bottom' && !fitsBottom && fitsTop) next = 'top';
		else if (placement === 'right' && !fitsRight && fitsLeft) next = 'left';
		else if (placement === 'left' && !fitsLeft && fitsRight) next = 'right';

		resolvedPlacement = next;

		let top = 0;
		let left = 0;
		switch (next) {
			case 'top':
				top = rect.top - tipRect.height - gap;
				left = rect.left + rect.width / 2 - tipRect.width / 2;
				break;
			case 'bottom':
				top = rect.bottom + gap;
				left = rect.left + rect.width / 2 - tipRect.width / 2;
				break;
			case 'left':
				top = rect.top + rect.height / 2 - tipRect.height / 2;
				left = rect.left - tipRect.width - gap;
				break;
			case 'right':
				top = rect.top + rect.height / 2 - tipRect.height / 2;
				left = rect.right + gap;
				break;
		}

		// Clamp to viewport so tooltip never clips off-screen.
		left = Math.max(margin, Math.min(left, vw - tipRect.width - margin));
		top = Math.max(margin, Math.min(top, vh - tipRect.height - margin));

		coords = { top, left };
	}

	$effect(() => {
		if (!open) return;
		// Re-read after mount so tipRect is real.
		computePosition();

		const onScroll = () => computePosition();
		const onResize = () => computePosition();
		window.addEventListener('scroll', onScroll, true);
		window.addEventListener('resize', onResize);

		return () => {
			window.removeEventListener('scroll', onScroll, true);
			window.removeEventListener('resize', onResize);
		};
	});

	$effect(() => {
		if (disabled && open) close();
	});

	$effect(() => {
		return () => {
			clearOpenTimer();
			releaseActive(closeFn);
		};
	});

	function handlePointerEnter(e: PointerEvent) {
		if (e.pointerType === 'touch') {
			isTouchInteraction = true;
			return;
		}
		isTouchInteraction = false;
		scheduleOpen();
	}

	function handlePointerLeave() {
		isTouchInteraction = false;
		close();
	}

	function handleFocusIn(e: FocusEvent) {
		// Only react to keyboard focus, not mouse-induced focus.
		const target = e.target as HTMLElement | null;
		if (!target) return;
		if (typeof target.matches === 'function' && target.matches(':focus-visible')) {
			openedByKeyboard = true;
			scheduleOpen(true);
		}
	}

	function handleFocusOut() {
		if (openedByKeyboard) close();
	}

	function handleKeyDown(e: KeyboardEvent) {
		if (e.key === 'Escape' && open) {
			close();
		}
	}

	function handleTouchStart() {
		isTouchInteraction = true;
		close();
	}

	const triggerProps: TriggerProps = $derived({
		'aria-describedby': open ? id : undefined
	});
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<span
	bind:this={anchorEl}
	class="tooltip-anchor"
	onpointerenter={handlePointerEnter}
	onpointerleave={handlePointerLeave}
	onfocusin={handleFocusIn}
	onfocusout={handleFocusOut}
	onkeydown={handleKeyDown}
	ontouchstart={handleTouchStart}
>
	{#if trigger}
		{@render trigger(triggerProps)}
	{:else if children}
		{@render children()}
	{/if}
</span>

{#if open && label && !disabled}
	<div
		bind:this={tooltipEl}
		{id}
		role="tooltip"
		class="tooltip tooltip--{resolvedPlacement}"
		style:top="{coords.top}px"
		style:left="{coords.left}px"
	>
		<span class="tooltip__label">{label}</span>
		{#if hotkey}
			<kbd class="tooltip__kbd">{hotkey}</kbd>
		{/if}
	</div>
{/if}

<style>
	.tooltip-anchor {
		display: contents;
	}

	.tooltip {
		position: fixed;
		z-index: var(--z-tooltip, 9999);
		display: inline-flex;
		align-items: center;
		max-width: 16rem;
		padding: 0.35rem 0.55rem;
		background: var(--color-navy-deep);
		border: 1px solid rgba(255, 255, 255, 0.12);
		border-radius: var(--radius-md);
		box-shadow:
			0 8px 24px rgba(0, 0, 0, 0.4),
			0 1px 0 rgba(255, 255, 255, 0.08) inset;
		color: var(--color-grey-200);
		font-family: var(--font-ui);
		font-size: 0.75rem;
		font-weight: var(--w-medium);
		line-height: 1.35;
		white-space: normal;
		pointer-events: none;
		opacity: 0;
		transform: translate3d(0, 0, 0);
		animation: tooltip-in 120ms var(--ease-out) forwards;
	}

	.tooltip--top {
		animation-name: tooltip-in-top;
	}
	.tooltip--bottom {
		animation-name: tooltip-in-bottom;
	}
	.tooltip--left {
		animation-name: tooltip-in-left;
	}
	.tooltip--right {
		animation-name: tooltip-in-right;
	}

	.tooltip__label {
		display: inline-block;
	}

	.tooltip__kbd {
		display: inline-flex;
		align-items: center;
		margin-left: 0.4rem;
		padding: 0 0.3rem;
		background: rgba(255, 255, 255, 0.08);
		border-radius: var(--radius-xs);
		color: var(--color-grey-400);
		font-family: var(--font-ui);
		font-size: 0.7rem;
		font-weight: var(--w-semibold);
		line-height: 1.4;
	}

	@keyframes tooltip-in-top {
		from {
			opacity: 0;
			transform: translate3d(0, 4px, 0);
		}
		to {
			opacity: 1;
			transform: translate3d(0, 0, 0);
		}
	}
	@keyframes tooltip-in-bottom {
		from {
			opacity: 0;
			transform: translate3d(0, -4px, 0);
		}
		to {
			opacity: 1;
			transform: translate3d(0, 0, 0);
		}
	}
	@keyframes tooltip-in-left {
		from {
			opacity: 0;
			transform: translate3d(4px, 0, 0);
		}
		to {
			opacity: 1;
			transform: translate3d(0, 0, 0);
		}
	}
	@keyframes tooltip-in-right {
		from {
			opacity: 0;
			transform: translate3d(-4px, 0, 0);
		}
		to {
			opacity: 1;
			transform: translate3d(0, 0, 0);
		}
	}

	@media (prefers-reduced-motion: reduce) {
		.tooltip,
		.tooltip--top,
		.tooltip--bottom,
		.tooltip--left,
		.tooltip--right {
			animation: tooltip-in-reduced 1ms linear forwards;
		}
		@keyframes tooltip-in-reduced {
			to {
				opacity: 1;
				transform: none;
			}
		}
	}
</style>
