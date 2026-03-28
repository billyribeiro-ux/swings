<script lang="ts">
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

	function handleScroll() {
		scrolled = window.scrollY > 20;
	}
</script>

<svelte:window onclick={handleWindowClick} onkeydown={handleKeydown} onscroll={handleScroll} />

<nav bind:this={navRef} class={['nav', scrolled && 'nav--scrolled']}>
	<div class="nav__inner">
		<!-- Logo -->
		<a href="/" class="nav__logo">
			<span class="nav__logo-brand">Explosive</span>
			<span class="nav__logo-accent">Swings</span>
		</a>

		<!-- Desktop Nav -->
		<div class="nav__desktop">
			<a href="/about" class="nav__link">About</a>

			<!-- Courses Dropdown -->
			<div class="nav__dropdown" bind:this={dropdownRef}>
				<button
					onclick={toggleCourses}
					aria-expanded={isCoursesOpen}
					aria-haspopup="true"
					class={['nav__link', 'nav__link--dropdown', isCoursesOpen && 'nav__link--active']}
				>
					Courses
					<CaretDown
						size={14}
						weight="bold"
						class={['nav__caret', isCoursesOpen && 'nav__caret--open']}
					/>
				</button>

				{#if isCoursesOpen}
					<div class="dropdown-panel" transition:fly={{ y: -8, duration: 250, easing: cubicOut }}>
						<div class="dropdown-panel__inner">
							{#each courses as course, i (course.id)}
								{@const Icon = iconMap[course.icon]}
								<a href="/courses/{course.slug}" class="dropdown-item" onclick={closeAll}>
									<div
										class="dropdown-item__icon"
										style="background: linear-gradient(135deg, {course.gradient.from}, {course
											.gradient.to});"
									>
										{#if Icon}
											<Icon size={22} weight="duotone" color="white" />
										{:else}
											<span class="dropdown-item__icon-fallback">{course.level.charAt(0)}</span>
										{/if}
									</div>
									<div class="dropdown-item__content">
										<h4 class="dropdown-item__title">{course.title}</h4>
										<p class="dropdown-item__desc">{course.description}</p>
										<div class="dropdown-item__meta">
											<span class="dropdown-item__price">${course.price}</span>
											<span class="dropdown-item__sep">·</span>
											<span class="dropdown-item__level">{course.level}</span>
										</div>
									</div>
									<ArrowRight size={14} weight="bold" class="dropdown-item__arrow" />
								</a>
							{/each}

							<div class="dropdown-panel__footer">
								<a href="/courses" class="dropdown-panel__view-all" onclick={closeAll}>
									View All Courses
									<ArrowRight size={14} weight="bold" />
								</a>
							</div>
						</div>
					</div>
				{/if}
			</div>

			<a href="/blog" class="nav__link">Blog</a>
			<a href="/#pricing" class="nav__link">Pricing</a>
		</div>

		<!-- Right: CTA + Mobile toggle -->
		<div class="nav__right">
			<div class="nav__cta-desktop">
				<Button variant="primary" href="#pricing">Get Instant Access</Button>
			</div>

			<!-- Mobile hamburger -->
			<button
				onclick={toggleMobile}
				aria-label={isMobileOpen ? 'Close menu' : 'Open menu'}
				aria-expanded={isMobileOpen}
				class="nav__hamburger"
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
		<div class="mobile-menu" transition:slide={{ duration: 300, easing: cubicOut }}>
			<div class="mobile-menu__inner">
				<!-- Courses Section -->
				<div class="mobile-menu__section">
					<p class="mobile-menu__label">Courses</p>
					<div class="mobile-menu__courses">
						{#each courses as course (course.id)}
							{@const Icon = iconMap[course.icon]}
							<a href="/courses/{course.slug}" class="mobile-course-item" onclick={closeAll}>
								<div
									class="mobile-course-item__icon"
									style="background: linear-gradient(135deg, {course.gradient.from}, {course
										.gradient.to});"
								>
									{#if Icon}
										<Icon size={20} weight="duotone" color="white" />
									{:else}
										<span class="mobile-course-item__icon-fallback">{course.level.charAt(0)}</span>
									{/if}
								</div>
								<div class="mobile-course-item__content">
									<h4 class="mobile-course-item__title">{course.title}</h4>
									<p class="mobile-course-item__meta">{course.level} · ${course.price}</p>
								</div>
								<ArrowRight size={16} weight="bold" class="mobile-course-item__arrow" />
							</a>
						{/each}
					</div>
				</div>

				<!-- Links -->
				<div class="mobile-menu__links">
					<a href="/about" class="mobile-menu__link" onclick={closeAll}>About</a>
					<a href="/courses" class="mobile-menu__link" onclick={closeAll}>All Courses</a>
					<a href="/blog" class="mobile-menu__link" onclick={closeAll}>Blog</a>
					<a href="/#pricing" class="mobile-menu__link" onclick={closeAll}>Pricing</a>
				</div>

				<!-- Mobile CTA -->
				<div class="mobile-menu__cta">
					<a href="#pricing" class="mobile-menu__cta-btn" onclick={closeAll}>
						Get Instant Access
						<ArrowRight size={16} weight="bold" />
					</a>
				</div>
			</div>
		</div>
	{/if}
</nav>

<style>
	/* ---- Nav bar ---- */
	.nav {
		position: fixed;
		top: 0;
		right: 0;
		left: 0;
		z-index: var(--z-50);
		border-bottom: 1px solid rgba(255, 255, 255, 0.1);
		background-color: rgba(11, 29, 58, 0.92);
		backdrop-filter: blur(16px);
		transition: all 500ms var(--ease-out);
	}

	.nav--scrolled {
		background-color: rgba(11, 29, 58, 0.98);
		border-bottom-color: rgba(255, 255, 255, 0.2);
		box-shadow:
			var(--shadow-2xl),
			0 8px 32px rgba(11, 29, 58, 0.5);
		backdrop-filter: blur(40px);
	}

	.nav__inner {
		max-width: var(--container-max);
		margin: 0 auto;
		display: flex;
		align-items: center;
		justify-content: space-between;
		height: 4rem;
		padding: 0 1rem;
	}

	@media (min-width: 640px) {
		.nav__inner {
			padding: 0 1.5rem;
		}
	}

	@media (min-width: 1024px) {
		.nav__inner {
			padding: 0 2rem;
		}
	}

	/* ---- Logo ---- */
	.nav__logo {
		position: relative;
		z-index: var(--z-10);
		display: flex;
		align-items: center;
		gap: 0.125rem;
		font-family: var(--font-heading);
		font-size: 1.25rem;
		font-weight: var(--w-bold);
		letter-spacing: -0.025em;
	}

	.nav__logo-brand {
		color: var(--color-white);
	}
	.nav__logo-accent {
		color: var(--color-teal-light);
	}

	/* ---- Desktop nav links ---- */
	.nav__desktop {
		display: none;
		align-items: center;
		gap: 1.5rem;
	}

	@media (min-width: 768px) {
		.nav__desktop {
			display: flex;
		}
	}

	.nav__link {
		color: var(--color-grey-300);
		border-radius: var(--radius-lg);
		padding: 0.5rem 0.75rem;
		font-size: var(--fs-sm);
		font-weight: var(--w-medium);
		transition: all 200ms var(--ease-out);
	}

	.nav__link:hover {
		background-color: rgba(255, 255, 255, 0.05);
		color: var(--color-white);
	}

	.nav__link--dropdown {
		display: flex;
		align-items: center;
		gap: 0.375rem;
	}

	.nav__link--active {
		background-color: rgba(255, 255, 255, 0.05);
		color: var(--color-white);
	}

	/* ---- Dropdown ---- */
	.nav__dropdown {
		position: relative;
	}

	:global(.nav__caret) {
		transition: transform 300ms var(--ease-out) !important;
	}

	:global(.nav__caret--open) {
		transform: rotate(180deg) !important;
	}

	.dropdown-panel {
		position: absolute;
		top: 100%;
		right: 0;
		margin-top: 0.75rem;
		width: 340px;
		transform-origin: top right;
		overflow: hidden;
		border-radius: var(--radius-2xl);
		border: 1px solid rgba(255, 255, 255, 0.1);
		background-color: rgba(11, 29, 58, 0.98);
		box-shadow: var(--shadow-2xl);
		backdrop-filter: blur(40px);
	}

	.dropdown-panel__inner {
		padding: 0.75rem;
	}

	.dropdown-item {
		display: flex;
		align-items: flex-start;
		gap: 1rem;
		border-radius: var(--radius-xl);
		padding: 1rem;
		transition: all 200ms var(--ease-out);
	}

	.dropdown-item:hover {
		background-color: rgba(255, 255, 255, 0.06);
	}

	.dropdown-item__icon {
		display: flex;
		align-items: center;
		justify-content: center;
		width: 2.75rem;
		height: 2.75rem;
		flex-shrink: 0;
		border-radius: var(--radius-xl);
		transition: transform 300ms var(--ease-out);
	}

	.dropdown-item:hover .dropdown-item__icon {
		transform: scale(1.05);
	}

	.dropdown-item__icon-fallback {
		font-size: var(--fs-xs);
		font-weight: var(--w-bold);
		color: var(--color-white);
	}

	.dropdown-item__content {
		min-width: 0;
		flex: 1;
	}

	.dropdown-item__title {
		font-size: 13px;
		font-weight: var(--w-semibold);
		color: var(--color-white);
	}

	.dropdown-item__desc {
		color: var(--color-grey-400);
		margin-top: 0.125rem;
		font-size: var(--fs-xs);
		overflow: hidden;
		display: -webkit-box;
		-webkit-line-clamp: 1;
		-webkit-box-orient: vertical;
	}

	.dropdown-item__meta {
		margin-top: 0.375rem;
		display: flex;
		align-items: center;
		gap: 0.5rem;
	}

	.dropdown-item__price {
		color: var(--color-teal-light);
		font-size: var(--fs-xs);
		font-weight: var(--w-semibold);
	}

	.dropdown-item__sep {
		color: var(--color-grey-600);
	}
	.dropdown-item__level {
		color: var(--color-grey-500);
		font-size: var(--fs-xs);
	}

	:global(.dropdown-item__arrow) {
		color: var(--color-grey-600) !important;
		flex-shrink: 0;
		margin-top: 0.25rem;
		transition: all 200ms var(--ease-out) !important;
	}

	.dropdown-item:hover :global(.dropdown-item__arrow) {
		color: var(--color-teal-light) !important;
		transform: translateX(2px) !important;
	}

	.dropdown-panel__footer {
		margin-top: 0.25rem;
		border-top: 1px solid rgba(255, 255, 255, 0.08);
		padding-top: 0.25rem;
	}

	.dropdown-panel__view-all {
		display: flex;
		align-items: center;
		justify-content: center;
		gap: 0.375rem;
		border-radius: var(--radius-xl);
		padding: 0.75rem;
		color: var(--color-teal-light);
		font-size: var(--fs-sm);
		font-weight: var(--w-semibold);
		transition: all 200ms var(--ease-out);
	}

	.dropdown-panel__view-all:hover {
		background-color: rgba(255, 255, 255, 0.06);
		color: var(--color-white);
	}

	/* ---- Right section ---- */
	.nav__right {
		display: flex;
		align-items: center;
		gap: 0.75rem;
	}

	.nav__cta-desktop {
		display: none;
	}

	@media (min-width: 640px) {
		.nav__cta-desktop {
			display: block;
		}
	}

	.nav__hamburger {
		position: relative;
		z-index: var(--z-10);
		display: flex;
		align-items: center;
		justify-content: center;
		width: 2.5rem;
		height: 2.5rem;
		border-radius: var(--radius-lg);
		color: var(--color-white);
		transition: all 200ms var(--ease-out);
	}

	.nav__hamburger:hover {
		background-color: rgba(255, 255, 255, 0.1);
	}

	@media (min-width: 768px) {
		.nav__hamburger {
			display: none;
		}
	}

	/* ---- Mobile menu ---- */
	.mobile-menu {
		background-color: rgba(11, 29, 58, 0.98);
		border-top: 1px solid rgba(255, 255, 255, 0.1);
		backdrop-filter: blur(40px);
	}

	@media (min-width: 768px) {
		.mobile-menu {
			display: none;
		}
	}

	.mobile-menu__inner {
		max-width: var(--container-max);
		margin: 0 auto;
		padding: 1rem 1rem 1.5rem;
	}

	.mobile-menu__section {
		margin-bottom: 0.5rem;
	}

	.mobile-menu__label {
		color: var(--color-grey-500);
		margin-bottom: 0.75rem;
		font-size: 11px;
		font-weight: var(--w-semibold);
		letter-spacing: 0.1em;
		text-transform: uppercase;
	}

	.mobile-menu__courses {
		display: flex;
		flex-direction: column;
		gap: 0.25rem;
	}

	.mobile-course-item {
		display: flex;
		align-items: center;
		gap: 0.75rem;
		border-radius: var(--radius-xl);
		padding: 0.75rem;
		transition: all 200ms var(--ease-out);
	}

	.mobile-course-item:hover {
		background-color: rgba(255, 255, 255, 0.06);
	}

	.mobile-course-item__icon {
		display: flex;
		align-items: center;
		justify-content: center;
		width: 2.5rem;
		height: 2.5rem;
		flex-shrink: 0;
		border-radius: var(--radius-lg);
	}

	.mobile-course-item__icon-fallback {
		font-size: var(--fs-xs);
		font-weight: var(--w-bold);
		color: var(--color-white);
	}

	.mobile-course-item__content {
		min-width: 0;
		flex: 1;
	}

	.mobile-course-item__title {
		font-size: var(--fs-sm);
		font-weight: var(--w-semibold);
		color: var(--color-white);
	}

	.mobile-course-item__meta {
		color: var(--color-grey-500);
		font-size: var(--fs-xs);
	}

	:global(.mobile-course-item__arrow) {
		color: var(--color-grey-600) !important;
		flex-shrink: 0;
	}

	.mobile-menu__links {
		border-top: 1px solid rgba(255, 255, 255, 0.08);
		padding-top: 1rem;
	}

	.mobile-menu__link {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		color: var(--color-grey-300);
		border-radius: var(--radius-xl);
		padding: 0.75rem;
		font-size: var(--fs-sm);
		font-weight: var(--w-medium);
		transition: all 200ms var(--ease-out);
	}

	.mobile-menu__link:hover {
		background-color: rgba(255, 255, 255, 0.06);
		color: var(--color-white);
	}

	.mobile-menu__cta {
		margin-top: 1rem;
	}

	@media (min-width: 640px) {
		.mobile-menu__cta {
			display: none;
		}
	}

	.mobile-menu__cta-btn {
		display: flex;
		width: 100%;
		align-items: center;
		justify-content: center;
		gap: 0.5rem;
		border-radius: var(--radius-xl);
		padding: 0.875rem 1.5rem;
		font-size: var(--fs-sm);
		font-weight: var(--w-semibold);
		color: var(--color-white);
		background-color: var(--color-teal);
		box-shadow:
			var(--shadow-lg),
			0 4px 14px rgba(15, 164, 175, 0.2);
		transition: all 200ms var(--ease-out);
	}

	.mobile-menu__cta-btn:hover {
		background-color: var(--color-teal-light);
	}
</style>
