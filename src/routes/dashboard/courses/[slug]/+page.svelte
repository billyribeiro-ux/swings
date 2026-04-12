<script lang="ts">
	import { page } from '$app/stores';
	import { onMount } from 'svelte';
	import { SvelteMap, SvelteSet } from 'svelte/reactivity';
	import { api } from '$lib/api/client';
	import type {
		CourseWithModules,
		CourseLesson,
		LessonProgress
	} from '$lib/api/types';
	import Play from 'phosphor-svelte/lib/Play';
	import CheckCircle from 'phosphor-svelte/lib/CheckCircle';
	import CaretDown from 'phosphor-svelte/lib/CaretDown';
	import CaretRight from 'phosphor-svelte/lib/CaretRight';
	import ArrowLeft from 'phosphor-svelte/lib/ArrowLeft';
	import ArrowRight from 'phosphor-svelte/lib/ArrowRight';
	import List from 'phosphor-svelte/lib/List';
	import X from 'phosphor-svelte/lib/X';

	let course = $state<CourseWithModules | null>(null);
	let lessonProgressMap = new SvelteMap<string, LessonProgress>();
	let currentLesson = $state<CourseLesson | null>(null);
	let currentModuleId = $state<string | null>(null);
	let loading = $state(true);
	let markingComplete = $state(false);
	let sidebarOpen = $state(false);
	let expandedModules = new SvelteSet<string>();

	let slug = $derived($page.params.slug);

	onMount(async () => {
		try {
			const courseData = await api.get<CourseWithModules>(`/api/courses/${slug}`);
			course = courseData;

			// Load progress for all lessons
			try {
				const progressList = await api.get<LessonProgress[]>(
					`/api/member/courses/${courseData.id}/progress`
				);
				lessonProgressMap.clear();
				for (const p of progressList) {
					lessonProgressMap.set(p.lesson_id, p);
				}
			} catch {
				// No progress yet
			}

			// Set initial lesson: find first incomplete, or first lesson
			const firstIncomplete = findFirstIncompleteLesson(courseData);
			if (firstIncomplete) {
				currentLesson = firstIncomplete.lesson;
				currentModuleId = firstIncomplete.moduleId;
				expandedModules.clear();
				expandedModules.add(firstIncomplete.moduleId);
			} else if (courseData.modules.length > 0 && courseData.modules[0].lessons.length > 0) {
				currentLesson = courseData.modules[0].lessons[0];
				currentModuleId = courseData.modules[0].id;
				expandedModules.clear();
				expandedModules.add(courseData.modules[0].id);
			}
		} catch {
			// handle silently
		} finally {
			loading = false;
		}
	});

	function findFirstIncompleteLesson(
		c: CourseWithModules
	): { lesson: CourseLesson; moduleId: string } | null {
		for (const mod of c.modules) {
			for (const lesson of mod.lessons) {
				const progress = lessonProgressMap.get(lesson.id);
				if (!progress || !progress.completed) {
					return { lesson, moduleId: mod.id };
				}
			}
		}
		return null;
	}

	let allLessons = $derived(
		course
			? course.modules.flatMap((m) => m.lessons.map((l) => ({ lesson: l, moduleId: m.id })))
			: []
	);

	let currentIndex = $derived(
		currentLesson ? allLessons.findIndex((item) => item.lesson.id === currentLesson!.id) : -1
	);

	let prevLesson = $derived(currentIndex > 0 ? allLessons[currentIndex - 1] : null);
	let nextLesson = $derived(
		currentIndex >= 0 && currentIndex < allLessons.length - 1
			? allLessons[currentIndex + 1]
			: null
	);

	let isCurrentComplete = $derived(
		currentLesson ? lessonProgressMap.get(currentLesson.id)?.completed ?? false : false
	);

	function selectLesson(lesson: CourseLesson, moduleId: string) {
		currentLesson = lesson;
		currentModuleId = moduleId;
		sidebarOpen = false;
		// Ensure module is expanded (SvelteSet is reactive on mutation)
		expandedModules.add(moduleId);
	}

	function toggleModule(moduleId: string) {
		if (expandedModules.has(moduleId)) {
			expandedModules.delete(moduleId);
		} else {
			expandedModules.add(moduleId);
		}
	}

	function isLessonCompleted(lessonId: string): boolean {
		return lessonProgressMap.get(lessonId)?.completed ?? false;
	}

	async function markComplete() {
		if (!currentLesson || !course) return;
		markingComplete = true;
		try {
			const progress = await api.post<LessonProgress>(
				`/api/member/courses/${course.id}/lessons/${currentLesson.id}/complete`
			);
			// SvelteMap is reactive on mutation
			lessonProgressMap.set(currentLesson.id, progress);
		} catch {
			// handle silently
		} finally {
			markingComplete = false;
		}
	}

	function goToLesson(item: { lesson: CourseLesson; moduleId: string }) {
		selectLesson(item.lesson, item.moduleId);
	}
