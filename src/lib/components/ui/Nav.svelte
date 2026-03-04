<script lang="ts">
	import { onMount } from 'svelte';
	import { courses } from '$lib/data/courses';
	import { fade, fly, slide } from 'svelte/transition';
	import { cubicOut } from 'svelte/easing';
	import Button from './Button.svelte';
	import CaretDown from 'phosphor-svelte/lib/CaretDown';
	import List from 'phosphor-svelte/lib/List';
	import X from 'phosphor-svelte/lib/X';
	import BookOpen from 'phosphor-svelte/lib/BookOpen';
	import Pulse from 'phosphor-svelte/lib/Pulse';
	import ArrowRight from 'phosphor-svelte/lib/ArrowRight';

	const iconMap: Record<string, typeof BookOpen> = { BookOpen, Pulse };

	let isCoursesOpen = $state(false);
	let isMobileOpen = $state(false);
	let dropdownRef: HTMLDivElement | undefined = $state();
	let scrolled = $state(false);
	let navRef: HTMLElement | undefined = $state();

	function toggleCourses() {
		isCoursesOpen = !isCoursesOpen;
	}

	function closeAll() {
		isCoursesOpen = false;
		isMobileOpen = false;
	}

	function toggleMobile() {
		isMobileOpen = !isMobileOpen;
		isCoursesOpen = false;
	}

	// Click-outside to close dropdown
	function handleWindowClick(e: MouseEvent) {
		if (isCoursesOpen && dropdownRef && !dropdownRef.contains(e.target as Node)) {
			isCoursesOpen = false;
		}
	}

	// Close mobile on Escape
	function handleKeydown(e: KeyboardEvent) {
		if (e.key === 'Escape') closeAll();
	}

	// Scroll-based effects
	onMount(() => {
		function handleScroll() {
			scrolled = window.scrollY > 20;
		}

		window.addEventListener('scroll', handleScroll, { passive: true });
		return () => window.removeEventListener('scroll', handleScroll);
	});
</script>

<svelte:window onclick={handleWindowClick} onkeydown={handleKeydown} />

<nav
	bind:this={navRef}
	class="fixed top-0 right-0 left-0 z-50 border-b transition-all duration-500 ease-out {scrolled
		? 'bg-navy/98 shadow-navy/50 border-white/20 shadow-2xl backdrop-blur-2xl'
		: 'bg-navy/92 border-white/10 backdrop-blur-xl'}"
