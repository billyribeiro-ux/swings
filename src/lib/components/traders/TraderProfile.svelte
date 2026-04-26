<script lang="ts">
	import type { Trader } from '$lib/data/traders';
	import { modal } from '$lib/stores/modal.svelte';
	import { safeHtml } from '$lib/utils/safeHtml';
	import ArrowLeftIcon from 'phosphor-svelte/lib/ArrowLeftIcon';
	import StarIcon from 'phosphor-svelte/lib/StarIcon';
	import PulseIcon from 'phosphor-svelte/lib/PulseIcon';
	import BookOpenIcon from 'phosphor-svelte/lib/BookOpenIcon';

	interface Props {
		trader: Trader;
	}

	let { trader }: Props = $props();

	const iconMap = {
		StarIcon,
		PulseIcon,
		BookOpenIcon
	};
</script>

<div class="profile">
	<!-- Back Button -->
	<button onclick={modal.backToGrid} class="profile__back">
		<ArrowLeftIcon size={20} />
		<span>All Traders</span>
	</button>

	<!-- Profile Header -->
	<div class="profile__header">
		<div
			class="profile__avatar"
			style="background: linear-gradient(135deg, {trader.avatarGradient.from} 0%, {trader
				.avatarGradient.to} 100%);"
		>
			{trader.initials}
		</div>
		<div>
			<h2 class="profile__name">{trader.name}</h2>
			<p class="profile__role">{trader.role}</p>
		</div>
	</div>

	<!-- Bio -->
	<div class="profile__bio">
		{#each trader.bio as paragraph (paragraph)}
			<!-- eslint-disable-next-line svelte/no-at-html-tags -- bio content sanitized via DOMPurify in safeHtml() -->
			<p class="profile__bio-text">{@html safeHtml(paragraph)}</p>
		{/each}
	</div>

	<!-- Highlights -->
	<div class="profile__highlights">
		{#each trader.highlights as highlight (highlight.label)}
			<div class="profile__highlight-card">
				<div class="kpi-value profile__highlight-value">{highlight.value}</div>
				<div class="kpi-label profile__highlight-label">{highlight.label}</div>
			</div>
		{/each}
	</div>

	<!-- Action Buttons -->
	<div class="profile__actions">
		{#each trader.actions as action (action.label)}
			{@const IconComponent = iconMap[action.icon as keyof typeof iconMap]}
			<button
				class={[
					'profile__action-btn',
					action.variant === 'primary'
						? 'profile__action-btn--primary'
						: 'profile__action-btn--outline'
				]}
			>
				<IconComponent size={18} weight={action.icon === 'StarIcon' ? 'fill' : 'regular'} />
				{action.label}
			</button>
		{/each}
	</div>
</div>

<style>
	.profile {
		max-width: 56rem;
		margin: 0 auto;
	}

	.profile__back {
		display: inline-flex;
		align-items: center;
		gap: 0.5rem;
		color: var(--color-grey-400);
		transition: color 200ms var(--ease-out);
		margin-bottom: 1.5rem;
	}

	.profile__back:hover {
		color: var(--color-white);
	}

	.profile__header {
		display: flex;
		align-items: center;
		gap: 1.5rem;
		margin-bottom: 2rem;
	}

	.profile__avatar {
		width: 6rem;
		height: 6rem;
		border-radius: var(--radius-full);
		display: flex;
		align-items: center;
		justify-content: center;
		font-size: var(--fs-2xl);
		font-weight: var(--w-bold);
		color: var(--color-white);
	}

	.profile__name {
		font-size: var(--fs-3xl);
		font-weight: var(--w-bold);
		color: var(--color-white);
		font-family: var(--font-heading);
	}

	.profile__role {
		color: var(--color-teal-light);
		font-size: var(--fs-lg);
	}

	.profile__bio {
		display: flex;
		flex-direction: column;
		gap: 1rem;
		margin-bottom: 2rem;
	}

	.profile__bio-text {
		color: var(--color-grey-300);
		line-height: 1.65;
	}

	.profile__highlights {
		display: grid;
		grid-template-columns: repeat(3, 1fr);
		gap: 1rem;
		margin-bottom: 2rem;
	}

	.profile__highlight-card {
		background-color: var(--color-navy-mid);
		border: 1px solid rgba(255, 255, 255, 0.1);
		border-radius: var(--radius-xl);
		padding: 1rem;
		text-align: center;
	}

	.profile__highlight-value {
		color: var(--color-teal-light);
		margin-bottom: 0.25rem;
	}

	.profile__highlight-label {
		color: var(--color-grey-400);
	}

	.profile__actions {
		display: flex;
		flex-wrap: wrap;
		gap: 1rem;
	}

	.profile__action-btn {
		display: inline-flex;
		align-items: center;
		gap: 0.5rem;
		padding: 0.75rem 1.5rem;
		border-radius: var(--radius-lg);
		font-weight: var(--w-semibold);
		font-size: var(--fs-sm);
		transition: all 200ms var(--ease-out);
	}

	.profile__action-btn--primary {
		background-color: var(--color-teal);
		color: var(--color-white);
		border: none;
	}

	.profile__action-btn--primary:hover {
		background-color: var(--color-teal-light);
	}

	.profile__action-btn--outline {
		background-color: transparent;
		border: 1px solid rgba(255, 255, 255, 0.2);
		color: var(--color-white);
	}

	.profile__action-btn--outline:hover {
		background-color: rgba(255, 255, 255, 0.1);
	}
</style>
