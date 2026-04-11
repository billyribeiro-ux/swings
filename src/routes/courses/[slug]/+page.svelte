<script lang="ts">
	import { onMount } from 'svelte';
	import { gsap } from 'gsap';
	import { createCinematicCascade, EASE, DURATION } from '$lib/utils/animations';
	import Button from '$lib/components/ui/Button.svelte';
	import ScrollReveal from '$lib/components/ui/ScrollReveal.svelte';
	import Seo from '$lib/seo/Seo.svelte';
	import { courseSchema, buildJsonLd } from '$lib/seo/jsonld';
	import CheckCircle from 'phosphor-svelte/lib/CheckCircle';
	import ArrowRight from 'phosphor-svelte/lib/ArrowRight';
	import ArrowLeft from 'phosphor-svelte/lib/ArrowLeft';
	import Clock from 'phosphor-svelte/lib/Clock';
	import GraduationCap from 'phosphor-svelte/lib/GraduationCap';
	import PlayCircle from 'phosphor-svelte/lib/PlayCircle';
	import BookOpen from 'phosphor-svelte/lib/BookOpen';
	import Pulse from 'phosphor-svelte/lib/Pulse';

	import type { PageProps } from './$types';

	let { data }: PageProps = $props();

	const iconMap: Record<string, typeof BookOpen> = { BookOpen, Pulse };

	const course = $derived(data.course);
	const Icon = $derived(iconMap[course.icon]);

	const jsonLd = $derived(
		buildJsonLd([
			courseSchema({
				name: course.title,
				description: course.description,
				slug: course.slug,
				level: course.level,
				duration: course.duration,
				modules: course.modules
			})
		])
	);

	let heroRef: HTMLElement | undefined = $state();

	onMount(() => {
		if (!heroRef) return;

		const ctx = gsap.context(() => {
			createCinematicCascade(heroRef!, [
				{
					selector: '.cd-back',
					duration: DURATION.fast,
					ease: EASE.soft,
					y: 16,
					blur: 4,
					scale: 0.95,
					overlap: 0
				},
				{
					selector: '.cd-badge',
					duration: DURATION.fast,
					ease: EASE.snappy,
					y: 20,
					blur: 6,
					scale: 0.9,
					overlap: 0.5
				},
				{
					selector: '.cd-title',
					duration: DURATION.cinematic,
					ease: EASE.cinematic,
					y: 36,
					blur: 10,
					scale: 0.95,
					overlap: 0.6
				},
				{
					selector: '.cd-desc',
					duration: DURATION.slow,
					ease: EASE.soft,
					y: 28,
					blur: 8,
					scale: 0.98,
					overlap: 0.6
				},
				{
					selector: '.cd-meta',
					duration: DURATION.normal,
					ease: EASE.snappy,
					y: 20,
					blur: 4,
					scale: 0.97,
					overlap: 0.55
				},
				{
					selector: '.cd-price',
					duration: DURATION.normal,
					ease: EASE.cinematic,
					y: 24,
					blur: 6,
					scale: 0.96,
					overlap: 0.55
				},
				{
					selector: '.cd-cta',
					duration: DURATION.normal,
					ease: EASE.snappy,
					y: 20,
					blur: 6,
					scale: 0.96,
					overlap: 0.5
				},
				{
					selector: '.cd-icon-box',
					duration: DURATION.slow,
					ease: EASE.back,
					y: 30,
					blur: 10,
					scale: 0.85,
					overlap: 0.6
				}
			]);
		}, heroRef as HTMLElement);

		return () => ctx.revert();
	});
</script>

<Seo
	title="{course.title} - Explosive Swings"
	description={course.description}
	ogTitle="{course.title} - Options Trading Course"
	{jsonLd}
/>

<!-- Hero Section -->
<section
	bind:this={heroRef}
	class="cd-hero"
	style="background: linear-gradient(145deg, {course.gradient.from} 0%, {course.gradient.to} 100%);"
