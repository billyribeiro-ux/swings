<script lang="ts">
	import { onMount } from 'svelte';
	import { auth } from '$lib/stores/auth.svelte';
	import { api } from '$lib/api/client';
	import type {
		SubscriptionStatusResponse,
		Watchlist,
		CourseEnrollment,
		PaginatedResponse
	} from '$lib/api/types';
	import ListChecks from 'phosphor-svelte/lib/ListChecks';
	import BookOpen from 'phosphor-svelte/lib/BookOpen';
	import CalendarCheck from 'phosphor-svelte/lib/CalendarCheck';
	import Lightning from 'phosphor-svelte/lib/Lightning';

	let subscription = $state<SubscriptionStatusResponse | null>(null);
	let recentWatchlists = $state<Watchlist[]>([]);
	let enrollments = $state<CourseEnrollment[]>([]);
	let loading = $state(true);

	onMount(async () => {
		try {
			const [subRes, wlRes, enrollRes] = await Promise.all([
				api.get<SubscriptionStatusResponse>('/api/member/subscription'),
				api.get<PaginatedResponse<Watchlist>>('/api/member/watchlists?per_page=3'),
				api.get<CourseEnrollment[]>('/api/member/courses')
			]);
			subscription = subRes;
			recentWatchlists = wlRes.data;
			enrollments = enrollRes;
		} catch {
			// Silently handle - data just won't show
		} finally {
			loading = false;
		}
	});

	function formatDate(dateStr: string): string {
		return new Date(dateStr).toLocaleDateString('en-US', {
			month: 'short',
			day: 'numeric',
			year: 'numeric'
		});
	}
</script>

<svelte:head>
	<title>Dashboard - Explosive Swings</title>
</svelte:head>

<div class="overview">
	<!-- Stats Cards -->
	<div class="overview__stats">
		<div class="stat-card">
			<div class="stat-card__icon stat-card__icon--teal">
				<Lightning size={22} weight="fill" />
			</div>
			<div>
				<p class="stat-card__label">Subscription</p>
				<p class="stat-card__value">
					{#if subscription?.is_active}
						{subscription.subscription?.plan === 'annual' ? 'Annual' : 'Monthly'}
					{:else}
						Inactive
					{/if}
				</p>
			</div>
		</div>

		<div class="stat-card">
			<div class="stat-card__icon stat-card__icon--blue">
				<CalendarCheck size={22} weight="fill" />
			</div>
			<div>
				<p class="stat-card__label">Next Renewal</p>
				<p class="stat-card__value">
					{#if subscription?.subscription}
						{formatDate(subscription.subscription.current_period_end)}
					{:else}
						-
					{/if}
				</p>
			</div>
		</div>

		<div class="stat-card">
			<div class="stat-card__icon stat-card__icon--green">
				<ListChecks size={22} weight="fill" />
			</div>
			<div>
				<p class="stat-card__label">Watchlists Available</p>
				<p class="stat-card__value">{recentWatchlists.length}+</p>
			</div>
		</div>

		<div class="stat-card">
			<div class="stat-card__icon stat-card__icon--purple">
				<BookOpen size={22} weight="fill" />
			</div>
			<div>
				<p class="stat-card__label">Enrolled Courses</p>
				<p class="stat-card__value">{enrollments.length}</p>
			</div>
		</div>
	</div>

	<!-- Recent Watchlists -->
	<section class="overview__section">
		<div class="overview__section-header">
			<h3 class="overview__section-title">Recent Watchlists</h3>
			<a href="/dashboard/watchlists" class="overview__link">View all</a>
		</div>

		{#if loading}
			<p class="overview__empty">Loading...</p>
		{:else if recentWatchlists.length === 0}
			<p class="overview__empty">No watchlists available yet. Check back Sunday night!</p>
		{:else}
			<div class="overview__list">
				{#each recentWatchlists as wl (wl.id)}
					<a href="/dashboard/watchlists/{wl.id}" class="wl-card">
						<div>
							<h4 class="wl-card__title">{wl.title}</h4>
							<p class="wl-card__date">Week of {wl.week_of}</p>
						</div>
						<span class="wl-card__badge">
							{wl.published ? 'Published' : 'Draft'}
						</span>
					</a>
				{/each}
			</div>
		{/if}
	</section>

	<!-- Enrolled Courses -->
	<section class="overview__section">
		<div class="overview__section-header">
			<h3 class="overview__section-title">Your Courses</h3>
			<a href="/dashboard/courses" class="overview__link">View all</a>
		</div>

		{#if enrollments.length === 0}
			<p class="overview__empty">
				No course enrollments yet. <a href="/courses">Browse courses</a>
			</p>
		{:else}
			<div class="overview__list">
				{#each enrollments as enrollment (enrollment.id)}
					<div class="course-card">
						<div>
							<h4 class="course-card__title">{enrollment.course_id}</h4>
							<p class="course-card__date">Enrolled {formatDate(enrollment.enrolled_at)}</p>
						</div>
						<div class="course-card__progress">
							<div class="course-card__bar">
								<div class="course-card__fill" style="width: {enrollment.progress}%"></div>
							</div>
							<span class="course-card__pct">{enrollment.progress}%</span>
						</div>
					</div>
				{/each}
			</div>
		{/if}
	</section>
</div>

<style>
	.overview__stats {
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(14rem, 1fr));
		gap: 1rem;
		margin-bottom: 2rem;
	}

	.stat-card {
		display: flex;
		align-items: center;
		gap: 1rem;
		padding: 1.25rem;
		background-color: var(--color-navy-mid);
		border: 1px solid rgba(255, 255, 255, 0.06);
		border-radius: var(--radius-xl);
	}

	.stat-card__icon {
		width: 2.75rem;
		height: 2.75rem;
		border-radius: var(--radius-lg);
		display: flex;
		align-items: center;
		justify-content: center;
		flex-shrink: 0;
	}

	.stat-card__icon--teal {
		background-color: rgba(15, 164, 175, 0.15);
		color: var(--color-teal);
	}

	.stat-card__icon--blue {
		background-color: rgba(59, 130, 246, 0.15);
		color: #3b82f6;
	}

	.stat-card__icon--green {
		background-color: rgba(34, 197, 94, 0.15);
		color: #22c55e;
	}

	.stat-card__icon--purple {
		background-color: rgba(168, 85, 247, 0.15);
		color: #a855f7;
	}

	.stat-card__label {
		font-size: var(--fs-xs);
		color: var(--color-grey-400);
		margin-bottom: 0.2rem;
	}

	.stat-card__value {
		font-size: var(--fs-lg);
		font-weight: var(--w-bold);
		color: var(--color-white);
	}

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

	.overview__list {
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
		font-size: var(--fs-base);
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

	.course-card {
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: 1rem 1.25rem;
		background-color: var(--color-navy-mid);
		border: 1px solid rgba(255, 255, 255, 0.06);
		border-radius: var(--radius-xl);
	}

	.course-card__title {
		font-size: var(--fs-base);
		font-weight: var(--w-semibold);
		color: var(--color-white);
		margin-bottom: 0.2rem;
	}

	.course-card__date {
		font-size: var(--fs-xs);
		color: var(--color-grey-400);
	}

	.course-card__progress {
		display: flex;
		align-items: center;
		gap: 0.75rem;
	}

	.course-card__bar {
		width: 6rem;
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
		font-weight: var(--w-semibold);
		color: var(--color-grey-300);
		min-width: 2.5rem;
		text-align: right;
	}
</style>
