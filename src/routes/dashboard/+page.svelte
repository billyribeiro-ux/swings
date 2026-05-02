<script lang="ts">
	import { onMount } from 'svelte';
	import { resolve } from '$app/paths';
	import { auth } from '$lib/stores/auth.svelte';
	import { api } from '$lib/api/client';
	import type {
		SubscriptionStatusResponse,
		Watchlist,
		WatchlistWithAlerts,
		WatchlistAlert,
		CourseEnrollment,
		PaginatedResponse,
		CourseListItem,
		CourseWithModules,
		CourseLesson,
		LessonProgress
	} from '$lib/api/types';
	import ListChecksIcon from 'phosphor-svelte/lib/ListChecksIcon';
	import BookOpenIcon from 'phosphor-svelte/lib/BookOpenIcon';
	import CheckCircleIcon from 'phosphor-svelte/lib/CheckCircleIcon';
	import PlayIcon from 'phosphor-svelte/lib/PlayIcon';
	import ArrowRightIcon from 'phosphor-svelte/lib/ArrowRightIcon';
	import ArrowUpIcon from 'phosphor-svelte/lib/ArrowUpIcon';
	import ArrowDownIcon from 'phosphor-svelte/lib/ArrowDownIcon';

	let subscription = $state<SubscriptionStatusResponse | null>(null);
	let enrollments = $state<CourseEnrollment[]>([]);
	let allCourses = $state<CourseListItem[]>([]);
	let progressByCourse = $state<Record<string, LessonProgress[]>>({});
	let lastAccessedCourseId = $state<string | null>(null);
	let nextLessonTitle = $state<string | null>(null);
	let featuredWatchlist = $state<WatchlistWithAlerts | null>(null);
	let loading = $state(true);

	onMount(async () => {
		try {
			const [subRes, wlRes, enrollRes, coursesRes] = await Promise.all([
				api.get<SubscriptionStatusResponse>('/api/member/subscription'),
				api.get<PaginatedResponse<Watchlist>>('/api/member/watchlists?per_page=1'),
				api.get<CourseEnrollment[]>('/api/member/courses'),
				api.get<PaginatedResponse<CourseListItem>>('/api/courses?per_page=50')
			]);
			subscription = subRes;
			enrollments = enrollRes;
			allCourses = coursesRes.data;

			// Fetch alerts for the most recent watchlist (if any).
			const latest = wlRes.data[0];
			if (latest) {
				try {
					featuredWatchlist = await api.get<WatchlistWithAlerts>(
						`/api/member/watchlists/${latest.id}`
					);
				} catch {
					/* ignore */
				}
			}

			// Fetch progress for every enrollment in parallel; ignore failures.
			const progressEntries = await Promise.all(
				enrollments.map(async (e) => {
					try {
						const progress = await api.get<LessonProgress[]>(
							`/api/member/courses/${e.course_id}/progress`
						);
						return [e.course_id, progress] as const;
					} catch {
						return [e.course_id, [] as LessonProgress[]] as const;
					}
				})
			);
			const map: Record<string, LessonProgress[]> = {};
			for (const [courseId, list] of progressEntries) map[courseId] = list;
			progressByCourse = map;

			// Determine last accessed course from the most recent
			// `last_accessed_at` across all progress records.
			let bestCourseId: string | null = null;
			let bestTs = '';
			for (const [courseId, list] of progressEntries) {
				for (const p of list) {
					if (p.last_accessed_at && p.last_accessed_at > bestTs) {
						bestTs = p.last_accessed_at;
						bestCourseId = courseId;
					}
				}
			}
			// Fallback to first enrollment if no progress activity yet.
			lastAccessedCourseId = bestCourseId ?? enrollments[0]?.course_id ?? null;

			// Resolve the next-up lesson title for the featured course.
			if (lastAccessedCourseId) {
				try {
					const course = allCourses.find((c) => c.id === lastAccessedCourseId);
					if (course) {
						const detail = await api.get<CourseWithModules>(
							`/api/courses/${course.slug}`
						);
						nextLessonTitle = pickNextLessonTitle(
							detail,
							progressByCourse[lastAccessedCourseId] ?? []
						);
					}
				} catch {
					/* ignore */
				}
			}
		} catch {
			// Silently handle - data just won't show
		} finally {
			loading = false;
		}
	});

	function pickNextLessonTitle(
		course: CourseWithModules,
		progress: LessonProgress[]
	): string | null {
		const completed = new Set(progress.filter((p) => p.completed).map((p) => p.lesson_id));
		let firstLesson: CourseLesson | null = null;
		for (const mod of course.modules) {
			for (const lesson of mod.lessons) {
				firstLesson ??= lesson;
				if (!completed.has(lesson.id)) return lesson.title;
			}
		}
		return firstLesson?.title ?? null;
	}

	function getCourseForEnrollment(courseId: string): CourseListItem | undefined {
		return allCourses.find((c) => c.id === courseId);
	}

	let lastAccessedEnrollment = $derived(
		lastAccessedCourseId
			? (enrollments.find((e) => e.course_id === lastAccessedCourseId) ?? null)
			: null
	);

	let lastAccessedCourse = $derived(
		lastAccessedCourseId
			? (allCourses.find((c) => c.id === lastAccessedCourseId) ?? null)
			: null
	);

	let lessonsCompleted = $derived(
		Object.values(progressByCourse).reduce(
			(sum, list) => sum + list.filter((p) => p.completed).length,
			0
		)
	);

	let weekAlertsCount = $derived(featuredWatchlist?.alerts.length ?? 0);

	let visibleAlerts = $derived<WatchlistAlert[]>(
		featuredWatchlist ? featuredWatchlist.alerts.slice(0, 8) : []
	);

	let extraAlertsCount = $derived(
		featuredWatchlist ? Math.max(0, featuredWatchlist.alerts.length - 8) : 0
	);

	const todayLabel = new Intl.DateTimeFormat('en-US', {
		weekday: 'long',
		month: 'long',
		day: 'numeric'
	}).format(new Date());

	let planLabel = $derived<string | null>(
		subscription?.subscription?.plan === 'annual'
			? 'Annual Plan'
			: subscription?.subscription?.plan === 'monthly'
				? 'Monthly Plan'
				: null
	);
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
				<p class="overview__welcome-date">{todayLabel}</p>
			</div>
			<div class="overview__status">
				{#if planLabel}
					<span class="overview__plan">{planLabel}</span>
				{/if}
				<span
					class="overview__badge"
					class:overview__badge--active={subscription?.is_active}
					class:overview__badge--expired={!subscription?.is_active}
				>
					{subscription?.is_active ? 'Active' : 'Expired'}
				</span>
			</div>
		</div>

		<!-- Stat Strip -->
		<section class="stat-strip">
			<div class="stat-strip__card stat-strip__card--teal">
				<div class="stat-strip__icon stat-strip__icon--teal">
					<BookOpenIcon size={22} weight="duotone" />
				</div>
				<div class="stat-strip__body">
					<span class="stat-strip__value">{enrollments.length}</span>
					<span class="stat-strip__label">Courses enrolled</span>
				</div>
			</div>
			<div class="stat-strip__card stat-strip__card--green">
				<div class="stat-strip__icon stat-strip__icon--green">
					<CheckCircleIcon size={22} weight="duotone" />
				</div>
				<div class="stat-strip__body">
					<span class="stat-strip__value">{lessonsCompleted}</span>
					<span class="stat-strip__label">Lessons completed</span>
				</div>
			</div>
			<div class="stat-strip__card stat-strip__card--gold">
				<div class="stat-strip__icon stat-strip__icon--gold">
					<ListChecksIcon size={22} weight="duotone" />
				</div>
				<div class="stat-strip__body">
					<span class="stat-strip__value">{weekAlertsCount}</span>
					<span class="stat-strip__label">This week's alerts</span>
				</div>
			</div>
		</section>

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
								<BookOpenIcon size={32} weight="duotone" />
							</div>
						{/if}
					</div>
					<div class="continue-card__info">
						<h4 class="continue-card__title">{lastAccessedCourse.title}</h4>
						<p class="continue-card__meta">
							{lastAccessedCourse.difficulty} &middot; {lastAccessedCourse.total_lessons}
							lessons
						</p>
						<div class="continue-card__progress">
							<div class="continue-card__bar">
								<div
									class="continue-card__fill"
									style="width: {lastAccessedEnrollment.progress}%"
								></div>
							</div>
							<span class="continue-card__pct"
								>{lastAccessedEnrollment.progress}%</span
							>
						</div>
						{#if nextLessonTitle}
							<p class="continue-card__next">Next up: "{nextLessonTitle}"</p>
						{/if}
						<a
							href={resolve('/dashboard/courses/[slug]', {
								slug: lastAccessedCourse.slug
							})}
							class="continue-card__resume"
						>
							<PlayIcon size={16} weight="fill" />
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
				<a href={resolve('/dashboard/courses')} class="overview__link">View all</a>
			</div>

			{#if enrollments.length === 0}
				<div class="empty-state">
					<div class="empty-state__icon">
						<BookOpenIcon size={40} weight="duotone" />
					</div>
					<h4 class="empty-state__title">No courses yet</h4>
					<p class="empty-state__body">
						Start learning options trading &mdash; browse our course library.
					</p>
					<a href={resolve('/dashboard/courses')} class="empty-state__cta">
						Browse Courses <ArrowRightIcon size={14} weight="bold" />
					</a>
				</div>
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
										<BookOpenIcon size={24} weight="duotone" />
									</div>
								{/if}
							</div>
							<div class="course-card__body">
								<h4 class="course-card__title">
									{course?.title ?? enrollment.course_id}
								</h4>
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
									<span class="course-card__circle-text"
										>{enrollment.progress}%</span
									>
								</div>
								<a
									href={resolve('/dashboard/courses/[slug]', {
										slug: course?.slug ?? enrollment.course_id
									})}
									class="course-card__continue"
								>
									Continue
									<ArrowRightIcon size={14} />
								</a>
							</div>
						</div>
					{/each}
				</div>
			{/if}
		</section>

		<!-- This Week's Watchlist -->
		<section class="overview__section">
			<div class="overview__section-header">
				<h3 class="overview__section-title">
					{#if featuredWatchlist}
						This Week's Watchlist &mdash; {featuredWatchlist.week_of}
					{:else}
						This Week's Watchlist
					{/if}
				</h3>
				<a href={resolve('/dashboard/watchlists')} class="overview__link">
					View all watchlists &rarr;
				</a>
			</div>

			{#if !featuredWatchlist}
				<p class="overview__empty">No watchlists available yet. Check back Sunday night!</p>
			{:else if visibleAlerts.length === 0}
				<p class="overview__empty">This watchlist has no alerts yet.</p>
			{:else}
				<ul class="ticker-row">
					{#each visibleAlerts as alert (alert.id)}
						<li>
							<a
								href={resolve('/dashboard/watchlists/[id]', {
									id: featuredWatchlist.id
								})}
								class="ticker-chip ticker-chip--{alert.direction}"
							>
								<span class="ticker-chip__head">
									<span class="ticker-chip__symbol">{alert.ticker}</span>
									{#if alert.direction === 'bullish'}
										<ArrowUpIcon size={14} weight="bold" />
									{:else}
										<ArrowDownIcon size={14} weight="bold" />
									{/if}
								</span>
								<span class="ticker-chip__zone">{alert.entry_zone}</span>
							</a>
						</li>
					{/each}
					{#if extraAlertsCount > 0}
						<li>
							<a
								href={resolve('/dashboard/watchlists/[id]', {
									id: featuredWatchlist.id
								})}
								class="ticker-chip ticker-chip--more"
							>
								<span class="ticker-chip__symbol">+{extraAlertsCount} more</span>
							</a>
						</li>
					{/if}
				</ul>
			{/if}
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
		margin-bottom: 1.5rem;
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

	.overview__welcome-date {
		color: var(--color-grey-500);
		font-size: var(--fs-xs);
		font-weight: var(--w-medium);
		font-variant-numeric: tabular-nums;
		margin-top: 0.4rem;
		letter-spacing: 0.01em;
	}

	.overview__status {
		display: inline-flex;
		align-items: center;
		gap: 0.6rem;
		flex-wrap: wrap;
	}

	.overview__plan {
		font-size: var(--fs-xs);
		font-weight: var(--w-medium);
		color: var(--color-grey-400);
		letter-spacing: 0.02em;
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

	/* Stat Strip */
	.stat-strip {
		display: grid;
		grid-template-columns: repeat(3, 1fr);
		gap: 1rem;
		margin-bottom: 2rem;
	}

	.stat-strip__card {
		display: flex;
		align-items: center;
		gap: 0.9rem;
		padding: 1rem 1.25rem;
		background-color: var(--color-navy-mid);
		border: 1px solid rgba(255, 255, 255, 0.06);
		border-top: 2px solid rgba(255, 255, 255, 0.08);
		border-radius: var(--radius-xl);
	}

	.stat-strip__card--teal {
		border-top-color: rgba(15, 164, 175, 0.4);
	}

	.stat-strip__card--green {
		border-top-color: rgba(34, 181, 115, 0.4);
	}

	.stat-strip__card--gold {
		border-top-color: rgba(212, 175, 55, 0.4);
	}

	.stat-strip__icon {
		display: flex;
		align-items: center;
		justify-content: center;
		width: 2.75rem;
		height: 2.75rem;
		border-radius: var(--radius-lg);
		flex-shrink: 0;
	}

	.stat-strip__icon--teal {
		background-color: rgba(15, 164, 175, 0.15);
		color: var(--color-teal);
	}

	.stat-strip__icon--green {
		background-color: rgba(34, 181, 115, 0.15);
		color: var(--color-green);
	}

	.stat-strip__icon--gold {
		background-color: rgba(212, 175, 55, 0.15);
		color: var(--color-gold);
	}

	.stat-strip__body {
		display: flex;
		flex-direction: column;
		gap: 0.1rem;
		min-width: 0;
	}

	.stat-strip__value {
		font-size: var(--fs-2xl);
		font-weight: var(--w-bold);
		color: var(--color-white);
		font-family: var(--font-heading);
		line-height: 1.1;
	}

	.stat-strip__label {
		font-size: var(--fs-xs);
		color: var(--color-grey-400);
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
		border-top: 2px solid var(--color-teal);
		border-radius: var(--radius-xl);
		margin-top: 0.75rem;
		box-shadow: 0 4px 24px rgba(0, 0, 0, 0.25);
	}

	.continue-card__thumb {
		width: 12rem;
		min-height: 8rem;
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

	.continue-card__next {
		font-size: var(--fs-xs);
		color: var(--color-grey-400);
		font-style: italic;
	}

	.continue-card__resume {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		gap: 0.5rem;
		padding: 0.55rem 1.25rem;
		min-width: 8rem;
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
		gap: 1rem;
		flex-wrap: wrap;
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
		white-space: nowrap;
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

	/* Polished empty state */
	.empty-state {
		text-align: center;
		padding: 2.5rem;
		border: 1px dashed rgba(255, 255, 255, 0.1);
		border-radius: var(--radius-xl);
		background-color: rgba(255, 255, 255, 0.01);
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 0.65rem;
	}

	.empty-state__icon {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		width: 4rem;
		height: 4rem;
		border-radius: var(--radius-full);
		background-color: rgba(15, 164, 175, 0.1);
		color: rgba(15, 164, 175, 0.7);
		margin-bottom: 0.25rem;
	}

	.empty-state__title {
		font-size: var(--fs-md);
		font-weight: var(--w-semibold);
		color: var(--color-white);
		font-family: var(--font-heading);
	}

	.empty-state__body {
		font-size: var(--fs-sm);
		color: var(--color-grey-400);
		line-height: var(--lh-relaxed);
		max-width: 22rem;
	}

	.empty-state__cta {
		display: inline-flex;
		align-items: center;
		gap: 0.4rem;
		margin-top: 0.5rem;
		padding: 0.55rem 1.1rem;
		font-size: var(--fs-sm);
		font-weight: var(--w-semibold);
		color: var(--color-teal);
		background-color: rgba(15, 164, 175, 0.1);
		border: 1px solid rgba(15, 164, 175, 0.25);
		border-radius: var(--radius-lg);
		text-decoration: none;
		transition:
			background-color 200ms var(--ease-out),
			border-color 200ms var(--ease-out);
	}

	.empty-state__cta:hover {
		background-color: rgba(15, 164, 175, 0.18);
		border-color: rgba(15, 164, 175, 0.45);
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
		transition:
			transform 200ms var(--ease-out),
			border-color 200ms var(--ease-out),
			box-shadow 200ms var(--ease-out);
	}

	.course-card:hover {
		border-color: rgba(15, 164, 175, 0.25);
		transform: translateY(-2px);
		box-shadow: 0 8px 24px rgba(0, 0, 0, 0.2);
	}

	.course-card__thumb {
		height: 7rem;
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
		width: 3.75rem;
		height: 3.75rem;
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

	/* Ticker chips */
	.ticker-row {
		display: flex;
		gap: 0.65rem;
		overflow-x: auto;
		padding: 0.25rem 0.1rem 0.75rem;
		margin: 0;
		list-style: none;
		scrollbar-width: thin;
		-webkit-overflow-scrolling: touch;
		-webkit-mask-image: linear-gradient(to right, black 85%, transparent 100%);
		mask-image: linear-gradient(to right, black 85%, transparent 100%);
	}

	.ticker-row > li {
		flex-shrink: 0;
		list-style: none;
	}

	.ticker-row::-webkit-scrollbar {
		height: 6px;
	}

	.ticker-row::-webkit-scrollbar-thumb {
		background-color: rgba(255, 255, 255, 0.12);
		border-radius: var(--radius-full);
	}

	.ticker-chip {
		display: inline-flex;
		flex-direction: column;
		gap: 0.2rem;
		padding: 0.6rem 0.9rem;
		min-width: 7.5rem;
		background-color: var(--color-navy-mid);
		border: 1px solid rgba(255, 255, 255, 0.06);
		border-left-width: 3px;
		border-radius: var(--radius-lg);
		text-decoration: none;
		flex-shrink: 0;
		transition:
			transform 150ms var(--ease-out),
			border-color 150ms var(--ease-out);
	}

	.ticker-chip:hover {
		transform: translateY(-1px);
	}

	.ticker-chip--bullish {
		border-left-color: var(--color-teal);
		color: var(--color-white);
	}

	.ticker-chip--bullish:hover {
		border-color: rgba(15, 164, 175, 0.4);
		border-left-color: var(--color-teal);
	}

	.ticker-chip--bearish {
		border-left-color: var(--color-red);
		color: var(--color-white);
	}

	.ticker-chip--bearish:hover {
		border-color: rgba(224, 72, 72, 0.4);
		border-left-color: var(--color-red);
	}

	.ticker-chip--more {
		border-left-color: var(--color-gold);
		justify-content: center;
		align-items: center;
		color: var(--color-grey-300);
	}

	.ticker-chip__head {
		display: inline-flex;
		align-items: center;
		gap: 0.35rem;
	}

	.ticker-chip--bullish .ticker-chip__head {
		color: var(--color-teal);
	}

	.ticker-chip--bearish .ticker-chip__head {
		color: var(--color-red);
	}

	.ticker-chip__symbol {
		font-size: var(--fs-sm);
		font-weight: var(--w-bold);
		font-family: var(--font-heading);
		letter-spacing: 0.02em;
	}

	.ticker-chip__zone {
		font-size: var(--fs-xs);
		color: var(--color-grey-400);
		font-variant-numeric: tabular-nums;
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

		.stat-strip {
			grid-template-columns: 1fr 1fr;
		}
	}

	@media (max-width: 480px) {
		.overview__courses-grid {
			grid-template-columns: 1fr;
		}
	}
</style>
