<script lang="ts">
	import { page } from '$app/state';
	import { goto } from '$app/navigation';
	import { onMount } from 'svelte';
	import { SvelteSet } from 'svelte/reactivity';
	import { api, ApiError } from '$lib/api/client';
	import ArrowLeft from 'phosphor-svelte/lib/ArrowLeft';
	import FloppyDisk from 'phosphor-svelte/lib/FloppyDisk';
	import Trash from 'phosphor-svelte/lib/Trash';
	import Plus from 'phosphor-svelte/lib/Plus';
	import CaretDown from 'phosphor-svelte/lib/CaretDown';
	import CaretUp from 'phosphor-svelte/lib/CaretUp';
	import VideoCamera from 'phosphor-svelte/lib/VideoCamera';
	import Eye from 'phosphor-svelte/lib/Eye';
	import EyeSlash from 'phosphor-svelte/lib/EyeSlash';
	import BookOpen from 'phosphor-svelte/lib/BookOpen';

	interface Lesson {
		id: string;
		title: string;
		video_url: string;
		is_preview: boolean;
		sort_order: number;
	}

	interface Module {
		id: string;
		title: string;
		sort_order: number;
		lessons: Lesson[];
	}

	interface Course {
		id: string;
		title: string;
		slug: string;
		description: string;
		difficulty: 'beginner' | 'intermediate' | 'advanced';
		price: number;
		is_free: boolean;
		thumbnail_url: string | null;
		is_published: boolean;
		modules: Module[];
	}

	let course = $state<Course | null>(null);
	let loading = $state(true);
	let saving = $state(false);
	let deleting = $state(false);
	let error = $state('');
	let successMsg = $state('');

	let title = $state('');
	let slug = $state('');
	let description = $state('');
	let difficulty = $state<'beginner' | 'intermediate' | 'advanced'>('beginner');
	let price = $state('');
	let thumbnailUrl = $state('');
	let modules = $state<Module[]>([]);
	let expandedModules = new SvelteSet<string>();

	onMount(async () => {
		try {
			const id = page.params.id;
			const data = await api.get<Course>(`/api/admin/courses/${id}`);
			course = data;
			title = data.title;
			slug = data.slug;
			description = data.description ?? '';
			difficulty = data.difficulty;
			price = String(data.price);
			thumbnailUrl = data.thumbnail_url ?? '';
			modules = data.modules ?? [];
			if (modules.length > 0) {
				expandedModules.add(modules[0].id);
			}
		} catch {
			error = 'Course not found';
		} finally {
			loading = false;
		}
	});

	function toggleModule(id: string) {
		if (expandedModules.has(id)) {
			expandedModules.delete(id);
		} else {
			expandedModules.add(id);
		}
	}

	function addModule() {
		const tempId = `new-${Date.now()}`;
		const newMod: Module = {
			id: tempId,
			title: '',
			sort_order: modules.length,
			lessons: []
		};
		modules = [...modules, newMod];
		expandedModules.add(tempId);
	}

	function removeModule(idx: number) {
		if (!confirm('Remove this module and all its lessons?')) return;
		modules = modules.filter((_, i) => i !== idx);
	}

	function addLesson(moduleIdx: number) {
		const tempId = `new-lesson-${Date.now()}`;
		const mod = modules[moduleIdx];
		const updated = {
			...mod,
			lessons: [
				...mod.lessons,
				{ id: tempId, title: '', video_url: '', is_preview: false, sort_order: mod.lessons.length }
			]
		};
		modules = modules.map((m, i) => (i === moduleIdx ? updated : m));
	}

	function removeLesson(moduleIdx: number, lessonIdx: number) {
		if (!confirm('Remove this lesson?')) return;
		const mod = modules[moduleIdx];
		const updated = {
			...mod,
			lessons: mod.lessons.filter((_, i) => i !== lessonIdx)
		};
		modules = modules.map((m, i) => (i === moduleIdx ? updated : m));
	}

	function updateModuleTitle(moduleIdx: number, val: string) {
		modules = modules.map((m, i) => (i === moduleIdx ? { ...m, title: val } : m));
	}

	function updateLessonField(moduleIdx: number, lessonIdx: number, field: keyof Lesson, val: string | boolean) {
		modules = modules.map((m, mi) => {
			if (mi !== moduleIdx) return m;
			return {
				...m,
				lessons: m.lessons.map((l, li) => (li === lessonIdx ? { ...l, [field]: val } : l))
			};
		});
	}

	async function handleSave(e: Event) {
		e.preventDefault();
		saving = true;
		error = '';
		successMsg = '';

		try {
			const payload = {
				title: title.trim(),
				slug: slug.trim(),
				description: description.trim(),
				difficulty,
				price: Number(price) || 0,
				thumbnail_url: thumbnailUrl.trim() || null,
				modules: modules.map((m, mi) => ({
					id: m.id.startsWith('new-') ? undefined : m.id,
					title: m.title,
					sort_order: mi,
					lessons: m.lessons.map((l, li) => ({
						id: l.id.startsWith('new-') ? undefined : l.id,
						title: l.title,
						video_url: l.video_url,
						is_preview: l.is_preview,
						sort_order: li
					}))
				}))
			};

			const updated = await api.put<Course>(`/api/admin/courses/${page.params.id}`, payload);
			course = updated;
			modules = updated.modules ?? [];
			successMsg = 'Course saved!';
			setTimeout(() => (successMsg = ''), 3000);
		} catch (err) {
			error = err instanceof ApiError ? err.message : 'Failed to save course';
		} finally {
			saving = false;
		}
	}

	async function handleDelete() {
		if (!confirm('Delete this course? This cannot be undone.')) return;
		deleting = true;
		try {
			await api.del(`/api/admin/courses/${page.params.id}`);
			goto('/admin/courses');
		} catch (err) {
			error = err instanceof ApiError ? err.message : 'Failed to delete course';
			deleting = false;
		}
	}
