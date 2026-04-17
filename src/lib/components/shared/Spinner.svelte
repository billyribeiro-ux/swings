<!--
  Spinner — pure-CSS circular loading indicator.

  A11y:
  - `role="status"` so AT announces the state.
  - `aria-label` via `label` prop (required).
  - Respects `prefers-reduced-motion` (replaces rotation with a subtle pulse
    so the indicator remains distinguishable from the background).
-->
<script lang="ts">
	interface Props {
		/** Visual size. */
		size?: 'sm' | 'md' | 'lg';
		/** Accessible name (required). Announced by assistive tech. */
		label: string;
		/**
		 * Render as inline-block so the spinner sits next to text without
		 * breaking flow. Defaults to inline-flex centred.
		 */
		inline?: boolean;
	}

	const { size = 'md', label, inline = false }: Props = $props();
</script>

<span class="spinner" class:inline data-size={size} role="status" aria-label={label}>
	<span class="ring" aria-hidden="true"></span>
</span>

<style>
	.spinner {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		vertical-align: middle;
		color: var(--brand-teal-500);
	}

	.spinner.inline {
		display: inline-block;
	}

	.ring {
		display: block;
		inline-size: var(--spinner-diameter);
		block-size: var(--spinner-diameter);
		border-radius: var(--radius-full);
		border: var(--spinner-stroke) solid oklch(0.66 0.12 197 / 0.2);
		border-block-start-color: currentColor;
		animation: spinner-rotate var(--duration-700) linear infinite;
	}

	.spinner[data-size='sm'] {
		--spinner-diameter: 1rem;
		--spinner-stroke: 2px;
	}
	.spinner[data-size='md'] {
		--spinner-diameter: 1.5rem;
		--spinner-stroke: 2px;
	}
	.spinner[data-size='lg'] {
		--spinner-diameter: 2.25rem;
		--spinner-stroke: 3px;
	}

	@keyframes spinner-rotate {
		to {
			transform: rotate(360deg);
		}
	}

	@keyframes spinner-pulse {
		50% {
			opacity: 0.5;
		}
	}

	@media (prefers-reduced-motion: reduce) {
		.ring {
			animation: spinner-pulse 1.6s ease-in-out infinite;
		}
	}
</style>