</script>

<svelte:head>
	<title>{currentLesson?.title ?? 'Course'} - Explosive Swings</title>
</svelte:head>

{#if loading}
	<div class="player__loading">
		<p>Loading...</p>
	</div>
{:else if !course}
	<div class="player__loading">
		<p>Course not found.</p>
	</div>
{:else}
	<!-- Mobile sidebar toggle -->
	<button class="player__mobile-toggle" onclick={() => (sidebarOpen = !sidebarOpen)}>
		{#if sidebarOpen}
			<X size={20} />
		{:else}
			<List size={20} />
		{/if}
		<span>{sidebarOpen ? 'Close' : 'Lessons'}</span>
	</button>

	<div class="player">
		<!-- Main Content Area -->
		<div class="player__main">
			{#if currentLesson}
				<!-- Video / Embed Area -->
				<div class="player__video">
					{#if currentLesson.video_url}
						<iframe
							src={currentLesson.video_url}
							title={currentLesson.title}
							class="player__iframe"
							allowfullscreen
							allow="accelerometer; autoplay; clipboard-write; encrypted-media; gyroscope; picture-in-picture"
						></iframe>
					{:else}
						<div class="player__placeholder">
							<Play size={48} weight="fill" />
							<p>No video available for this lesson</p>
						</div>
					{/if}
				</div>

				<!-- Lesson Info -->
				<div class="player__lesson-info">
					<div class="player__lesson-header">
						<h2 class="player__lesson-title">{currentLesson.title}</h2>
						<button
							class="player__mark-complete"
							class:player__mark-complete--done={isCurrentComplete}
							disabled={markingComplete || isCurrentComplete}
							onclick={markComplete}
						>
							<CheckCircle size={18} weight={isCurrentComplete ? 'fill' : 'regular'} />
							{#if markingComplete}
								Marking...
							{:else if isCurrentComplete}
								Completed
							{:else}
								Mark as Complete
							{/if}
						</button>
					</div>

					{#if currentLesson.description}
						<p class="player__lesson-desc">{currentLesson.description}</p>
					{/if}

					{#if currentLesson.content}
						<div class="player__lesson-content">
							{@html currentLesson.content}
						</div>
					{/if}
				</div>

				<!-- Prev / Next -->
				<div class="player__nav-buttons">
					{#if prevLesson}
						<button
							class="player__nav-btn"
							onclick={() => goToLesson(prevLesson!)}
						>
							<ArrowLeft size={16} />
							Previous
						</button>
					{:else}
						<div></div>
					{/if}
					{#if nextLesson}
						<button
							class="player__nav-btn player__nav-btn--next"
							onclick={() => goToLesson(nextLesson!)}
						>
							Next
							<ArrowRight size={16} />
						</button>
					{:else}
						<div></div>
					{/if}
				</div>
			{:else}
				<div class="player__placeholder">
					<p>Select a lesson to begin.</p>
				</div>
			{/if}
		</div>

		<!-- Sidebar -->
		<aside class="player__sidebar" class:player__sidebar--open={sidebarOpen}>
			<h3 class="player__sidebar-title">{course.title}</h3>

			<div class="player__modules">
				{#each course.modules as mod (mod.id)}
					<div class="player__module">
						<button class="player__module-header" onclick={() => toggleModule(mod.id)}>
							{#if expandedModules.has(mod.id)}
								<CaretDown size={16} />
							{:else}
								<CaretRight size={16} />
							{/if}
							<span class="player__module-title">{mod.title}</span>
							<span class="player__module-count">
								{mod.lessons.filter((l) => isLessonCompleted(l.id)).length}/{mod.lessons.length}
							</span>
						</button>

						{#if expandedModules.has(mod.id)}
							<ul class="player__lessons">
								{#each mod.lessons as lesson (lesson.id)}
									{@const isCurrent = currentLesson?.id === lesson.id}
									{@const completed = isLessonCompleted(lesson.id)}
									<li>
										<button
											class="player__lesson-item"
											class:player__lesson-item--active={isCurrent}
											class:player__lesson-item--completed={completed}
											onclick={() => selectLesson(lesson, mod.id)}
										>
											<span class="player__lesson-check">
												{#if completed}
													<CheckCircle size={16} weight="fill" />
												{:else}
													<span class="player__lesson-dot"></span>
												{/if}
											</span>
											<span class="player__lesson-name">{lesson.title}</span>
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
	.player__loading {
		display: flex;
		align-items: center;
		justify-content: center;
		min-height: 20rem;
		color: var(--color-grey-400);
		font-size: var(--fs-sm);
	}

	/* Mobile Toggle */
	.player__mobile-toggle {
		display: none;
		align-items: center;
		gap: 0.5rem;
		padding: 0.6rem 1rem;
		margin-bottom: 1rem;
		background-color: var(--color-navy-mid);
		border: 1px solid rgba(255, 255, 255, 0.08);
		border-radius: var(--radius-lg);
		color: var(--color-grey-300);
		font-size: var(--fs-sm);
		font-weight: var(--w-medium);
		cursor: pointer;
	}

	@media (max-width: 768px) {
		.player__mobile-toggle {
			display: inline-flex;
		}
	}

	/* Layout */
	.player {
		display: flex;
		gap: 1.5rem;
	}

	.player__main {
		flex: 1;
		min-width: 0;
	}

	/* Video */
	.player__video {
		position: relative;
		width: 100%;
		padding-top: 56.25%;
		background-color: var(--color-navy-mid);
		border-radius: var(--radius-xl);
		overflow: hidden;
		margin-bottom: 1.5rem;
	}

	.player__iframe {
		position: absolute;
		inset: 0;
		width: 100%;
		height: 100%;
		border: none;
	}

	.player__placeholder {
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

	/* Lesson Info */
	.player__lesson-info {
		margin-bottom: 1.5rem;
	}

	.player__lesson-header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 1rem;
		margin-bottom: 1rem;
		flex-wrap: wrap;
	}

	.player__lesson-title {
		font-size: var(--fs-xl);
		font-weight: var(--w-bold);
		color: var(--color-white);
		font-family: var(--font-heading);
	}

	.player__mark-complete {
		display: inline-flex;
		align-items: center;
		gap: 0.4rem;
		padding: 0.5rem 1rem;
		background-color: rgba(255, 255, 255, 0.06);
		border: 1px solid rgba(255, 255, 255, 0.12);
		border-radius: var(--radius-lg);
		color: var(--color-grey-300);
		font-size: var(--fs-xs);
		font-weight: var(--w-semibold);
		cursor: pointer;
		transition: all 200ms var(--ease-out);
		flex-shrink: 0;
	}

	.player__mark-complete:hover:not(:disabled) {
		border-color: var(--color-teal);
		color: var(--color-teal);
	}

	.player__mark-complete--done {
		background-color: rgba(34, 181, 115, 0.15);
		border-color: rgba(34, 181, 115, 0.3);
		color: var(--color-green);
	}

	.player__mark-complete:disabled {
		cursor: default;
	}

	.player__lesson-desc {
		color: var(--color-grey-400);
		font-size: var(--fs-sm);
		line-height: var(--lh-relaxed);
		margin-bottom: 1rem;
	}

	.player__lesson-content {
		color: var(--color-grey-200);
		font-size: var(--fs-sm);
		line-height: var(--lh-relaxed);
	}

	.player__lesson-content :global(h1),
	.player__lesson-content :global(h2),
	.player__lesson-content :global(h3) {
		color: var(--color-white);
		font-family: var(--font-heading);
		margin: 1.5rem 0 0.75rem;
	}

	.player__lesson-content :global(p) {
		margin-bottom: 0.75rem;
	}

	.player__lesson-content :global(code) {
		background-color: rgba(255, 255, 255, 0.06);
		padding: 0.15rem 0.4rem;
		border-radius: var(--radius-sm);
		font-size: 0.9em;
	}

	.player__lesson-content :global(pre) {
		background-color: rgba(255, 255, 255, 0.04);
		padding: 1rem;
		border-radius: var(--radius-lg);
		overflow-x: auto;
		margin-bottom: 1rem;
	}

	/* Nav Buttons */
	.player__nav-buttons {
		display: flex;
		justify-content: space-between;
		margin-top: 1.5rem;
		padding-top: 1.5rem;
		border-top: 1px solid rgba(255, 255, 255, 0.06);
	}

	.player__nav-btn {
		display: inline-flex;
		align-items: center;
		gap: 0.5rem;
		padding: 0.6rem 1.25rem;
		background-color: rgba(255, 255, 255, 0.06);
		border: 1px solid rgba(255, 255, 255, 0.1);
		border-radius: var(--radius-lg);
		color: var(--color-grey-300);
		font-size: var(--fs-sm);
		font-weight: var(--w-medium);
		cursor: pointer;
		transition: all 200ms var(--ease-out);
	}

	.player__nav-btn:hover {
		border-color: var(--color-teal);
		color: var(--color-teal);
	}

	.player__nav-btn--next {
		background: linear-gradient(135deg, var(--color-teal), #0d8a94);
		border-color: transparent;
		color: var(--color-white);
	}

	.player__nav-btn--next:hover {
		opacity: 0.9;
		color: var(--color-white);
	}

	/* Sidebar */
	.player__sidebar {
		width: 20rem;
		flex-shrink: 0;
		background-color: var(--color-navy-mid);
		border: 1px solid rgba(255, 255, 255, 0.06);
		border-radius: var(--radius-xl);
		padding: 1.25rem;
		max-height: calc(100vh - 8rem);
		overflow-y: auto;
		position: sticky;
		top: 6rem;
	}

	.player__sidebar-title {
		font-size: var(--fs-md);
		font-weight: var(--w-bold);
		color: var(--color-white);
		font-family: var(--font-heading);
		margin-bottom: 1.25rem;
		padding-bottom: 1rem;
		border-bottom: 1px solid rgba(255, 255, 255, 0.06);
	}

	.player__modules {
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
	}

	.player__module {
		border-bottom: 1px solid rgba(255, 255, 255, 0.04);
		padding-bottom: 0.5rem;
	}

	.player__module:last-child {
		border-bottom: none;
	}

	.player__module-header {
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

	.player__module-header:hover {
		color: var(--color-white);
	}

	.player__module-title {
		flex: 1;
	}

	.player__module-count {
		font-size: var(--fs-2xs);
		color: var(--color-grey-500);
		font-weight: var(--w-medium);
	}

	.player__lessons {
		list-style: none;
		padding: 0;
		margin: 0.25rem 0 0 0;
	}

	.player__lesson-item {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		width: 100%;
		padding: 0.45rem 0.5rem;
		padding-left: 1.5rem;
		background: none;
		border: none;
		border-radius: var(--radius-md);
		color: var(--color-grey-400);
		font-size: var(--fs-xs);
		cursor: pointer;
		text-align: left;
		transition: all 150ms var(--ease-out);
	}

	.player__lesson-item:hover {
		background-color: rgba(255, 255, 255, 0.04);
		color: var(--color-grey-200);
	}

	.player__lesson-item--active {
		background-color: rgba(15, 164, 175, 0.12);
		color: var(--color-teal);
	}

	.player__lesson-item--completed {
		color: var(--color-green);
	}

	.player__lesson-item--active.player__lesson-item--completed {
		color: var(--color-teal);
	}

	.player__lesson-check {
		display: flex;
		align-items: center;
		flex-shrink: 0;
	}

	.player__lesson-dot {
		width: 0.5rem;
		height: 0.5rem;
		border-radius: var(--radius-full);
		background-color: rgba(255, 255, 255, 0.15);
		margin: 0.25rem;
	}

	.player__lesson-name {
		line-height: var(--lh-snug);
	}

	/* Mobile: sidebar becomes toggle drawer */
	@media (max-width: 768px) {
		.player {
			flex-direction: column;
		}

		.player__sidebar {
			display: none;
			width: 100%;
			position: fixed;
			top: 0;
			left: 0;
			right: 0;
			bottom: 0;
			max-height: 100vh;
			border-radius: 0;
			z-index: var(--z-50);
			padding-top: 4rem;
		}

		.player__sidebar--open {
			display: block;
		}

		.player__mobile-toggle {
			position: relative;
			z-index: 51;
		}
	}
</style>
