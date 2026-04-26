<script lang="ts">
	import { gsap } from 'gsap';
	import { EASE, DURATION, isReducedMotion } from '$lib/utils/animations';

	interface Props {
		eyebrow: string;
		title: string;
		subtitle?: string;
		dark?: boolean;
		animated?: boolean;
	}

	let { eyebrow, title, subtitle, dark = false, animated = true }: Props = $props();

	let headerRef: HTMLElement | undefined = $state();
	let eyebrowRef: HTMLElement | undefined = $state();
	let titleRef: HTMLElement | undefined = $state();
	let subtitleRef: HTMLElement | undefined = $state();
	let lineRef: HTMLElement | undefined = $state();

	$effect(() => {
		if (!animated || !headerRef) return;
		const header = headerRef;

		const reduced = isReducedMotion();

		// Set initial states
		if (eyebrowRef) {
			gsap.set(eyebrowRef, { opacity: 0, x: reduced ? 0 : -30 });
		}
		if (titleRef) {
			gsap.set(titleRef, {
				opacity: 0,
				y: reduced ? 0 : 20,
				filter: reduced ? 'none' : 'blur(4px)'
			});
		}
		if (subtitleRef) {
			gsap.set(subtitleRef, { opacity: 0, y: reduced ? 0 : 15 });
		}
		if (lineRef) {
			gsap.set(lineRef, { scaleX: 0, transformOrigin: 'center' });
		}

		const ctx = gsap.context(() => {
			const tl = gsap.timeline({
				scrollTrigger: {
					trigger: header,
					start: 'top 85%',
					once: true
				}
			});

			// Eyebrow slides in
			if (eyebrowRef) {
				tl.to(
					eyebrowRef,
					{
						opacity: 1,
						x: 0,
						duration: reduced ? 0.01 : DURATION.fast,
						ease: reduced ? 'none' : EASE.snappy
					},
					0
				);
			}

			// Title fades up
			if (titleRef) {
				tl.to(
					titleRef,
					{
						opacity: 1,
						y: 0,
						filter: 'blur(0px)',
						duration: reduced ? 0.01 : DURATION.cinematic,
						ease: reduced ? 'none' : EASE.cinematic
					},
					reduced ? 0 : 0.15
				);
			}

			// Subtitle follows
			if (subtitleRef) {
				tl.to(
					subtitleRef,
					{
						opacity: 1,
						y: 0,
						duration: reduced ? 0.01 : DURATION.slow,
						ease: reduced ? 'none' : EASE.soft
					},
					reduced ? 0 : 0.35
				);
			}

			// Decorative line draws
			if (lineRef) {
				tl.to(
					lineRef,
					{
						scaleX: 1,
						duration: reduced ? 0.01 : DURATION.normal,
						ease: reduced ? 'none' : EASE.snappy
					},
					reduced ? 0 : 0.5
				);
			}
		}, header);

		return () => ctx.revert();
	});
</script>

<div bind:this={headerRef} class="section-header" data-dark={dark || undefined}>
	<span bind:this={eyebrowRef} class="section-header__eyebrow hero-eyebrow">{eyebrow}</span>
	<h2 bind:this={titleRef} class="section-header__title">
		<!-- eslint-disable-next-line svelte/no-at-html-tags -- titles are hard-coded in callers with intentional <br/> and <strong> markup -->
		{@html title}
	</h2>
	{#if subtitle}
		<p bind:this={subtitleRef} class="section-header__subtitle">
			{subtitle}
		</p>
	{/if}
	<div bind:this={lineRef} class="section-header__line"></div>
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

	.section-header__line {
		width: 60px;
		height: 3px;
		background: linear-gradient(90deg, var(--color-teal), var(--color-teal-light));
		margin: 1.5rem auto 0;
		border-radius: var(--radius-full);
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
