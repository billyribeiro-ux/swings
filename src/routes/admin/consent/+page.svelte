<!--
  CONSENT-07 admin index — navigation hub across the 5 sub-surfaces:
  banners, categories, services, policies, log (+ integrity).
-->
<script lang="ts">
	import ShieldCheckIcon from 'phosphor-svelte/lib/ShieldCheckIcon';
	import MegaphoneIcon from 'phosphor-svelte/lib/MegaphoneIcon';
	import StackIcon from 'phosphor-svelte/lib/StackIcon';
	import PlugIcon from 'phosphor-svelte/lib/PlugIcon';
	import FileTextIcon from 'phosphor-svelte/lib/FileTextIcon';
	import ClockIcon from 'phosphor-svelte/lib/ClockIcon';
	import ArrowRightIcon from 'phosphor-svelte/lib/ArrowRightIcon';

	const sections = [
		{
			href: '/admin/consent/banner',
			icon: MegaphoneIcon,
			eyebrow: 'Surface',
			title: 'Banners',
			description: 'Region × locale banner copy with live preview at the 9 PE7 breakpoints.'
		},
		{
			href: '/admin/consent/categories',
			icon: StackIcon,
			eyebrow: 'Catalogue',
			title: 'Categories',
			description: 'Consent-category catalogue. `necessary` is protected; reorder others via sort_order.'
		},
		{
			href: '/admin/consent/services',
			icon: PlugIcon,
			eyebrow: 'Integrations',
			title: 'Services',
			description: 'Third-party services grouped under a category (GA, Meta Pixel, Stripe, …).'
		},
		{
			href: '/admin/consent/policies',
			icon: FileTextIcon,
			eyebrow: 'Legal',
			title: 'Policies',
			description: 'Versioned markdown privacy policy. New versions force re-consent.'
		},
		{
			href: '/admin/consent/log',
			icon: ClockIcon,
			eyebrow: 'Audit',
			title: 'Log + integrity',
			description: 'Read-only view of consent_records (CONSENT-03) plus tamper-evident anchors.'
		}
	] as const;
</script>

<svelte:head>
	<title>Consent · Admin</title>
</svelte:head>

<div class="consent-hub">
	<header class="consent-hub__header">
		<div class="consent-hub__title-row">
			<ShieldCheckIcon size={28} weight="duotone" />
			<div class="consent-hub__copy">
				<h1 class="consent-hub__title">Consent</h1>
				<p class="consent-hub__subtitle">
					Banner, category, service, and policy configuration plus the append-only consent log.
				</p>
			</div>
		</div>
	</header>

	<div class="consent-hub__grid">
		{#each sections as s (s.href)}
			<a class="consent-card" href={s.href}>
				<div class="consent-card__head">
					<div class="consent-card__icon">
						<s.icon size={20} weight="duotone" />
					</div>
					<span class="consent-card__eyebrow">{s.eyebrow}</span>
				</div>
				<h2 class="consent-card__title">{s.title}</h2>
				<p class="consent-card__desc">{s.description}</p>
				<span class="consent-card__cta">
					Open
					<ArrowRightIcon size={14} weight="bold" />
				</span>
			</a>
		{/each}
	</div>
</div>

<style>
	.consent-hub {
		max-width: 80rem;
		padding: 0 0 3rem;
	}
	.consent-hub__header {
		margin-bottom: 1.5rem;
	}
	.consent-hub__title-row {
		display: flex;
		align-items: flex-start;
		gap: 0.85rem;
		color: var(--color-white);
	}
	.consent-hub__copy {
		min-width: 0;
	}
	.consent-hub__title {
		margin: 0;
		font-family: var(--font-heading);
		font-size: 1.5rem;
		font-weight: 700;
		color: var(--color-white);
		letter-spacing: -0.01em;
		line-height: 1.15;
	}
	.consent-hub__subtitle {
		margin: 0.35rem 0 0;
		font-size: 0.875rem;
		color: var(--color-grey-400);
		max-width: 60ch;
		line-height: 1.55;
	}
	.consent-hub__grid {
		display: grid;
		gap: 0.85rem;
		grid-template-columns: 1fr;
	}
	.consent-card {
		display: flex;
		flex-direction: column;
		gap: 0.6rem;
		padding: 1.25rem;
		background: var(--color-navy-mid);
		border: 1px solid rgba(255, 255, 255, 0.06);
		border-radius: var(--radius-xl);
		text-decoration: none;
		color: inherit;
		box-shadow:
			0 1px 0 rgba(255, 255, 255, 0.03) inset,
			0 12px 32px rgba(0, 0, 0, 0.18);
		transition:
			transform 200ms var(--ease-out),
			border-color 200ms var(--ease-out);
	}
	.consent-card:hover {
		transform: translateY(-2px);
		border-color: rgba(15, 164, 175, 0.4);
	}
	.consent-card__head {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 0.5rem;
	}
	.consent-card__icon {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		width: 2.25rem;
		height: 2.25rem;
		border-radius: var(--radius-lg);
		background: rgba(15, 164, 175, 0.15);
		color: var(--color-teal);
	}
	.consent-card__eyebrow {
		font-size: 0.6875rem;
		font-weight: 700;
		color: var(--color-grey-500);
		text-transform: uppercase;
		letter-spacing: 0.06em;
	}
	.consent-card__title {
		margin: 0;
		font-family: var(--font-heading);
		font-size: 1rem;
		font-weight: 600;
		color: var(--color-white);
	}
	.consent-card__desc {
		margin: 0;
		font-size: 0.875rem;
		color: var(--color-grey-400);
		line-height: 1.55;
	}
	.consent-card__cta {
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
		.consent-hub__grid {
			grid-template-columns: repeat(2, minmax(0, 1fr));
		}
	}
	@media (min-width: 768px) {
		.consent-card {
			padding: 1.75rem;
			border-radius: var(--radius-2xl);
		}
	}
	@media (min-width: 1024px) {
		.consent-hub__grid {
			grid-template-columns: repeat(3, minmax(0, 1fr));
		}
	}
</style>
