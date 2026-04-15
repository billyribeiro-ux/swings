<script lang="ts">
	import { onMount } from 'svelte';
	import { gsap } from 'gsap';
	import { createCinematicCascade, EASE, DURATION } from '$lib/utils/animations';
	import { courses } from '$lib/data/courses';
	import SectionHeader from '$lib/components/ui/SectionHeader.svelte';
	import ScrollReveal from '$lib/components/ui/ScrollReveal.svelte';
	import Seo from '$lib/seo/Seo.svelte';
	import { webPageSchema, courseSchema, buildJsonLd } from '$lib/seo/jsonld';
	import BookOpen from 'phosphor-svelte/lib/BookOpen';
	import Pulse from 'phosphor-svelte/lib/Pulse';
	import ArrowRight from 'phosphor-svelte/lib/ArrowRight';
	import Clock from 'phosphor-svelte/lib/Clock';
	import GraduationCap from 'phosphor-svelte/lib/GraduationCap';

	const jsonLd = buildJsonLd([
		webPageSchema({
			path: '/courses',
			title: 'Options Trading Courses - Structured Learning Paths',
			description:
				'Follow structured options courses that turn market concepts into repeatable execution, risk management habits, and review workflows.',
			speakable: '.courses-title, .courses-subtitle'
		}),
		...courses.map((c) =>
			courseSchema({
				name: c.title,
				description: c.description,
				slug: c.slug,
				level: c.level,
				duration: c.duration,
				modules: c.modules
			})
		)
	]);

	const iconMap: Record<string, typeof BookOpen> = { BookOpen, Pulse };

	let heroRef: HTMLElement | undefined = $state();

	onMount(() => {
		if (!heroRef) return;

		const ctx = gsap.context(() => {
			createCinematicCascade(heroRef!, [
				{
					selector: '.courses-badge',
					duration: DURATION.fast,
					ease: EASE.snappy,
					y: 20,
					blur: 6,
					scale: 0.9,
					overlap: 0
				},
				{
					selector: '.courses-title',
					duration: DURATION.cinematic,
					ease: EASE.cinematic,
					y: 36,
					blur: 10,
					scale: 0.95,
					overlap: 0.6
				},
				{
					selector: '.courses-subtitle',
					duration: DURATION.slow,
					ease: EASE.soft,
					y: 28,
					blur: 8,
					scale: 0.98,
					overlap: 0.6
				}
			]);
		}, heroRef as HTMLElement);

		return () => ctx.revert();
	});
</script>

<Seo
	title="Options Trading Courses - Precision Options Signals"
	description="Follow structured options courses that turn market concepts into repeatable execution, risk management habits, and review workflows."
	ogTitle="Options Trading Courses - Structured Learning Paths"
	{jsonLd}
/>

<!-- Hero Section -->
<section bind:this={heroRef} class="page-hero">
	<div class="page-hero__grid-overlay"></div>

	<div class="page-hero__inner">
		<div class="courses-badge page-badge">
			<GraduationCap size={18} weight="duotone" color="#15C5D1" />
			<span class="page-badge__text">Education</span>
		</div>

		<h1 class="courses-title page-hero__title">Level Up Your Options Game</h1>

		<p class="courses-subtitle page-hero__subtitle">
			Structured courses designed to take you from the basics to confidently trading options at
			your own pace.
		</p>
	</div>
</section>

