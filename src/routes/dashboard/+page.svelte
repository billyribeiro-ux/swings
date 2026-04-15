<script lang="ts">
	import { onMount } from 'svelte';
	import { auth } from '$lib/stores/auth.svelte';
	import { api } from '$lib/api/client';
	import type {
		SubscriptionStatusResponse,
		Watchlist,
		CourseEnrollment,
		PaginatedResponse,
		CourseListItem
	} from '$lib/api/types';
	import ListChecks from 'phosphor-svelte/lib/ListChecks';
	import BookOpen from 'phosphor-svelte/lib/BookOpen';
	import Gear from 'phosphor-svelte/lib/Gear';
	import Play from 'phosphor-svelte/lib/Play';
	import ArrowRight from 'phosphor-svelte/lib/ArrowRight';

	let subscription = $state<SubscriptionStatusResponse | null>(null);
	let recentWatchlists = $state<Watchlist[]>([]);
	let enrollments = $state<CourseEnrollment[]>([]);
	let allCourses = $state<CourseListItem[]>([]);
	let loading = $state(true);

	onMount(async () => {
		try {
			const [subRes, wlRes, enrollRes, coursesRes] = await Promise.all([
				api.get<SubscriptionStatusResponse>('/api/member/subscription'),
				api.get<PaginatedResponse<Watchlist>>('/api/member/watchlists?per_page=3'),
				api.get<CourseEnrollment[]>('/api/member/courses'),
				api.get<PaginatedResponse<CourseListItem>>('/api/courses?per_page=50')
			]);
			subscription = subRes;
			recentWatchlists = wlRes.data;
			enrollments = enrollRes;
			allCourses = coursesRes.data;
		} catch {
			// Silently handle - data just won't show
		} finally {
			loading = false;
		}
	});

	let lastAccessedEnrollment = $derived(
		enrollments.length > 0
			? enrollments.reduce((latest, e) => {
					if (!latest) return e;
					return e.enrolled_at > latest.enrolled_at ? e : latest;
				}, enrollments[0])
			: null
	);

	let lastAccessedCourse = $derived(
		lastAccessedEnrollment
			? allCourses.find((c) => c.id === lastAccessedEnrollment.course_id) ?? null
			: null
	);

	function getCourseForEnrollment(courseId: string): CourseListItem | undefined {
		return allCourses.find((c) => c.id === courseId);
	}
</script>

<svelte:head>
	<title>Dashboard - Precision Options Signals</title>
</svelte:head>

