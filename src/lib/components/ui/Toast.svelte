<script lang="ts">
	import { toast } from '$lib/stores/toast.svelte.js';

	const iconMap: Record<string, string> = {
		success: '✓',
		error: '✕',
		warning: '⚠',
		info: 'ℹ'
	};

</script>

<div class="toast-container" aria-live="polite" aria-relevant="additions removals">
	{#each toast.toasts as t (t.id)}
		<div class="toast toast--{t.type}" role="alert">
			<div class="toast__icon">
				{iconMap[t.type]}
			</div>
			<p class="toast__message">{t.message}</p>
			<button
				class="toast__close"
				onclick={() => toast.remove(t.id)}
				aria-label="Dismiss notification"
			>
				✕
			</button>
			<div
				class="toast__progress"
				style="animation-duration: {t.duration}ms;"
			></div>
		</div>
	{/each}
</div>

<style>
	.toast-container {
		position: fixed;
		bottom: var(--space-6);
		right: var(--space-6);
		z-index: var(--z-50);
		display: flex;
		flex-direction: column-reverse;
		gap: var(--space-3);
		max-width: 420px;
		width: calc(100vw - var(--space-8));
		pointer-events: none;
	}

	@media (max-width: 480px) {
		.toast-container {
			bottom: var(--space-4);
			right: var(--space-4);
			left: var(--space-4);
			width: auto;
			max-width: none;
		}
	}

	.toast {
		display: flex;
		align-items: flex-start;
		gap: var(--space-3);
		padding: var(--space-4);
		border-radius: var(--radius-xl);
		background: var(--color-navy-mid);
		border: 1px solid rgba(255, 255, 255, 0.08);
		box-shadow: var(--shadow-xl);
		font-family: var(--font-ui);
		color: var(--color-off-white);
		position: relative;
		overflow: hidden;
		pointer-events: auto;
		animation: toast-slide-in var(--duration-300) var(--ease-out) forwards;
	}

	@keyframes toast-slide-in {
		from {
			opacity: 0;
			transform: translateX(100%);
		}
		to {
			opacity: 1;
			transform: translateX(0);
		}
	}

	.toast__icon {
		display: flex;
		align-items: center;
		justify-content: center;
		width: 24px;
		height: 24px;
		border-radius: var(--radius-full);
		font-size: var(--fs-xs);
		font-weight: var(--w-bold);
		flex-shrink: 0;
	}

	.toast--success .toast__icon {
		background: rgba(34, 181, 115, 0.2);
		color: var(--color-green);
	}

	.toast--error .toast__icon {
		background: rgba(224, 72, 72, 0.2);
		color: var(--color-red);
	}

	.toast--warning .toast__icon {
		background: rgba(212, 168, 67, 0.2);
		color: var(--color-gold);
	}

	.toast--info .toast__icon {
		background: rgba(15, 164, 175, 0.2);
		color: var(--color-teal);
	}

	.toast__message {
		flex: 1;
		margin: 0;
		font-size: var(--fs-sm);
		line-height: var(--lh-normal);
		padding-top: 2px;
	}

	.toast__close {
		display: flex;
		align-items: center;
		justify-content: center;
		width: 20px;
		height: 20px;
		padding: 0;
		border: none;
		border-radius: var(--radius-md);
		background: transparent;
		color: var(--color-grey-400);
		cursor: pointer;
		font-size: var(--fs-xs);
		flex-shrink: 0;
		transition: color var(--duration-150) var(--ease-out),
			background var(--duration-150) var(--ease-out);
	}

	.toast__close:hover {
		color: var(--color-white);
		background: rgba(255, 255, 255, 0.1);
	}

	.toast__progress {
		position: absolute;
		bottom: 0;
		left: 0;
		height: 3px;
		width: 100%;
		transform-origin: left;
		animation: toast-progress linear forwards;
	}

	@keyframes toast-progress {
		from {
			transform: scaleX(1);
		}
		to {
			transform: scaleX(0);
		}
	}

	.toast--success .toast__progress {
		background: var(--color-green);
	}

	.toast--error .toast__progress {
		background: var(--color-red);
	}

	.toast--warning .toast__progress {
		background: var(--color-gold);
	}

	.toast--info .toast__progress {
		background: var(--color-teal);
	}
</style>
