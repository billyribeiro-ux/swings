<script lang="ts">
	interface Props {
		width?: string;
		height?: string;
		variant?: 'text' | 'circle' | 'card' | 'rect';
	}

	let {
		width,
		height,
		variant = 'text'
	}: Props = $props();

	const variantDefaults: Record<string, { w: string; h: string }> = {
		text: { w: '100%', h: '1em' },
		circle: { w: '48px', h: '48px' },
		card: { w: '100%', h: '200px' },
		rect: { w: '100%', h: '100px' }
	};

	const resolvedWidth = $derived(width ?? variantDefaults[variant].w);
	const resolvedHeight = $derived(height ?? variantDefaults[variant].h);
</script>

<div
	class="skeleton skeleton--{variant}"
	style:width={resolvedWidth}
	style:height={resolvedHeight}
	aria-hidden="true"
	role="presentation"
></div>

<style>
	.skeleton {
		background: var(--color-navy-mid);
		border-radius: var(--radius-md);
		position: relative;
		overflow: hidden;
	}

	.skeleton::after {
		content: '';
		position: absolute;
		inset: 0;
		background: linear-gradient(
			90deg,
			transparent 0%,
			rgba(255, 255, 255, 0.04) 20%,
			rgba(255, 255, 255, 0.08) 50%,
			rgba(255, 255, 255, 0.04) 80%,
			transparent 100%
		);
		animation: skeleton-shimmer 1.8s ease-in-out infinite;
	}

	@keyframes skeleton-shimmer {
		0% {
			transform: translateX(-100%);
		}
		100% {
			transform: translateX(100%);
		}
	}

	.skeleton--text {
		border-radius: var(--radius-default);
	}

	.skeleton--circle {
		border-radius: var(--radius-full);
	}

	.skeleton--card {
		border-radius: var(--radius-xl);
		border: 1px solid rgba(255, 255, 255, 0.05);
	}

	.skeleton--rect {
		border-radius: var(--radius-lg);
	}
</style>
