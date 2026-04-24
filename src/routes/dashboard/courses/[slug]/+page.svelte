<script lang="ts">
	import { page } from '$app/state';
	import { onMount } from 'svelte';
	import { SvelteMap, SvelteSet } from 'svelte/reactivity';
	import { api } from '$lib/api/client';
	import { safeHtml } from '$lib/utils/safeHtml';
	import type { CourseWithModules, CourseLesson, LessonProgress } from '$lib/api/types';
	import PlayIcon from 'phosphor-svelte/lib/PlayIcon';
	import CheckCircleIcon from 'phosphor-svelte/lib/CheckCircleIcon';
	import CaretDownIcon from 'phosphor-svelte/lib/CaretDownIcon';
	import CaretRightIcon from 'phosphor-svelte/lib/CaretRightIcon';
	import ArrowLeftIcon from 'phosphor-svelte/lib/ArrowLeftIcon';
	import ArrowRightIcon from 'phosphor-svelte/lib/ArrowRightIcon';
	import ListIcon from 'phosphor-svelte/lib/ListIcon';
	import XIcon from 'phosphor-svelte/lib/XIcon';

	let course = $state<CourseWithModules | null>(null);
	let progressMap = new SvelteMap<string, LessonProgress>();
	let currentLesson = $state<CourseLesson | null>(null);
	let loading = $state(true);
	let markingComplete = $state(false);
	let sidebarOpen = $state(false);
	let expandedModules = new SvelteSet<string>();
	let slug = $derived(page.params.slug);

	onMount(async () => {
		try {
			const data = await api.get<CourseWithModules>(`/api/courses/${slug}`);
			course = data;
			try {
				const list = await api.get<LessonProgress[]>(
					`/api/member/courses/${data.id}/progress`
				);
				for (const p of list) progressMap.set(p.lesson_id, p);
			} catch {
				/* no progress yet */
			}
			const first = findFirstIncomplete(data);
			if (first) {
				currentLesson = first.lesson;
				expandedModules.add(first.moduleId);
			} else if (data.modules.length > 0 && data.modules[0].lessons.length > 0) {
				currentLesson = data.modules[0].lessons[0];
				expandedModules.add(data.modules[0].id);
			}
		} catch {
			/* handle silently */
		} finally {
			loading = false;
		}
	});

	function findFirstIncomplete(
		c: CourseWithModules
	): { lesson: CourseLesson; moduleId: string } | null {
		for (const mod of c.modules)
			for (const lesson of mod.lessons)
				if (!progressMap.get(lesson.id)?.completed) return { lesson, moduleId: mod.id };
		return null;
	}

	let allLessons = $derived(
		course
			? course.modules.flatMap((m) => m.lessons.map((l) => ({ lesson: l, moduleId: m.id })))
			: []
	);
	let currentIndex = $derived(
		currentLesson ? allLessons.findIndex((i) => i.lesson.id === currentLesson!.id) : -1
	);
	let prevLesson = $derived(currentIndex > 0 ? allLessons[currentIndex - 1] : null);
	let nextLesson = $derived(
		currentIndex >= 0 && currentIndex < allLessons.length - 1
			? allLessons[currentIndex + 1]
			: null
	);
	let isCurrentComplete = $derived(
		currentLesson ? (progressMap.get(currentLesson.id)?.completed ?? false) : false
	);

	function selectLesson(lesson: CourseLesson, moduleId: string) {
		currentLesson = lesson;
		sidebarOpen = false;
		expandedModules.add(moduleId);
	}

	function toggleModule(id: string) {
		if (expandedModules.has(id)) expandedModules.delete(id);
		else expandedModules.add(id);
	}
	function isDone(id: string): boolean {
		return progressMap.get(id)?.completed ?? false;
	}

	async function markComplete() {
		if (!currentLesson || !course) return;
		markingComplete = true;
		try {
			const p = await api.post<LessonProgress>(
				`/api/member/courses/${course.id}/lessons/${currentLesson.id}/complete`
			);
			progressMap.set(currentLesson.id, p);
		} catch {
			/* silently */
		} finally {
			markingComplete = false;
		}
	}

	function goTo(item: { lesson: CourseLesson; moduleId: string }) {
		selectLesson(item.lesson, item.moduleId);
	}
