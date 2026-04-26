<script lang="ts">
	import { gsap } from 'gsap';
	import { ScrollTrigger } from 'gsap/ScrollTrigger';
	import { isReducedMotion } from '$lib/utils/animations';
	import SectionHeader from '$lib/components/ui/SectionHeader.svelte';
	import CheckCircleIcon from 'phosphor-svelte/lib/CheckCircleIcon';

	const features = [
		{
			title: '5-7 Top Picks Every Sunday Night',
			description:
				"A detailed watchlist delivered before the week starts - complete with entries, targets, exits, and stops so you're fully prepared before Monday's open."
		},
		{
			title: 'Charts with Marked Zones',
			description:
				'Every alert comes with annotated charts showing entry, invalidation, and profit targets visually.'
		},
		{
			title: 'Entries, Targets, Exits & Stops',
			description:
				'Every pick comes with precise levels - no ambiguity. You know exactly where to get in, where to take profit, and where to cut it.'
		},
		{
			title: 'Detailed Weekly Video Walkthrough',
			description:
				'A thorough breakdown of every pick on the watchlist - so you understand the setup, the levels, and can set your alerts ahead of time with confidence.'
		},
		{
			title: 'Multi-Channel Delivery',
			description:
				'Watchlist delivered via email, SMS, and your members-only dashboard every Sunday night - so you never miss a week.',
			fullWidth: true
		}
	];

	let sectionRef: HTMLElement | undefined = $state();
	$effect(() => {
		if (!sectionRef || isReducedMotion()) return;
		const section = sectionRef;

		const ctx = gsap.context(() => {
			const itemsRef = [...section.querySelectorAll<HTMLElement>('.what-you-get__item')];
			const checksRef = [...section.querySelectorAll<HTMLElement>('.what-you-get__check')];

			// Animate items with staggered entrance
			itemsRef.forEach((item, i) => {
				gsap.set(item, { opacity: 0, x: -30 });

				const check = checksRef[i];
				if (check) {
					gsap.set(check, { opacity: 0, scale: 0.5 });
				}

				ScrollTrigger.create({
					trigger: item,
					start: 'top 85%',
					once: true,
					onEnter: () => {
						// Item slides in
						gsap.to(item, {
							opacity: 1,
							x: 0,
							duration: 0.7,
							delay: i * 0.1,
							ease: 'power3.out'
						});

						// Check pops in
						if (check) {
							gsap.to(check, {
								opacity: 1,
								scale: 1,
								duration: 0.5,
								delay: i * 0.1 + 0.3,
								ease: 'back.out(2)'
							});
						}
					}
				});
			});
		}, section);

		return () => ctx.revert();
	});
</script>

<section bind:this={sectionRef} class="what-you-get">
	<div class="what-you-get__container">
		<SectionHeader
			eyebrow="What's Included"
			title="Your Sunday Night Watchlist"
			subtitle="Every Sunday night, your watchlist drops with everything you need to trade the week ahead - no guessing, no scrambling Monday morning."
		/>

		<div class="what-you-get__grid">
			{#each features as feature (feature.title)}
				<div class={['what-you-get__item', { 'what-you-get__item--full': feature.fullWidth }]}>
					<span class="what-you-get__check" aria-hidden="true">
						<CheckCircleIcon size={24} weight="duotone" color="#0FA4AF" />
					</span>
					<div>
						<h3 class="what-you-get__title">{feature.title}</h3>
						<p class="what-you-get__desc">{feature.description}</p>
					</div>
				</div>
			{/each}
		</div>
	</div>
</section>

<style>
	.what-you-get {
		background-color: var(--color-white);
		padding: 5rem 0;
	}

	@media (min-width: 1024px) {
		.what-you-get {
			padding: 8rem 0;
		}
	}

	.what-you-get__container {
		max-width: var(--container-max);
		margin: 0 auto;
		padding: 0 1rem;
	}

	@media (min-width: 640px) {
		.what-you-get__container {
			padding: 0 1.5rem;
		}
	}
	@media (min-width: 1024px) {
		.what-you-get__container {
			padding: 0 2rem;
		}
	}

	.what-you-get__grid {
		max-width: 64rem;
		margin: 0 auto;
		display: grid;
		gap: 1.5rem;
	}

	@media (min-width: 768px) {
		.what-you-get__grid {
			grid-template-columns: repeat(2, 1fr);
		}
	}

	.what-you-get__item {
		display: flex;
		gap: 1rem;
		border: 1px solid var(--color-grey-200);
		background-color: var(--color-off-white);
		border-radius: var(--radius-lg);
		padding: 1.5rem;
	}

	@media (min-width: 768px) {
		.what-you-get__item--full {
			grid-column: span 2;
		}
	}

	.what-you-get__check {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		margin-top: 0.125rem;
		width: 1.5rem;
		height: 1.5rem;
		flex-shrink: 0;
		color: #0fa4af;
	}

	.what-you-get__title {
		color: var(--color-navy);
		font-family: var(--font-heading);
		font-size: 1rem;
		font-weight: var(--w-bold);
		margin-bottom: 0.5rem;
	}

	.what-you-get__desc {
		color: var(--color-grey-600);
		font-size: var(--fs-sm);
		line-height: 1.65;
	}
</style>
