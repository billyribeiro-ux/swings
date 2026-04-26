<script lang="ts">
	import SectionHeader from '$lib/components/ui/SectionHeader.svelte';
	import ScrollReveal from '$lib/components/ui/ScrollReveal.svelte';
	import BookOpenIcon from 'phosphor-svelte/lib/BookOpenIcon';
	import PulseIcon from 'phosphor-svelte/lib/PulseIcon';
	import ArrowRightIcon from 'phosphor-svelte/lib/ArrowRightIcon';
	import ClockIcon from 'phosphor-svelte/lib/ClockIcon';
	import GraduationCapIcon from 'phosphor-svelte/lib/GraduationCapIcon';
	import { courses } from '$lib/data/courses';

	const iconMap: Record<string, typeof BookOpenIcon> = { BookOpenIcon, PulseIcon };
</script>

<section class="courses-section">
	<div class="courses-section__container">
		<ScrollReveal>
			<SectionHeader
				eyebrow="Education"
				title="Level Up Your Options Game"
				subtitle="Structured courses designed to take you from the basics to confidently trading options - at your own pace."
			/>

			<div class="courses-section__grid">
				{#each courses as course, i (course.id)}
					{@const Icon = iconMap[course.icon]}
					<a
						href="/courses/{course.slug}"
						class="reveal-item course-card"
						style="--i: {i};"
					>
						<!-- Visual Header -->
						<div
							class="course-card__header"
							style="background: linear-gradient(145deg, {course.gradient
								.from} 0%, {course.gradient.to} 100%);"
						>
							<div class="course-card__grid-pattern"></div>

							<div class="course-card__level-badge">
								<span class="course-card__level-text">{course.level}</span>
							</div>

							<div class="course-card__icon-box">
								{#if Icon}
									<Icon size={32} weight="duotone" color="white" />
								{:else}
									<BookOpenIcon size={32} weight="duotone" color="white" />
								{/if}
							</div>
						</div>

						<!-- Body -->
						<div class="course-card__body">
							<h3 class="course-card__title">{course.title}</h3>
							<p class="course-card__desc">{course.description}</p>

							<div class="course-card__footer">
								<div class="course-card__meta">
									<span class="course-card__meta-item">
										<ClockIcon
											size={13}
											weight="bold"
											class="course-card__meta-icon"
										/>
										{course.duration}
									</span>
									<span class="course-card__meta-item">
										<GraduationCapIcon
											size={13}
											weight="bold"
											class="course-card__meta-icon"
										/>
										{course.level}
									</span>
								</div>
								<span class="course-card__link">
									Learn More
									<ArrowRightIcon size={14} weight="bold" />
								</span>
							</div>
						</div>
					</a>
				{/each}
			</div>
		</ScrollReveal>
	</div>
</section>

<style>
	.courses-section {
		background-color: var(--color-off-white);
		padding: 4rem 0;
	}

	@media (min-width: 640px) {
		.courses-section {
			padding: 5rem 0;
		}
	}
	@media (min-width: 1024px) {
		.courses-section {
			padding: 7rem 0;
		}
	}

	.courses-section__container {
		max-width: var(--container-max);
		margin: 0 auto;
		padding: 0 1rem;
	}

	@media (min-width: 640px) {
		.courses-section__container {
			padding: 0 1.5rem;
		}
	}
	@media (min-width: 1024px) {
		.courses-section__container {
			padding: 0 2rem;
		}
	}

	.courses-section__grid {
		max-width: 960px;
		margin: 0 auto;
		display: grid;
		gap: 1.5rem;
	}

	@media (min-width: 640px) {
		.courses-section__grid {
			gap: 2rem;
		}
	}
	@media (min-width: 768px) {
		.courses-section__grid {
			grid-template-columns: repeat(2, 1fr);
		}
	}

	.course-card {
		display: flex;
		flex-direction: column;
		overflow: hidden;
		border-radius: var(--radius-2xl);
		background-color: var(--color-white);
		box-shadow: var(--shadow-sm);
		outline: 1px solid rgba(216, 220, 228, 0.8);
		outline-offset: -1px;
		transition: all 500ms var(--ease-out);
		transition-delay: calc(var(--i, 0) * 0.08s);
	}

	@media (prefers-reduced-motion: reduce) {
		.course-card {
			transition-delay: 0s;
		}
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
		height: 10rem;
		overflow: hidden;
	}

	@media (min-width: 640px) {
		.course-card__header {
			height: 11rem;
		}
	}

	.course-card__grid-pattern {
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
		width: 4rem;
		height: 4rem;
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

	.course-card__title {
		color: var(--color-navy);
		font-family: var(--font-heading);
		font-size: var(--fs-lg);
		font-weight: var(--w-bold);
		margin-bottom: 0.5rem;
	}

	@media (min-width: 640px) {
		.course-card__title {
			font-size: var(--fs-xl);
		}
	}

	.course-card__desc {
		color: var(--color-grey-600);
		font-size: var(--fs-sm);
		line-height: 1.65;
		margin-bottom: 1.25rem;
	}

	.course-card__footer {
		margin-top: auto;
		display: flex;
		align-items: center;
		justify-content: space-between;
	}

	.course-card__meta {
		display: flex;
		align-items: center;
		gap: 0.75rem;
		color: var(--color-grey-500);
		font-size: var(--fs-xs);
	}

	.course-card__meta-item {
		display: flex;
		align-items: center;
		gap: 0.25rem;
	}

	:global(.course-card__meta-icon) {
		color: var(--color-grey-400) !important;
	}

	.course-card__link {
		display: inline-flex;
		align-items: center;
		gap: 0.25rem;
		color: var(--color-teal);
		font-size: var(--fs-sm);
		font-weight: var(--w-semibold);
		transition: all 300ms var(--ease-out);
	}

	.course-card:hover .course-card__link {
		color: var(--color-teal-light);
		transform: translateX(2px);
	}
</style>
