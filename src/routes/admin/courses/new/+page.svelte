<script lang="ts">
	import { goto } from '$app/navigation';
	import { api } from '$lib/api/client';
	import ArrowLeft from 'phosphor-svelte/lib/ArrowLeft';
	import FloppyDisk from 'phosphor-svelte/lib/FloppyDisk';
	import Image from 'phosphor-svelte/lib/Image';
	import VideoCamera from 'phosphor-svelte/lib/VideoCamera';
	import CurrencyDollar from 'phosphor-svelte/lib/CurrencyDollar';
	import Clock from 'phosphor-svelte/lib/Clock';
	import GraduationCap from 'phosphor-svelte/lib/GraduationCap';

	interface CourseResponse {
		id: string;
		title: string;
		slug: string;
	}

	let title = $state('');
	let slug = $state('');
	let slugManual = $state(false);
	let description = $state('');
	let shortDescription = $state('');
	let difficulty: 'beginner' | 'intermediate' | 'advanced' = $state('beginner');
	let isFree = $state(false);
	let price = $state(0);
	let thumbnailUrl = $state('');
	let trailerVideoUrl = $state('');
	let estimatedDuration = $state('');
	let isPublished = $state(false);

	let saving = $state(false);
	let error = $state('');

	function generateSlug(text: string): string {
		return text
			.toLowerCase()
			.replace(/[^a-z0-9\s-]/g, '')
			.replace(/\s+/g, '-')
			.replace(/-+/g, '-')
			.replace(/^-|-$/g, '');
	}

	function handleTitleInput() {
		if (!slugManual) {
			slug = generateSlug(title);
		}
	}

	function handleSlugInput() {
		slugManual = true;
	}

	async function handleSubmit(e: Event) {
		e.preventDefault();
		if (!title.trim()) {
			error = 'Title is required.';
			return;
		}

		saving = true;
		error = '';

		try {
			const payload: Record<string, unknown> = {
				title: title.trim(),
				slug: slug.trim() || generateSlug(title),
				description: description.trim(),
				short_description: shortDescription.trim() || null,
				difficulty,
				is_free: isFree,
				price: isFree ? 0 : price,
				thumbnail_url: thumbnailUrl.trim() || null,
				trailer_video_url: trailerVideoUrl.trim() || null,
				estimated_duration: estimatedDuration.trim() || null,
				is_published: isPublished
			};

			const res = await api.post<CourseResponse>('/api/admin/courses', payload);
			await goto(`/admin/courses/${res.id}`);
		} catch (e) {
			console.error('Failed to create course', e);
			error = e instanceof Error ? e.message : 'Failed to create course.';
		} finally {
			saving = false;
		}
	}
</script>

<svelte:head>
	<title>New Course -- Admin</title>
</svelte:head>