>
	<div class="cd-hero__grid-overlay"></div>

	<div class="cd-hero__inner">
		<!-- Back Link -->
		<a href="/courses" class="cd-back cd-hero__back">
			<ArrowLeft size={16} weight="bold" />
			All Courses
		</a>

		<div class="cd-hero__layout">
			<!-- Left Column -->
			<div>
				<div class="cd-badge cd-hero__badge">
					<GraduationCap size={16} weight="bold" color="white" />
					<span class="cd-hero__badge-text">{course.level}</span>
				</div>

				<h1 class="cd-title cd-hero__title">{course.title}</h1>

				<p class="cd-desc cd-hero__desc">{course.description}</p>

				<div class="cd-meta cd-hero__meta">
					<div class="cd-hero__meta-item">
						<Clock size={18} weight="bold" />
						<span>{course.duration}</span>
					</div>
					<div class="cd-hero__meta-item">
						<PlayCircle size={18} weight="bold" />
						<span>{course.modules} modules</span>
					</div>
					<div class="cd-hero__meta-item">
						{#if Icon}
							<Icon size={18} weight="bold" />
						{/if}
						<span>Self-paced</span>
					</div>
				</div>

				<div class="cd-price cd-hero__price-row">
					<span class="cd-hero__price">${course.price}</span>
					<span class="cd-hero__price-label">one-time</span>
				</div>

				<div class="cd-cta">
					<Button variant="primary" href="#enroll">
						Enroll Now
						<ArrowRight size={18} weight="bold" />
					</Button>
				</div>
			</div>

			<!-- Right Column - Icon -->
			<div class="cd-icon-box cd-hero__icon-col">
				<div class="cd-hero__icon-box">
					{#if Icon}
						<Icon size={100} weight="duotone" color="white" />
					{:else}
						<BookOpen size={100} weight="duotone" color="white" />
					{/if}
				</div>
			</div>
		</div>
	</div>
</section>

<!-- What You'll Learn -->
<section class="page-section page-section--white">
	<div class="page-container">
		<ScrollReveal>
			<h2 class="page-section__heading page-section__heading--center">What You'll Learn</h2>

			<div class="learn-grid">
				{#each course.whatYouLearn as item, i}
					<div class="reveal-item learn-grid__item" style="transition-delay: {i * 0.06}s">
						<CheckCircle size={22} weight="fill" color="#0FA4AF" class="learn-grid__icon" />
						<p class="learn-grid__text">{item}</p>
					</div>
				{/each}
			</div>
		</ScrollReveal>
	</div>
</section>

<!-- Curriculum -->
<section class="page-section page-section--off-white">
	<div class="page-container">
		<ScrollReveal>
			<h2 class="page-section__heading page-section__heading--center">Course Curriculum</h2>

			<div class="curriculum">
				{#each course.curriculum as module, i}
					<div class="reveal-item curriculum__module" style="transition-delay: {i * 0.06}s">
						<div class="curriculum__module-header">
							<h3 class="curriculum__module-title">{module.title}</h3>
						</div>
						<ul class="curriculum__lesson-list">
							{#each module.lessons as lesson}
								<li class="curriculum__lesson">
									<PlayCircle
										size={18}
										weight="fill"
										color="#0FA4AF"
										class="curriculum__lesson-icon"
									/>
									<span>{lesson}</span>
								</li>
							{/each}
						</ul>
					</div>
				{/each}
			</div>
		</ScrollReveal>
	</div>
</section>

<!-- Features -->
<section class="page-section page-section--white">
	<div class="page-container">
		<ScrollReveal>
			<h2 class="page-section__heading page-section__heading--center">What's Included</h2>

			<div class="features-grid">
				{#each course.features as feature, i}
					<div class="reveal-item features-grid__item" style="transition-delay: {i * 0.06}s">
						<CheckCircle size={22} weight="fill" color="#0FA4AF" class="features-grid__icon" />
						<p class="features-grid__text">{feature}</p>
					</div>
				{/each}
			</div>
		</ScrollReveal>
	</div>
</section>

<!-- Enroll CTA -->
<section id="enroll" class="page-section page-section--dark">
	<div class="page-container page-container--center">
		<ScrollReveal>
			<h2 class="reveal-item page-dark-heading">Ready to Start Learning?</h2>

			<p class="reveal-item page-dark-subtitle">
				Get lifetime access to {course.title} for a one-time payment of ${course.price}.
			</p>

			<div class="reveal-item page-cta__actions">
				<Button variant="primary" href="#">
					Enroll Now -- ${course.price}
					<ArrowRight size={18} weight="bold" />
				</Button>
				<Button variant="ghost" href="/courses">View All Courses</Button>
			</div>
		</ScrollReveal>
	</div>
</section>

<style>
	/* Course detail hero */
	.cd-hero {
		position: relative;
		overflow: hidden;
		padding-top: 4rem;
	}

	.cd-hero__grid-overlay {
		position: absolute;
		inset: 0;
		opacity: 0.05;
		background-image:
			linear-gradient(to right, white 1px, transparent 1px),
			linear-gradient(to bottom, white 1px, transparent 1px);
		background-size: 60px 60px;
	}

	.cd-hero__inner {
		position: relative;
		z-index: var(--z-10);
		max-width: var(--container-max);
		margin: 0 auto;
		padding: 3rem 1rem;
	}

	@media (min-width: 640px) {
		.cd-hero__inner {
			padding: 4rem 1.5rem;
		}
	}
	@media (min-width: 1024px) {
		.cd-hero__inner {
			padding: 6rem 2rem;
		}
	}

	.cd-hero__back {
		display: inline-flex;
		align-items: center;
		gap: 0.5rem;
		border-radius: var(--radius-lg);
		padding: 0.5rem 0.75rem;
		font-size: var(--fs-sm);
		font-weight: var(--w-medium);
		color: rgba(255, 255, 255, 0.7);
		transition: all 200ms var(--ease-out);
		margin-bottom: 2rem;
	}

	.cd-hero__back:hover {
		background-color: rgba(255, 255, 255, 0.1);
		color: var(--color-white);
	}

	.cd-hero__layout {
		display: grid;
		gap: 2.5rem;
		align-items: center;
	}

	@media (min-width: 1024px) {
		.cd-hero__layout {
			grid-template-columns: 1fr auto;
			gap: 4rem;
		}
	}

	.cd-hero__badge {
		display: inline-flex;
		align-items: center;
		gap: 0.5rem;
		border-radius: var(--radius-full);
		border: 1px solid rgba(255, 255, 255, 0.25);
		background-color: rgba(255, 255, 255, 0.15);
		padding: 0.375rem 1rem;
		margin-bottom: 1.25rem;
		backdrop-filter: blur(4px);
	}

	.cd-hero__badge-text {
		font-size: 11px;
		font-weight: var(--w-semibold);
		letter-spacing: 0.05em;
		text-transform: uppercase;
		color: var(--color-white);
	}

	.cd-hero__title {
		font-family: var(--font-heading);
		font-size: var(--fs-3xl);
		font-weight: var(--w-bold);
		color: var(--color-white);
		line-height: 1.15;
		margin-bottom: 1.25rem;
	}

	@media (min-width: 640px) {
		.cd-hero__title {
			font-size: var(--fs-4xl);
		}
	}
	@media (min-width: 768px) {
		.cd-hero__title {
			font-size: clamp(2.5rem, 5vw, 3rem);
		}
	}
	@media (min-width: 1024px) {
		.cd-hero__title {
			font-size: 3.5rem;
		}
	}

	.cd-hero__desc {
		max-width: 36rem;
		font-size: 1rem;
		line-height: 1.65;
		color: rgba(255, 255, 255, 0.85);
		margin-bottom: 2rem;
	}

	@media (min-width: 640px) {
		.cd-hero__desc {
			font-size: var(--fs-lg);
		}
	}

	.cd-hero__meta {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: 1.25rem;
		font-size: var(--fs-sm);
		color: rgba(255, 255, 255, 0.7);
		margin-bottom: 2rem;
	}

	.cd-hero__meta-item {
		display: flex;
		align-items: center;
		gap: 0.5rem;
	}

	.cd-hero__price-row {
		display: flex;
		align-items: baseline;
		gap: 0.5rem;
		margin-bottom: 2rem;
	}

	.cd-hero__price {
		font-family: var(--font-heading);
		font-size: clamp(2.25rem, 5vw, 3rem);
		font-weight: var(--w-bold);
		color: var(--color-white);
	}

	.cd-hero__price-label {
		font-size: 1rem;
		color: rgba(255, 255, 255, 0.6);
	}

	.cd-hero__icon-col {
		display: none;
	}

	@media (min-width: 1024px) {
		.cd-hero__icon-col {
			display: flex;
		}
	}

	.cd-hero__icon-box {
		display: flex;
		align-items: center;
		justify-content: center;
		width: 14rem;
		height: 14rem;
		border-radius: 1.5rem;
		border: 1px solid rgba(255, 255, 255, 0.15);
		background-color: rgba(255, 255, 255, 0.08);
		backdrop-filter: blur(4px);
	}

	@media (min-width: 1280px) {
		.cd-hero__icon-box {
			width: 16rem;
			height: 16rem;
		}
	}

	/* Shared page patterns */
	.page-section {
		padding: 4rem 0;
	}
	@media (min-width: 640px) {
		.page-section {
			padding: 5rem 0;
		}
	}
	@media (min-width: 1024px) {
		.page-section {
			padding: 7rem 0;
		}
	}

	.page-section--white {
		background-color: var(--color-white);
	}
	.page-section--off-white {
		background-color: var(--color-off-white);
	}
	.page-section--dark {
		background: linear-gradient(
			to bottom right,
			var(--color-navy),
			var(--color-navy-mid),
			var(--color-deep-blue)
		);
	}

	.page-container {
		max-width: var(--container-max);
		margin: 0 auto;
		padding: 0 1rem;
	}

	@media (min-width: 640px) {
		.page-container {
			padding: 0 1.5rem;
		}
	}
	@media (min-width: 1024px) {
		.page-container {
			padding: 0 2rem;
		}
	}

	.page-container--center {
		text-align: center;
	}

	.page-section__heading {
		color: var(--color-navy);
		font-family: var(--font-heading);
		font-size: var(--fs-2xl);
		font-weight: var(--w-bold);
		margin-bottom: 2.5rem;
	}

	@media (min-width: 640px) {
		.page-section__heading {
			font-size: var(--fs-3xl);
		}
	}
	@media (min-width: 768px) {
		.page-section__heading {
			font-size: var(--fs-4xl);
		}
	}

	.page-section__heading--center {
		text-align: center;
	}

	/* Learn grid */
	.learn-grid {
		max-width: 56rem;
		margin: 0 auto;
		display: grid;
		gap: 1rem;
	}

	@media (min-width: 640px) {
		.learn-grid {
			grid-template-columns: repeat(2, 1fr);
			gap: 1.25rem;
		}
	}

	.learn-grid__item {
		display: flex;
		gap: 1rem;
		border: 1px solid rgba(216, 220, 228, 0.8);
		background-color: var(--color-off-white);
		border-radius: var(--radius-xl);
		padding: 1.25rem;
	}

	@media (min-width: 640px) {
		.learn-grid__item {
			padding: 1.5rem;
		}
	}

	:global(.learn-grid__icon) {
		flex-shrink: 0;
		margin-top: 0.125rem;
	}

	.learn-grid__text {
		color: var(--color-grey-800);
		font-size: var(--fs-sm);
		line-height: 1.65;
	}

	@media (min-width: 640px) {
		.learn-grid__text {
			font-size: 1rem;
		}
	}

	/* Curriculum */
	.curriculum {
		max-width: 48rem;
		margin: 0 auto;
		display: flex;
		flex-direction: column;
		gap: 1rem;
	}

	@media (min-width: 640px) {
		.curriculum {
			gap: 1.25rem;
		}
	}

	.curriculum__module {
		border: 1px solid rgba(216, 220, 228, 0.8);
		overflow: hidden;
		border-radius: var(--radius-xl);
		background-color: var(--color-white);
	}

	.curriculum__module-header {
		border-bottom: 1px solid var(--color-grey-100);
		padding: 1.25rem 1.5rem;
	}

	@media (min-width: 640px) {
		.curriculum__module-header {
			padding: 1.5rem 2rem;
		}
	}

	.curriculum__module-title {
		color: var(--color-navy);
		font-family: var(--font-heading);
		font-size: 1rem;
		font-weight: var(--w-bold);
	}

	@media (min-width: 640px) {
		.curriculum__module-title {
			font-size: var(--fs-lg);
		}
	}

	.curriculum__lesson-list > * + * {
		border-top: 1px solid var(--color-grey-100);
	}

	.curriculum__lesson {
		display: flex;
		align-items: center;
		gap: 0.75rem;
		color: var(--color-grey-700);
		font-size: var(--fs-sm);
		padding: 0.875rem 1.5rem;
	}

	@media (min-width: 640px) {
		.curriculum__lesson {
			padding: 0.875rem 2rem;
			font-size: 1rem;
		}
	}

	:global(.curriculum__lesson-icon) {
		flex-shrink: 0;
	}

	/* Features grid */
	.features-grid {
		max-width: 56rem;
		margin: 0 auto;
		display: grid;
		gap: 1rem;
	}

	@media (min-width: 640px) {
		.features-grid {
			grid-template-columns: repeat(2, 1fr);
			gap: 1.25rem;
		}
	}
	@media (min-width: 1024px) {
		.features-grid {
			grid-template-columns: repeat(3, 1fr);
		}
	}

	.features-grid__item {
		display: flex;
		gap: 0.75rem;
		border: 1px solid rgba(216, 220, 228, 0.8);
		background-color: var(--color-off-white);
		border-radius: var(--radius-xl);
		padding: 1.25rem;
	}

	:global(.features-grid__icon) {
		flex-shrink: 0;
	}

	.features-grid__text {
		color: var(--color-grey-800);
		font-size: var(--fs-sm);
		line-height: 1.65;
	}

	/* Dark CTA */
	.page-dark-heading {
		font-family: var(--font-heading);
		font-size: var(--fs-2xl);
		font-weight: var(--w-bold);
		color: var(--color-white);
		margin-bottom: 1.25rem;
	}

	@media (min-width: 640px) {
		.page-dark-heading {
			font-size: var(--fs-3xl);
		}
	}
	@media (min-width: 768px) {
		.page-dark-heading {
			font-size: var(--fs-4xl);
		}
	}
	@media (min-width: 1024px) {
		.page-dark-heading {
			font-size: clamp(2.5rem, 4vw, 3rem);
		}
	}

	.page-dark-subtitle {
		color: var(--color-grey-300);
		max-width: 42rem;
		margin: 0 auto 2.5rem;
		font-size: 1rem;
		line-height: 1.65;
	}

	@media (min-width: 640px) {
		.page-dark-subtitle {
			font-size: var(--fs-lg);
		}
	}

	.page-cta__actions {
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 1rem;
	}

	@media (min-width: 640px) {
		.page-cta__actions {
			flex-direction: row;
			justify-content: center;
		}
	}
</style>
