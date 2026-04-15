<script lang="ts">
	import { type Snippet } from 'svelte';

	interface Props {
		variant?: 'primary' | 'ghost' | 'outline';
		href?: string;
		onclick?: () => void;
		disabled?: boolean;
		children: Snippet;
		magnetic?: boolean;
	}

	let {
		variant = 'primary',
		href,
		onclick,
		disabled = false,
		children,
		magnetic = false
	}: Props = $props();

	let buttonRef: HTMLElement | undefined = $state();
	let magneticRaf = 0;
	let lastMagneticEvent: MouseEvent | null = null;

	const classes = $derived(`btn btn--${variant}${magnetic ? ' btn--magnetic' : ''}`);

	function applyMagneticFromEvent(e: MouseEvent) {
		if (!buttonRef) return;
		const rect = buttonRef.getBoundingClientRect();
		const x = e.clientX - rect.left - rect.width / 2;
		const y = e.clientY - rect.top - rect.height / 2;
		buttonRef.style.transform = `translate(${x * 0.15}px, ${y * 0.15}px)`;
	}

	function handleMouseMove(e: MouseEvent) {
		if (!magnetic || !buttonRef || disabled) return;
		lastMagneticEvent = e;
		if (magneticRaf) return;
		magneticRaf = requestAnimationFrame(() => {
			magneticRaf = 0;
			const ev = lastMagneticEvent;
			if (!ev || !buttonRef || !magnetic || disabled) return;
			applyMagneticFromEvent(ev);
		});
	}

	function handleMouseLeave() {
		lastMagneticEvent = null;
		if (magneticRaf) cancelAnimationFrame(magneticRaf);
		magneticRaf = 0;
		if (!buttonRef) return;
		buttonRef.style.transform = '';
	}

	function createRipple(e: MouseEvent) {
		if (!buttonRef) return;

		const rect = buttonRef.getBoundingClientRect();
		const size = Math.max(rect.width, rect.height);
		const x = e.clientX - rect.left - size / 2;
		const y = e.clientY - rect.top - size / 2;

		const ripple = document.createElement('span');
		ripple.className = 'btn__ripple';
		ripple.style.width = ripple.style.height = `${size}px`;
		ripple.style.left = `${x}px`;
		ripple.style.top = `${y}px`;

		buttonRef.appendChild(ripple);

		setTimeout(() => ripple.remove(), 600);
	}

	function handleClick(e: MouseEvent) {
		if (disabled) {
			e.preventDefault();
			return;
		}
		createRipple(e);
		onclick?.();
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
	</a>
{:else}
	<button
		onclick={handleClick}
		{disabled}
		class={classes}
		bind:this={buttonRef}
		onmousemove={handleMouseMove}
		onmouseleave={handleMouseLeave}
	>
		{@render children()}
	</button>
{/if}

<style>
	.btn {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		gap: 0.5rem;
		padding: 0.75rem 1.5rem;
		border-radius: var(--radius-xl);
		font-family: var(--font-ui);
		font-weight: var(--w-semibold);
		font-size: var(--fs-sm);
		transition: all 300ms cubic-bezier(0, 0, 0.2, 1);
		cursor: pointer;
		text-decoration: none;
		line-height: 1.5;
		position: relative;
		overflow: hidden;
	}

	.btn:focus-visible {
		outline: none;
		box-shadow:
			0 0 0 2px var(--color-navy),
			0 0 0 4px rgba(15, 164, 175, 0.7);
	}

	.btn:active {
		transform: scale(0.97);
	}

	.btn:disabled,
	.btn[aria-disabled='true'] {
		pointer-events: none;
		opacity: 0.5;
	}

	/* Magnetic effect base */
	.btn--magnetic {
		transition: transform 0.3s cubic-bezier(0.22, 1, 0.36, 1);
	}

	/* Ripple effect */
	:global(.btn__ripple) {
		position: absolute;
		border-radius: 50%;
		background: rgba(255, 255, 255, 0.4);
		transform: scale(0);
		animation: ripple 0.6s ease-out;
		pointer-events: none;
	}

	@keyframes ripple {
		to {
			transform: scale(2.5);
			opacity: 0;
		}
	}

	.btn--primary {
		background-color: var(--color-teal);
		color: var(--color-white);
		box-shadow:
			var(--shadow-lg),
			0 4px 14px rgba(15, 164, 175, 0.25);
	}

	.btn--primary:hover {
		background-color: var(--color-teal-light);
		transform: translateY(-1px);
		box-shadow:
			var(--shadow-xl),
			0 8px 20px rgba(15, 164, 175, 0.3);
	}

	.btn--ghost {
		background-color: rgba(255, 255, 255, 0.08);
		color: var(--color-white);
		border: 1px solid rgba(255, 255, 255, 0.15);
		backdrop-filter: blur(4px);
	}

	.btn--ghost:hover {
		background-color: rgba(255, 255, 255, 0.15);
		transform: translateY(-1px);
	}

	.btn--outline {
		background-color: transparent;
		color: var(--color-navy);
		border: 2px solid rgba(11, 29, 58, 0.8);
	}

	.btn--outline:hover {
		background-color: var(--color-navy);
		color: var(--color-white);
		transform: translateY(-1px);
		box-shadow:
			var(--shadow-lg),
			0 4px 14px rgba(11, 29, 58, 0.15);
	}
</style>