</script>

<svelte:head>
	<title>{course ? `Edit: ${course.title}` : 'Edit Course'} -- Admin</title>
</svelte:head>

<div class="ce">
	<a href="/admin/courses" class="ce__back">
		<ArrowLeft size={18} />
		Back to Courses
	</a>

	{#if loading}
		<p class="ce__status">Loading course...</p>
	{:else if error && !course}
		<p class="ce__status ce__status--error">{error}</p>
	{:else if course}
		<div class="ce__header">
			<BookOpen size={24} weight="bold" />
			<h1 class="ce__title">Edit Course</h1>
		</div>

		{#if error}
			<div class="ce__alert ce__alert--error">{error}</div>
		{/if}
		{#if successMsg}
			<div class="ce__alert ce__alert--success">{successMsg}</div>
		{/if}

		<form onsubmit={handleSave} class="ce__form">
			<div class="ce__details">
				<h2 class="ce__section-title">Course Details</h2>
				<div class="ce__grid">
					<div class="ce__field">
						<label for="ce-title" class="ce__label">Title</label>
						<input id="ce-title" type="text" bind:value={title} required class="ce__input" placeholder="Course title" />
					</div>
					<div class="ce__field">
						<label for="ce-slug" class="ce__label">Slug</label>
						<input id="ce-slug" type="text" bind:value={slug} class="ce__input ce__input--mono" placeholder="course-slug" />
					</div>
					<div class="ce__field ce__field--full">
						<label for="ce-desc" class="ce__label">Description</label>
						<textarea id="ce-desc" bind:value={description} class="ce__textarea" rows="4" placeholder="Course description..."></textarea>
					</div>
					<div class="ce__field">
						<label for="ce-difficulty" class="ce__label">Difficulty</label>
						<select id="ce-difficulty" bind:value={difficulty} class="ce__input">
							<option value="beginner">Beginner</option>
							<option value="intermediate">Intermediate</option>
							<option value="advanced">Advanced</option>
						</select>
					</div>
					<div class="ce__field">
						<label for="ce-price" class="ce__label">Price ($)</label>
						<input id="ce-price" type="number" step="0.01" min="0" bind:value={price} class="ce__input" placeholder="0.00" />
					</div>
					<div class="ce__field ce__field--full">
						<label for="ce-thumb" class="ce__label">Thumbnail URL</label>
						<input id="ce-thumb" type="url" bind:value={thumbnailUrl} class="ce__input" placeholder="https://..." />
					</div>
				</div>
				{#if thumbnailUrl}
					<div class="ce__thumb-preview">
						<img src={thumbnailUrl} alt="Thumbnail" class="ce__thumb-img" />
					</div>
				{/if}
			</div>

			<!-- Modules -->
			<div class="ce__modules-section">
				<div class="ce__modules-header">
					<h2 class="ce__section-title">Modules & Lessons</h2>
					<button type="button" onclick={addModule} class="ce__add-btn">
						<Plus size={16} weight="bold" />
						Add Module
					</button>
				</div>

				{#if modules.length === 0}
					<div class="ce__empty">No modules yet. Add one to get started.</div>
				{/if}

				<div class="ce__modules-list">
					{#each modules as mod, mi (mod.id)}
						<div class="ce__module-card">
							<button type="button" class="ce__module-toggle" onclick={() => toggleModule(mod.id)}>
								<span class="ce__module-num">{mi + 1}</span>
								<input
									type="text"
									value={mod.title}
									oninput={(e) => updateModuleTitle(mi, (e.target as HTMLInputElement).value)}
									onclick={(e) => e.stopPropagation()}
									class="ce__module-title-input"
									placeholder="Module title..."
								/>
								<div class="ce__module-meta">
									<span class="ce__lesson-count">{mod.lessons.length} lessons</span>
									{#if expandedModules.has(mod.id)}
										<CaretUp size={16} />
									{:else}
										<CaretDown size={16} />
									{/if}
								</div>
							</button>

							{#if expandedModules.has(mod.id)}
								<div class="ce__module-body">
									{#if mod.lessons.length === 0}
										<p class="ce__no-lessons">No lessons in this module.</p>
									{/if}

									{#each mod.lessons as lesson, li (lesson.id)}
										<div class="ce__lesson-row">
											<span class="ce__lesson-num">{mi + 1}.{li + 1}</span>
											<div class="ce__lesson-fields">
												<input
													type="text"
													value={lesson.title}
													oninput={(e) => updateLessonField(mi, li, 'title', (e.target as HTMLInputElement).value)}
													class="ce__input ce__input--sm"
													placeholder="Lesson title"
												/>
												<div class="ce__lesson-url-row">
													<VideoCamera size={14} />
													<input
														type="url"
														value={lesson.video_url}
														oninput={(e) => updateLessonField(mi, li, 'video_url', (e.target as HTMLInputElement).value)}
														class="ce__input ce__input--sm"
														placeholder="Video URL"
													/>
												</div>
											</div>
											<div class="ce__lesson-actions">
												<button
													type="button"
													class="ce__icon-btn"
													title={lesson.is_preview ? 'Disable preview' : 'Enable preview'}
													onclick={() => updateLessonField(mi, li, 'is_preview', !lesson.is_preview)}
												>
													{#if lesson.is_preview}
														<Eye size={16} weight="fill" />
													{:else}
														<EyeSlash size={16} />
													{/if}
												</button>
												<button
													type="button"
													class="ce__icon-btn ce__icon-btn--danger"
													title="Remove lesson"
													onclick={() => removeLesson(mi, li)}
												>
													<Trash size={14} />
												</button>
											</div>
										</div>
									{/each}

									<div class="ce__module-footer">
										<button type="button" onclick={() => addLesson(mi)} class="ce__add-lesson-btn">
											<Plus size={14} weight="bold" />
											Add Lesson
										</button>
										<button type="button" onclick={() => removeModule(mi)} class="ce__remove-module-btn">
											<Trash size={14} />
											Remove Module
										</button>
									</div>
								</div>
							{/if}
						</div>
					{/each}
				</div>
			</div>

			<!-- Actions -->
			<div class="ce__actions">
				<button type="button" onclick={handleDelete} disabled={deleting} class="ce__delete-btn">
					<Trash size={16} weight="bold" />
					{deleting ? 'Deleting...' : 'Delete Course'}
				</button>
				<div class="ce__actions-right">
					<a href="/admin/courses" class="ce__cancel">Cancel</a>
					<button type="submit" disabled={saving} class="ce__save-btn">
						<FloppyDisk size={16} weight="bold" />
						{saving ? 'Saving...' : 'Save Course'}
					</button>
				</div>
			</div>
		</form>
	{/if}
</div>

<style>
	.ce__back {
		display: inline-flex;
		align-items: center;
		gap: 0.5rem;
		color: var(--color-grey-400);
		font-size: var(--fs-sm);
		text-decoration: none;
		margin-bottom: 1.5rem;
		transition: color 200ms var(--ease-out);
	}
	.ce__back:hover { color: var(--color-white); }
	.ce__status {
		text-align: center;
		padding: 3rem;
		color: var(--color-grey-400);
	}
	.ce__status--error { color: var(--color-red); }
	.ce__header {
		display: flex;
		align-items: center;
		gap: 0.75rem;
		color: var(--color-teal);
		margin-bottom: 1.5rem;
	}
	.ce__title {
		font-size: var(--fs-2xl);
		font-weight: var(--w-bold);
		color: var(--color-white);
		font-family: var(--font-heading);
	}
	.ce__alert {
		padding: 0.75rem 1rem;
		border-radius: var(--radius-lg);
		font-size: var(--fs-sm);
		margin-bottom: 1rem;
	}
	.ce__alert--error {
		background-color: rgba(239, 68, 68, 0.1);
		border: 1px solid rgba(239, 68, 68, 0.3);
		color: #fca5a5;
	}
	.ce__alert--success {
		background-color: rgba(34, 197, 94, 0.1);
		border: 1px solid rgba(34, 197, 94, 0.3);
		color: #86efac;
	}
	.ce__form {
		display: flex;
		flex-direction: column;
		gap: 1.5rem;
	}
	.ce__details {
		background-color: var(--color-navy-mid);
		border: 1px solid rgba(255, 255, 255, 0.06);
		border-radius: var(--radius-xl);
		padding: 1.5rem;
	}
	.ce__section-title {
		font-size: var(--fs-lg);
		font-weight: var(--w-bold);
		color: var(--color-white);
		font-family: var(--font-heading);
		margin: 0 0 1.25rem 0;
	}
	.ce__grid {
		display: grid;
		grid-template-columns: 1fr 1fr;
		gap: 1.25rem;
	}
	.ce__field {
		display: flex;
		flex-direction: column;
		gap: 0.4rem;
	}
	.ce__field--full { grid-column: 1 / -1; }
	.ce__label {
		font-size: var(--fs-sm);
		font-weight: var(--w-medium);
		color: var(--color-grey-300);
	}
	.ce__input,
	.ce__textarea {
		width: 100%;
		padding: 0.65rem 0.85rem;
		background-color: rgba(255, 255, 255, 0.05);
		border: 1px solid rgba(255, 255, 255, 0.1);
		border-radius: var(--radius-lg);
		color: var(--color-white);
		font-size: var(--fs-sm);
		font-family: inherit;
		transition: border-color 200ms var(--ease-out);
	}
	.ce__input:focus,
	.ce__textarea:focus {
		outline: none;
		border-color: var(--color-teal);
	}
	.ce__input::placeholder,
	.ce__textarea::placeholder {
		color: var(--color-grey-500);
	}
	.ce__textarea { resize: vertical; }
	.ce__input--mono {
		font-family: 'SF Mono', 'Fira Code', monospace;
		font-size: var(--fs-xs);
	}
	.ce__input--sm { padding: 0.45rem 0.7rem; font-size: var(--fs-xs); }
	.ce__thumb-preview {
		margin-top: 1rem;
		border-radius: var(--radius-lg);
		overflow: hidden;
		border: 1px solid rgba(255, 255, 255, 0.08);
		max-width: 20rem;
	}
	.ce__thumb-img {
		width: 100%;
		height: auto;
		display: block;
		max-height: 10rem;
		object-fit: cover;
	}
	/* Modules */
	.ce__modules-section {
		background-color: var(--color-navy-mid);
		border: 1px solid rgba(255, 255, 255, 0.06);
		border-radius: var(--radius-xl);
		padding: 1.5rem;
	}
	.ce__modules-header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		margin-bottom: 1.25rem;
	}
	.ce__modules-header .ce__section-title { margin-bottom: 0; }
	.ce__add-btn {
		display: inline-flex;
		align-items: center;
		gap: 0.35rem;
		padding: 0.5rem 1rem;
		background-color: rgba(15, 164, 175, 0.1);
		border: 1px solid rgba(15, 164, 175, 0.3);
		border-radius: var(--radius-lg);
		color: var(--color-teal);
		font-size: var(--fs-sm);
		font-weight: var(--w-semibold);
		cursor: pointer;
		transition: background-color 200ms var(--ease-out);
	}
	.ce__add-btn:hover { background-color: rgba(15, 164, 175, 0.2); }
	.ce__empty {
		text-align: center;
		padding: 2rem;
		color: var(--color-grey-500);
		font-size: var(--fs-sm);
	}
	.ce__modules-list {
		display: flex;
		flex-direction: column;
		gap: 0.75rem;
	}
	.ce__module-card {
		border: 1px solid rgba(255, 255, 255, 0.08);
		border-radius: var(--radius-lg);
		overflow: hidden;
		background-color: rgba(0, 0, 0, 0.15);
	}
	.ce__module-toggle {
		display: flex;
		align-items: center;
		gap: 0.75rem;
		width: 100%;
		padding: 0.85rem 1rem;
		background: none;
		border: none;
		cursor: pointer;
		color: var(--color-grey-300);
		transition: background-color 150ms var(--ease-out);
	}
	.ce__module-toggle:hover { background-color: rgba(255, 255, 255, 0.03); }
	.ce__module-num {
		display: flex;
		align-items: center;
		justify-content: center;
		width: 1.75rem;
		height: 1.75rem;
		border-radius: var(--radius-md);
		background-color: rgba(15, 164, 175, 0.15);
		color: var(--color-teal-light);
		font-size: var(--fs-xs);
		font-weight: var(--w-bold);
		flex-shrink: 0;
	}
	.ce__module-title-input {
		flex: 1;
		background: transparent;
		border: 1px solid transparent;
		border-radius: var(--radius-md);
		padding: 0.3rem 0.5rem;
		color: var(--color-white);
		font-size: var(--fs-sm);
		font-weight: var(--w-semibold);
		transition: border-color 200ms var(--ease-out);
	}
	.ce__module-title-input:focus {
		outline: none;
		border-color: var(--color-teal);
		background-color: rgba(255, 255, 255, 0.05);
	}
	.ce__module-title-input::placeholder { color: var(--color-grey-500); }
	.ce__module-meta {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		flex-shrink: 0;
	}
	.ce__lesson-count {
		font-size: var(--fs-xs);
		color: var(--color-grey-500);
	}
	.ce__module-body {
		padding: 0.75rem 1rem 1rem;
		border-top: 1px solid rgba(255, 255, 255, 0.06);
	}
	.ce__no-lessons {
		color: var(--color-grey-500);
		font-size: var(--fs-xs);
		padding: 0.5rem 0;
		margin: 0;
	}
	.ce__lesson-row {
		display: flex;
		align-items: flex-start;
		gap: 0.65rem;
		padding: 0.65rem 0;
		border-bottom: 1px solid rgba(255, 255, 255, 0.04);
	}
	.ce__lesson-row:last-of-type { border-bottom: none; }
	.ce__lesson-num {
		font-size: var(--fs-xs);
		color: var(--color-grey-500);
		font-weight: var(--w-medium);
		min-width: 2rem;
		padding-top: 0.5rem;
	}
	.ce__lesson-fields {
		flex: 1;
		display: flex;
		flex-direction: column;
		gap: 0.4rem;
		min-width: 0;
	}
	.ce__lesson-url-row {
		display: flex;
		align-items: center;
		gap: 0.4rem;
		color: var(--color-grey-500);
	}
	.ce__lesson-url-row .ce__input { flex: 1; }
	.ce__lesson-actions {
		display: flex;
		align-items: center;
		gap: 0.25rem;
		padding-top: 0.35rem;
		flex-shrink: 0;
	}
	.ce__icon-btn {
		display: flex;
		align-items: center;
		justify-content: center;
		width: 2rem;
		height: 2rem;
		border: none;
		border-radius: var(--radius-md);
		background: rgba(255, 255, 255, 0.05);
		color: var(--color-grey-400);
		cursor: pointer;
		transition: all 150ms var(--ease-out);
	}
	.ce__icon-btn:hover {
		background: rgba(255, 255, 255, 0.1);
		color: var(--color-teal-light);
	}
	.ce__icon-btn--danger:hover {
		background: rgba(239, 68, 68, 0.15);
		color: var(--color-red);
	}
	.ce__module-footer {
		display: flex;
		align-items: center;
		justify-content: space-between;
		margin-top: 0.75rem;
		padding-top: 0.75rem;
		border-top: 1px solid rgba(255, 255, 255, 0.06);
	}
	.ce__add-lesson-btn {
		display: inline-flex;
		align-items: center;
		gap: 0.3rem;
		padding: 0.4rem 0.85rem;
		background: transparent;
		border: 1px dashed rgba(15, 164, 175, 0.4);
		border-radius: var(--radius-md);
		color: var(--color-teal);
		font-size: var(--fs-xs);
		font-weight: var(--w-semibold);
		cursor: pointer;
		transition: all 150ms var(--ease-out);
	}
	.ce__add-lesson-btn:hover {
		background: rgba(15, 164, 175, 0.08);
		border-color: var(--color-teal);
	}
	.ce__remove-module-btn {
		display: inline-flex;
		align-items: center;
		gap: 0.3rem;
		padding: 0.4rem 0.85rem;
		background: transparent;
		border: 1px solid rgba(239, 68, 68, 0.2);
		border-radius: var(--radius-md);
		color: var(--color-grey-500);
		font-size: var(--fs-xs);
		cursor: pointer;
		transition: all 150ms var(--ease-out);
	}
	.ce__remove-module-btn:hover {
		background: rgba(239, 68, 68, 0.1);
		color: var(--color-red);
		border-color: rgba(239, 68, 68, 0.4);
	}
	/* Actions */
	.ce__actions {
		display: flex;
		align-items: center;
		justify-content: space-between;
		background-color: var(--color-navy-mid);
		border: 1px solid rgba(255, 255, 255, 0.06);
		border-radius: var(--radius-xl);
		padding: 1.25rem 1.5rem;
	}
	.ce__actions-right {
		display: flex;
		gap: 1rem;
		align-items: center;
	}
	.ce__delete-btn {
		display: flex;
		align-items: center;
		gap: 0.4rem;
		padding: 0.6rem 1.25rem;
		background-color: rgba(239, 68, 68, 0.08);
		border: 1px solid rgba(239, 68, 68, 0.25);
		border-radius: var(--radius-lg);
		color: var(--color-red);
		font-size: var(--fs-sm);
		font-weight: var(--w-semibold);
		cursor: pointer;
		transition: background-color 200ms var(--ease-out);
	}
	.ce__delete-btn:hover:not(:disabled) { background-color: rgba(239, 68, 68, 0.18); }
	.ce__delete-btn:disabled { opacity: 0.5; cursor: not-allowed; }
	.ce__cancel {
		padding: 0.6rem 1.25rem;
		border: 1px solid rgba(255, 255, 255, 0.1);
		border-radius: var(--radius-lg);
		color: var(--color-grey-400);
		font-size: var(--fs-sm);
		font-weight: var(--w-medium);
		text-decoration: none;
		transition: border-color 200ms var(--ease-out), color 200ms var(--ease-out);
	}
	.ce__cancel:hover {
		border-color: rgba(255, 255, 255, 0.2);
		color: var(--color-white);
	}
	.ce__save-btn {
		display: flex;
		align-items: center;
		gap: 0.4rem;
		padding: 0.6rem 1.5rem;
		background: linear-gradient(135deg, var(--color-teal), #0d8a94);
		color: var(--color-white);
		font-weight: var(--w-semibold);
		font-size: var(--fs-sm);
		border-radius: var(--radius-lg);
		cursor: pointer;
		transition: opacity 200ms var(--ease-out);
	}
	.ce__save-btn:hover:not(:disabled) { opacity: 0.9; }
	.ce__save-btn:disabled { opacity: 0.5; cursor: not-allowed; }

	@media (max-width: 768px) {
		.ce__grid { grid-template-columns: 1fr; }
		.ce__field--full { grid-column: 1; }
		.ce__actions { flex-direction: column; gap: 1rem; }
		.ce__lesson-row { flex-wrap: wrap; }
	}
</style>
