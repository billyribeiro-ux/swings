<script lang="ts">
	interface Props {
		eyebrow: string;
		title: string;
		subtitle?: string;
		dark?: boolean;
	}

	let { eyebrow, title, subtitle, dark = false }: Props = $props();
</script>

<div class="section-header" data-dark={dark || undefined}>
	<span class="section-header__eyebrow hero-eyebrow">{eyebrow}</span>
	<h2 class="section-header__title">
		{@html title}
	</h2>
	{#if subtitle}
		<p class="section-header__subtitle">
			{subtitle}
		</p>
	{/if}
</div>

<style>
	.section-header {
		text-align: center;
		max-width: 48rem;
		margin-left: auto;
		margin-right: auto;
		margin-bottom: 3rem;
	}

	.section-header__eyebrow {
		display: block;
		color: var(--color-teal);
		margin-bottom: 1rem;
	}

	.section-header__title {
		font-size: var(--fs-2xl);
		font-weight: var(--w-bold);
		color: var(--color-navy);
		margin-bottom: 1rem;
	}

	.section-header__subtitle {
		font-size: 1rem;
		color: var(--color-grey-600);
		max-width: 42rem;
		margin-left: auto;
		margin-right: auto;
	}

	/* Dark variant */
	.section-header[data-dark] .section-header__title {
		color: var(--color-white);
	}

	.section-header[data-dark] .section-header__subtitle {
		color: var(--color-grey-400);
	}

	@media (min-width: 768px) {
		.section-header__title {
			font-size: var(--fs-3xl);
		}

		.section-header__subtitle {
			font-size: var(--fs-lg);
		}
	}

	@media (min-width: 1024px) {
		.section-header__title {
			font-size: var(--fs-4xl);
		}
	}
</style>
