<script lang="ts">
	import { type Snippet } from 'svelte';
	import { gsap } from 'gsap';
	import { isReducedMotion } from '$lib/utils/animations';

	interface Props {
		variant?: 'primary' | 'ghost' | 'outline' | 'tertiary' | 'secondary';
		size?: 'sm' | 'md' | 'lg';
		href?: string;
		onclick?: (e: MouseEvent) => void;
		disabled?: boolean;
		fullWidth?: boolean;
		children: Snippet;
		magnetic?: boolean;
	}

	let {
		variant = 'primary',
		size = 'md',
		href,
		onclick,
		disabled = false,
		fullWidth = false,
		children,
		magnetic = false
	}: Props = $props();

	let buttonRef: HTMLElement | undefined = $state();
	let ripples = $state<{ x: number; y: number; id: number; size: number }[]>([]);
	let rippleCount = 0;

	// GSAP quickTo instances for buttery smooth magnetic effect
	let xTo: gsap.QuickToFunc | undefined;
	let yTo: gsap.QuickToFunc | undefined;

	const classes = $derived(
		[
			'btn',
			`btn--${variant}`,
			`btn--${size}`,
			magnetic && 'btn--magnetic',
			fullWidth && 'btn--full'
		]
			.filter(Boolean)
			.join(' ')
	);

	function initMagnetic() {
		if (!buttonRef || !magnetic || isReducedMotion()) return;
		xTo = gsap.quickTo(buttonRef, 'x', { duration: 0.6, ease: 'power4.out' });
		yTo = gsap.quickTo(buttonRef, 'y', { duration: 0.6, ease: 'power4.out' });
	}

	$effect(() => {
		if (buttonRef && magnetic) {
			initMagnetic();
		}
	});

	function handleMouseMove(e: MouseEvent) {
		if (!buttonRef || disabled) return;

		// Spotlight glow effect tracking
		const rect = buttonRef.getBoundingClientRect();
		const x = e.clientX - rect.left;
		const y = e.clientY - rect.top;
		buttonRef.style.setProperty('--mouse-x', `${x}px`);
		buttonRef.style.setProperty('--mouse-y', `${y}px`);

		// Magnetic physical effect
		if (magnetic && xTo && yTo && !isReducedMotion()) {
			const cx = e.clientX - rect.left - rect.width / 2;
			const cy = e.clientY - rect.top - rect.height / 2;
			// Move 20% toward the mouse
			xTo(cx * 0.2);
			yTo(cy * 0.2);
		}
	}

	function handleMouseLeave() {
		if (!buttonRef) return;
		buttonRef.style.setProperty('--mouse-x', `-100px`);
		buttonRef.style.setProperty('--mouse-y', `-100px`);

		if (magnetic && xTo && yTo && !isReducedMotion()) {
			// Spring back to center
			xTo(0);
			yTo(0);
		}
	}

	function createRipple(e: MouseEvent) {
		if (!buttonRef || isReducedMotion()) return;
		const rect = buttonRef.getBoundingClientRect();
		const size = Math.max(rect.width, rect.height);
		const x = e.clientX - rect.left - size / 2;
		const y = e.clientY - rect.top - size / 2;
		const id = rippleCount++;

		ripples = [...ripples, { x, y, id, size }];
		setTimeout(() => {
			ripples = ripples.filter((r) => r.id !== id);
		}, 800); // Wait for animation to complete
	}

	function handleClick(e: MouseEvent) {
		if (disabled) {
			e.preventDefault();
			return;
		}
		createRipple(e);
		onclick?.(e);
	}
</script>

