<script lang="ts">
	import { type Component } from 'svelte';

	interface Props {
		icon?: Component;
		title: string;
		description?: string;
		actionLabel?: string;
		actionHref?: string;
		onaction?: () => void;
	}

	let { icon, title, description, actionLabel, actionHref, onaction }: Props = $props();

	function handleActionClick() {
		onaction?.();
	}
</script>

<div class="empty-state">
	{#if icon}
		<div class="empty-state__icon">
			{@render iconContent()}
		</div>
	{/if}

	<h3 class="empty-state__title">{title}</h3>

	{#if description}
		<p class="empty-state__description">{description}</p>
	{/if}

	{#if actionLabel}
		{#if actionHref}
			<!-- eslint-disable-next-line svelte/no-navigation-without-resolve -- actionHref is a caller-supplied prop; resolve() must be applied at the call site -->
			<a href={actionHref} class="empty-state__action">
				{actionLabel}
			</a>
		{:else}
			<button class="empty-state__action" onclick={handleActionClick}>
				{actionLabel}
			</button>
		{/if}
	{/if}
</div>

{#snippet iconContent()}
	{@const IconComponent = icon}
	{#if IconComponent}
		<IconComponent />
	{/if}
{/snippet}

<style>
	.empty-state {
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		text-align: center;
		padding: var(--space-12) var(--space-6);
		animation: empty-state-enter var(--duration-500) var(--ease-out) forwards;
	}

	@keyframes empty-state-enter {
		from {
			opacity: 0;
			transform: translateY(12px);
		}
		to {
			opacity: 1;
			transform: translateY(0);
		}
	}

	.empty-state__icon {
		display: flex;
		align-items: center;
		justify-content: center;
		width: 72px;
		height: 72px;
		border-radius: var(--radius-full);
		background: rgba(15, 164, 175, 0.1);
		color: var(--color-teal);
		margin-bottom: var(--space-6);
	}

	.empty-state__icon :global(svg) {
		width: 32px;
		height: 32px;
	}

	.empty-state__title {
		font-family: var(--font-heading);
		font-size: var(--fs-xl);
		font-weight: var(--w-semibold);
		color: var(--color-white);
		margin: 0 0 var(--space-2) 0;
		line-height: var(--lh-snug);
	}

	.empty-state__description {
		font-family: var(--font-ui);
		font-size: var(--fs-sm);
		color: var(--color-grey-400);
		line-height: var(--lh-relaxed);
		margin: 0 0 var(--space-6) 0;
		max-width: 360px;
	}

	.empty-state__action {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		gap: var(--space-2);
		padding: var(--space-2-5) var(--space-6);
		background: var(--color-teal);
		color: var(--color-white);
		border: none;
		border-radius: var(--radius-xl);
		font-family: var(--font-ui);
		font-weight: var(--w-semibold);
		font-size: var(--fs-sm);
		text-decoration: none;
		cursor: pointer;
		transition: all var(--duration-200) var(--ease-out);
		box-shadow:
			var(--shadow-lg),
			0 4px 14px rgba(15, 164, 175, 0.25);
	}

	.empty-state__action:hover {
		background: var(--color-teal-light);
		transform: translateY(-1px);
		box-shadow:
			var(--shadow-xl),
			0 8px 20px rgba(15, 164, 175, 0.3);
	}

	.empty-state__action:focus-visible {
		outline: none;
		box-shadow:
			0 0 0 2px var(--color-navy),
			0 0 0 4px rgba(15, 164, 175, 0.7);
	}

	.empty-state__action:active {
		transform: scale(0.97);
	}

	@media (max-width: 480px) {
		.empty-state {
			padding: var(--space-8) var(--space-4);
		}

		.empty-state__icon {
			width: 56px;
			height: 56px;
		}

		.empty-state__icon :global(svg) {
			width: 24px;
			height: 24px;
		}
	}
</style>
