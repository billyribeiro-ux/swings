<script lang="ts">
	import { onMount } from 'svelte';
	import { api } from '$lib/api/client';
	import type { CourseEnrollment } from '$lib/api/types';
	import { courses } from '$lib/data/courses';
	import BookOpen from 'phosphor-svelte/lib/BookOpen';

	let enrollments = $state<CourseEnrollment[]>([]);
	let loading = $state(true);

	onMount(async () => {
		try {
			enrollments = await api.get<CourseEnrollment[]>('/api/member/courses');
		} catch {
			// handle
		} finally {
			loading = false;
		}
	});

	function getCourse(courseId: string) {
		return courses.find((c) => c.id === courseId);
	}

	function formatDate(dateStr: string): string {
		return new Date(dateStr).toLocaleDateString('en-US', {
			month: 'short',
			day: 'numeric',
			year: 'numeric'
		});
	}
</script>

<svelte:head>
	<title>My Courses - Explosive Swings</title>
</svelte:head>

<div class="courses-page">
	<h2 class="courses-page__title">My Courses</h2>
	<p class="courses-page__subtitle">Track your progress across enrolled courses.</p>

	{#if loading}
		<p class="courses-page__loading">Loading...</p>
	{:else if enrollments.length === 0}
		<div class="courses-page__empty">
			<BookOpen size={40} weight="duotone" color="rgba(255,255,255,0.2)" />
			<p>You haven't enrolled in any courses yet.</p>
			<a href="/courses" class="courses-page__browse">Browse Courses</a>
		</div>
	{:else}
		<div class="courses-page__grid">
			{#each enrollments as enrollment (enrollment.id)}
				{@const course = getCourse(enrollment.course_id)}
				<div class="enroll-card">
					{#if course}
						<div
							class="enroll-card__banner"
							style="background: linear-gradient(145deg, {course.gradient.from}, {course.gradient
								.to});"
						>
							<span class="enroll-card__level">{course.level}</span>
						</div>
						<div class="enroll-card__body">
							<h3 class="enroll-card__title">{course.title}</h3>
							<p class="enroll-card__meta">{course.duration} · {course.modules} modules</p>

							<div class="enroll-card__progress">
								<div class="enroll-card__bar">
									<div class="enroll-card__fill" style="width: {enrollment.progress}%"></div>
								</div>
								<span class="enroll-card__pct">{enrollment.progress}%</span>
							</div>

							{#if enrollment.completed_at}
								<p class="enroll-card__completed">
									Completed {formatDate(enrollment.completed_at)}
								</p>
							{:else}
								<p class="enroll-card__enrolled">
									Enrolled {formatDate(enrollment.enrolled_at)}
								</p>
							{/if}

							<a href="/courses/{course.slug}" class="enroll-card__continue">
								{enrollment.progress > 0 ? 'Continue Learning' : 'Start Course'} →
							</a>
						</div>
					{:else}
						<div class="enroll-card__body">
							<h3 class="enroll-card__title">{enrollment.course_id}</h3>
							<p class="enroll-card__meta">Course data unavailable</p>
						</div>
					{/if}
				</div>
			{/each}
		</div>
	{/if}
</div>

<style>
	.courses-page__title {
		font-size: var(--fs-2xl);
		font-weight: var(--w-bold);
		color: var(--color-white);
		font-family: var(--font-heading);
		margin-bottom: 0.5rem;
	}

	.courses-page__subtitle {
		color: var(--color-grey-400);
		margin-bottom: 2rem;
	}

	.courses-page__loading {
		text-align: center;
		color: var(--color-grey-400);
		padding: 3rem;
	}

	.courses-page__empty {
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 1rem;
		padding: 3rem;
		background-color: var(--color-navy-mid);
		border-radius: var(--radius-xl);
		border: 1px dashed rgba(255, 255, 255, 0.1);
		text-align: center;
		color: var(--color-grey-400);
	}

	.courses-page__browse {
		padding: 0.6rem 1.5rem;
		background: linear-gradient(135deg, var(--color-teal), #0d8a94);
		color: var(--color-white);
		font-weight: var(--w-semibold);
		font-size: var(--fs-sm);
		border-radius: var(--radius-lg);
		text-decoration: none;
	}

	.courses-page__grid {
		display: grid;
		gap: 1.5rem;
	}

	@media (min-width: 768px) {
		.courses-page__grid {
			grid-template-columns: repeat(2, 1fr);
		}
	}

	.enroll-card {
		background-color: var(--color-navy-mid);
		border: 1px solid rgba(255, 255, 255, 0.06);
		border-radius: var(--radius-xl);
		overflow: hidden;
	}

	.enroll-card__banner {
		height: 5rem;
		display: flex;
		align-items: flex-end;
		padding: 0.75rem 1rem;
	}

	.enroll-card__level {
		font-size: var(--fs-xs);
		font-weight: var(--w-semibold);
		color: rgba(255, 255, 255, 0.85);
		background-color: rgba(0, 0, 0, 0.3);
		padding: 0.2rem 0.6rem;
		border-radius: var(--radius-full);
	}

	.enroll-card__body {
		padding: 1.25rem;
	}

	.enroll-card__title {
		font-size: var(--fs-lg);
		font-weight: var(--w-bold);
		color: var(--color-white);
		margin-bottom: 0.25rem;
	}

	.enroll-card__meta {
		font-size: var(--fs-xs);
		color: var(--color-grey-400);
		margin-bottom: 1rem;
	}

	.enroll-card__progress {
		display: flex;
		align-items: center;
		gap: 0.75rem;
		margin-bottom: 0.75rem;
	}

	.enroll-card__bar {
		flex: 1;
		height: 0.4rem;
		background-color: rgba(255, 255, 255, 0.1);
		border-radius: var(--radius-full);
		overflow: hidden;
	}

	.enroll-card__fill {
		height: 100%;
		background: linear-gradient(90deg, var(--color-teal), #22c55e);
		border-radius: var(--radius-full);
		transition: width 300ms var(--ease-out);
	}

	.enroll-card__pct {
		font-size: var(--fs-xs);
		font-weight: var(--w-bold);
		color: var(--color-grey-300);
		min-width: 2.5rem;
		text-align: right;
	}

	.enroll-card__completed {
		font-size: var(--fs-xs);
		color: #22c55e;
		margin-bottom: 0.75rem;
	}

	.enroll-card__enrolled {
		font-size: var(--fs-xs);
		color: var(--color-grey-400);
		margin-bottom: 0.75rem;
	}

	.enroll-card__continue {
		font-size: var(--fs-sm);
		color: var(--color-teal);
		font-weight: var(--w-semibold);
		text-decoration: none;
	}

	.enroll-card__continue:hover {
		text-decoration: underline;
	}
</style>
