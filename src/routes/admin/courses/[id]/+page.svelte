<script lang="ts">
	import { page } from '$app/state';
	import { goto } from '$app/navigation';
	import { resolve } from '$app/paths';
	import { onMount } from 'svelte';
	import { SvelteSet } from 'svelte/reactivity';
	import { api, ApiError } from '$lib/api/client';
	import ArrowLeftIcon from 'phosphor-svelte/lib/ArrowLeftIcon';
	import FloppyDiskIcon from 'phosphor-svelte/lib/FloppyDiskIcon';
	import TrashIcon from 'phosphor-svelte/lib/TrashIcon';
	import PlusIcon from 'phosphor-svelte/lib/PlusIcon';
	import CaretDownIcon from 'phosphor-svelte/lib/CaretDownIcon';
	import CaretUpIcon from 'phosphor-svelte/lib/CaretUpIcon';
	import VideoCameraIcon from 'phosphor-svelte/lib/VideoCameraIcon';
	import EyeIcon from 'phosphor-svelte/lib/EyeIcon';
	import EyeSlashIcon from 'phosphor-svelte/lib/EyeSlashIcon';
	import BookOpenIcon from 'phosphor-svelte/lib/BookOpenIcon';
	import { confirmDialog } from '$lib/stores/confirm.svelte';

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
			if (modules.length > 0) expandedModules.add(modules[0]!.id);
		} catch {
			error = 'Course not found';
		} finally {
			loading = false;
		}
	});

	function toggleModule(id: string) {
		if (expandedModules.has(id)) expandedModules.delete(id);
		else expandedModules.add(id);
	}

	function addModule() {
		const tempId = `new-${Date.now()}`;
		modules = [...modules, { id: tempId, title: '', sort_order: modules.length, lessons: [] }];
		expandedModules.add(tempId);
	}

	async function removeModule(idx: number) {
		const ok = await confirmDialog({
			title: 'Remove this module?',
			message:
				'The module and every lesson it contains will be removed from the course outline. The change is staged until you save.',
			confirmLabel: 'Remove module',
			variant: 'danger'
		});
		if (!ok) return;
		modules = modules.filter((_, i) => i !== idx);
	}

	function addLesson(mi: number) {
		modules = modules.map((m, i) =>
			i !== mi
				? m
				: {
						...m,
						lessons: [
							...m.lessons,
							{
								id: `new-lesson-${Date.now()}`,
								title: '',
								video_url: '',
								is_preview: false,
								sort_order: m.lessons.length
							}
						]
					}
		);
	}

	async function removeLesson(mi: number, li: number) {
		const ok = await confirmDialog({
			title: 'Remove this lesson?',
			message:
				'The lesson will be removed from this module. The change is staged until you save.',
			confirmLabel: 'Remove lesson',
			variant: 'danger'
		});
		if (!ok) return;
		modules = modules.map((m, i) =>
			i !== mi ? m : { ...m, lessons: m.lessons.filter((_, j) => j !== li) }
		);
	}

	function updateModuleTitle(mi: number, val: string) {
		modules = modules.map((m, i) => (i === mi ? { ...m, title: val } : m));
	}

	function updateLesson(mi: number, li: number, field: keyof Lesson, val: string | boolean) {
		modules = modules.map((m, i) =>
			i !== mi
				? m
				: {
						...m,
						lessons: m.lessons.map((l, j) => (j === li ? { ...l, [field]: val } : l))
					}
		);
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
		const ok = await confirmDialog({
			title: 'Delete this course?',
			message:
				'The course, its modules, and every lesson will be permanently removed. Enrolled members will lose access.',
			confirmLabel: 'Delete course',
			variant: 'danger'
		});
		if (!ok) return;
		deleting = true;
		try {
			await api.del(`/api/admin/courses/${page.params.id}`);
			goto(resolve('/admin/courses'));
		} catch (err) {
			error = err instanceof ApiError ? err.message : 'Failed to delete';
			deleting = false;
		}
	}
</script>

<svelte:head>
	<title>{course ? `Edit: ${course.title}` : 'Edit Course'} -- Admin</title>
</svelte:head>