<div class="new-course">
	<!-- Header -->
	<div class="new-course__header">
		<a href="/admin/courses" class="back-link">
			<ArrowLeft size={18} weight="bold" />
			Back to Courses
		</a>
		<h1 class="new-course__title">Create New Course</h1>
		<p class="new-course__subtitle">Set up your course details and settings</p>
	</div>

	{#if error}
		<div class="error-banner">{error}</div>
	{/if}

	<form onsubmit={handleSubmit} class="new-course__layout">
		<!-- Main content -->
		<div class="new-course__main">
			<div class="form-card">
				<h2 class="form-card__heading">Course Details</h2>

				<div class="field">
					<label for="course-title" class="field__label">Title</label>
					<input
						id="course-title"
						name="title"
						type="text"
						class="field__input"
						placeholder="e.g. Beginning Options Trading"
						bind:value={title}
						oninput={handleTitleInput}
						required
					/>
				</div>

				<div class="field">
					<label for="course-slug" class="field__label">
						Slug
						<span class="field__hint">Auto-generated from title</span>
					</label>
					<input
						id="course-slug"
						name="slug"
						type="text"
						class="field__input field__input--mono"
						placeholder="beginning-options-trading"
						bind:value={slug}
						oninput={handleSlugInput}
					/>
				</div>

				<div class="field">
					<label for="course-short-desc" class="field__label">
						Short Description
						<span class="field__hint">One-liner for card previews</span>
					</label>
					<input
						id="course-short-desc"
						name="short_description"
						type="text"
						class="field__input"
						placeholder="A brief summary of the course..."
						bind:value={shortDescription}
					/>
				</div>

				<div class="field">
					<label for="course-desc" class="field__label">Description</label>
					<textarea
						id="course-desc"
						name="description"
						class="field__textarea"
						placeholder="Full course description. Supports markdown..."
						rows="8"
						bind:value={description}
					></textarea>
				</div>
			</div>
		</div>

		<!-- Sidebar settings -->
		<div class="new-course__sidebar">
			<!-- Publish -->
			<div class="form-card">
				<h2 class="form-card__heading">Publish</h2>

				<div class="toggle-row">
					<label for="publish-toggle" class="toggle-row__label">
						Publish immediately
					</label>
					<button
						id="publish-toggle"
						type="button"
						class="toggle"
						class:toggle--active={isPublished}
						onclick={() => (isPublished = !isPublished)}
						role="switch"
						aria-checked={isPublished}
						aria-label="Publish immediately"
					>
						<span class="toggle__knob"></span>
					</button>
				</div>

				<button type="submit" class="btn-create" disabled={saving || !title.trim()}>
					<FloppyDisk size={18} weight="bold" />
					{saving ? 'Creating...' : 'Create Course'}
				</button>
			</div>

			<!-- Difficulty -->
			<div class="form-card">
				<h2 class="form-card__heading">
					<GraduationCap size={18} weight="duotone" />
					Difficulty
				</h2>

				<div class="radio-group">
					{#each ['beginner', 'intermediate', 'advanced'] as level (level)}
						<label class="radio-pill" class:radio-pill--active={difficulty === level}>
							<input
								type="radio"
								name="difficulty"
								value={level}
								bind:group={difficulty}
								class="sr-only"
							/>
							<span class="radio-pill__text">{level.charAt(0).toUpperCase() + level.slice(1)}</span>
						</label>
					{/each}
				</div>
			</div>

			<!-- Pricing -->
			<div class="form-card">
				<h2 class="form-card__heading">
					<CurrencyDollar size={18} weight="duotone" />
					Pricing
				</h2>

				<div class="toggle-row">
					<label for="free-toggle" class="toggle-row__label">Free course</label>
					<button
						id="free-toggle"
						type="button"
						class="toggle"
						class:toggle--active={isFree}
						onclick={() => (isFree = !isFree)}
						role="switch"
						aria-checked={isFree}
					>
						<span class="toggle__knob"></span>
					</button>
				</div>

				{#if !isFree}
					<div class="field">
						<label for="course-price" class="field__label">Price (USD)</label>
						<div class="field__input-group">
							<span class="field__input-prefix">$</span>
							<input
								id="course-price"
								name="price"
								type="number"
								min="0"
								step="1"
								class="field__input field__input--prefixed"
								bind:value={price}
							/>
						</div>
					</div>
				{/if}
			</div>

			<!-- Media -->
			<div class="form-card">
				<h2 class="form-card__heading">
					<Image size={18} weight="duotone" />
					Media
				</h2>

				<div class="field">
					<label for="course-thumb" class="field__label">Thumbnail URL</label>
					<input
						id="course-thumb"
						name="thumbnail_url"
						type="url"
						class="field__input"
						placeholder="https://..."
						bind:value={thumbnailUrl}
					/>
				</div>

				{#if thumbnailUrl}
					<div class="thumb-preview">
						<img src={thumbnailUrl} alt="Thumbnail preview" class="thumb-preview__img" />
					</div>
				{/if}

				<div class="field">
					<label for="course-trailer" class="field__label">
						<VideoCamera size={14} weight="bold" />
						Trailer Video URL
					</label>
					<input
						id="course-trailer"
						name="trailer_video_url"
						type="url"
						class="field__input"
						placeholder="https://..."
						bind:value={trailerVideoUrl}
					/>
				</div>
			</div>

			<!-- Duration -->
			<div class="form-card">
				<h2 class="form-card__heading">
					<Clock size={18} weight="duotone" />
					Duration
				</h2>

				<div class="field">
					<label for="course-duration" class="field__label">Estimated Duration</label>
					<input
						id="course-duration"
						name="estimated_duration"
						type="text"
						class="field__input"
						placeholder="e.g. 4 weeks, 12 hours"
						bind:value={estimatedDuration}
					/>
				</div>
			</div>
		</div>
	</form>
</div>

<style>
	.new-course {
		max-width: 100%;
	}

	/* ── Header ─────────────────────────── */
	.new-course__header {
		margin-bottom: 1.5rem;
	}

	.back-link {
		display: inline-flex;
		align-items: center;
		gap: 0.4rem;
		color: var(--color-grey-400);
		font-size: var(--fs-sm);
		font-weight: var(--w-medium);
		text-decoration: none;
		margin-bottom: 1rem;
		transition: color var(--duration-150) var(--ease-out);
	}

	.back-link:hover {
		color: var(--color-teal-light);
	}

	.new-course__title {
		font-size: var(--fs-xl);
		font-weight: var(--w-bold);
		font-family: var(--font-heading);
		color: var(--color-white);
		margin: 0;
	}

	.new-course__subtitle {
		font-size: var(--fs-sm);
		color: var(--color-grey-400);
		margin: 0.25rem 0 0 0;
	}

	/* ── Error banner ───────────────────── */
	.error-banner {
		padding: 0.75rem 1rem;
		background: rgba(239, 68, 68, 0.1);
		border: 1px solid rgba(239, 68, 68, 0.3);
		border-radius: var(--radius-lg);
		color: #fca5a5;
		font-size: var(--fs-sm);
		margin-bottom: 1.5rem;
	}

	/* ── Layout ─────────────────────────── */
	.new-course__layout {
		display: flex;
		flex-direction: column;
		gap: 1.25rem;
	}

	.new-course__main {
		display: flex;
		flex-direction: column;
		gap: 1.25rem;
	}

	.new-course__sidebar {
		display: flex;
		flex-direction: column;
		gap: 1.25rem;
	}

	/* ── Form card ──────────────────────── */
	.form-card {
		background: rgba(255, 255, 255, 0.03);
		border: 1px solid rgba(255, 255, 255, 0.06);
		border-radius: var(--radius-xl);
		padding: 1.25rem;
		backdrop-filter: blur(12px);
	}

	.form-card__heading {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		font-size: var(--fs-sm);
		font-weight: var(--w-semibold);
		color: var(--color-white);
		margin: 0 0 1rem 0;
		padding-bottom: 0.75rem;
		border-bottom: 1px solid rgba(255, 255, 255, 0.06);
	}

	/* ── Fields ─────────────────────────── */
	.field {
		margin-bottom: 1rem;
	}

	.field:last-child {
		margin-bottom: 0;
	}

	.field__label {
		display: flex;
		align-items: center;
		gap: 0.35rem;
		font-size: var(--fs-xs);
		font-weight: var(--w-medium);
		color: var(--color-grey-300);
		margin-bottom: 0.4rem;
	}

	.field__hint {
		font-weight: var(--w-regular);
		color: var(--color-grey-500);
		font-size: var(--fs-2xs);
	}

	.field__input {
		width: 100%;
		padding: 0.6rem 0.85rem;
		background: rgba(0, 0, 0, 0.2);
		border: 1px solid rgba(255, 255, 255, 0.1);
		border-radius: var(--radius-lg);
		color: var(--color-white);
		font-size: var(--fs-sm);
		outline: none;
		transition: border-color var(--duration-200) var(--ease-out);
	}

	.field__input:focus {
		border-color: var(--color-teal);
	}

	.field__input::placeholder {
		color: var(--color-grey-500);
	}

	.field__input--mono {
		font-family: 'SF Mono', 'Fira Code', monospace;
		font-size: var(--fs-xs);
	}

	.field__textarea {
		width: 100%;
		padding: 0.75rem 0.85rem;
		background: rgba(0, 0, 0, 0.2);
		border: 1px solid rgba(255, 255, 255, 0.1);
		border-radius: var(--radius-lg);
		color: var(--color-white);
		font-size: var(--fs-sm);
		font-family: var(--font-ui);
		line-height: var(--lh-relaxed);
		outline: none;
		resize: vertical;
		min-height: 6rem;
		transition: border-color var(--duration-200) var(--ease-out);
	}

	.field__textarea:focus {
		border-color: var(--color-teal);
	}

	.field__textarea::placeholder {
		color: var(--color-grey-500);
	}

	.field__input-group {
		position: relative;
	}

	.field__input-prefix {
		position: absolute;
		left: 0.85rem;
		top: 50%;
		transform: translateY(-50%);
		color: var(--color-grey-400);
		font-size: var(--fs-sm);
		font-weight: var(--w-medium);
		pointer-events: none;
	}

	.field__input--prefixed {
		padding-left: 1.75rem;
	}

	/* ── Toggle ─────────────────────────── */
	.toggle-row {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 0.75rem;
		margin-bottom: 1rem;
	}

	.toggle-row:last-child {
		margin-bottom: 0;
	}

	.toggle-row__label {
		font-size: var(--fs-sm);
		color: var(--color-grey-300);
	}

	.toggle {
		position: relative;
		width: 2.75rem;
		height: 1.5rem;
		background: rgba(255, 255, 255, 0.1);
		border: 1px solid rgba(255, 255, 255, 0.12);
		border-radius: var(--radius-full);
		cursor: pointer;
		padding: 0;
		flex-shrink: 0;
		transition: all var(--duration-200) var(--ease-out);
	}

	.toggle--active {
		background: var(--color-teal);
		border-color: var(--color-teal);
	}

	.toggle__knob {
		position: absolute;
		top: 2px;
		left: 2px;
		width: 1.15rem;
		height: 1.15rem;
		background: var(--color-white);
		border-radius: var(--radius-full);
		transition: transform var(--duration-200) var(--ease-out);
	}

	.toggle--active .toggle__knob {
		transform: translateX(1.25rem);
	}

	/* ── Radio pills ────────────────────── */
	.radio-group {
		display: flex;
		gap: 0.5rem;
		flex-wrap: wrap;
	}

	.radio-pill {
		display: inline-flex;
		align-items: center;
		padding: 0.4rem 0.85rem;
		border-radius: var(--radius-lg);
		border: 1px solid rgba(255, 255, 255, 0.1);
		background: rgba(0, 0, 0, 0.15);
		cursor: pointer;
		transition: all var(--duration-150) var(--ease-out);
	}

	.radio-pill:hover {
		border-color: rgba(255, 255, 255, 0.2);
	}

	.radio-pill--active {
		border-color: var(--color-teal);
		background: rgba(15, 164, 175, 0.12);
	}

	.radio-pill__text {
		font-size: var(--fs-xs);
		font-weight: var(--w-medium);
		color: var(--color-grey-300);
	}

	.radio-pill--active .radio-pill__text {
		color: var(--color-teal-light);
	}

	.sr-only {
		position: absolute;
		width: 1px;
		height: 1px;
		padding: 0;
		margin: -1px;
		overflow: hidden;
		clip: rect(0, 0, 0, 0);
		white-space: nowrap;
		border-width: 0;
	}

	/* ── Thumbnail preview ──────────────── */
	.thumb-preview {
		margin-bottom: 1rem;
		border-radius: var(--radius-lg);
		overflow: hidden;
		border: 1px solid rgba(255, 255, 255, 0.08);
	}

	.thumb-preview__img {
		width: 100%;
		height: auto;
		display: block;
		max-height: 10rem;
		object-fit: cover;
	}

	/* ── Create button ──────────────────── */
	.btn-create {
		display: flex;
		align-items: center;
		justify-content: center;
		gap: 0.5rem;
		width: 100%;
		padding: 0.75rem 1.25rem;
		background: var(--color-teal);
		color: var(--color-white);
		font-weight: var(--w-semibold);
		font-size: var(--fs-sm);
		border: none;
		border-radius: var(--radius-lg);
		cursor: pointer;
		transition:
			opacity var(--duration-150) var(--ease-out),
			transform var(--duration-150) var(--ease-out);
	}

	.btn-create:hover:not(:disabled) {
		opacity: 0.9;
		transform: translateY(-1px);
	}

	.btn-create:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}

	/* ── Tablet+ ────────────────────────── */
	@media (min-width: 768px) {
		.new-course__header {
			margin-bottom: 2rem;
		}

		.new-course__title {
			font-size: var(--fs-2xl);
		}

		.form-card {
			padding: 1.5rem;
		}
	}

	/* ── Desktop ────────────────────────── */
	@media (min-width: 1024px) {
		.new-course__layout {
			flex-direction: row;
			align-items: flex-start;
		}

		.new-course__main {
			flex: 1;
			min-width: 0;
		}

		.new-course__sidebar {
			width: 22rem;
			flex-shrink: 0;
			position: sticky;
			top: 1.5rem;
		}

		.form-card {
			padding: 1.75rem;
		}
	}
</style>