<div class="overview">
	{#if loading}
		<p class="overview__loading">Loading...</p>
	{:else}
		<!-- Welcome + Status -->
		<div class="overview__welcome">
			<div>
				<h2 class="overview__welcome-name">
					Welcome back, {auth.user?.name?.split(' ')[0] ?? 'Member'}
				</h2>
				<p class="overview__welcome-sub">Here's what's happening with your account.</p>
			</div>
			<span
				class="overview__badge"
				class:overview__badge--active={subscription?.is_active}
				class:overview__badge--expired={!subscription?.is_active}
			>
				{subscription?.is_active ? 'Active' : 'Expired'}
			</span>
		</div>

		<!-- Continue Learning -->
		{#if lastAccessedEnrollment && lastAccessedCourse}
			<section class="overview__continue">
				<h3 class="overview__section-title">Continue Learning</h3>
				<div class="continue-card">
					<div class="continue-card__thumb">
						{#if lastAccessedCourse.thumbnail_url}
							<img
								src={lastAccessedCourse.thumbnail_url}
								alt={lastAccessedCourse.title}
								class="continue-card__img"
							/>
						{:else}
							<div class="continue-card__placeholder">
								<BookOpen size={32} weight="duotone" />
							</div>
						{/if}
					</div>
					<div class="continue-card__info">
						<h4 class="continue-card__title">{lastAccessedCourse.title}</h4>
						<p class="continue-card__meta">
							{lastAccessedCourse.difficulty} &middot; {lastAccessedCourse.total_lessons} lessons
						</p>
						<div class="continue-card__progress">
							<div class="continue-card__bar">
								<div
									class="continue-card__fill"
									style="width: {lastAccessedEnrollment.progress}%"
								></div>
							</div>
							<span class="continue-card__pct">{lastAccessedEnrollment.progress}%</span>
						</div>
						<a
							href="/dashboard/courses/{lastAccessedCourse.slug}"
							class="continue-card__resume"
						>
							<Play size={16} weight="fill" />
							Resume
						</a>
					</div>
				</div>
			</section>
		{/if}

		<!-- Enrolled Courses Grid -->
		<section class="overview__section">
			<div class="overview__section-header">
				<h3 class="overview__section-title">Enrolled Courses</h3>
				<a href="/dashboard/courses" class="overview__link">View all</a>
			</div>

			{#if enrollments.length === 0}
				<p class="overview__empty">
					No course enrollments yet. <a href="/dashboard/courses">Browse courses</a>
				</p>
			{:else}
				<div class="overview__courses-grid">
					{#each enrollments as enrollment (enrollment.id)}
						{@const course = getCourseForEnrollment(enrollment.course_id)}
						<div class="course-card">
							<div class="course-card__thumb">
								{#if course?.thumbnail_url}
									<img
										src={course.thumbnail_url}
										alt={course?.title ?? 'Course'}
										class="course-card__img"
									/>
								{:else}
									<div class="course-card__placeholder">
										<BookOpen size={24} weight="duotone" />
									</div>
								{/if}
							</div>
							<div class="course-card__body">
								<h4 class="course-card__title">{course?.title ?? enrollment.course_id}</h4>
								<div class="course-card__circle-wrap">
									<svg class="course-card__circle" viewBox="0 0 36 36">
										<path
											class="course-card__circle-bg"
											d="M18 2.0845 a 15.9155 15.9155 0 0 1 0 31.831 a 15.9155 15.9155 0 0 1 0 -31.831"
										/>
										<path
											class="course-card__circle-fg"
											stroke-dasharray="{enrollment.progress}, 100"
											d="M18 2.0845 a 15.9155 15.9155 0 0 1 0 31.831 a 15.9155 15.9155 0 0 1 0 -31.831"
										/>
									</svg>
									<span class="course-card__circle-text">{enrollment.progress}%</span>
								</div>
								<a
									href="/dashboard/courses/{course?.slug ?? enrollment.course_id}"
									class="course-card__continue"
								>
									Continue
									<ArrowRight size={14} />
								</a>
							</div>
						</div>
					{/each}
				</div>
			{/if}
		</section>

		<!-- Latest Watchlists -->
		<section class="overview__section">
			<div class="overview__section-header">
				<h3 class="overview__section-title">Latest Watchlists</h3>
				<a href="/dashboard/watchlists" class="overview__link">View all</a>
			</div>

			{#if recentWatchlists.length === 0}
				<p class="overview__empty">No watchlists available yet. Check back Sunday night!</p>
			{:else}
				<div class="overview__watchlists">
					{#each recentWatchlists as wl (wl.id)}
						<a href="/dashboard/watchlists/{wl.id}" class="wl-card">
							<div class="wl-card__info">
								<h4 class="wl-card__title">{wl.title}</h4>
								<p class="wl-card__date">Week of {wl.week_of}</p>
							</div>
							<div class="wl-card__right">
								{#if wl.published}
									<span class="wl-card__badge">Published</span>
								{:else}
									<span class="wl-card__badge wl-card__badge--draft">Draft</span>
								{/if}
							</div>
						</a>
					{/each}
				</div>
			{/if}
		</section>

		<!-- Quick Links -->
		<section class="overview__quick-links">
			<a href="/dashboard/account" class="quick-link">
				<Gear size={20} weight="duotone" />
				<span>Account Settings</span>
			</a>
			<a href="/dashboard/courses" class="quick-link">
				<BookOpen size={20} weight="duotone" />
				<span>Browse Courses</span>
			</a>
			<a href="/dashboard/watchlists" class="quick-link">
				<ListChecks size={20} weight="duotone" />
				<span>Watchlists</span>
			</a>
		</section>
	{/if}
</div>

<style>
	.overview {
		max-width: var(--container-max);
	}

	.overview__loading {
		text-align: center;
		color: var(--color-grey-400);
		padding: 3rem;
		font-size: var(--fs-sm);
	}

	/* Welcome */
	.overview__welcome {
		display: flex;
		align-items: center;
		justify-content: space-between;
		margin-bottom: 2rem;
		flex-wrap: wrap;
		gap: 1rem;
	}

	.overview__welcome-name {
		font-size: var(--fs-2xl);
		font-weight: var(--w-bold);
		color: var(--color-white);
		font-family: var(--font-heading);
		margin-bottom: 0.25rem;
	}

	.overview__welcome-sub {
		color: var(--color-grey-400);
		font-size: var(--fs-sm);
	}

	.overview__badge {
		padding: 0.35rem 1rem;
		border-radius: var(--radius-full);
		font-size: var(--fs-xs);
		font-weight: var(--w-bold);
		text-transform: uppercase;
		letter-spacing: 0.05em;
	}

	.overview__badge--active {
		background-color: rgba(34, 181, 115, 0.15);
		color: var(--color-green);
	}

	.overview__badge--expired {
		background-color: rgba(224, 72, 72, 0.15);
		color: var(--color-red);
	}

	/* Continue Learning */
	.overview__continue {
		margin-bottom: 2rem;
	}

	.continue-card {
		display: flex;
		gap: 1.5rem;
		padding: 1.25rem;
		background-color: var(--color-navy-mid);
		border: 1px solid rgba(255, 255, 255, 0.06);
		border-radius: var(--radius-xl);
		margin-top: 0.75rem;
	}

	.continue-card__thumb {
		width: 10rem;
		min-height: 7rem;
		border-radius: var(--radius-lg);
		overflow: hidden;
		flex-shrink: 0;
	}

	.continue-card__img {
		width: 100%;
		height: 100%;
		object-fit: cover;
	}

	.continue-card__placeholder {
		width: 100%;
		height: 100%;
		display: flex;
		align-items: center;
		justify-content: center;
		background: linear-gradient(145deg, var(--color-navy), var(--color-deep-blue));
		color: var(--color-grey-500);
	}

	.continue-card__info {
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
		flex: 1;
	}

	.continue-card__title {
		font-size: var(--fs-lg);
		font-weight: var(--w-bold);
		color: var(--color-white);
	}

	.continue-card__meta {
		font-size: var(--fs-xs);
		color: var(--color-grey-400);
		text-transform: capitalize;
	}

	.continue-card__progress {
		display: flex;
		align-items: center;
		gap: 0.75rem;
	}

	.continue-card__bar {
		flex: 1;
		max-width: 16rem;
		height: 0.4rem;
		background-color: rgba(255, 255, 255, 0.1);
		border-radius: var(--radius-full);
		overflow: hidden;
	}

	.continue-card__fill {
		height: 100%;
		background: linear-gradient(90deg, var(--color-teal), #22c55e);
		border-radius: var(--radius-full);
		transition: width 300ms var(--ease-out);
	}

	.continue-card__pct {
		font-size: var(--fs-xs);
		font-weight: var(--w-bold);
		color: var(--color-grey-300);
	}

	.continue-card__resume {
		display: inline-flex;
		align-items: center;
		gap: 0.5rem;
		padding: 0.55rem 1.25rem;
		background: linear-gradient(135deg, var(--color-teal), #0d8a94);
		color: var(--color-white);
		font-weight: var(--w-semibold);
		font-size: var(--fs-sm);
		border-radius: var(--radius-lg);
		text-decoration: none;
		align-self: flex-start;
		transition: opacity 200ms var(--ease-out);
	}

	.continue-card__resume:hover {
		opacity: 0.9;
	}

	/* Sections */
	.overview__section {
		margin-bottom: 2rem;
	}

	.overview__section-header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		margin-bottom: 1rem;
	}

	.overview__section-title {
		font-size: var(--fs-lg);
		font-weight: var(--w-bold);
		color: var(--color-white);
		font-family: var(--font-heading);
	}

	.overview__link {
		color: var(--color-teal);
		font-size: var(--fs-sm);
		font-weight: var(--w-semibold);
		text-decoration: none;
	}

	.overview__link:hover {
		text-decoration: underline;
	}

	.overview__empty {
		color: var(--color-grey-400);
		font-size: var(--fs-sm);
		padding: 2rem;
		text-align: center;
		background-color: var(--color-navy-mid);
		border-radius: var(--radius-xl);
		border: 1px dashed rgba(255, 255, 255, 0.1);
	}

	.overview__empty a {
		color: var(--color-teal);
	}

	/* Enrolled Courses Grid */
	.overview__courses-grid {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(14rem, 1fr));
		gap: 1rem;
	}

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

	.course-card__thumb {
		height: 5rem;
		overflow: hidden;
	}

	.course-card__img {
		width: 100%;
		height: 100%;
		object-fit: cover;
	}

	.course-card__placeholder {
		width: 100%;
		height: 100%;
		display: flex;
		align-items: center;
		justify-content: center;
		background: linear-gradient(145deg, var(--color-navy), var(--color-deep-blue));
		color: var(--color-grey-500);
	}

	.course-card__body {
		padding: 1rem;
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 0.75rem;
	}

	.course-card__title {
		font-size: var(--fs-sm);
		font-weight: var(--w-semibold);
		color: var(--color-white);
		text-align: center;
		line-height: var(--lh-snug);
	}

	.course-card__circle-wrap {
		position: relative;
		width: 3.5rem;
		height: 3.5rem;
	}

	.course-card__circle {
		width: 100%;
		height: 100%;
		transform: rotate(-90deg);
	}

	.course-card__circle-bg {
		fill: none;
		stroke: rgba(255, 255, 255, 0.08);
		stroke-width: 3;
	}

	.course-card__circle-fg {
		fill: none;
		stroke: var(--color-teal);
		stroke-width: 3;
		stroke-linecap: round;
		transition: stroke-dasharray 300ms var(--ease-out);
	}

	.course-card__circle-text {
		position: absolute;
		inset: 0;
		display: flex;
		align-items: center;
		justify-content: center;
		font-size: var(--fs-2xs);
		font-weight: var(--w-bold);
		color: var(--color-grey-300);
	}

	.course-card__continue {
		display: inline-flex;
		align-items: center;
		gap: 0.35rem;
		font-size: var(--fs-xs);
		color: var(--color-teal);
		font-weight: var(--w-semibold);
		text-decoration: none;
	}

	.course-card__continue:hover {
		text-decoration: underline;
	}

	/* Watchlists */
	.overview__watchlists {
		display: flex;
		flex-direction: column;
		gap: 0.75rem;
	}

	.wl-card {
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: 1rem 1.25rem;
		background-color: var(--color-navy-mid);
		border: 1px solid rgba(255, 255, 255, 0.06);
		border-radius: var(--radius-xl);
		text-decoration: none;
		transition: border-color 200ms var(--ease-out);
	}

	.wl-card:hover {
		border-color: rgba(15, 164, 175, 0.3);
	}

	.wl-card__title {
		font-size: var(--fs-sm);
		font-weight: var(--w-semibold);
		color: var(--color-white);
		margin-bottom: 0.2rem;
	}

	.wl-card__date {
		font-size: var(--fs-xs);
		color: var(--color-grey-400);
	}

	.wl-card__badge {
		font-size: var(--fs-xs);
		font-weight: var(--w-medium);
		padding: 0.25rem 0.75rem;
		border-radius: var(--radius-full);
		background-color: rgba(15, 164, 175, 0.15);
		color: var(--color-teal);
	}

	.wl-card__badge--draft {
		background-color: rgba(255, 255, 255, 0.06);
		color: var(--color-grey-400);
	}

	/* Quick Links */
	.overview__quick-links {
		display: flex;
		gap: 1rem;
		flex-wrap: wrap;
	}

	.quick-link {
		display: flex;
		align-items: center;
		gap: 0.6rem;
		padding: 0.75rem 1.25rem;
		background-color: var(--color-navy-mid);
		border: 1px solid rgba(255, 255, 255, 0.06);
		border-radius: var(--radius-xl);
		color: var(--color-grey-300);
		font-size: var(--fs-sm);
		font-weight: var(--w-medium);
		text-decoration: none;
		transition:
			border-color 200ms var(--ease-out),
			color 200ms var(--ease-out);
	}

	.quick-link:hover {
		border-color: rgba(15, 164, 175, 0.3);
		color: var(--color-teal);
	}

	@media (max-width: 768px) {
		.continue-card {
			flex-direction: column;
		}

		.continue-card__thumb {
			width: 100%;
			min-height: 8rem;
		}

		.overview__courses-grid {
			grid-template-columns: 1fr 1fr;
		}
	}

	@media (max-width: 480px) {
		.overview__courses-grid {
			grid-template-columns: 1fr;
		}
	}
</style>