<!-- Courses Grid -->
<section class="page-section page-section--off-white">
	<div class="page-container page-container--narrow">
		<ScrollReveal>
			<div class="courses-grid">
				{#each courses as course, i}
					{@const Icon = iconMap[course.icon]}
					<a
						href="/courses/{course.slug}"
						class="reveal-item course-card"
						style="transition-delay: {i * 0.08}s"
					>
						<!-- Visual Header -->
						<div
							class="course-card__header"
							style="background: linear-gradient(145deg, {course.gradient.from} 0%, {course.gradient
								.to} 100%);"
						>
							<div class="course-card__header-grid"></div>

							<div class="course-card__level-badge">
								<span class="course-card__level-text">{course.level}</span>
							</div>

							<div class="course-card__icon-box">
								{#if Icon}
									<Icon size={40} weight="duotone" color="white" />
								{:else}
									<BookOpen size={40} weight="duotone" color="white" />
								{/if}
							</div>
						</div>

						<!-- Body -->
						<div class="course-card__body">
							<h3 class="course-card__title">{course.title}</h3>
							<p class="course-card__desc">{course.description}</p>

							<div class="course-card__meta">
								<div class="course-card__meta-item">
									<Clock size={15} weight="bold" class="course-card__meta-icon" />
									<span>{course.duration}</span>
								</div>
								<div class="course-card__meta-item">
									<GraduationCap size={15} weight="bold" class="course-card__meta-icon" />
									<span>{course.modules} modules</span>
								</div>
							</div>

							<div class="course-card__footer">
								<div class="course-card__price-row">
									<span class="course-card__price">${course.price}</span>
									<span class="course-card__price-label">one-time</span>
								</div>
								<span class="course-card__cta-pill">
									Learn More
									<ArrowRight size={16} weight="bold" class="course-card__cta-arrow" />
								</span>
							</div>
						</div>
					</a>
				{/each}
			</div>
		</ScrollReveal>
	</div>
</section>

<!-- Why Take a Course -->
<section class="page-section page-section--white">
	<div class="page-container">
		<ScrollReveal>
			<SectionHeader
				eyebrow="Why Learn Options?"
				title="Build Skills That Last a Lifetime"
				subtitle="Options trading gives you leverage, flexibility, and the ability to profit in any market condition - but only if you know what you're doing."
			/>

			<div class="why-grid">
				<div class="reveal-item why-item">
					<div class="why-item__icon">
						<BookOpen size={28} weight="duotone" color="#0FA4AF" />
					</div>
					<h3 class="why-item__title">Learn at Your Pace</h3>
					<p class="why-item__desc">
						Lifetime access means you can revisit lessons whenever you need a refresher.
					</p>
				</div>

				<div class="reveal-item why-item">
					<div class="why-item__icon">
						<Pulse size={28} weight="duotone" color="#0FA4AF" />
					</div>
					<h3 class="why-item__title">Real-World Examples</h3>
					<p class="why-item__desc">
						Every lesson includes actual chart examples and trade setups you can apply immediately.
					</p>
				</div>

				<div class="reveal-item why-item">
					<div class="why-item__icon">
						<GraduationCap size={28} weight="duotone" color="#0FA4AF" />
					</div>
					<h3 class="why-item__title">Community Support</h3>
					<p class="why-item__desc">
						Join a private community of traders learning the same strategies.
					</p>
				</div>
			</div>
		</ScrollReveal>
	</div>
</section>

<style>
	/* Page hero */
	.page-hero {
		position: relative;
		overflow: hidden;
		padding-top: 4rem;
		background: linear-gradient(
			to bottom right,
			var(--color-navy),
			var(--color-navy-mid),
			var(--color-deep-blue)
		);
	}

	.page-hero__grid-overlay {
		position: absolute;
		inset: 0;
		opacity: 0.02;
		background-image:
			linear-gradient(to right, white 1px, transparent 1px),
			linear-gradient(to bottom, white 1px, transparent 1px);
		background-size: 60px 60px;
	}

	.page-hero__inner {
		position: relative;
		z-index: var(--z-10);
		max-width: var(--container-max);
		margin: 0 auto;
		padding: 5rem 1rem;
		text-align: center;
	}

	@media (min-width: 640px) {
		.page-hero__inner {
			padding: 5rem 1.5rem;
		}
	}
	@media (min-width: 1024px) {
		.page-hero__inner {
			padding: 7rem 2rem;
		}
	}

	.page-badge {
		display: inline-flex;
		align-items: center;
		gap: 0.5rem;
		border-radius: var(--radius-full);
		border: 1px solid rgba(15, 164, 175, 0.3);
		background-color: rgba(15, 164, 175, 0.1);
		padding: 0.5rem 1rem;
		margin-bottom: 1.5rem;
	}

	.page-badge__text {
		color: var(--color-teal-light);
		font-size: var(--fs-xs);
		font-weight: var(--w-semibold);
		letter-spacing: 0.05em;
		text-transform: uppercase;
	}

	.page-hero__title {
		font-family: var(--font-heading);
		font-size: var(--fs-3xl);
		font-weight: var(--w-bold);
		color: var(--color-white);
		line-height: 1.15;
		margin-bottom: 1.5rem;
	}

	@media (min-width: 640px) {
		.page-hero__title {
			font-size: var(--fs-4xl);
		}
	}
	@media (min-width: 768px) {
		.page-hero__title {
			font-size: clamp(2.5rem, 5vw, 3rem);
		}
	}
	@media (min-width: 1024px) {
		.page-hero__title {
			font-size: clamp(3rem, 5vw, 3.75rem);
		}
	}

	.page-hero__subtitle {
		color: var(--color-grey-300);
		font-size: 1rem;
		line-height: 1.65;
		max-width: 42rem;
		margin: 0 auto;
	}

	@media (min-width: 640px) {
		.page-hero__subtitle {
			font-size: var(--fs-lg);
		}
	}
	@media (min-width: 1024px) {
		.page-hero__subtitle {
			font-size: var(--fs-xl);
		}
	}

	/* Sections */
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

	.page-container--narrow {
		max-width: 60rem;
	}

	/* Courses grid */
	.courses-grid {
		display: grid;
		gap: 1.5rem;
	}

	@media (min-width: 640px) {
		.courses-grid {
			gap: 2rem;
		}
	}
	@media (min-width: 768px) {
		.courses-grid {
			grid-template-columns: repeat(2, 1fr);
		}
	}

	.course-card {
		position: relative;
		display: flex;
		flex-direction: column;
		overflow: hidden;
		border-radius: var(--radius-2xl);
		background-color: var(--color-white);
		box-shadow: var(--shadow-sm);
		outline: 1px solid rgba(216, 220, 228, 0.8);
		outline-offset: -1px;
		transition: all 500ms var(--ease-out);
		text-decoration: none;
	}

	.course-card:hover {
		transform: translateY(-0.25rem);
		box-shadow: var(--shadow-xl);
		outline-color: rgba(15, 164, 175, 0.3);
	}

	.course-card__header {
		position: relative;
		display: flex;
		align-items: center;
		justify-content: center;
		height: 11rem;
		overflow: hidden;
	}

	@media (min-width: 640px) {
		.course-card__header {
			height: 13rem;
		}
	}

	.course-card__header-grid {
		position: absolute;
		inset: 0;
		opacity: 0.06;
		background-image:
			linear-gradient(to right, white 1px, transparent 1px),
			linear-gradient(to bottom, white 1px, transparent 1px);
		background-size: 32px 32px;
	}

	.course-card__level-badge {
		position: absolute;
		top: 1rem;
		left: 1rem;
	}

	.course-card__level-text {
		border-radius: var(--radius-full);
		border: 1px solid rgba(255, 255, 255, 0.25);
		background-color: rgba(255, 255, 255, 0.15);
		padding: 0.25rem 0.75rem;
		font-size: 11px;
		font-weight: var(--w-semibold);
		color: var(--color-white);
		backdrop-filter: blur(4px);
	}

	.course-card__icon-box {
		position: relative;
		z-index: var(--z-10);
		display: flex;
		align-items: center;
		justify-content: center;
		width: 5rem;
		height: 5rem;
		border-radius: var(--radius-2xl);
		border: 1px solid rgba(255, 255, 255, 0.2);
		background-color: rgba(255, 255, 255, 0.1);
		backdrop-filter: blur(4px);
		transition: transform 500ms var(--ease-out);
	}

	.course-card:hover .course-card__icon-box {
		transform: scale(1.05);
	}

	.course-card__body {
		display: flex;
		flex: 1;
		flex-direction: column;
		padding: 1.5rem;
	}

	@media (min-width: 640px) {
		.course-card__body {
			padding: 2rem;
		}
	}

	.course-card__title {
		color: var(--color-navy);
		font-family: var(--font-heading);
		font-size: var(--fs-xl);
		font-weight: var(--w-bold);
		line-height: 1.3;
		margin-bottom: 0.5rem;
	}

	@media (min-width: 640px) {
		.course-card__title {
			font-size: var(--fs-2xl);
		}
	}

	.course-card__desc {
		color: var(--color-grey-600);
		font-size: var(--fs-sm);
		line-height: 1.65;
		margin-bottom: 1.5rem;
	}

	@media (min-width: 640px) {
		.course-card__desc {
			font-size: 1rem;
		}
	}

	.course-card__meta {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: 1rem;
		color: var(--color-grey-500);
		font-size: 13px;
		margin-bottom: 1.5rem;
	}

	.course-card__meta-item {
		display: flex;
		align-items: center;
		gap: 0.375rem;
	}

	:global(.course-card__meta-icon) {
		color: var(--color-grey-400) !important;
	}

	.course-card__footer {
		margin-top: auto;
		display: flex;
		align-items: center;
		justify-content: space-between;
		border-top: 1px solid var(--color-grey-100);
		padding-top: 1.5rem;
	}

	.course-card__price-row {
		display: flex;
		align-items: baseline;
		gap: 0.375rem;
	}

	.course-card__price {
		color: var(--color-navy);
		font-family: var(--font-heading);
		font-size: var(--fs-3xl);
		font-weight: var(--w-bold);
	}

	.course-card__price-label {
		color: var(--color-grey-500);
		font-size: var(--fs-sm);
	}

	.course-card__cta-pill {
		display: inline-flex;
		align-items: center;
		gap: 0.375rem;
		border-radius: var(--radius-lg);
		padding: 0.5rem 1rem;
		font-size: var(--fs-sm);
		font-weight: var(--w-semibold);
		background-color: rgba(15, 164, 175, 0.1);
		color: var(--color-teal);
		transition: all 300ms var(--ease-out);
	}

	.course-card:hover .course-card__cta-pill {
		background-color: var(--color-teal);
		color: var(--color-white);
	}

	:global(.course-card__cta-arrow) {
		transition: transform 300ms var(--ease-out) !important;
	}

	.course-card:hover :global(.course-card__cta-arrow) {
		transform: translateX(2px);
	}

	/* Why section */
	.why-grid {
		max-width: 56rem;
		margin: 0 auto;
		display: grid;
		gap: 1.5rem;
	}

	@media (min-width: 640px) {
		.why-grid {
			grid-template-columns: repeat(3, 1fr);
			gap: 2rem;
		}
	}

	.why-item {
		text-align: center;
	}

	.why-item__icon {
		display: flex;
		align-items: center;
		justify-content: center;
		width: 3.5rem;
		height: 3.5rem;
		border-radius: var(--radius-2xl);
		background-color: rgba(15, 164, 175, 0.1);
		margin: 0 auto 1.25rem;
	}

	.why-item__title {
		color: var(--color-navy);
		font-family: var(--font-heading);
		font-size: 1rem;
		font-weight: var(--w-bold);
		margin-bottom: 0.5rem;
	}

	@media (min-width: 640px) {
		.why-item__title {
			font-size: var(--fs-lg);
		}
	}

	.why-item__desc {
		color: var(--color-grey-600);
		font-size: var(--fs-sm);
		line-height: 1.65;
	}
</style>