</script>

<svelte:head
	><title>{currentLesson?.title ?? 'Course'} - Precision Options Signals</title></svelte:head
>

{#if loading}
	<div class="player-loading"><p>Loading course...</p></div>
{:else if !course}
	<div class="player-loading"><p>Course not found.</p></div>
{:else}
	<button class="mobile-toggle" onclick={() => (sidebarOpen = !sidebarOpen)}>
		{#if sidebarOpen}<XIcon size={20} />{:else}<ListIcon size={20} />{/if}
		<span>{sidebarOpen ? 'Close' : 'Lessons'}</span>
	</button>

	<div class="player">
		<div class="player-main">
			{#if currentLesson}
				<div class="video-wrap">
					{#if currentLesson.video_url}
						<iframe
							src={currentLesson.video_url}
							title={currentLesson.title}
							class="video-iframe"
							allowfullscreen
							allow="accelerometer; autoplay; clipboard-write; encrypted-media; gyroscope; picture-in-picture"
						></iframe>
					{:else}
						<div class="video-ph">
							<PlayIcon size={48} weight="fill" />
							<p>No video available</p>
						</div>
					{/if}
				</div>
				<div class="lesson-info">
					<div class="lesson-head">
						<h2 class="lesson-title">{currentLesson.title}</h2>
						<button
							class="mark-btn"
							class:mark-btn--done={isCurrentComplete}
							disabled={markingComplete || isCurrentComplete}
							onclick={markComplete}
						>
							<CheckCircleIcon
								size={18}
								weight={isCurrentComplete ? 'fill' : 'regular'}
							/>
							{markingComplete
								? 'Marking...'
								: isCurrentComplete
									? 'Completed'
									: 'Mark Complete'}
						</button>
					</div>
					{#if currentLesson.description}<p class="lesson-desc">
							{currentLesson.description}
						</p>{/if}
					{#if currentLesson.content}<div class="lesson-body">
							{@html safeHtml(currentLesson.content)}
						</div>{/if}
				</div>
				<div class="nav-row">
					{#if prevLesson}<button class="nav-btn" onclick={() => goTo(prevLesson!)}
							><ArrowLeftIcon size={16} /> Previous</button
						>{:else}<div></div>{/if}
					{#if nextLesson}<button
							class="nav-btn nav-btn--next"
							onclick={() => goTo(nextLesson!)}>Next <ArrowRightIcon size={16} /></button
						>{:else}<div></div>{/if}
				</div>
			{:else}
				<div class="video-wrap">
					<div class="video-ph"><p>Select a lesson to begin.</p></div>
				</div>
			{/if}
		</div>

		<aside class="sidebar" class:sidebar--open={sidebarOpen}>
			<h3 class="sidebar-title">{course.title}</h3>
			<div class="modules">
				{#each course.modules as mod (mod.id)}
					<div class="mod-block">
						<button class="mod-head" onclick={() => toggleModule(mod.id)}>
							{#if expandedModules.has(mod.id)}<CaretDownIcon
									size={16}
								/>{:else}<CaretRightIcon size={16} />{/if}
							<span class="mod-name">{mod.title}</span>
							<span class="mod-count"
								>{mod.lessons.filter((l) => isDone(l.id)).length}/{mod.lessons
									.length}</span
							>
						</button>
						{#if expandedModules.has(mod.id)}
							<ul class="lesson-list">
								{#each mod.lessons as lesson (lesson.id)}
									{@const active = currentLesson?.id === lesson.id}
									{@const done = isDone(lesson.id)}
									<li>
										<button
											class="lesson-row"
											class:lesson-row--active={active}
											class:lesson-row--done={done}
											onclick={() => selectLesson(lesson, mod.id)}
										>
											<span class="lesson-icon"
												>{#if done}<CheckCircleIcon
														size={16}
														weight="fill"
													/>{:else}<span class="dot"></span>{/if}</span
											>
											<span class="lesson-label">{lesson.title}</span>
										</button>
									</li>
								{/each}
							</ul>
						{/if}
					</div>
				{/each}
			</div>
		</aside>
	</div>
{/if}

<style>
	.player-loading {
		display: flex;
		align-items: center;
		justify-content: center;
		min-height: 20rem;
		color: var(--color-grey-400);
		font-size: var(--fs-sm);
	}
	.mobile-toggle {
		display: none;
		align-items: center;
		gap: 0.5rem;
		padding: 0.6rem 1rem;
		margin-bottom: 1rem;
		background: var(--color-navy-mid);
		border: 1px solid rgba(255, 255, 255, 0.08);
		border-radius: var(--radius-lg);
		color: var(--color-grey-300);
		font-size: var(--fs-sm);
		font-weight: var(--w-medium);
		cursor: pointer;
	}
	@media (max-width: 768px) {
		.mobile-toggle {
			display: inline-flex;
		}
	}

	.player {
		display: flex;
		gap: 1.5rem;
	}
	.player-main {
		flex: 7;
		min-width: 0;
	}

	.video-wrap {
		position: relative;
		width: 100%;
		padding-top: 56.25%;
		background: var(--color-navy-mid);
		border-radius: var(--radius-xl);
		overflow: hidden;
		margin-bottom: 1.5rem;
	}
	.video-iframe {
		position: absolute;
		inset: 0;
		width: 100%;
		height: 100%;
		border: none;
	}
	.video-ph {
		position: absolute;
		inset: 0;
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		gap: 1rem;
		color: var(--color-grey-500);
		font-size: var(--fs-sm);
	}

	.lesson-info {
		margin-bottom: 1.5rem;
	}
	.lesson-head {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 1rem;
		margin-bottom: 1rem;
		flex-wrap: wrap;
	}
	.lesson-title {
		font-size: var(--fs-xl);
		font-weight: var(--w-bold);
		color: var(--color-white);
		font-family: var(--font-heading);
	}
	.mark-btn {
		display: inline-flex;
		align-items: center;
		gap: 0.4rem;
		padding: 0.5rem 1rem;
		background: rgba(255, 255, 255, 0.06);
		border: 1px solid rgba(255, 255, 255, 0.12);
		border-radius: var(--radius-lg);
		color: var(--color-grey-300);
		font-size: var(--fs-xs);
		font-weight: var(--w-semibold);
		cursor: pointer;
		transition: all 200ms var(--ease-out);
		flex-shrink: 0;
	}
	.mark-btn:hover:not(:disabled) {
		border-color: var(--color-teal);
		color: var(--color-teal);
	}
	.mark-btn--done {
		background: rgba(34, 181, 115, 0.15);
		border-color: rgba(34, 181, 115, 0.3);
		color: var(--color-green);
	}
	.mark-btn:disabled {
		cursor: default;
	}
	.lesson-desc {
		color: var(--color-grey-400);
		font-size: var(--fs-sm);
		line-height: var(--lh-relaxed);
		margin-bottom: 1rem;
	}
	.lesson-body {
		color: var(--color-grey-200);
		font-size: var(--fs-sm);
		line-height: var(--lh-relaxed);
	}
	.lesson-body :global(h1),
	.lesson-body :global(h2),
	.lesson-body :global(h3) {
		color: var(--color-white);
		font-family: var(--font-heading);
		margin: 1.5rem 0 0.75rem;
	}
	.lesson-body :global(p) {
		margin-bottom: 0.75rem;
	}
	.lesson-body :global(code) {
		background: rgba(255, 255, 255, 0.06);
		padding: 0.15rem 0.4rem;
		border-radius: var(--radius-sm);
		font-size: 0.9em;
	}
	.lesson-body :global(pre) {
		background: rgba(255, 255, 255, 0.04);
		padding: 1rem;
		border-radius: var(--radius-lg);
		overflow-x: auto;
		margin-bottom: 1rem;
	}

	.nav-row {
		display: flex;
		justify-content: space-between;
		margin-top: 1.5rem;
		padding-top: 1.5rem;
		border-top: 1px solid rgba(255, 255, 255, 0.06);
	}
	.nav-btn {
		display: inline-flex;
		align-items: center;
		gap: 0.5rem;
		padding: 0.6rem 1.25rem;
		background: rgba(255, 255, 255, 0.06);
		border: 1px solid rgba(255, 255, 255, 0.1);
		border-radius: var(--radius-lg);
		color: var(--color-grey-300);
		font-size: var(--fs-sm);
		font-weight: var(--w-medium);
		cursor: pointer;
		transition: all 200ms var(--ease-out);
	}
	.nav-btn:hover {
		border-color: var(--color-teal);
		color: var(--color-teal);
	}
	.nav-btn--next {
		background: linear-gradient(135deg, var(--color-teal), #0d8a94);
		border-color: transparent;
		color: var(--color-white);
	}
	.nav-btn--next:hover {
		opacity: 0.9;
		color: var(--color-white);
	}

	.sidebar {
		width: 20rem;
		flex-shrink: 0;
		background: var(--color-navy-mid);
		border: 1px solid rgba(255, 255, 255, 0.06);
		border-radius: var(--radius-xl);
		padding: 1.25rem;
		max-height: calc(100vh - 8rem);
		overflow-y: auto;
		position: sticky;
		top: 6rem;
	}
	.sidebar-title {
		font-size: var(--fs-md);
		font-weight: var(--w-bold);
		color: var(--color-white);
		font-family: var(--font-heading);
		margin-bottom: 1.25rem;
		padding-bottom: 1rem;
		border-bottom: 1px solid rgba(255, 255, 255, 0.06);
	}
	.modules {
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
	}
	.mod-block {
		border-bottom: 1px solid rgba(255, 255, 255, 0.04);
		padding-bottom: 0.5rem;
	}
	.mod-block:last-child {
		border-bottom: none;
	}
	.mod-head {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		width: 100%;
		padding: 0.5rem 0.25rem;
		background: none;
		border: none;
		color: var(--color-grey-300);
		font-size: var(--fs-xs);
		font-weight: var(--w-semibold);
		cursor: pointer;
		text-align: left;
		transition: color 200ms var(--ease-out);
	}
	.mod-head:hover {
		color: var(--color-white);
	}
	.mod-name {
		flex: 1;
	}
	.mod-count {
		font-size: var(--fs-2xs);
		color: var(--color-grey-500);
		font-weight: var(--w-medium);
	}
	.lesson-list {
		list-style: none;
		padding: 0;
		margin: 0.25rem 0 0;
	}
	.lesson-row {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		width: 100%;
		padding: 0.45rem 0.5rem 0.45rem 1.5rem;
		background: none;
		border: none;
		border-radius: var(--radius-md);
		color: var(--color-grey-400);
		font-size: var(--fs-xs);
		cursor: pointer;
		text-align: left;
		transition: all 150ms var(--ease-out);
	}
	.lesson-row:hover {
		background: rgba(255, 255, 255, 0.04);
		color: var(--color-grey-200);
	}
	.lesson-row--active {
		background: rgba(15, 164, 175, 0.12);
		color: var(--color-teal);
	}
	.lesson-row--done {
		color: var(--color-green);
	}
	.lesson-row--active.lesson-row--done {
		color: var(--color-teal);
	}
	.lesson-icon {
		display: flex;
		align-items: center;
		flex-shrink: 0;
	}
	.dot {
		width: 0.5rem;
		height: 0.5rem;
		border-radius: var(--radius-full);
		background: rgba(255, 255, 255, 0.15);
		margin: 0.25rem;
	}
	.lesson-label {
		line-height: var(--lh-snug);
	}

	@media (max-width: 768px) {
		.player {
			flex-direction: column;
		}
		.sidebar {
			display: none;
			width: 100%;
			position: fixed;
			inset: 0;
			max-height: 100vh;
			border-radius: 0;
			z-index: var(--z-50);
			padding-top: 4rem;
		}
		.sidebar--open {
			display: block;
		}
		.mobile-toggle {
			position: relative;
			z-index: 51;
		}
	}
</style>
