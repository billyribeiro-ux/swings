<script lang="ts">
	import { onMount } from 'svelte';
	import { api } from '$lib/api/client';
	import MagnifyingGlassIcon from 'phosphor-svelte/lib/MagnifyingGlassIcon';
	import PlusIcon from 'phosphor-svelte/lib/PlusIcon';
	import BookOpenIcon from 'phosphor-svelte/lib/BookOpenIcon';
	import ClockIcon from 'phosphor-svelte/lib/ClockIcon';
	import GraduationCapIcon from 'phosphor-svelte/lib/GraduationCapIcon';
	import PencilSimpleIcon from 'phosphor-svelte/lib/PencilSimpleIcon';
	import CaretLeftIcon from 'phosphor-svelte/lib/CaretLeftIcon';
	import CaretRightIcon from 'phosphor-svelte/lib/CaretRightIcon';
	import CaretDownIcon from 'phosphor-svelte/lib/CaretDownIcon';

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

	// One-shot initial load. The previous `$effect(() => loadCourses())` would
	// also fire whenever the *captured* read of `page`/`statusFilter`/`search`
	// inside `loadCourses` changed — combined with the explicit `loadCourses()`
	// call inside `handleSearch`/`changeStatus` that meant every keystroke
	// fired the API twice and the loop interacted with `loading=true` writes.
	// `onMount` runs exactly once; subsequent loads are user-triggered.
	onMount(loadCourses);

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

	function changeStatus(value: string) {
		statusFilter = value as 'all' | 'published' | 'draft';
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
		if (!d) return '—';
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
	<title>Courses - Admin</title>
</svelte:head>

<div class="courses-admin">
	<!-- Header -->
	<header class="courses-admin__header">
		<div class="courses-admin__heading">
			<span class="courses-admin__eyebrow">Education</span>
			<h1 class="courses-admin__title">Courses</h1>
			<p class="courses-admin__subtitle">
				Manage your course catalog: drafts, published lessons, and learner progression.
			</p>
		</div>
		<a href="/admin/courses/new" class="btn-primary">
			<PlusIcon size={16} weight="bold" />
			<span>New course</span>
		</a>
	</header>

	<!-- Stats bar -->
	{#if !loading}
		<div class="stats-bar">
			<div class="stat-card">
				<div class="stat-card__icon">
					<BookOpenIcon size={20} weight="duotone" />
				</div>
				<div class="stat-card__content">
					<span class="stat-card__label">Total courses</span>
					<span class="stat-card__value">{total}</span>
				</div>
			</div>
			<div class="stat-card">
				<div class="stat-card__icon stat-card__icon--green">
					<GraduationCapIcon size={20} weight="duotone" />
				</div>
				<div class="stat-card__content">
					<span class="stat-card__label">Published</span>
					<span class="stat-card__value">{publishedCount}</span>
				</div>
			</div>
			<div class="stat-card">
				<div class="stat-card__icon stat-card__icon--amber">
					<ClockIcon size={20} weight="duotone" />
				</div>
				<div class="stat-card__content">
					<span class="stat-card__label">Drafts</span>
					<span class="stat-card__value">{draftCount}</span>
				</div>
			</div>
		</div>
	{/if}

	<!-- Filters card -->
	<div class="filter-card">
		<div class="filter-field filter-field--search">
			<label class="filter-field__label" for="course-search">Search</label>
			<div class="search-wrap">
				<MagnifyingGlassIcon size={16} weight="bold" class="search-icon" />
				<input
					id="course-search"
					name="course-search"
					type="search"
					class="filter-input filter-input--search"
					placeholder="Search by title…"
					oninput={handleSearch}
				/>
			</div>
		</div>
		<div class="filter-field">
			<label class="filter-field__label" for="course-status">Status</label>
			<div class="select-wrap">
				<select
					id="course-status"
					name="course-status"
					class="filter-input filter-input--select"
					value={statusFilter}
					onchange={(e) => changeStatus(e.currentTarget.value)}
				>
					<option value="all">All statuses</option>
					<option value="published">Published</option>
					<option value="draft">Drafts</option>
				</select>
				<CaretDownIcon size={14} weight="bold" class="select-caret" />
			</div>
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
			<BookOpenIcon size={48} weight="duotone" />
			<h2 class="empty-state__title">No courses found</h2>
			<p class="empty-state__desc">
				{#if search || statusFilter !== 'all'}
					Try adjusting your search or filters to see more courses.
				{:else}
					Get started by creating your first course.
				{/if}
			</p>
			{#if !search && statusFilter === 'all'}
				<a href="/admin/courses/new" class="btn-primary">
					<PlusIcon size={16} weight="bold" />
					<span>Create course</span>
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
							<img
								src={course.thumbnail_url}
								alt={course.title}
								class="course-card__img"
								width="640"
								height="360"
								loading="lazy"
								decoding="async"
							/>
						{:else}
							<div class="course-card__placeholder">
								<BookOpenIcon size={28} weight="duotone" />
							</div>
						{/if}
					</div>
					<div class="course-card__body">
						<div class="course-card__top">
							<h3 class="course-card__title">{course.title}</h3>
							<span class="badge {course.is_published ? 'badge--published' : 'badge--draft'}">
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
						<th class="th-num">Lessons</th>
						<th>Duration</th>
						<th>Status</th>
						<th>Date</th>
						<th class="th-actions" aria-label="Actions"></th>
					</tr>
				</thead>
				<tbody>
					{#each courses as course (course.id)}
						<tr>
							<td class="td-thumb">
								<a href="/admin/courses/{course.id}" class="thumb-link">
									{#if course.thumbnail_url}
										<img src={course.thumbnail_url} alt={course.title} class="table-thumb" />
									{:else}
										<div class="table-thumb-placeholder">
											<BookOpenIcon size={18} weight="duotone" />
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
								<span class="num-sub">{course.modules_count} mods</span>
							</td>
							<td class="td-duration">{course.estimated_duration || '—'}</td>
							<td>
								<span class="badge {course.is_published ? 'badge--published' : 'badge--draft'}">
									{course.is_published ? 'Published' : 'Draft'}
								</span>
							</td>
							<td class="td-date">
								{course.is_published
									? formatDate(course.published_at)
									: formatDate(course.created_at)}
							</td>
							<td class="td-actions">
								<a
									href="/admin/courses/{course.id}"
									class="icon-btn"
									title="Edit {course.title}"
									aria-label="Edit {course.title}"
								>
									<PencilSimpleIcon size={16} weight="bold" />
								</a>
							</td>
						</tr>
					{/each}
				</tbody>
			</table>
		</div>

		<!-- Pagination -->
		{#if totalPages > 1}
			<nav class="courses-admin__pagination" aria-label="Pagination">
				<button
					type="button"
					class="page-btn"
					disabled={page <= 1}
					onclick={() => {
						page--;
						loadCourses();
					}}
				>
					<CaretLeftIcon size={14} weight="bold" />
					<span>Previous</span>
				</button>
				<span class="page-info">Page {page} of {totalPages}</span>
				<button
					type="button"
					class="page-btn"
					disabled={page >= totalPages}
					onclick={() => {
						page++;
						loadCourses();
					}}
				>
					<span>Next</span>
					<CaretRightIcon size={14} weight="bold" />
				</button>
			</nav>
		{/if}
	{/if}
</div>

<style>
	.courses-admin {
		display: flex;
		flex-direction: column;
		gap: 1.25rem;
		max-width: 100%;
	}

	/* ── Header ─────────────────────────── */
	.courses-admin__header {
		display: flex;
		flex-direction: column;
		gap: 1rem;
		align-items: flex-start;
	}

	.courses-admin__heading {
		display: flex;
		flex-direction: column;
		gap: 0.4rem;
		min-width: 0;
	}

	.courses-admin__eyebrow {
		font-size: 0.6875rem;
		font-weight: 700;
		color: var(--color-grey-500);
		text-transform: uppercase;
		letter-spacing: 0.06em;
	}

	.courses-admin__title {
		font-size: 1.5rem;
		font-weight: 700;
		font-family: var(--font-heading);
		color: var(--color-white);
		line-height: 1.15;
		letter-spacing: -0.01em;
		margin: 0;
	}

	.courses-admin__subtitle {
		font-size: 0.875rem;
		color: var(--color-grey-400);
		max-width: 60ch;
		line-height: 1.55;
		margin: 0;
	}

	.btn-primary {
		display: inline-flex;
		align-items: center;
		gap: 0.5rem;
		min-height: 2.5rem;
		padding: 0.55rem 1rem;
		border-radius: var(--radius-lg);
		background: linear-gradient(135deg, var(--color-teal), var(--color-teal-dark));
		color: var(--color-white);
		font-weight: 600;
		font-size: 0.875rem;
		font-family: var(--font-ui);
		text-decoration: none;
		border: none;
		cursor: pointer;
		white-space: nowrap;
		box-shadow: 0 6px 16px -4px rgba(15, 164, 175, 0.45);
		transition:
			transform 150ms var(--ease-out),
			box-shadow 150ms var(--ease-out),
			opacity 150ms var(--ease-out);
	}

	.btn-primary:hover {
		transform: translateY(-1px);
		box-shadow: 0 10px 22px -4px rgba(15, 164, 175, 0.55);
	}

	.btn-primary:active {
		transform: translateY(0);
	}

	/* ── Stats bar ──────────────────────── */
	.stats-bar {
		display: grid;
		grid-template-columns: repeat(3, 1fr);
		gap: 0.75rem;
	}

	.stat-card {
		display: flex;
		align-items: center;
		gap: 0.85rem;
		padding: 1.25rem;
		background-color: var(--color-navy-mid);
		border: 1px solid rgba(255, 255, 255, 0.06);
		border-radius: var(--radius-xl);
		box-shadow:
			0 1px 0 rgba(255, 255, 255, 0.03) inset,
			0 12px 32px rgba(0, 0, 0, 0.18);
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
		gap: 0.1rem;
		min-width: 0;
	}

	.stat-card__label {
		font-size: 0.6875rem;
		font-weight: 600;
		color: var(--color-grey-500);
		text-transform: uppercase;
		letter-spacing: 0.05em;
		white-space: nowrap;
	}

	.stat-card__value {
		font-size: 1.5rem;
		font-weight: 700;
		color: var(--color-white);
		line-height: 1.15;
		font-variant-numeric: tabular-nums;
	}

	/* ── Filters card ───────────────────── */
	.filter-card {
		display: flex;
		flex-direction: column;
		gap: 0.75rem;
		padding: 1.25rem;
		background-color: var(--color-navy-mid);
		border: 1px solid rgba(255, 255, 255, 0.06);
		border-radius: var(--radius-xl);
		box-shadow:
			0 1px 0 rgba(255, 255, 255, 0.03) inset,
			0 12px 32px rgba(0, 0, 0, 0.18);
	}

	.filter-field {
		display: flex;
		flex-direction: column;
		gap: 0.35rem;
		min-width: 0;
	}

	.filter-field__label {
		font-size: 0.6875rem;
		font-weight: 600;
		color: var(--color-grey-400);
		text-transform: uppercase;
		letter-spacing: 0.05em;
	}

	.search-wrap,
	.select-wrap {
		position: relative;
	}

	:global(.search-icon) {
		position: absolute;
		left: 0.75rem;
		top: 50%;
		transform: translateY(-50%);
		color: var(--color-grey-500);
		pointer-events: none;
	}

	:global(.select-caret) {
		position: absolute;
		right: 0.75rem;
		top: 50%;
		transform: translateY(-50%);
		color: var(--color-grey-500);
		pointer-events: none;
	}

	.filter-input {
		width: 100%;
		min-height: 2.5rem;
		padding: 0.65rem 0.875rem;
		background-color: rgba(255, 255, 255, 0.05);
		border: 1px solid rgba(255, 255, 255, 0.1);
		border-radius: var(--radius-lg);
		color: var(--color-white);
		font-size: 0.875rem;
		font-family: var(--font-ui);
		outline: none;
		transition:
			border-color 150ms var(--ease-out),
			box-shadow 150ms var(--ease-out);
	}

	.filter-input:focus {
		border-color: var(--color-teal);
		box-shadow: 0 0 0 3px rgba(15, 164, 175, 0.15);
	}

	.filter-input::placeholder {
		color: var(--color-grey-500);
	}

	.filter-input--search {
		padding-left: 2.25rem;
	}

	.filter-input--select {
		appearance: none;
		-webkit-appearance: none;
		-moz-appearance: none;
		padding-right: 2.25rem;
		cursor: pointer;
	}

	.filter-input--select option {
		background-color: var(--color-navy-mid);
		color: var(--color-white);
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
		background-color: var(--color-navy-mid);
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
		gap: 0.85rem;
		padding: 3.5rem 2rem;
		background-color: var(--color-navy-mid);
		border: 1px solid rgba(255, 255, 255, 0.06);
		border-radius: var(--radius-xl);
		color: var(--color-grey-500);
	}

	.empty-state :global(svg) {
		color: var(--color-grey-500);
	}

	.empty-state__title {
		font-size: 1rem;
		font-weight: 600;
		color: var(--color-white);
		margin: 0;
	}

	.empty-state__desc {
		font-size: 0.875rem;
		color: var(--color-grey-400);
		max-width: 36ch;
		line-height: 1.55;
		margin: 0;
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
		background-color: var(--color-navy-mid);
		border: 1px solid rgba(255, 255, 255, 0.06);
		border-radius: var(--radius-xl);
		text-decoration: none;
		transition:
			background-color 200ms var(--ease-out),
			border-color 200ms var(--ease-out);
	}

	.course-card:hover {
		background-color: rgba(255, 255, 255, 0.04);
		border-color: rgba(15, 164, 175, 0.25);
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
		gap: 0.4rem;
	}

	.course-card__top {
		display: flex;
		justify-content: space-between;
		align-items: flex-start;
		gap: 0.5rem;
	}

	.course-card__title {
		font-size: 0.875rem;
		font-weight: 600;
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
		font-size: 0.75rem;
		font-weight: 600;
		color: var(--color-grey-300);
		font-variant-numeric: tabular-nums;
	}

	.course-card__meta {
		display: flex;
		align-items: center;
		gap: 0.4rem;
		font-size: 0.6875rem;
		color: var(--color-grey-400);
	}

	.course-card__meta-dot {
		width: 3px;
		height: 3px;
		border-radius: 50%;
		background: var(--color-grey-600);
	}

	.course-card__date {
		font-size: 0.6875rem;
		color: var(--color-grey-500);
		font-variant-numeric: tabular-nums;
	}

	/* ── Badges ─────────────────────────── */
	.badge {
		display: inline-flex;
		align-items: center;
		padding: 0.15rem 0.5rem;
		border-radius: var(--radius-full);
		font-size: 0.6875rem;
		font-weight: 600;
		text-transform: uppercase;
		letter-spacing: 0.05em;
		white-space: nowrap;
	}

	.badge--published {
		background: rgba(15, 164, 175, 0.12);
		color: #5eead4;
	}

	.badge--draft {
		background: rgba(255, 255, 255, 0.06);
		color: var(--color-grey-300);
	}

	.badge--beginner {
		background: rgba(59, 130, 246, 0.15);
		color: #60a5fa;
	}

	.badge--intermediate {
		background: rgba(245, 158, 11, 0.12);
		color: #fcd34d;
	}

	.badge--advanced {
		background: rgba(239, 68, 68, 0.12);
		color: #fca5a5;
	}

	.badge--free {
		background: rgba(15, 164, 175, 0.12);
		color: #5eead4;
	}

	/* ── Pagination ─────────────────────── */
	.courses-admin__pagination {
		display: flex;
		align-items: center;
		justify-content: center;
		gap: 0.75rem;
		margin-top: 0.5rem;
	}

	.page-btn {
		display: inline-flex;
		align-items: center;
		gap: 0.4rem;
		min-height: 2.25rem;
		padding: 0.45rem 0.75rem;
		background-color: rgba(255, 255, 255, 0.05);
		border: 1px solid rgba(255, 255, 255, 0.1);
		border-radius: var(--radius-lg);
		color: var(--color-white);
		font-size: 0.875rem;
		font-weight: 600;
		font-family: var(--font-ui);
		cursor: pointer;
		transition:
			background-color 150ms var(--ease-out),
			border-color 150ms var(--ease-out);
	}

	.page-btn:hover:not(:disabled) {
		background-color: rgba(255, 255, 255, 0.1);
		border-color: rgba(255, 255, 255, 0.18);
	}

	.page-btn:disabled {
		opacity: 0.4;
		cursor: not-allowed;
	}

	.page-info {
		font-size: 0.75rem;
		font-weight: 500;
		color: var(--color-grey-400);
		font-variant-numeric: tabular-nums;
	}

	/* ── Tablet 480px+ ──────────────────── */
	@media (min-width: 480px) {
		.filter-card {
			flex-direction: row;
			align-items: flex-end;
		}

		.filter-field--search {
			flex: 1 1 18rem;
		}

		.filter-field {
			flex: 0 0 12rem;
		}
	}

	/* ── Tablet+ 768px+ ─────────────────── */
	@media (min-width: 768px) {
		.courses-admin {
			gap: 1.5rem;
		}

		.courses-admin__header {
			flex-direction: row;
			align-items: flex-end;
			justify-content: space-between;
			gap: 1.5rem;
		}


		.stats-bar {
			gap: 1rem;
		}

		.stat-card {
			padding: 1.5rem;
		}


		.filter-card {
			padding: 1.5rem;
		}

		.courses-admin__cards {
			display: none;
		}

		.courses-admin__table-wrap {
			display: block;
			overflow-x: auto;
			background-color: var(--color-navy-mid);
			border: 1px solid rgba(255, 255, 255, 0.06);
			border-radius: var(--radius-xl);
			box-shadow:
				0 1px 0 rgba(255, 255, 255, 0.03) inset,
				0 12px 32px rgba(0, 0, 0, 0.18);
		}

		.courses-admin__table {
			width: 100%;
			border-collapse: collapse;
			min-width: 720px;
		}

		.courses-admin__table thead {
			background-color: rgba(255, 255, 255, 0.02);
		}

		.courses-admin__table th {
			text-align: left;
			padding: 0.875rem 1rem;
			font-size: 0.6875rem;
			font-weight: 600;
			text-transform: uppercase;
			letter-spacing: 0.05em;
			color: var(--color-grey-500);
			border-bottom: 1px solid rgba(255, 255, 255, 0.06);
			white-space: nowrap;
		}

		.courses-admin__table td {
			padding: 0.875rem 1rem;
			border-bottom: 1px solid rgba(255, 255, 255, 0.04);
			font-size: 0.875rem;
			color: var(--color-grey-300);
			vertical-align: middle;
			line-height: 1.45;
		}

		.courses-admin__table tbody tr {
			transition: background-color 150ms var(--ease-out);
		}

		.courses-admin__table tbody tr:hover {
			background-color: rgba(255, 255, 255, 0.02);
		}

		.courses-admin__table tbody tr:last-child td {
			border-bottom: none;
		}

		.th-thumb,
		.td-thumb {
			width: 4rem;
		}

		.th-num,
		.td-num {
			text-align: right;
			white-space: nowrap;
			font-variant-numeric: tabular-nums;
		}

		.th-actions,
		.td-actions {
			width: 3rem;
			text-align: right;
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
			font-weight: 600;
			text-decoration: none;
			transition: color 150ms var(--ease-out);
		}

		.title-link:hover {
			color: var(--color-teal-light);
		}

		.title-sub {
			font-size: 0.75rem;
			color: var(--color-grey-400);
			margin-top: 0.15rem;
			font-variant-numeric: tabular-nums;
		}

		.num-main {
			font-weight: 600;
			color: var(--color-white);
		}

		.num-sub {
			font-size: 0.75rem;
			color: var(--color-grey-500);
			margin-left: 0.35rem;
		}

		.td-duration {
			white-space: nowrap;
			color: var(--color-grey-300);
		}

		.td-date {
			white-space: nowrap;
			font-size: 0.75rem;
			color: var(--color-grey-400);
			font-variant-numeric: tabular-nums;
		}

		.icon-btn {
			display: inline-flex;
			align-items: center;
			justify-content: center;
			width: 2rem;
			height: 2rem;
			background-color: rgba(255, 255, 255, 0.05);
			border: 1px solid rgba(255, 255, 255, 0.1);
			border-radius: var(--radius-md);
			color: var(--color-grey-300);
			text-decoration: none;
			cursor: pointer;
			transition:
				background-color 150ms var(--ease-out),
				border-color 150ms var(--ease-out),
				color 150ms var(--ease-out);
		}

		.icon-btn:hover {
			background-color: rgba(15, 164, 175, 0.12);
			border-color: rgba(15, 164, 175, 0.3);
			color: var(--color-teal-light);
		}

		.skeleton-grid {
			gap: 0.5rem;
		}
	}

	/* ── Desktop 1024px+ ────────────────── */
	@media (min-width: 1024px) {
		.stat-card {
			padding: 1.75rem;
		}

		.stat-card__icon {
			width: 3rem;
			height: 3rem;
		}
	}
</style>