{#if href}
	<a
		href={disabled ? undefined : href}
		class={classes}
		aria-disabled={disabled || undefined}
		bind:this={buttonRef}
		onmousemove={handleMouseMove}
		onmouseleave={handleMouseLeave}
		onclick={handleClick}
	>
		{@render children()}
		{#each ripples as r (r.id)}
			<span
				class="btn__ripple"
				style="width: {r.size}px; height: {r.size}px; left: {r.x}px; top: {r.y}px;"
			></span>
		{/each}
	</a>
{:else}
	<button
		{disabled}
		class={classes}
		bind:this={buttonRef}
		onmousemove={handleMouseMove}
		onmouseleave={handleMouseLeave}
		onclick={handleClick}
	>
		{@render children()}
		{#each ripples as r (r.id)}
			<span
				class="btn__ripple"
				style="width: {r.size}px; height: {r.size}px; left: {r.x}px; top: {r.y}px;"
			></span>
		{/each}
	</button>
{/if}

<style>
	.btn {
		--mouse-x: -100px;
		--mouse-y: -100px;
		display: inline-flex;
		align-items: center;
		justify-content: center;
		gap: 0.5rem;
		border-radius: var(--radius-xl);
		font-family: var(--font-ui);
		font-weight: var(--w-semibold);
		transition:
			background-color 300ms cubic-bezier(0.22, 1, 0.36, 1),
			border-color 300ms cubic-bezier(0.22, 1, 0.36, 1),
			color 300ms cubic-bezier(0.22, 1, 0.36, 1),
			box-shadow 400ms cubic-bezier(0.22, 1, 0.36, 1),
			transform 200ms cubic-bezier(0.22, 1, 0.36, 1);
		cursor: pointer;
		text-decoration: none;
		line-height: 1.5;
		position: relative;
		overflow: hidden;
		border: 1px solid transparent;
		isolation: isolate;
	}

	.btn--sm {
		padding: 0.5rem 1rem;
		font-size: var(--fs-xs);
		border-radius: var(--radius-lg);
	}
	.btn--md {
		padding: 0.75rem 1.5rem;
		font-size: var(--fs-sm);
	}
	.btn--lg {
		padding: 1rem 2rem;
		font-size: var(--fs-md);
	}

	.btn--full {
		width: 100%;
	}

	.btn:focus-visible {
		outline: none;
		box-shadow:
			0 0 0 2px var(--color-navy),
			0 0 0 4px rgba(15, 164, 175, 0.7);
	}

	.btn:active {
		transform: scale(0.96) !important;
	}

	.btn:disabled,
	.btn[aria-disabled='true'] {
		pointer-events: none;
		opacity: 0.5;
		filter: grayscale(0.5);
	}

	/* Spotlight radial gradient overlay (Apple-like hover glow) */
	.btn::before {
		content: '';
		position: absolute;
		inset: 0;
		background: radial-gradient(
			circle 3.5rem at var(--mouse-x) var(--mouse-y),
			rgba(255, 255, 255, 0.12),
			transparent 100%
		);
		opacity: 0;
		transition: opacity 0.3s ease;
		pointer-events: none;
		z-index: 1;
	}

	.btn:hover::before {
		opacity: 1;
	}

	/* Ripple effect via Svelte state */
	.btn__ripple {
		position: absolute;
		border-radius: 50%;
		background: rgba(255, 255, 255, 0.3);
		transform: scale(0);
		animation: ripple 0.8s cubic-bezier(0.22, 1, 0.36, 1) forwards;
		pointer-events: none;
		z-index: 0;
	}

	@keyframes ripple {
		to {
			transform: scale(2.5);
			opacity: 0;
		}
	}

	/* --- Variants --- */
	.btn--primary {
		background-color: var(--color-teal);
		color: var(--color-white);
		box-shadow:
			inset 0 1px 1px rgba(255, 255, 255, 0.15),
			var(--shadow-lg),
			0 4px 14px rgba(15, 164, 175, 0.25);
		border-color: rgba(255, 255, 255, 0.05);
	}

	.btn--primary:hover {
		background-color: var(--color-teal-light);
		box-shadow:
			inset 0 1px 1px rgba(255, 255, 255, 0.2),
			var(--shadow-xl),
			0 8px 24px rgba(15, 164, 175, 0.35);
	}

	.btn--secondary {
		background-color: rgba(255, 255, 255, 0.08);
		color: var(--color-white);
		border-color: rgba(255, 255, 255, 0.1);
	}

	.btn--secondary:hover {
		background-color: rgba(255, 255, 255, 0.15);
		border-color: rgba(255, 255, 255, 0.2);
		box-shadow: 0 4px 12px rgba(0, 0, 0, 0.1);
	}

	.btn--tertiary {
		background-color: transparent;
		color: var(--color-grey-300);
	}

	.btn--tertiary:hover {
		background-color: rgba(255, 255, 255, 0.05);
		color: var(--color-white);
	}

	.btn--ghost {
		background-color: rgba(255, 255, 255, 0.04);
		color: var(--color-white);
		border: 1px solid rgba(255, 255, 255, 0.12);
		backdrop-filter: blur(8px);
	}

	.btn--ghost:hover {
		background-color: rgba(255, 255, 255, 0.1);
		border-color: rgba(255, 255, 255, 0.25);
		box-shadow: 0 4px 16px rgba(0, 0, 0, 0.15);
	}

	.btn--outline {
		background-color: transparent;
		color: var(--color-navy);
		border: 2px solid rgba(11, 29, 58, 0.8);
	}

	.btn--outline:hover {
		background-color: var(--color-navy);
		color: var(--color-white);
		box-shadow:
			var(--shadow-lg),
			0 4px 14px rgba(11, 29, 58, 0.15);
	}
	
	.btn--outline::before {
		background: radial-gradient(
			circle 3.5rem at var(--mouse-x) var(--mouse-y),
			rgba(11, 29, 58, 0.1),
			transparent 100%
		);
	}
</style>
