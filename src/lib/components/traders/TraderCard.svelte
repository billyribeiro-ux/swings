<script lang="ts">
	import type { Trader } from '$lib/data/traders';
	import { modal } from '$lib/stores/modal.svelte';
	import ArrowRightIcon from 'phosphor-svelte/lib/ArrowRightIcon';
	import { hoverTilt } from '$lib/utils/animations';

	interface Props {
		trader: Trader;
	}

	let { trader }: Props = $props();
</script>

<button 
	onclick={() => modal.showProfile(trader.id)} 
	class="trader-card"
	{@attach hoverTilt({ maxTilt: 10, scale: 1.04 })}
>
	<!-- Avatar -->
	<div
		class="trader-card__avatar"
		style="background: linear-gradient(135deg, {trader.avatarGradient.from} 0%, {trader
			.avatarGradient.to} 100%);"
	>
		{trader.initials}
	</div>

	<!-- Name & Role -->
	<h3 class="trader-card__name">{trader.name}</h3>
	<p class="trader-card__role">{trader.role}</p>

	<!-- Tagline -->
	<p class="trader-card__tagline">{trader.tagline}</p>

	<!-- View Profile Link -->
	<span class="trader-card__link">
		View Profile
		<ArrowRightIcon size={16} />
	</span>
</button>

<style>
	.trader-card {
		text-align: left;
		width: 100%;
		background-color: var(--color-navy-mid);
		border: 1px solid rgba(255, 255, 255, 0.1);
		border-radius: var(--radius-xl);
		padding: 1.5rem;
		transition: all 300ms var(--ease-out);
	}

	.trader-card:hover {
		border-color: rgba(15, 164, 175, 0.5);
		transform: translateY(-0.25rem);
		box-shadow:
			var(--shadow-lg),
			0 4px 14px rgba(15, 164, 175, 0.1);
	}

	.trader-card__avatar {
		width: 4rem;
		height: 4rem;
		border-radius: var(--radius-full);
		display: flex;
		align-items: center;
		justify-content: center;
		font-size: var(--fs-lg);
		font-weight: var(--w-bold);
		color: var(--color-white);
		margin-bottom: 1rem;
	}

	.trader-card__name {
		font-size: var(--fs-xl);
		font-weight: var(--w-bold);
		color: var(--color-white);
		margin-bottom: 0.25rem;
		font-family: var(--font-heading);
	}

	.trader-card__role {
		color: var(--color-teal-light);
		font-size: var(--fs-sm);
		font-weight: var(--w-medium);
		margin-bottom: 0.75rem;
	}

	.trader-card__tagline {
		color: var(--color-grey-400);
		font-size: var(--fs-sm);
		line-height: 1.65;
		margin-bottom: 1rem;
	}

	.trader-card__link {
		display: inline-flex;
		align-items: center;
		gap: 0.25rem;
		color: var(--color-teal);
		font-size: var(--fs-sm);
		font-weight: var(--w-semibold);
		transition: color 200ms var(--ease-out);
	}

	.trader-card:hover .trader-card__link {
		color: var(--color-teal-light);
	}
</style>
