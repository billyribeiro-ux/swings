<script lang="ts">
	import { onMount } from 'svelte';
	import { resolve } from '$app/paths';
	import { api } from '$lib/api/client';
	import type { CourseEnrollment, CourseListItem, PaginatedResponse } from '$lib/api/types';
	import BookOpenIcon from 'phosphor-svelte/lib/BookOpenIcon';
	import MagnifyingGlassIcon from 'phosphor-svelte/lib/MagnifyingGlassIcon';
	import FunnelIcon from 'phosphor-svelte/lib/FunnelIcon';
	import GraduationCapIcon from 'phosphor-svelte/lib/GraduationCapIcon';
	import ClockIcon from 'phosphor-svelte/lib/ClockIcon';
	import ArrowRightIcon from 'phosphor-svelte/lib/ArrowRightIcon';

	type Tab = 'my-courses' | 'browse';

	let activeTab = $state<Tab>('my-courses');
	let enrollments = $state<CourseEnrollment[]>([]);
	let allCourses = $state<CourseListItem[]>([]);
	let loading = $state(true);
	let enrolling = $state<string | null>(null);
	let searchQuery = $state('');
	let difficultyFilter = $state('all');

	onMount(async () => {
		try {
			const [enrollRes, coursesRes] = await Promise.all([
				api.get<CourseEnrollment[]>('/api/member/courses'),
				api.get<PaginatedResponse<CourseListItem>>('/api/courses?per_page=50')
			]);
			enrollments = enrollRes;
			allCourses = coursesRes.data;
		} catch {
			// handle silently
		} finally {
			loading = false;
		}
	});

	let enrolledCourseIds = $derived(new Set(enrollments.map((e) => e.course_id)));

	let enrolledCourses = $derived(allCourses.filter((c) => enrolledCourseIds.has(c.id)));

	let browseCourses = $derived(allCourses.filter((c) => c.published));

	function filterCourses(courses: CourseListItem[]): CourseListItem[] {
		let result = courses;
		if (searchQuery.trim()) {
			const q = searchQuery.toLowerCase();
			result = result.filter(
				(c) =>
					c.title.toLowerCase().includes(q) ||
					(c.short_description ?? '').toLowerCase().includes(q) ||
					c.instructor_name.toLowerCase().includes(q)
			);
		}
		if (difficultyFilter !== 'all') {
			result = result.filter((c) => c.difficulty === difficultyFilter);
		}
		return result;
	}

	let filteredMyCourses = $derived(filterCourses(enrolledCourses));
	let filteredBrowseCourses = $derived(filterCourses(browseCourses));

	function getEnrollment(courseId: string): CourseEnrollment | undefined {
		return enrollments.find((e) => e.course_id === courseId);
	}

	function formatDuration(minutes: number): string {
		if (minutes < 60) return `${minutes}m`;
		const h = Math.floor(minutes / 60);
		const m = minutes % 60;
		return m > 0 ? `${h}h ${m}m` : `${h}h`;
	}

	function difficultyColor(difficulty: string): string {
		switch (difficulty) {
			case 'beginner':
				return 'var(--color-green)';
			case 'intermediate':
				return 'var(--color-gold)';
			case 'advanced':
				return 'var(--color-red)';
			default:
				return 'var(--color-grey-400)';
		}
	}

	async function handleEnroll(courseId: string) {
		enrolling = courseId;
		try {
			const enrollment = await api.post<CourseEnrollment>(
				`/api/member/courses/${courseId}/enroll`
			);
			enrollments = [...enrollments, enrollment];
		} catch {
			// handle silently
		} finally {
			enrolling = null;
		}
	}
</script>

<svelte:head>
	<title>Courses - Precision Options Signals</title>
</svelte:head>