<div class="ce">
	<a href={resolve('/admin/courses')} class="ce__back"
		><ArrowLeftIcon size={18} /> Back to Courses</a
	>

	{#if loading}
		<p class="ce__status">Loading course...</p>
	{:else if error && !course}
		<p class="ce__status ce__status--err">{error}</p>
	{:else if course}
		<div class="ce__hdr">
			<BookOpenIcon size={24} weight="bold" />
			<h1 class="ce__title">Edit Course</h1>
		</div>

		{#if error}<div class="ce__alert ce__alert--err">{error}</div>{/if}
		{#if successMsg}<div class="ce__alert ce__alert--ok">{successMsg}</div>{/if}

		<form onsubmit={handleSave} class="ce__form">
			<div class="ce__card">
				<h2 class="ce__stitle">Course Details</h2>
				<div class="ce__grid">
					<div class="ce__f">
						<label for="ce-title" class="ce__lbl">Title</label>
						<input
							id="ce-title"
							type="text"
							bind:value={title}
							required
							class="ce__inp"
							placeholder="Course title"
						/>
					</div>
					<div class="ce__f">
						<label for="ce-slug" class="ce__lbl">Slug</label>
						<input
							id="ce-slug"
							type="text"
							bind:value={slug}
							class="ce__inp ce__inp--mono"
							placeholder="course-slug"
						/>
					</div>
					<div class="ce__f ce__f--full">
						<label for="ce-desc" class="ce__lbl">Description</label>
						<textarea
							id="ce-desc"
							bind:value={description}
							class="ce__ta"
							rows="4"
							placeholder="Course description..."
						></textarea>
					</div>
					<div class="ce__f">
						<label for="ce-diff" class="ce__lbl">Difficulty</label>
						<select id="ce-diff" bind:value={difficulty} class="ce__inp">
							<option value="beginner">Beginner</option>
							<option value="intermediate">Intermediate</option>
							<option value="advanced">Advanced</option>
						</select>
					</div>
					<div class="ce__f">
						<label for="ce-price" class="ce__lbl">Price ($)</label>
						<input
							id="ce-price"
							type="number"
							step="0.01"
							min="0"
							bind:value={price}
							class="ce__inp"
							placeholder="0.00"
						/>
					</div>
					<div class="ce__f ce__f--full">
						<label for="ce-thumb" class="ce__lbl">Thumbnail URL</label>
						<input
							id="ce-thumb"
							type="url"
							bind:value={thumbnailUrl}
							class="ce__inp"
							placeholder="https://..."
						/>
					</div>
				</div>
				{#if thumbnailUrl}<div class="ce__tp">
						<img src={thumbnailUrl} alt="Thumbnail" class="ce__timg" />
					</div>{/if}
			</div>

			<div class="ce__card">
				<div class="ce__mhdr">
					<h2 class="ce__stitle">Modules & Lessons</h2>
					<button type="button" onclick={addModule} class="ce__add"
						><PlusIcon size={16} weight="bold" /> Add Module</button
					>
				</div>
				{#if modules.length === 0}<div class="ce__empty">
						No modules yet. Add one to get started.
					</div>{/if}
				<div class="ce__mlist">
					{#each modules as mod, mi (mod.id)}
						<div class="ce__mc">
							<button
								type="button"
								class="ce__mtog"
								onclick={() => toggleModule(mod.id)}
							>
								<span class="ce__mnum">{mi + 1}</span>
								<input
									id={`ce-mod-${mod.id}-title`}
									name="module-title"
									type="text"
									aria-label={`Module ${mi + 1} title`}
									value={mod.title}
									oninput={(e) =>
										updateModuleTitle(mi, (e.target as HTMLInputElement).value)}
									onclick={(e) => e.stopPropagation()}
									class="ce__minp"
									placeholder="Module title..."
								/>
								<div class="ce__mmeta">
									<span class="ce__lcnt">{mod.lessons.length} lessons</span>
									{#if expandedModules.has(mod.id)}<CaretUpIcon
											size={16}
										/>{:else}<CaretDownIcon size={16} />{/if}
								</div>
							</button>
							{#if expandedModules.has(mod.id)}
								<div class="ce__mbody">
									{#if mod.lessons.length === 0}<p class="ce__nol">
											No lessons in this module.
										</p>{/if}
									{#each mod.lessons as lesson, li (lesson.id)}
										<div class="ce__lr">
											<span class="ce__lnum">{mi + 1}.{li + 1}</span>
											<div class="ce__lf">
												<input
													id={`ce-lesson-${lesson.id}-title`}
													name="lesson-title"
													type="text"
													aria-label={`Lesson ${mi + 1}.${li + 1} title`}
													value={lesson.title}
													oninput={(e) =>
														updateLesson(
															mi,
															li,
															'title',
															(e.target as HTMLInputElement).value
														)}
													class="ce__inp ce__inp--sm"
													placeholder="Lesson title"
												/>
												<div class="ce__lurl">
													<VideoCameraIcon size={14} /><input
														id={`ce-lesson-${lesson.id}-video`}
														name="lesson-video-url"
														type="url"
														aria-label={`Lesson ${mi + 1}.${li + 1} video URL`}
														value={lesson.video_url}
														oninput={(e) =>
															updateLesson(
																mi,
																li,
																'video_url',
																(e.target as HTMLInputElement).value
															)}
														class="ce__inp ce__inp--sm"
														placeholder="Video URL"
													/>
												</div>
											</div>
											<div class="ce__la">
												<button
													type="button"
													class="ce__ib"
													title={lesson.is_preview
														? 'Disable preview'
														: 'Enable preview'}
													onclick={() =>
														updateLesson(
															mi,
															li,
															'is_preview',
															!lesson.is_preview
														)}
												>
													{#if lesson.is_preview}<EyeIcon
															size={16}
															weight="fill"
														/>{:else}<EyeSlashIcon size={16} />{/if}
												</button>
												<button
													type="button"
													class="ce__ib ce__ib--d"
													title="Remove lesson"
													onclick={() => removeLesson(mi, li)}
													><TrashIcon size={14} /></button
												>
											</div>
										</div>
									{/each}
									<div class="ce__mft">
										<button
											type="button"
											onclick={() => addLesson(mi)}
											class="ce__alb"
											><PlusIcon size={14} weight="bold" /> Add Lesson</button
										>
										<button
											type="button"
											onclick={() => removeModule(mi)}
											class="ce__rmb"
											><TrashIcon size={14} /> Remove Module</button
										>
									</div>
								</div>
							{/if}
						</div>
					{/each}
				</div>
			</div>

			<div class="ce__acts">
				<button type="button" onclick={handleDelete} disabled={deleting} class="ce__del"
					><TrashIcon size={16} weight="bold" />
					{deleting ? 'Deleting...' : 'Delete Course'}</button
				>
				<div class="ce__ar">
					<a href={resolve('/admin/courses')} class="ce__cancel">Cancel</a>
					<button type="submit" disabled={saving} class="ce__save"
						><FloppyDiskIcon size={16} weight="bold" />
						{saving ? 'Saving...' : 'Save Course'}</button
					>
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
	.ce__back:hover {
		color: var(--color-white);
	}
	.ce__status {
		text-align: center;
		padding: 3rem;
		color: var(--color-grey-400);
	}
	.ce__status--err {
		color: var(--color-red);
	}
	.ce__hdr {
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
		border-radius: var(--radius-2xl);
		font-size: var(--fs-sm);
		margin-bottom: 1rem;
	}
	.ce__alert--err {
		background-color: rgba(239, 68, 68, 0.1);
		border: 1px solid rgba(239, 68, 68, 0.3);
		color: #fca5a5;
	}
	.ce__alert--ok {
		background-color: rgba(34, 197, 94, 0.1);
		border: 1px solid rgba(34, 197, 94, 0.3);
		color: #86efac;
	}
	.ce__form {
		display: flex;
		flex-direction: column;
		gap: 1.5rem;
	}
	.ce__card {
		background: rgba(19, 43, 80, 0.35);
		backdrop-filter: blur(24px);
		-webkit-backdrop-filter: blur(24px);
		border: 1px solid rgba(255, 255, 255, 0.06);
		border-radius: var(--radius-2xl);
		padding: 1.5rem;
	}
	.ce__stitle {
		font-size: var(--fs-lg);
		font-weight: var(--w-bold);
		color: var(--color-white);
		font-family: var(--font-heading);
		margin: 0 0 1.25rem;
	}
	.ce__grid {
		display: grid;
		grid-template-columns: 1fr 1fr;
		gap: 1.25rem;
	}
	.ce__f {
		display: flex;
		flex-direction: column;
		gap: 0.4rem;
	}
	.ce__f--full {
		grid-column: 1 / -1;
	}
	.ce__lbl {
		font-size: var(--fs-sm);
		font-weight: var(--w-medium);
		color: var(--color-grey-300);
	}
	.ce__inp,
	.ce__ta {
		width: 100%;
		padding: 0.65rem 0.85rem;
		background-color: rgba(255, 255, 255, 0.05);
		border: 1px solid rgba(255, 255, 255, 0.1);
		border-radius: var(--radius-2xl);
		color: var(--color-white);
		font-size: var(--fs-sm);
		font-family: inherit;
		transition: border-color 200ms var(--ease-out);
	}
	.ce__inp:focus,
	.ce__ta:focus {
		outline: none;
		border-color: var(--color-teal);
	}
	.ce__inp::placeholder,
	.ce__ta::placeholder {
		color: var(--color-grey-500);
	}
	.ce__ta {
		resize: vertical;
	}
	.ce__inp--mono {
		font-family: 'SF Mono', 'Fira Code', monospace;
		font-size: var(--fs-xs);
	}
	.ce__inp--sm {
		padding: 0.45rem 0.7rem;
		font-size: var(--fs-xs);
	}
	.ce__tp {
		margin-top: 1rem;
		border-radius: var(--radius-2xl);
		overflow: hidden;
		border: 1px solid rgba(255, 255, 255, 0.08);
		max-width: 20rem;
	}
	.ce__timg {
		width: 100%;
		height: auto;
		display: block;
		max-height: 10rem;
		object-fit: cover;
	}
	.ce__mhdr {
		display: flex;
		align-items: center;
		justify-content: space-between;
		margin-bottom: 1.25rem;
	}
	.ce__mhdr .ce__stitle {
		margin-bottom: 0;
	}
	.ce__add {
		display: inline-flex;
		align-items: center;
		gap: 0.35rem;
		padding: 0.5rem 1rem;
		background-color: rgba(15, 164, 175, 0.1);
		border: 1px solid rgba(15, 164, 175, 0.3);
		border-radius: var(--radius-2xl);
		color: var(--color-teal);
		font-size: var(--fs-sm);
		font-weight: var(--w-semibold);
		cursor: pointer;
		transition: background-color 200ms var(--ease-out);
	}
	.ce__add:hover {
		background-color: rgba(15, 164, 175, 0.2);
	}
	.ce__empty {
		text-align: center;
		padding: 2rem;
		color: var(--color-grey-500);
		font-size: var(--fs-sm);
	}
	.ce__mlist {
		display: flex;
		flex-direction: column;
		gap: 0.75rem;
	}
	.ce__mc {
		border: 1px solid rgba(255, 255, 255, 0.08);
		border-radius: var(--radius-2xl);
		overflow: hidden;
		background-color: rgba(0, 0, 0, 0.15);
	}
	.ce__mtog {
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
	.ce__mtog:hover {
		background-color: rgba(255, 255, 255, 0.03);
	}
	.ce__mnum {
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
	.ce__minp {
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
	.ce__minp:focus {
		outline: none;
		border-color: var(--color-teal);
		background-color: rgba(255, 255, 255, 0.05);
	}
	.ce__minp::placeholder {
		color: var(--color-grey-500);
	}
	.ce__mmeta {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		flex-shrink: 0;
	}
	.ce__lcnt {
		font-size: var(--fs-xs);
		color: var(--color-grey-500);
	}
	.ce__mbody {
		padding: 0.75rem 1rem 1rem;
		border-top: 1px solid rgba(255, 255, 255, 0.06);
	}
	.ce__nol {
		color: var(--color-grey-500);
		font-size: var(--fs-xs);
		padding: 0.5rem 0;
		margin: 0;
	}
	.ce__lr {
		display: flex;
		align-items: flex-start;
		gap: 0.65rem;
		padding: 0.65rem 0;
		border-bottom: 1px solid rgba(255, 255, 255, 0.04);
	}
	.ce__lr:last-of-type {
		border-bottom: none;
	}
	.ce__lnum {
		font-size: var(--fs-xs);
		color: var(--color-grey-500);
		font-weight: var(--w-medium);
		min-width: 2rem;
		padding-top: 0.5rem;
	}
	.ce__lf {
		flex: 1;
		display: flex;
		flex-direction: column;
		gap: 0.4rem;
		min-width: 0;
	}
	.ce__lurl {
		display: flex;
		align-items: center;
		gap: 0.4rem;
		color: var(--color-grey-500);
	}
	.ce__lurl .ce__inp {
		flex: 1;
	}
	.ce__la {
		display: flex;
		align-items: center;
		gap: 0.25rem;
		padding-top: 0.35rem;
		flex-shrink: 0;
	}
	.ce__ib {
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
	.ce__ib:hover {
		background: rgba(255, 255, 255, 0.1);
		color: var(--color-teal-light);
	}
	.ce__ib--d:hover {
		background: rgba(239, 68, 68, 0.15);
		color: var(--color-red);
	}
	.ce__mft {
		display: flex;
		align-items: center;
		justify-content: space-between;
		margin-top: 0.75rem;
		padding-top: 0.75rem;
		border-top: 1px solid rgba(255, 255, 255, 0.06);
	}
	.ce__alb {
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
	.ce__alb:hover {
		background: rgba(15, 164, 175, 0.08);
		border-color: var(--color-teal);
	}
	.ce__rmb {
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
	.ce__rmb:hover {
		background: rgba(239, 68, 68, 0.1);
		color: var(--color-red);
		border-color: rgba(239, 68, 68, 0.4);
	}
	.ce__acts {
		display: flex;
		align-items: center;
		justify-content: space-between;
		background: rgba(19, 43, 80, 0.35);
		backdrop-filter: blur(24px);
		-webkit-backdrop-filter: blur(24px);
		border: 1px solid rgba(255, 255, 255, 0.06);
		border-radius: var(--radius-2xl);
		padding: 1.25rem 1.5rem;
	}
	.ce__ar {
		display: flex;
		gap: 1rem;
		align-items: center;
	}
	.ce__del {
		display: flex;
		align-items: center;
		gap: 0.4rem;
		padding: 0.6rem 1.25rem;
		background-color: rgba(239, 68, 68, 0.08);
		border: 1px solid rgba(239, 68, 68, 0.25);
		border-radius: var(--radius-2xl);
		color: var(--color-red);
		font-size: var(--fs-sm);
		font-weight: var(--w-semibold);
		cursor: pointer;
		transition: background-color 200ms var(--ease-out);
	}
	.ce__del:hover:not(:disabled) {
		background-color: rgba(239, 68, 68, 0.18);
	}
	.ce__del:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}
	.ce__cancel {
		padding: 0.6rem 1.25rem;
		border: 1px solid rgba(255, 255, 255, 0.1);
		border-radius: var(--radius-2xl);
		color: var(--color-grey-400);
		font-size: var(--fs-sm);
		font-weight: var(--w-medium);
		text-decoration: none;
		transition:
			border-color 200ms var(--ease-out),
			color 200ms var(--ease-out);
	}
	.ce__cancel:hover {
		border-color: rgba(255, 255, 255, 0.2);
		color: var(--color-white);
	}
	.ce__save {
		display: flex;
		align-items: center;
		gap: 0.4rem;
		padding: 0.6rem 1.5rem;
		background: linear-gradient(135deg, var(--color-teal), #0d8a94);
		color: var(--color-white);
		font-weight: var(--w-semibold);
		font-size: var(--fs-sm);
		border-radius: var(--radius-2xl);
		cursor: pointer;
		transition: opacity 200ms var(--ease-out);
	}
	.ce__save:hover:not(:disabled) {
		opacity: 0.9;
	}
	.ce__save:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}
	@media (max-width: 768px) {
		.ce__grid {
			grid-template-columns: 1fr;
		}
		.ce__f--full {
			grid-column: 1;
		}
		.ce__acts {
			flex-direction: column;
			gap: 1rem;
		}
		.ce__lr {
			flex-wrap: wrap;
		}
	}
</style>
