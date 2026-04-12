<script lang="ts">
	import { api } from '$lib/api/client';
	import MagnifyingGlass from 'phosphor-svelte/lib/MagnifyingGlass';
	import Plus from 'phosphor-svelte/lib/Plus';
	import BookOpen from 'phosphor-svelte/lib/BookOpen';
	import Clock from 'phosphor-svelte/lib/Clock';
	import GraduationCap from 'phosphor-svelte/lib/GraduationCap';

	interface Course {
		id: string;
		title: string;
		slug: string;
		description: string;
		short_description: string | null;
		thumbnail_url: string | null;
		trailer_video_url: string | null;
		difficulty: 'beginner' | 'intermediate' | 'advanced';
		price: number;
		is_free: boolean;
		is_published: boolean;
		estimated_duration: string | null;
		lessons_count: number;
		modules_count: number;
		published_at: string | null;
		created_at: string;
		updated_at: string;
	}

	interface CoursesResponse {
		data: Course[];
		total: number;
		page: number;
		per_page: number;
		total_pages: number;
	}

	let courses: Course[] = $state([]);
	let total = $state(0);
	let page = $state(1);
	let totalPages = $state(1);
	let loading = $state(true);
	let search = $state('');
	let statusFilter: 'all' | 'published' | 'draft' = $state('all');
	let searchTimeout: ReturnType<typeof setTimeout>;

	const publishedCount = $derived(courses.filter((c) => c.is_published).length);
	const draftCount = $derived(courses.filter((c) => !c.is_published).length);

	$effect(() => {
		loadCourses();
	});

	async function loadCourses() {
		loading = true;
		try {
			let url = `/api/admin/courses?page=${page}&per_page=20`;
			if (statusFilter === 'published') url += '&is_published=true';
			else if (statusFilter === 'draft') url += '&is_published=false';
			if (search) url += `&search=${encodeURIComponent(search)}`;
			const res = await api.get<CoursesResponse>(url);
			courses = res.data;
			total = res.total;
			totalPages = res.total_pages;
		} catch (e) {
			console.error('Failed to load courses', e);
		} finally {
			loading = false;
		}
	}

	function handleSearch(e: Event) {
		const val = (e.target as HTMLInputElement).value;
		clearTimeout(searchTimeout);
		searchTimeout = setTimeout(() => {
			search = val;
			page = 1;
			loadCourses();
		}, 300);
	}

	function changeStatus(s: 'all' | 'published' | 'draft') {
		statusFilter = s;
		page = 1;
		loadCourses();
	}

	function difficultyBadgeClass(d: string): string {
		const map: Record<string, string> = {
			beginner: 'badge--beginner',
			intermediate: 'badge--intermediate',
			advanced: 'badge--advanced'
		};
		return map[d] || '';
	}

	function formatDate(d: string | null): string {
		if (!d) return '--';
		return new Date(d).toLocaleDateString('en-US', {
			month: 'short',
			day: 'numeric',
			year: 'numeric'
		});
	}

	function capitalize(s: string): string {
		return s.charAt(0).toUpperCase() + s.slice(1);
	}
</script>

<svelte:head>
	<title>Courses -- Admin</title>
</svelte:head>