<div class="courses-page">
	<h2 class="courses-page__title">Courses</h2>
	<p class="courses-page__subtitle">Learn options trading from beginner to advanced.</p>

	<!-- Tabs -->
	<div class="courses-page__tabs">
		<button
			class="courses-page__tab"
			class:courses-page__tab--active={activeTab === 'my-courses'}
			onclick={() => (activeTab = 'my-courses')}
		>
			My Courses
		</button>
		<button
			class="courses-page__tab"
			class:courses-page__tab--active={activeTab === 'browse'}
			onclick={() => (activeTab = 'browse')}
		>
			Browse All
		</button>
	</div>

	<!-- Search + Filter -->
	<div class="courses-page__filters">
		<div class="courses-page__search">
			<MagnifyingGlassIcon size={16} />
			<input
				type="text"
				placeholder="Search courses..."
				bind:value={searchQuery}
				class="courses-page__search-input"
			/>
		</div>
		<div class="courses-page__filter">
			<FunnelIcon size={16} />
			<select bind:value={difficultyFilter} class="courses-page__select">
				<option value="all">All Levels</option>
				<option value="beginner">Beginner</option>
				<option value="intermediate">Intermediate</option>
				<option value="advanced">Advanced</option>
			</select>
		</div>
	</div>

	{#if loading}
		<p class="courses-page__loading">Loading...</p>
	{:else if activeTab === 'my-courses'}
		<!-- My Courses Tab -->
		{#if filteredMyCourses.length === 0}
			<div class="courses-page__empty">
				<BookOpenIcon size={40} weight="duotone" color="rgba(255,255,255,0.2)" />
				{#if enrollments.length === 0}
					<p>You haven't enrolled in any courses yet.</p>
					<button class="courses-page__browse-btn" onclick={() => (activeTab = 'browse')}>
						Browse Courses
					</button>
				{:else}
					<p>No courses match your search.</p>
				{/if}
			</div>
		{:else}
			<div class="courses-page__grid">
				{#each filteredMyCourses as course (course.id)}
					{@const enrollment = getEnrollment(course.id)}
					<div class="course-card">
						<div class="course-card__banner">
							{#if course.thumbnail_url}
								<img
									src={course.thumbnail_url}
									alt={course.title}
									class="course-card__banner-img"
									width="640"
									height="360"
									loading="lazy"
									decoding="async"
								/>
							{:else}
								<div class="course-card__banner-placeholder">
									<BookOpenIcon size={28} weight="duotone" />
								</div>
							{/if}
							<span
								class="course-card__difficulty"
								style="color: {difficultyColor(course.difficulty)}"
							>
								{course.difficulty}
							</span>
						</div>
						<div class="course-card__body">
							<h3 class="course-card__title">{course.title}</h3>
							<div class="course-card__meta">
								<span class="course-card__meta-item">
									<GraduationCapIcon size={14} />
									{course.instructor_name}
								</span>
								<span class="course-card__meta-item">
									<ClockIcon size={14} />
									{formatDuration(course.estimated_duration_minutes)}
								</span>
								<span class="course-card__meta-item">
									<BookOpenIcon size={14} />
									{course.total_lessons} lessons
								</span>
							</div>

							{#if enrollment}
								<div class="course-card__progress">
									<div class="course-card__bar">
										<div
											class="course-card__fill"
											style="width: {enrollment.progress}%"
										></div>
									</div>
									<span class="course-card__pct">{enrollment.progress}%</span>
								</div>
							{/if}

							<a
								href={resolve('/dashboard/courses/[slug]', { slug: course.slug })}
								class="course-card__action"
							>
								{enrollment && enrollment.progress > 0
									? 'Continue Learning'
									: 'Start Course'}
								<ArrowRightIcon size={14} />
							</a>
						</div>
					</div>
				{/each}
			</div>
		{/if}
	{:else}
		<!-- Browse All Tab -->
		{#if filteredBrowseCourses.length === 0}
			<div class="courses-page__empty">
				<BookOpenIcon size={40} weight="duotone" color="rgba(255,255,255,0.2)" />
				<p>No courses found matching your criteria.</p>
			</div>
		{:else}
			<div class="courses-page__grid">
				{#each filteredBrowseCourses as course (course.id)}
					{@const isEnrolled = enrolledCourseIds.has(course.id)}
					{@const enrollment = getEnrollment(course.id)}
					<div class="course-card">
						<div class="course-card__banner">
							{#if course.thumbnail_url}
								<img
									src={course.thumbnail_url}
									alt={course.title}
									class="course-card__banner-img"
									width="640"
									height="360"
									loading="lazy"
									decoding="async"
								/>
							{:else}
								<div class="course-card__banner-placeholder">
									<BookOpenIcon size={28} weight="duotone" />
								</div>
							{/if}
							<span
								class="course-card__difficulty"
								style="color: {difficultyColor(course.difficulty)}"
							>
								{course.difficulty}
							</span>
						</div>
						<div class="course-card__body">
							<h3 class="course-card__title">{course.title}</h3>
							<div class="course-card__meta">
								<span class="course-card__meta-item">
									<GraduationCapIcon size={14} />
									{course.instructor_name}
								</span>
								<span class="course-card__meta-item">
									<ClockIcon size={14} />
									{formatDuration(course.estimated_duration_minutes)}
								</span>
								<span class="course-card__meta-item">
									<BookOpenIcon size={14} />
									{course.total_lessons} lessons
								</span>
							</div>

							{#if isEnrolled && enrollment}
								<div class="course-card__progress">
									<div class="course-card__bar">
										<div
											class="course-card__fill"
											style="width: {enrollment.progress}%"
										></div>
									</div>
									<span class="course-card__pct">{enrollment.progress}%</span>
								</div>
								<a
									href={resolve('/dashboard/courses/[slug]', { slug: course.slug })}
									class="course-card__action"
								>
									Continue Learning
									<ArrowRightIcon size={14} />
								</a>
							{:else}
								<button
									class="course-card__enroll"
									disabled={enrolling === course.id}
									onclick={() => handleEnroll(course.id)}
								>
									{enrolling === course.id ? 'Enrolling...' : 'Enroll'}
								</button>
							{/if}
						</div>
					</div>
				{/each}
			</div>
		{/if}
	{/if}
</div>

<style>
	.courses-page {
		max-width: var(--container-max);
	}

	.courses-page__title {
		font-size: var(--fs-2xl);
		font-weight: var(--w-bold);
		color: var(--color-white);
		font-family: var(--font-heading);
		margin-bottom: 0.5rem;
	}

	.courses-page__subtitle {
		color: var(--color-grey-400);
		font-size: var(--fs-sm);
		margin-bottom: 1.5rem;
	}

	/* Tabs */
	.courses-page__tabs {
		display: flex;
		gap: 0;
		margin-bottom: 1.5rem;
		border-bottom: 1px solid rgba(255, 255, 255, 0.08);
	}

	.courses-page__tab {
		padding: 0.75rem 1.5rem;
		background: none;
		border: none;
		border-bottom: 2px solid transparent;
		color: var(--color-grey-400);
		font-size: var(--fs-sm);
		font-weight: var(--w-semibold);
		cursor: pointer;
		transition: all 200ms var(--ease-out);
	}

	.courses-page__tab:hover {
		color: var(--color-grey-200);
	}

	.courses-page__tab--active {
		color: var(--color-teal);
		border-bottom-color: var(--color-teal);
	}

	/* Filters */
	.courses-page__filters {
		display: flex;
		gap: 1rem;
		margin-bottom: 1.5rem;
		flex-wrap: wrap;
	}

	.courses-page__search {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		flex: 1;
		min-width: 12rem;
		padding: 0.55rem 0.85rem;
		background-color: rgba(255, 255, 255, 0.05);
		border: 1px solid rgba(255, 255, 255, 0.1);
		border-radius: var(--radius-lg);
		color: var(--color-grey-400);
		transition: border-color 200ms var(--ease-out);
	}

	.courses-page__search:focus-within {
		border-color: var(--color-teal);
	}

	.courses-page__search-input {
		flex: 1;
		background: none;
		border: none;
		outline: none;
		color: var(--color-white);
		font-size: var(--fs-sm);
	}

	.courses-page__search-input::placeholder {
		color: var(--color-grey-500);
	}

	.courses-page__filter {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		padding: 0.55rem 0.85rem;
		background-color: rgba(255, 255, 255, 0.05);
		border: 1px solid rgba(255, 255, 255, 0.1);
		border-radius: var(--radius-lg);
		color: var(--color-grey-400);
	}

	.courses-page__select {
		background: none;
		border: none;
		outline: none;
		color: var(--color-white);
		font-size: var(--fs-sm);
		cursor: pointer;
	}

	.courses-page__select option {
		background-color: var(--color-navy-mid);
		color: var(--color-white);
	}

	/* Loading/Empty */
	.courses-page__loading {
		text-align: center;
		color: var(--color-grey-400);
		padding: 3rem;
		font-size: var(--fs-sm);
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

	.courses-page__browse-btn {
		padding: 0.6rem 1.5rem;
		background: linear-gradient(135deg, var(--color-teal), #0d8a94);
		color: var(--color-white);
		font-weight: var(--w-semibold);
		font-size: var(--fs-sm);
		border-radius: var(--radius-lg);
		border: none;
		cursor: pointer;
		transition: opacity 200ms var(--ease-out);
	}

	.courses-page__browse-btn:hover {
		opacity: 0.9;
	}

	/* Grid */
	.courses-page__grid {
		display: grid;
		gap: 1.5rem;
		grid-template-columns: 1fr;
	}

	@media (min-width: 640px) {
		.courses-page__grid {
			grid-template-columns: repeat(2, 1fr);
		}
	}

	@media (min-width: 1024px) {
		.courses-page__grid {
			grid-template-columns: repeat(3, 1fr);
		}
	}

	/* Course Card */
	.course-card {
		background-color: var(--color-navy-mid);
		border: 1px solid rgba(255, 255, 255, 0.06);
		border-radius: var(--radius-xl);
		overflow: hidden;
		transition: border-color 200ms var(--ease-out);
	}

	.course-card:hover {
		border-color: rgba(15, 164, 175, 0.25);
	}

	.course-card__banner {
		position: relative;
		height: 6rem;
		overflow: hidden;
	}

	.course-card__banner-img {
		width: 100%;
		height: 100%;
		object-fit: cover;
	}

	.course-card__banner-placeholder {
		width: 100%;
		height: 100%;
		display: flex;
		align-items: center;
		justify-content: center;
		background: linear-gradient(145deg, var(--color-navy), var(--color-deep-blue));
		color: var(--color-grey-500);
	}

	.course-card__difficulty {
		position: absolute;
		top: 0.5rem;
		right: 0.5rem;
		font-size: var(--fs-2xs);
		font-weight: var(--w-bold);
		text-transform: uppercase;
		letter-spacing: 0.05em;
		padding: 0.2rem 0.6rem;
		border-radius: var(--radius-full);
		background-color: rgba(0, 0, 0, 0.5);
	}

	.course-card__body {
		padding: 1.25rem;
		display: flex;
		flex-direction: column;
		gap: 0.75rem;
	}

	.course-card__title {
		font-size: var(--fs-md);
		font-weight: var(--w-bold);
		color: var(--color-white);
		line-height: var(--lh-snug);
	}

	.course-card__meta {
		display: flex;
		flex-wrap: wrap;
		gap: 0.75rem;
	}

	.course-card__meta-item {
		display: inline-flex;
		align-items: center;
		gap: 0.3rem;
		font-size: var(--fs-xs);
		color: var(--color-grey-400);
	}

	.course-card__progress {
		display: flex;
		align-items: center;
		gap: 0.75rem;
	}

	.course-card__bar {
		flex: 1;
		height: 0.4rem;
		background-color: rgba(255, 255, 255, 0.1);
		border-radius: var(--radius-full);
		overflow: hidden;
	}

	.course-card__fill {
		height: 100%;
		background: linear-gradient(90deg, var(--color-teal), #22c55e);
		border-radius: var(--radius-full);
		transition: width 300ms var(--ease-out);
	}

	.course-card__pct {
		font-size: var(--fs-xs);
		font-weight: var(--w-bold);
		color: var(--color-grey-300);
		min-width: 2.5rem;
		text-align: right;
	}

	.course-card__action {
		display: inline-flex;
		align-items: center;
		gap: 0.4rem;
		font-size: var(--fs-sm);
		color: var(--color-teal);
		font-weight: var(--w-semibold);
		text-decoration: none;
		align-self: flex-start;
		transition: opacity 200ms var(--ease-out);
	}

	.course-card__action:hover {
		text-decoration: underline;
	}

	.course-card__enroll {
		padding: 0.55rem 1.25rem;
		background: linear-gradient(135deg, var(--color-teal), #0d8a94);
		color: var(--color-white);
		font-weight: var(--w-semibold);
		font-size: var(--fs-sm);
		border-radius: var(--radius-lg);
		border: none;
		cursor: pointer;
		align-self: flex-start;
		transition: opacity 200ms var(--ease-out);
	}

	.course-card__enroll:hover:not(:disabled) {
		opacity: 0.9;
	}

	.course-card__enroll:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}
</style>
