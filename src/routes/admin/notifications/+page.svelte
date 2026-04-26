<!--
  Phase 2.1 — Notifications admin hub. Section card grid linking to
  templates, deliveries, suppression. Mirrors the shape of
  `admin/consent/+page.svelte` so the operator sees a consistent jump-off.
-->
<script lang="ts">
	import { resolve } from '$app/paths';
	import BellIcon from 'phosphor-svelte/lib/BellIcon';
	import EnvelopeIcon from 'phosphor-svelte/lib/EnvelopeIcon';
	import PaperPlaneTiltIcon from 'phosphor-svelte/lib/PaperPlaneTiltIcon';
	import XCircleIcon from 'phosphor-svelte/lib/XCircleIcon';
	import ArrowRightIcon from 'phosphor-svelte/lib/ArrowRightIcon';

	const sections = [
		{
			href: '/admin/notifications/templates',
			icon: EnvelopeIcon,
			eyebrow: 'Catalogue',
			title: 'Email templates',
			description:
				'Versioned transactional and marketing templates. Edit, preview, send a test to any recipient.'
		},
		{
			href: '/admin/notifications/deliveries',
			icon: PaperPlaneTiltIcon,
			eyebrow: 'Activity',
			title: 'Delivery log',
			description:
				'Recent sends with status, provider id, rendered subject, and full delivery payload inspector.'
		},
		{
			href: '/admin/notifications/suppression',
			icon: XCircleIcon,
			eyebrow: 'Hygiene',
			title: 'Suppression list',
			description:
				'Manually suppressed addresses (bounces, complaints, opt-outs). Add or remove with audit trail.'
		}
	] as const;
</script>

<svelte:head>
	<title>Notifications · Admin</title>
</svelte:head>

<div class="page" data-testid="admin-notifications-hub">
	<header class="page__header">
		<div class="page__title-row">
			<BellIcon size={28} weight="duotone" />
			<div class="page__copy">
				<span class="eyebrow">Operations</span>
				<h1 class="page__title">Notifications</h1>
				<p class="page__subtitle">
					Manage transactional + marketing templates, inspect every delivery the platform
					has attempted, and curate the suppression list that gates outbound sends.
				</p>
			</div>
		</div>
	</header>

	<div class="grid">
		{#each sections as s (s.href)}
			<a class="card" href={resolve(s.href)}>
				<div class="card__head">
					<div class="card__icon">
						<s.icon size={20} weight="duotone" />
					</div>
					<span class="card__eyebrow">{s.eyebrow}</span>
				</div>
				<h2 class="card__title">{s.title}</h2>
				<p class="card__desc">{s.description}</p>
				<span class="card__cta">
					Open
					<ArrowRightIcon size={14} weight="bold" />
				</span>
			</a>
		{/each}
	</div>
</div>

<style>
	.page {
		max-width: 80rem;
		padding: 0 0 3rem;
	}
	.page__header {
		margin-bottom: 1.5rem;
	}
	.page__title-row {
		display: flex;
		align-items: flex-start;
		gap: 0.85rem;
		color: var(--color-white);
	}
	.page__copy {
		min-width: 0;
	}
	.eyebrow {
		display: inline-block;
		font-size: 0.6875rem;
		font-weight: 700;
		line-height: 1;
		letter-spacing: 0.08em;
		color: var(--color-grey-500);
		text-transform: uppercase;
		margin-bottom: 0.4rem;
	}
	.page__title {
		margin: 0;
		font-family: var(--font-heading);
		font-size: 1.5rem;
		font-weight: 700;
		color: var(--color-white);
		letter-spacing: -0.01em;
		line-height: 1.2;
	}
	.page__subtitle {
		margin: 0.35rem 0 0;
		font-size: 0.875rem;
		color: var(--color-grey-400);
		max-width: 42rem;
		line-height: 1.5;
	}
	.grid {
		display: grid;
		gap: 0.85rem;
		grid-template-columns: 1fr;
	}
	.card {
		display: flex;
		flex-direction: column;
		gap: 0.6rem;
		padding: 1.25rem;
		background: rgba(19, 43, 80, 0.35);
		backdrop-filter: blur(24px);
		-webkit-backdrop-filter: blur(24px);
		border: 1px solid rgba(255, 255, 255, 0.06);
		border-radius: var(--radius-2xl);
		text-decoration: none;
		color: inherit;
		box-shadow:
			0 1px 0 rgba(255, 255, 255, 0.03) inset,
			0 12px 32px rgba(0, 0, 0, 0.18);
		transition:
			transform 200ms var(--ease-out),
			border-color 200ms var(--ease-out);
	}
	.card:hover {
		transform: translateY(-2px);
		border-color: rgba(15, 164, 175, 0.4);
	}
	.card__head {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 0.5rem;
	}
	.card__icon {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		width: 2.25rem;
		height: 2.25rem;
		border-radius: var(--radius-2xl);
		background: rgba(15, 164, 175, 0.15);
		color: var(--color-teal);
	}
	.card__eyebrow {
		font-size: 0.6875rem;
		font-weight: 700;
		color: var(--color-grey-500);
		text-transform: uppercase;
		letter-spacing: 0.06em;
	}
	.card__title {
		margin: 0;
		font-family: var(--font-heading);
		font-size: 1rem;
		font-weight: 600;
		color: var(--color-white);
	}
	.card__desc {
		margin: 0;
		font-size: 0.875rem;
		color: var(--color-grey-400);
		line-height: 1.55;
	}
	.card__cta {
		display: inline-flex;
		align-items: center;
		gap: 0.35rem;
		margin-top: auto;
		padding-top: 0.5rem;
		font-size: 0.6875rem;
		font-weight: 600;
		color: var(--color-teal);
		text-transform: uppercase;
		letter-spacing: 0.05em;
	}

	@media (min-width: 480px) {
		.grid {
			grid-template-columns: repeat(2, minmax(0, 1fr));
		}
	}
	@media (min-width: 768px) {
		.card {
			padding: 1.75rem;
			border-radius: var(--radius-2xl);
		}
	}
	@media (min-width: 1024px) {
		.grid {
			grid-template-columns: repeat(3, minmax(0, 1fr));
		}
	}
</style>
