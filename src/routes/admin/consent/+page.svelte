<!--
  CONSENT-07 admin index — navigation hub across the 5 sub-surfaces:
  banners, categories, services, policies, log (+ integrity).
-->
<script lang="ts">
	const sections = [
		{
			href: '/admin/consent/banner',
			title: 'Banners',
			description: 'Region × locale banner copy + live preview at the 9 PE7 breakpoints.'
		},
		{
			href: '/admin/consent/categories',
			title: 'Categories',
			description:
				'Consent-category catalogue. `necessary` is protected; reorder others via sort_order.'
		},
		{
			href: '/admin/consent/services',
			title: 'Services',
			description:
				'Third-party services grouped under a category (GA, Meta Pixel, Stripe, …).'
		},
		{
			href: '/admin/consent/policies',
			title: 'Policies',
			description:
				'Versioned markdown privacy policy. Creating a new version forces re-consent.'
		},
		{
			href: '/admin/consent/log',
			title: 'Log + integrity',
			description:
				'Read-only view of consent_records (CONSENT-03) + tamper-evident anchors.'
		}
	] as const;
</script>

<svelte:head>
	<title>Consent · Admin</title>
</svelte:head>

<div class="wrap">
	<header class="head">
		<h1>Consent</h1>
		<p class="sub">
			Banner + category + service + policy configuration, plus the append-only consent log.
		</p>
	</header>

	<div class="grid">
		{#each sections as s (s.href)}
			<a class="card" href={s.href}>
				<h2>{s.title}</h2>
				<p>{s.description}</p>
			</a>
		{/each}
	</div>
</div>

<style>
	.wrap {
		display: flex;
		flex-direction: column;
		gap: var(--space-6);
	}
	.head h1 {
		margin: 0;
		font-family: var(--font-heading);
		font-size: var(--fs-2xl);
		font-weight: var(--w-bold);
		color: var(--surface-fg-default);
	}
	.sub {
		margin-block-start: var(--space-2);
		margin-block-end: 0;
		color: var(--surface-fg-muted);
	}
	.grid {
		display: grid;
		gap: var(--space-4);
		grid-template-columns: repeat(auto-fill, minmax(18rem, 1fr));
	}
	.card {
		display: block;
		padding: var(--space-5);
		border: 1px solid var(--surface-border-subtle);
		border-radius: var(--radius-lg);
		background-color: var(--surface-bg-canvas);
		color: var(--surface-fg-default);
		text-decoration: none;
		transition: border-color var(--duration-150) var(--ease-out);
	}
	.card:hover {
		border-color: var(--brand-teal-500);
	}
	.card h2 {
		margin: 0;
		font-size: var(--fs-lg);
		font-weight: var(--w-semibold);
	}
	.card p {
		margin-block-start: var(--space-2);
		margin-block-end: 0;
		font-size: var(--fs-sm);
		color: var(--surface-fg-muted);
	}
</style>