<div class="courses-admin">
	<!-- Header -->
	<div class="courses-admin__header">
		<div>
			<h1 class="courses-admin__title">Courses</h1>
			<p class="courses-admin__subtitle">Manage your course catalog</p>
		</div>
		<a href="/admin/courses/new" class="btn-primary">
			<Plus size={18} weight="bold" />
			New Course
		</a>
	</div>

	<!-- Stats bar -->
	{#if !loading}
		<div class="stats-bar">
			<div class="stat-card">
				<div class="stat-card__icon">
					<BookOpen size={20} weight="duotone" />
				</div>
				<div class="stat-card__content">
					<span class="stat-card__value">{total}</span>
					<span class="stat-card__label">Total Courses</span>
				</div>
			</div>
			<div class="stat-card">
				<div class="stat-card__icon stat-card__icon--green">
					<GraduationCap size={20} weight="duotone" />
				</div>
				<div class="stat-card__content">
					<span class="stat-card__value">{publishedCount}</span>
					<span class="stat-card__label">Published</span>
				</div>
			</div>
			<div class="stat-card">
				<div class="stat-card__icon stat-card__icon--amber">
					<Clock size={20} weight="duotone" />
				</div>
				<div class="stat-card__content">
					<span class="stat-card__value">{draftCount}</span>
					<span class="stat-card__label">Draft</span>
				</div>
			</div>
		</div>
	{/if}

	<!-- Filters -->
	<div class="courses-admin__filters">
		<div class="courses-admin__status-tabs">
			<button class:active={statusFilter === 'all'} onclick={() => changeStatus('all')}>
				All
			</button>
			<button
				class:active={statusFilter === 'published'}
				onclick={() => changeStatus('published')}
			>
				Published
			</button>
			<button class:active={statusFilter === 'draft'} onclick={() => changeStatus('draft')}>
				Drafts
			</button>
		</div>

		<div class="courses-admin__search-wrap">
			<MagnifyingGlass size={16} weight="bold" class="search-icon" />
			<input
				id="course-search"
				name="search"
				type="search"
				class="courses-admin__search"
				placeholder="Search courses..."
				oninput={handleSearch}
			/>
		</div>
	</div>

	<!-- Loading skeleton -->
	{#if loading}
		<div class="skeleton-grid">
			{#each Array(4) as _, i (i)}
				<div class="skeleton-card">
					<div class="skeleton-card__thumb"></div>
					<div class="skeleton-card__body">
						<div class="skeleton-line skeleton-line--title"></div>
						<div class="skeleton-line skeleton-line--short"></div>
						<div class="skeleton-line skeleton-line--meta"></div>
					</div>
				</div>
			{/each}
		</div>
	{:else if courses.length === 0}
		<!-- Empty state -->
		<div class="empty-state">
			<div class="empty-state__icon">
				<BookOpen size={48} weight="duotone" />
			</div>
			<h2 class="empty-state__title">No courses found</h2>
			<p class="empty-state__desc">
				{#if search || statusFilter !== 'all'}
					Try adjusting your search or filters.
				{:else}
					Get started by creating your first course.
				{/if}
			</p>
			{#if !search && statusFilter === 'all'}
				<a href="/admin/courses/new" class="btn-primary">
					<Plus size={18} weight="bold" />
					Create Course
				</a>
			{/if}
		</div>
	{:else}
		<!-- Mobile: Cards -->
		<div class="courses-admin__cards">
			{#each courses as course (course.id)}
				<a href="/admin/courses/{course.id}" class="course-card">
					<div class="course-card__thumb">
						{#if course.thumbnail_url}
							<img src={course.thumbnail_url} alt={course.title} class="course-card__img" />
						{:else}
							<div class="course-card__placeholder">
								<BookOpen size={28} weight="duotone" />
							</div>
						{/if}
					</div>
					<div class="course-card__body">
						<div class="course-card__top">
							<h3 class="course-card__title">{course.title}</h3>
							<span
								class="badge {course.is_published ? 'badge--published' : 'badge--draft'}"
							>
								{course.is_published ? 'Published' : 'Draft'}
							</span>
						</div>
						<div class="course-card__tags">
							<span class="badge {difficultyBadgeClass(course.difficulty)}">
								{capitalize(course.difficulty)}
							</span>
							{#if course.is_free}
								<span class="badge badge--free">Free</span>
							{:else}
								<span class="course-card__price">${course.price}</span>
							{/if}
						</div>
						<div class="course-card__meta">
							<span>{course.modules_count} modules</span>
							<span class="course-card__meta-dot"></span>
							<span>{course.lessons_count} lessons</span>
							{#if course.estimated_duration}
								<span class="course-card__meta-dot"></span>
								<span>{course.estimated_duration}</span>
							{/if}
						</div>
						<div class="course-card__date">
							{course.is_published ? formatDate(course.published_at) : formatDate(course.created_at)}
						</div>
					</div>
				</a>
			{/each}
		</div>

		<!-- Desktop: Table -->
		<div class="courses-admin__table-wrap">
			<table class="courses-admin__table">
				<thead>
					<tr>
						<th class="th-thumb"></th>
						<th>Title</th>
						<th>Difficulty</th>
						<th>Lessons</th>
						<th>Duration</th>
						<th>Status</th>
						<th>Date</th>
					</tr>
				</thead>
				<tbody>
					{#each courses as course (course.id)}
						<tr>
							<td class="td-thumb">
								<a href="/admin/courses/{course.id}" class="thumb-link">
									{#if course.thumbnail_url}
										<img
											src={course.thumbnail_url}
											alt={course.title}
											class="table-thumb"
										/>
									{:else}
										<div class="table-thumb-placeholder">
											<BookOpen size={18} weight="duotone" />
										</div>
									{/if}
								</a>
							</td>
							<td>
								<a href="/admin/courses/{course.id}" class="title-link">
									{course.title}
								</a>
								<div class="title-sub">
									{#if course.is_free}
										Free
									{:else}
										${course.price}
									{/if}
								</div>
							</td>
							<td>
								<span class="badge {difficultyBadgeClass(course.difficulty)}">
									{capitalize(course.difficulty)}
								</span>
							</td>
							<td class="td-num">
								<span class="num-main">{course.lessons_count}</span>
								<span class="num-sub">{course.modules_count} modules</span>
							</td>
							<td class="td-duration">{course.estimated_duration || '--'}</td>
							<td>
								<span
									class="badge {course.is_published ? 'badge--published' : 'badge--draft'}"
								>
									{course.is_published ? 'Published' : 'Draft'}
								</span>
							</td>
							<td class="td-date">
								{course.is_published
									? formatDate(course.published_at)
									: formatDate(course.created_at)}
							</td>
						</tr>
					{/each}
				</tbody>
			</table>
		</div>

		<!-- Pagination -->
		{#if totalPages > 1}
			<div class="courses-admin__pagination">
				<button
					disabled={page <= 1}
					onclick={() => {
						page--;
						loadCourses();
					}}>Prev</button
				>
				<span>Page {page} of {totalPages} ({total} courses)</span>
				<button
					disabled={page >= totalPages}
					onclick={() => {
						page++;
						loadCourses();
					}}>Next</button
				>
			</div>
		{/if}
	{/if}
</div>

<style>
	.courses-admin {
		max-width: 100%;
	}

	/* ── Header ─────────────────────────── */
	.courses-admin__header {
		display: flex;
		flex-direction: column;
		gap: 1rem;
		margin-bottom: 1.5rem;
	}

	.courses-admin__title {
		font-size: var(--fs-xl);
		font-weight: var(--w-bold);
		font-family: var(--font-heading);
		color: var(--color-white);
		margin: 0;
	}

	.courses-admin__subtitle {
		font-size: var(--fs-sm);
		color: var(--color-grey-400);
		margin: 0.25rem 0 0 0;
	}

	.btn-primary {
		display: inline-flex;
		align-items: center;
		gap: 0.4rem;
		padding: 0.65rem 1.15rem;
		border-radius: var(--radius-lg);
		background: var(--color-teal);
		color: #fff;
		font-weight: var(--w-semibold);
		font-size: var(--fs-sm);
		text-decoration: none;
		border: none;
		cursor: pointer;
		transition:
			opacity var(--duration-150) var(--ease-out),
			transform var(--duration-150) var(--ease-out);
		white-space: nowrap;
	}

	.btn-primary:hover {
		opacity: 0.9;
		transform: translateY(-1px);
	}

	/* ── Stats bar ──────────────────────── */
	.stats-bar {
		display: grid;
		grid-template-columns: repeat(3, 1fr);
		gap: 0.75rem;
		margin-bottom: 1.5rem;
	}

	.stat-card {
		display: flex;
		align-items: center;
		gap: 0.75rem;
		padding: 1rem;
		background: rgba(255, 255, 255, 0.03);
		border: 1px solid rgba(255, 255, 255, 0.06);
		border-radius: var(--radius-xl);
		backdrop-filter: blur(12px);
	}

	.stat-card__icon {
		display: flex;
		align-items: center;
		justify-content: center;
		width: 2.5rem;
		height: 2.5rem;
		border-radius: var(--radius-lg);
		background: rgba(15, 164, 175, 0.12);
		color: var(--color-teal-light);
		flex-shrink: 0;
	}

	.stat-card__icon--green {
		background: rgba(34, 181, 115, 0.12);
		color: var(--color-green);
	}

	.stat-card__icon--amber {
		background: rgba(212, 168, 67, 0.12);
		color: var(--color-gold);
	}

	.stat-card__content {
		display: flex;
		flex-direction: column;
		min-width: 0;
	}

	.stat-card__value {
		font-size: var(--fs-xl);
		font-weight: var(--w-bold);
		color: var(--color-white);
		line-height: 1.2;
	}

	.stat-card__label {
		font-size: var(--fs-2xs);
		color: var(--color-grey-400);
		white-space: nowrap;
	}

	/* ── Filters ────────────────────────── */
	.courses-admin__filters {
		display: flex;
		flex-direction: column;
		gap: 0.75rem;
		margin-bottom: 1.25rem;
	}

	.courses-admin__status-tabs {
		display: flex;
		gap: 0.25rem;
	}

	.courses-admin__status-tabs button {
		padding: 0.4rem 0.75rem;
		border: none;
		border-radius: var(--radius-md);
		background: transparent;
		color: var(--color-grey-400);
		font-size: var(--fs-xs);
		cursor: pointer;
		transition: all var(--duration-150) var(--ease-out);
	}

	.courses-admin__status-tabs button:hover {
		color: var(--color-white);
	}

	.courses-admin__status-tabs button.active {
		background: rgba(15, 164, 175, 0.15);
		color: var(--color-teal-light);
	}

	.courses-admin__search-wrap {
		position: relative;
	}

	:global(.search-icon) {
		position: absolute;
		left: 0.75rem;
		top: 50%;
		transform: translateY(-50%);
		color: var(--color-grey-500) !important;
		pointer-events: none;
	}

	.courses-admin__search {
		width: 100%;
		padding: 0.55rem 0.75rem 0.55rem 2.25rem;
		border: 1px solid rgba(255, 255, 255, 0.1);
		border-radius: var(--radius-lg);
		background: rgba(0, 0, 0, 0.2);
		color: var(--color-white);
		font-size: var(--fs-sm);
		outline: none;
		transition: border-color var(--duration-200) var(--ease-out);
	}

	.courses-admin__search:focus {
		border-color: var(--color-teal);
	}

	.courses-admin__search::placeholder {
		color: var(--color-grey-500);
	}

	/* ── Skeleton loading ───────────────── */
	.skeleton-grid {
		display: flex;
		flex-direction: column;
		gap: 0.75rem;
	}

	.skeleton-card {
		display: flex;
		gap: 1rem;
		padding: 1rem;
		background: rgba(255, 255, 255, 0.03);
		border: 1px solid rgba(255, 255, 255, 0.06);
		border-radius: var(--radius-xl);
		animation: pulse 1.5s ease-in-out infinite;
	}

	.skeleton-card__thumb {
		width: 4.5rem;
		height: 3.25rem;
		border-radius: var(--radius-md);
		background: rgba(255, 255, 255, 0.06);
		flex-shrink: 0;
	}

	.skeleton-card__body {
		flex: 1;
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
	}

	.skeleton-line {
		border-radius: var(--radius-sm);
		background: rgba(255, 255, 255, 0.06);
		height: 0.75rem;
	}

	.skeleton-line--title {
		width: 65%;
		height: 0.9rem;
	}

	.skeleton-line--short {
		width: 40%;
	}

	.skeleton-line--meta {
		width: 55%;
	}

	@keyframes pulse {
		0%,
		100% {
			opacity: 1;
		}
		50% {
			opacity: 0.5;
		}
	}

	/* ── Empty state ────────────────────── */
	.empty-state {
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		text-align: center;
		padding: 4rem 2rem;
	}

	.empty-state__icon {
		display: flex;
		align-items: center;
		justify-content: center;
		width: 5rem;
		height: 5rem;
		border-radius: var(--radius-2xl);
		background: rgba(255, 255, 255, 0.04);
		color: var(--color-grey-500);
		margin-bottom: 1.5rem;
	}

	.empty-state__title {
		font-size: var(--fs-lg);
		font-weight: var(--w-semibold);
		color: var(--color-white);
		margin: 0 0 0.5rem 0;
	}

	.empty-state__desc {
		font-size: var(--fs-sm);
		color: var(--color-grey-400);
		margin: 0 0 1.5rem 0;
		max-width: 24rem;
	}

	/* ── Mobile Cards ───────────────────── */
	.courses-admin__cards {
		display: flex;
		flex-direction: column;
		gap: 0.75rem;
	}

	.courses-admin__table-wrap {
		display: none;
	}

	.course-card {
		display: flex;
		gap: 1rem;
		padding: 1rem;
		background: rgba(255, 255, 255, 0.03);
		border: 1px solid rgba(255, 255, 255, 0.06);
		border-radius: var(--radius-xl);
		text-decoration: none;
		transition: all var(--duration-200) var(--ease-out);
	}

	.course-card:hover {
		background: rgba(255, 255, 255, 0.05);
		border-color: rgba(15, 164, 175, 0.2);
	}

	.course-card__thumb {
		width: 4.5rem;
		height: 3.25rem;
		border-radius: var(--radius-md);
		overflow: hidden;
		flex-shrink: 0;
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
		background: rgba(255, 255, 255, 0.04);
		color: var(--color-grey-500);
	}

	.course-card__body {
		flex: 1;
		min-width: 0;
		display: flex;
		flex-direction: column;
		gap: 0.35rem;
	}

	.course-card__top {
		display: flex;
		justify-content: space-between;
		align-items: flex-start;
		gap: 0.5rem;
	}

	.course-card__title {
		font-size: var(--fs-sm);
		font-weight: var(--w-semibold);
		color: var(--color-white);
		margin: 0;
		line-height: var(--lh-snug);
	}

	.course-card__tags {
		display: flex;
		gap: 0.35rem;
		align-items: center;
		flex-wrap: wrap;
	}

	.course-card__price {
		font-size: var(--fs-xs);
		font-weight: var(--w-semibold);
		color: var(--color-grey-300);
	}

	.course-card__meta {
		display: flex;
		align-items: center;
		gap: 0.35rem;
		font-size: var(--fs-2xs);
		color: var(--color-grey-400);
	}

	.course-card__meta-dot {
		width: 3px;
		height: 3px;
		border-radius: 50%;
		background: var(--color-grey-600);
	}

	.course-card__date {
		font-size: var(--fs-2xs);
		color: var(--color-grey-500);
	}

	/* ── Badges ─────────────────────────── */
	.badge {
		display: inline-block;
		padding: 0.15rem 0.5rem;
		border-radius: var(--radius-md);
		font-size: var(--fs-2xs);
		font-weight: var(--w-semibold);
		white-space: nowrap;
	}

	.badge--published {
		background: rgba(34, 181, 115, 0.15);
		color: var(--color-green);
	}

	.badge--draft {
		background: rgba(148, 163, 184, 0.15);
		color: #94a3b8;
	}

	.badge--beginner {
		background: rgba(59, 130, 246, 0.15);
		color: #60a5fa;
	}

	.badge--intermediate {
		background: rgba(234, 179, 8, 0.15);
		color: #eab308;
	}

	.badge--advanced {
		background: rgba(239, 68, 68, 0.15);
		color: #f87171;
	}

	.badge--free {
		background: rgba(15, 164, 175, 0.15);
		color: var(--color-teal-light);
	}

	/* ── Pagination ─────────────────────── */
	.courses-admin__pagination {
		display: flex;
		align-items: center;
		justify-content: center;
		gap: 0.75rem;
		margin-top: 1.5rem;
	}

	.courses-admin__pagination button {
		padding: 0.4rem 0.85rem;
		border: 1px solid rgba(255, 255, 255, 0.1);
		border-radius: var(--radius-md);
		background: transparent;
		color: var(--color-grey-300);
		font-size: var(--fs-xs);
		cursor: pointer;
		transition: all var(--duration-150) var(--ease-out);
	}

	.courses-admin__pagination button:hover:not(:disabled) {
		background: rgba(255, 255, 255, 0.05);
		border-color: rgba(255, 255, 255, 0.15);
	}

	.courses-admin__pagination button:disabled {
		opacity: 0.3;
		cursor: not-allowed;
	}

	.courses-admin__pagination span {
		font-size: var(--fs-xs);
		color: var(--color-grey-400);
	}

	/* ── Tablet+ ────────────────────────── */
	@media (min-width: 768px) {
		.courses-admin__header {
			flex-direction: row;
			align-items: center;
			justify-content: space-between;
			margin-bottom: 2rem;
		}

		.courses-admin__title {
			font-size: var(--fs-2xl);
		}

		.btn-primary {
			padding: 0.7rem 1.35rem;
		}

		.stats-bar {
			gap: 1rem;
			margin-bottom: 2rem;
		}

		.stat-card {
			padding: 1.25rem;
		}

		.stat-card__value {
			font-size: var(--fs-2xl);
		}

		.stat-card__label {
			font-size: var(--fs-xs);
		}

		.courses-admin__filters {
			flex-direction: row;
			align-items: center;
			justify-content: space-between;
			margin-bottom: 1.5rem;
		}

		.courses-admin__search-wrap {
			width: 16rem;
		}

		.courses-admin__cards {
			display: none;
		}

		.courses-admin__table-wrap {
			display: block;
			overflow-x: auto;
			background: rgba(255, 255, 255, 0.02);
			border: 1px solid rgba(255, 255, 255, 0.06);
			border-radius: var(--radius-xl);
		}

		.courses-admin__table {
			width: 100%;
			border-collapse: collapse;
		}

		.courses-admin__table th {
			text-align: left;
			padding: 0.75rem 1rem;
			font-size: var(--fs-xs);
			font-weight: var(--w-semibold);
			text-transform: uppercase;
			letter-spacing: 0.05em;
			color: var(--color-grey-400);
			border-bottom: 1px solid rgba(255, 255, 255, 0.06);
		}

		.courses-admin__table td {
			padding: 0.85rem 1rem;
			border-bottom: 1px solid rgba(255, 255, 255, 0.04);
			font-size: var(--fs-sm);
			color: var(--color-grey-200);
			vertical-align: middle;
		}

		.courses-admin__table tbody tr {
			transition: background var(--duration-150) var(--ease-out);
		}

		.courses-admin__table tbody tr:hover {
			background: rgba(255, 255, 255, 0.02);
		}

		.th-thumb {
			width: 4rem;
		}

		.td-thumb {
			width: 4rem;
		}

		.thumb-link {
			display: block;
		}

		.table-thumb {
			width: 3.5rem;
			height: 2.5rem;
			object-fit: cover;
			border-radius: var(--radius-md);
		}

		.table-thumb-placeholder {
			width: 3.5rem;
			height: 2.5rem;
			border-radius: var(--radius-md);
			display: flex;
			align-items: center;
			justify-content: center;
			background: rgba(255, 255, 255, 0.04);
			color: var(--color-grey-500);
		}

		.title-link {
			color: var(--color-white);
			font-weight: var(--w-semibold);
			text-decoration: none;
			transition: color var(--duration-150) var(--ease-out);
		}

		.title-link:hover {
			color: var(--color-teal-light);
		}

		.title-sub {
			font-size: var(--fs-xs);
			color: var(--color-grey-400);
			margin-top: 0.15rem;
		}

		.td-num {
			white-space: nowrap;
		}

		.num-main {
			font-weight: var(--w-semibold);
			color: var(--color-white);
		}

		.num-sub {
			font-size: var(--fs-xs);
			color: var(--color-grey-500);
			margin-left: 0.35rem;
		}

		.td-duration {
			white-space: nowrap;
			color: var(--color-grey-300);
		}

		.td-date {
			white-space: nowrap;
			font-size: var(--fs-xs);
			color: var(--color-grey-400);
		}

		.skeleton-grid {
			gap: 0.5rem;
		}
	}

	/* ── Desktop ────────────────────────── */
	@media (min-width: 1024px) {
		.stat-card {
			padding: 1.5rem;
		}

		.stat-card__icon {
			width: 3rem;
			height: 3rem;
		}
	}
</style>