>
	<div class="mx-auto flex h-16 max-w-[1200px] items-center justify-between px-4 sm:px-6 lg:px-8">
		<!-- Logo -->
		<a
			href="/"
			class="font-heading relative z-10 flex items-center gap-0.5 text-xl font-bold tracking-tight"
		>
			<span class="text-white">Explosive</span>
			<span class="text-teal-light">Swings</span>
		</a>

		<!-- Desktop Nav -->
		<div class="hidden items-center gap-6 md:flex">
			<a
				href="/about"
				class="text-grey-300 rounded-lg px-3 py-2 text-sm font-medium transition-all duration-200 hover:bg-white/5 hover:text-white"
			>
				About
			</a>

			<!-- Courses Dropdown -->
			<div class="relative" bind:this={dropdownRef}>
				<button
					onclick={toggleCourses}
					aria-expanded={isCoursesOpen}
					aria-haspopup="true"
					class="text-grey-300 flex items-center gap-1.5 rounded-lg px-3 py-2 text-sm font-medium transition-all duration-200 hover:text-white {isCoursesOpen
						? 'bg-white/5 text-white'
						: ''}"
				>
					Courses
					<CaretDown
						size={14}
						weight="bold"
						class="transition-transform duration-300 ease-out {isCoursesOpen ? 'rotate-180' : ''}"
					/>
				</button>

				{#if isCoursesOpen}
					<div
						class="bg-navy/98 absolute top-full right-0 mt-3 w-[340px] origin-top-right overflow-hidden rounded-2xl border border-white/10 shadow-2xl backdrop-blur-2xl"
						transition:fly={{ y: -8, duration: 250, easing: cubicOut }}
					>
						<div class="p-3">
							{#each courses as course, i}
								{@const Icon = iconMap[course.icon]}
								<a
									href="/courses/{course.slug}"
									class="group flex items-start gap-4 rounded-xl p-4 transition-all duration-200 hover:bg-white/6"
									onclick={closeAll}
								>
									<div
										class="flex h-11 w-11 shrink-0 items-center justify-center rounded-xl transition-transform duration-300 group-hover:scale-105"
										style="background: linear-gradient(135deg, {course.gradient.from}, {course
											.gradient.to});"
									>
										{#if Icon}
											<Icon size={22} weight="duotone" color="white" />
										{:else}
											<span class="text-xs font-bold text-white">{course.level.charAt(0)}</span>
										{/if}
									</div>
									<div class="min-w-0 flex-1">
										<h4 class="text-[13px] font-semibold text-white">{course.title}</h4>
										<p class="text-grey-400 mt-0.5 line-clamp-1 text-xs">{course.description}</p>
										<div class="mt-1.5 flex items-center gap-2">
											<span class="text-teal-light text-xs font-semibold">${course.price}</span>
											<span class="text-grey-600">·</span>
											<span class="text-grey-500 text-xs">{course.level}</span>
										</div>
									</div>
									<ArrowRight
										size={14}
										weight="bold"
										class="text-grey-600 group-hover:text-teal-light mt-1 shrink-0 transition-all duration-200 group-hover:translate-x-0.5"
									/>
								</a>
							{/each}

							<div class="mt-1 border-t border-white/8 pt-1">
								<a
									href="/courses"
									class="text-teal-light flex items-center justify-center gap-1.5 rounded-xl p-3 text-sm font-semibold transition-all duration-200 hover:bg-white/6 hover:text-white"
									onclick={closeAll}
								>
									View All Courses
									<ArrowRight size={14} weight="bold" />
								</a>
							</div>
						</div>
					</div>
				{/if}
			</div>

			<a
				href="/blog"
				class="text-grey-300 rounded-lg px-3 py-2 text-sm font-medium transition-all duration-200 hover:bg-white/5 hover:text-white"
			>
				Blog
			</a>

			<a
				href="/#pricing"
				class="text-grey-300 rounded-lg px-3 py-2 text-sm font-medium transition-all duration-200 hover:bg-white/5 hover:text-white"
			>
				Pricing
			</a>
		</div>

		<!-- Right: CTA + Mobile toggle -->
		<div class="flex items-center gap-3">
			<div class="hidden sm:block">
				<Button variant="primary" href="#pricing">Get Instant Access</Button>
			</div>

			<!-- Mobile hamburger -->
			<button
				onclick={toggleMobile}
				aria-label={isMobileOpen ? 'Close menu' : 'Open menu'}
				aria-expanded={isMobileOpen}
				class="relative z-10 flex h-10 w-10 items-center justify-center rounded-lg text-white transition-all duration-200 hover:bg-white/10 md:hidden"
			>
				{#if isMobileOpen}
					<X size={24} weight="bold" />
				{:else}
					<List size={24} weight="bold" />
				{/if}
			</button>
		</div>
	</div>

	<!-- Mobile Menu -->
	{#if isMobileOpen}
		<div
			class="bg-navy/98 border-t border-white/10 backdrop-blur-2xl md:hidden"
			transition:slide={{ duration: 300, easing: cubicOut }}
		>
			<div class="mx-auto max-w-[1200px] px-4 pt-4 pb-6">
				<!-- Courses Section -->
				<div class="mb-2">
					<p class="text-grey-500 mb-3 text-[11px] font-semibold tracking-widest uppercase">
						Courses
					</p>
					<div class="space-y-1">
						{#each courses as course}
							{@const Icon = iconMap[course.icon]}
							<a
								href="/courses/{course.slug}"
								class="flex items-center gap-3 rounded-xl px-3 py-3 transition-all duration-200 hover:bg-white/6"
								onclick={closeAll}
							>
								<div
									class="flex h-10 w-10 shrink-0 items-center justify-center rounded-lg"
									style="background: linear-gradient(135deg, {course.gradient.from}, {course
										.gradient.to});"
								>
									{#if Icon}
										<Icon size={20} weight="duotone" color="white" />
									{:else}
										<span class="text-xs font-bold text-white">{course.level.charAt(0)}</span>
									{/if}
								</div>
								<div class="min-w-0 flex-1">
									<h4 class="text-sm font-semibold text-white">{course.title}</h4>
									<p class="text-grey-500 text-xs">{course.level} · ${course.price}</p>
								</div>
								<ArrowRight size={16} weight="bold" class="text-grey-600 shrink-0" />
							</a>
						{/each}
					</div>
				</div>

				<!-- Links -->
				<div class="border-t border-white/8 pt-4">
					<a
						href="/about"
						class="text-grey-300 flex items-center gap-2 rounded-xl px-3 py-3 text-sm font-medium transition-all duration-200 hover:bg-white/6 hover:text-white"
						onclick={closeAll}
					>
						About
					</a>
					<a
						href="/courses"
						class="text-grey-300 flex items-center gap-2 rounded-xl px-3 py-3 text-sm font-medium transition-all duration-200 hover:bg-white/6 hover:text-white"
						onclick={closeAll}
					>
						All Courses
					</a>
					<a
						href="/blog"
						class="text-grey-300 flex items-center gap-2 rounded-xl px-3 py-3 text-sm font-medium transition-all duration-200 hover:bg-white/6 hover:text-white"
						onclick={closeAll}
					>
						Blog
					</a>
					<a
						href="/#pricing"
						class="text-grey-300 flex items-center gap-2 rounded-xl px-3 py-3 text-sm font-medium transition-all duration-200 hover:bg-white/6 hover:text-white"
						onclick={closeAll}
					>
						Pricing
					</a>
				</div>

				<!-- Mobile CTA -->
				<div class="mt-4 sm:hidden">
					<a
						href="#pricing"
						class="bg-teal shadow-teal/20 hover:bg-teal-light flex w-full items-center justify-center gap-2 rounded-xl px-6 py-3.5 text-sm font-semibold text-white shadow-lg transition-all duration-200"
						onclick={closeAll}
					>
						Get Instant Access
						<ArrowRight size={16} weight="bold" />
					</a>
				</div>
			</div>
		</div>
	{/if}
</nav>
