<script lang="ts">
	import { courses } from '$lib/data/courses';
	import Button from './Button.svelte';
	import CaretDown from 'phosphor-svelte/lib/CaretDown';

	let isCoursesOpen = $state(false);

	function toggleCourses() {
		isCoursesOpen = !isCoursesOpen;
	}

	function closeCourses() {
		isCoursesOpen = false;
	}
</script>

<nav class="bg-navy/92 fixed top-0 right-0 left-0 z-50 border-b border-white/10 backdrop-blur-md">
	<div class="mx-auto flex h-16 max-w-[1200px] items-center justify-between px-4 sm:px-6 lg:px-8">
		<!-- Logo -->
		<a href="/" class="font-heading flex items-center gap-0 text-xl font-bold">
			<span class="text-white">Explosive</span>
			<span class="text-teal-light">Swings</span>
		</a>

		<!-- Nav Links -->
		<div class="hidden items-center gap-8 md:flex">
			<!-- Courses Dropdown -->
			<div class="relative">
				<button
					onclick={toggleCourses}
					class="text-grey-300 flex items-center gap-1 text-sm font-medium transition-colors hover:text-white"
				>
					Courses
					<CaretDown
						size={16}
						weight="bold"
						class={isCoursesOpen ? 'rotate-180 transition-transform' : 'transition-transform'}
					/>
				</button>

				{#if isCoursesOpen}
					<div
						class="bg-navy/95 absolute top-full right-0 mt-2 w-80 overflow-hidden rounded-xl border border-white/10 shadow-xl backdrop-blur-lg"
						onmouseleave={closeCourses}
					>
						<div class="p-2">
							{#each courses as course}
								<a
									href="/courses/{course.slug}"
									class="block rounded-lg p-4 transition-colors hover:bg-white/5"
									onclick={closeCourses}
								>
									<div class="flex items-start gap-3">
										<div
											class="flex h-10 w-10 shrink-0 items-center justify-center rounded-lg"
											style="background: linear-gradient(135deg, {course.gradient.from} 0%, {course
												.gradient.to} 100%);"
										>
											<span class="text-xs font-bold text-white">{course.level.charAt(0)}</span>
										</div>
										<div>
											<h4 class="mb-1 text-sm font-semibold text-white">{course.title}</h4>
											<p class="text-grey-400 line-clamp-2 text-xs">{course.description}</p>
											<div class="mt-2 flex items-center gap-2">
												<span class="text-teal-light text-xs font-semibold">${course.price}</span>
												<span class="text-grey-500 text-xs">•</span>
												<span class="text-grey-500 text-xs">{course.level}</span>
											</div>
										</div>
									</div>
								</a>
							{/each}

							<div class="mt-2 border-t border-white/10 pt-2">
								<a
									href="/courses"
									class="text-teal-light hover:text-teal block rounded-lg p-3 text-center text-sm font-semibold transition-colors hover:bg-white/5"
									onclick={closeCourses}
								>
									View All Courses →
								</a>
							</div>
						</div>
					</div>
				{/if}
			</div>

			<a
				href="/#pricing"
				class="text-grey-300 text-sm font-medium transition-colors hover:text-white"
			>
				Pricing
			</a>
		</div>

		<!-- CTA -->
		<Button variant="primary" href="#pricing">Get Instant Access</Button>
	</div>
</nav>
