<script lang="ts">
	import { courses } from '$lib/data/courses';
	import { fly, slide } from 'svelte/transition';
	import { quintOut } from 'svelte/easing';
	import { prefersReducedMotion } from 'svelte/motion';
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

	const motion = $derived(!prefersReducedMotion.current);
	const tDur = (ms: number) => (motion ? ms : 1);
	const tDelay = (ms: number) => (motion ? ms : 0);

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
		scrolled = window.scrollY > 16;
	}

	$effect(() => {
		if (typeof document === 'undefined') return;
		document.body.style.overflow = isMobileOpen ? 'hidden' : '';
		return () => {
			document.body.style.overflow = '';
		};
	});
</script>

<svelte:window onclick={handleWindowClick} onkeydown={handleKeydown} onscroll={handleScroll} />

<nav
	bind:this={navRef}
	class={['nav', scrolled && 'nav--scrolled', isMobileOpen && 'nav--mobile-open']}
	aria-label="Primary"
>
	<div class="nav__glow" aria-hidden="true"></div>
	<div class="nav__inner">
		<!-- Logo -->
		<a href="/" class="nav__logo">
			<span class="nav__logo-brand">Explosive</span>
			<span class="nav__logo-accent">Swings</span>
		</a>

		<!-- Desktop Nav -->
		<div class="nav__desktop">
			<div class="nav__pill">
				<a href="/about" class="nav__link">About</a>

				<!-- Courses Dropdown -->
				<div class="nav__dropdown" bind:this={dropdownRef}>
					<button
						type="button"
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
						<div
							class="dropdown-panel"
							transition:fly={{
								y: -12,
								opacity: 0,
								duration: tDur(300),
								easing: quintOut
							}}
						>
							<div class="dropdown-panel__inner">
								{#each courses as course (course.id)}
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
				<a href="/pricing/monthly" class="nav__link">Pricing</a>
			</div>
		</div>

		<!-- Right: CTA + Mobile toggle -->
		<div class="nav__right">
			<div class="nav__cta-desktop">
				<Button variant="primary" href="/pricing/monthly">Get Instant Access</Button>
			</div>

			<!-- Mobile hamburger -->
			<button
				type="button"
				onclick={toggleMobile}
				aria-label={isMobileOpen ? 'Close menu' : 'Open menu'}
				aria-expanded={isMobileOpen}
				class="nav__hamburger"
			>
				<span class="nav__hamburger-icons" aria-hidden="true">
					{#if isMobileOpen}
						<X size={24} weight="bold" class="nav__hamburger-svg" />
					{:else}
						<List size={24} weight="bold" class="nav__hamburger-svg" />
					{/if}
				</span>
			</button>
		</div>
	</div>

	<!-- Mobile Menu -->
	{#if isMobileOpen}
		<div class="mobile-menu" transition:slide={{ duration: tDur(360), easing: quintOut }}>
			<div class="mobile-menu__inner">
				<!-- Courses Section -->
				<div
					class="mobile-menu__section"
					in:fly={{ y: 14, duration: tDur(380), delay: tDelay(40), easing: quintOut }}
				>
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
				<div
					class="mobile-menu__links"
					in:fly={{ y: 14, duration: tDur(380), delay: tDelay(90), easing: quintOut }}
				>
					<a href="/about" class="mobile-menu__link" onclick={closeAll}>About</a>
					<a href="/courses" class="mobile-menu__link" onclick={closeAll}>All Courses</a>
					<a href="/blog" class="mobile-menu__link" onclick={closeAll}>Blog</a>
					<a href="/pricing/monthly" class="mobile-menu__link" onclick={closeAll}>Pricing</a>
				</div>

				<!-- Mobile CTA -->
				<div
					class="mobile-menu__cta"
					in:fly={{ y: 14, duration: tDur(380), delay: tDelay(130), easing: quintOut }}
				>
					<a href="/pricing/monthly" class="mobile-menu__cta-btn" onclick={closeAll}>
						Get Instant Access
						<ArrowRight size={16} weight="bold" />
					</a>
				</div>
			</div>
		</div>
	{/if}
</nav>

<style>
	/* ---- Nav shell ---- */
	.nav {
		position: fixed;
		top: 0;
		right: 0;
		left: 0;
		z-index: var(--z-50);
		isolation: isolate;
		border-bottom: 1px solid rgba(255, 255, 255, 0.08);
		background: linear-gradient(180deg, rgba(11, 29, 58, 0.94) 0%, rgba(11, 29, 58, 0.88) 100%);
		backdrop-filter: blur(20px) saturate(1.2);
		-webkit-backdrop-filter: blur(20px) saturate(1.2);
		transition:
			background 0.45s cubic-bezier(0.22, 1, 0.36, 1),
			border-color 0.45s ease,
			box-shadow 0.45s ease,
			backdrop-filter 0.45s ease;
	}

	.nav::before {
		content: '';
		position: absolute;
		inset: 0 0 auto 0;
		height: 1px;
		background: linear-gradient(90deg, transparent, rgba(15, 164, 175, 0.45) 50%, transparent);
		opacity: 0.9;
		pointer-events: none;
	}

	.nav__glow {
		pointer-events: none;
		position: absolute;
		inset: auto 0 0 50%;
		width: min(80vw, 42rem);
		height: 5rem;
		transform: translateX(-50%) translateY(40%);
		background: radial-gradient(
			ellipse at center top,
			rgba(15, 164, 175, 0.12) 0%,
			transparent 65%
		);
		opacity: 0.85;
		z-index: 0;
	}

	.nav--scrolled {
		border-bottom-color: rgba(255, 255, 255, 0.14);
		background: linear-gradient(180deg, rgba(8, 22, 48, 0.97) 0%, rgba(11, 29, 58, 0.96) 100%);
		box-shadow:
			0 1px 0 rgba(15, 164, 175, 0.12),
			0 18px 40px -12px rgba(0, 0, 0, 0.45),
			0 0 0 1px rgba(0, 0, 0, 0.15) inset;
		backdrop-filter: blur(28px) saturate(1.35);
		-webkit-backdrop-filter: blur(28px) saturate(1.35);
	}

	.nav--mobile-open {
		background: rgba(8, 20, 42, 0.98);
		border-bottom-color: rgba(255, 255, 255, 0.12);
	}

	.nav__inner {
		position: relative;
		z-index: 1;
		max-width: var(--container-max);
		margin: 0 auto;
		display: flex;
		align-items: center;
		justify-content: space-between;
		min-height: 4rem;
		padding: 0 1rem;
		transition: min-height 0.45s cubic-bezier(0.22, 1, 0.36, 1);
	}

	.nav--scrolled .nav__inner {
		min-height: 3.5rem;
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
		z-index: calc(var(--z-10) + 1);
		display: flex;
		align-items: center;
		gap: 0.125rem;
		font-family: var(--font-heading);
		font-size: 1.25rem;
		font-weight: var(--w-bold);
		letter-spacing: -0.025em;
		transition:
			transform 0.35s cubic-bezier(0.22, 1, 0.36, 1),
			opacity 0.25s ease;
	}

	.nav__logo:hover {
		transform: translateY(-1px);
	}

	.nav__logo:focus-visible {
		outline: 2px solid var(--color-teal);
		outline-offset: 4px;
		border-radius: var(--radius-sm);
	}

	.nav__logo-brand {
		color: var(--color-white);
		transition: color 0.25s ease;
	}

	.nav__logo:hover .nav__logo-brand {
		color: rgba(255, 255, 255, 0.95);
	}

	.nav__logo-accent {
		color: var(--color-teal-light);
		transition: color 0.25s ease;
	}

	.nav__logo:hover .nav__logo-accent {
		color: #5eead4;
	}

	/* ---- Desktop: centered pill ---- */
	.nav__desktop {
		display: none;
		align-items: center;
	}

	@media (min-width: 768px) {
		.nav__desktop {
			display: flex;
			position: absolute;
			left: 50%;
			top: 50%;
			transform: translate(-50%, -50%);
			z-index: calc(var(--z-10) + 1);
		}
	}

	.nav__pill {
		display: flex;
		align-items: center;
		gap: 0.125rem;
		padding: 0.2rem 0.35rem;
		border-radius: var(--radius-full);
		border: 1px solid rgba(255, 255, 255, 0.08);
		background: rgba(255, 255, 255, 0.04);
		box-shadow: 0 1px 0 rgba(255, 255, 255, 0.04) inset;
	}

	.nav__link {
		position: relative;
		color: var(--color-grey-300);
		border: none;
		border-radius: var(--radius-full);
		padding: 0.45rem 0.85rem;
		font-size: var(--fs-sm);
		font-weight: var(--w-medium);
		font-family: inherit;
		cursor: pointer;
		text-decoration: none;
		transition:
			color 0.22s ease,
			background 0.22s ease,
			transform 0.22s ease;
	}

	.nav__link::after {
		content: '';
		position: absolute;
		left: 50%;
		bottom: 0.28rem;
		width: 0;
		height: 2px;
		border-radius: 2px;
		background: linear-gradient(90deg, var(--color-teal), var(--color-teal-light));
		transform: translateX(-50%);
		transition: width 0.28s cubic-bezier(0.22, 1, 0.36, 1);
		opacity: 0.95;
	}

	.nav__link:hover {
		background-color: rgba(255, 255, 255, 0.06);
		color: var(--color-white);
	}

	.nav__link:hover::after {
		width: calc(100% - 1.5rem);
	}

	.nav__link:focus-visible {
		outline: 2px solid rgba(15, 164, 175, 0.6);
		outline-offset: 2px;
	}

	.nav__link--dropdown {
		display: inline-flex;
		align-items: center;
		gap: 0.35rem;
	}

	.nav__link--active {
		background-color: rgba(15, 164, 175, 0.12);
		color: var(--color-white);
	}

	.nav__link--active::after {
		width: calc(100% - 1.5rem);
	}

	/* ---- Dropdown ---- */
	.nav__dropdown {
		position: relative;
	}

	:global(.nav__caret) {
		transition: transform 0.32s cubic-bezier(0.22, 1, 0.36, 1) !important;
		opacity: 0.85;
	}

	:global(.nav__caret--open) {
		transform: rotate(180deg) !important;
	}

	.dropdown-panel {
		position: absolute;
		top: 100%;
		right: 0;
		margin-top: 0.65rem;
		width: min(340px, calc(100vw - 2rem));
		transform-origin: top right;
		overflow: hidden;
		border-radius: var(--radius-2xl);
		border: 1px solid rgba(255, 255, 255, 0.12);
		background: linear-gradient(165deg, rgba(14, 32, 62, 0.98) 0%, rgba(11, 29, 58, 0.97) 100%);
		box-shadow:
			0 24px 48px -12px rgba(0, 0, 0, 0.55),
			0 0 0 1px rgba(0, 0, 0, 0.2) inset,
			0 0 60px -20px rgba(15, 164, 175, 0.2);
		backdrop-filter: blur(28px) saturate(1.2);
		-webkit-backdrop-filter: blur(28px) saturate(1.2);
	}

	.dropdown-panel::before {
		content: '';
		position: absolute;
		top: 0;
		left: 0;
		right: 0;
		height: 3px;
		background: linear-gradient(
			90deg,
			transparent,
			var(--color-teal) 30%,
			var(--color-teal-light) 70%,
			transparent
		);
		opacity: 0.85;
		pointer-events: none;
	}

	.dropdown-panel__inner {
		padding: 0.75rem;
		padding-top: 0.85rem;
	}

	.dropdown-item {
		display: flex;
		align-items: flex-start;
		gap: 1rem;
		border-radius: var(--radius-xl);
		padding: 1rem;
		transition:
			background 0.22s ease,
			transform 0.22s ease;
	}

	@media (prefers-reduced-motion: no-preference) {
		.dropdown-item {
			animation: nav-dropdown-row 0.45s cubic-bezier(0.22, 1, 0.36, 1) backwards;
		}

		.dropdown-item:nth-child(1) {
			animation-delay: 0.02s;
		}
		.dropdown-item:nth-child(2) {
			animation-delay: 0.06s;
		}
		.dropdown-item:nth-child(3) {
			animation-delay: 0.1s;
		}
		.dropdown-item:nth-child(4) {
			animation-delay: 0.14s;
		}
	}

	@keyframes nav-dropdown-row {
		from {
			opacity: 0;
			transform: translateY(-6px);
		}
		to {
			opacity: 1;
			transform: translateY(0);
		}
	}

	.dropdown-item:hover {
		background-color: rgba(255, 255, 255, 0.07);
	}

	.dropdown-item__icon {
		display: flex;
		align-items: center;
		justify-content: center;
		width: 2.75rem;
		height: 2.75rem;
		flex-shrink: 0;
		border-radius: var(--radius-xl);
		transition: transform 0.32s cubic-bezier(0.22, 1, 0.36, 1);
		box-shadow: 0 4px 12px rgba(0, 0, 0, 0.2);
	}

	.dropdown-item:hover .dropdown-item__icon {
		transform: scale(1.06) rotate(-2deg);
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
		line-clamp: 1;
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
		transition:
			color 0.22s ease,
			transform 0.28s cubic-bezier(0.22, 1, 0.36, 1) !important;
	}

	.dropdown-item:hover :global(.dropdown-item__arrow) {
		color: var(--color-teal-light) !important;
		transform: translateX(4px) !important;
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
		transition:
			background 0.22s ease,
			color 0.22s ease,
			transform 0.22s ease;
	}

	.dropdown-panel__view-all:hover {
		background-color: rgba(15, 164, 175, 0.12);
		color: var(--color-white);
	}

	.dropdown-panel__view-all:focus-visible {
		outline: 2px solid var(--color-teal);
		outline-offset: 2px;
	}

	/* ---- Right section ---- */
	.nav__right {
		position: relative;
		z-index: calc(var(--z-10) + 1);
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
		width: 2.625rem;
		height: 2.625rem;
		border-radius: var(--radius-lg);
		border: 1px solid rgba(255, 255, 255, 0.08);
		color: var(--color-white);
		background: rgba(255, 255, 255, 0.04);
		transition:
			background 0.25s ease,
			border-color 0.25s ease,
			box-shadow 0.25s ease,
			transform 0.25s ease;
	}

	.nav__hamburger:hover {
		background: rgba(15, 164, 175, 0.12);
		border-color: rgba(15, 164, 175, 0.35);
		box-shadow: 0 0 20px rgba(15, 164, 175, 0.15);
	}

	.nav__hamburger:focus-visible {
		outline: 2px solid var(--color-teal);
		outline-offset: 3px;
	}

	.nav__hamburger-icons {
		display: flex;
		align-items: center;
		justify-content: center;
	}

	:global(.nav__hamburger-svg) {
		transition: transform 0.35s cubic-bezier(0.22, 1, 0.36, 1);
	}

	.nav__hamburger:active :global(.nav__hamburger-svg) {
		transform: scale(0.92);
	}

	@media (min-width: 768px) {
		.nav__hamburger {
			display: none;
		}
	}

	/* ---- Mobile menu ---- */
	.mobile-menu {
		position: relative;
		background: linear-gradient(180deg, rgba(11, 29, 58, 0.99) 0%, rgba(8, 20, 42, 0.98) 100%);
		border-top: 1px solid rgba(255, 255, 255, 0.1);
		box-shadow: inset 0 1px 0 rgba(255, 255, 255, 0.04);
		backdrop-filter: blur(24px);
		-webkit-backdrop-filter: blur(24px);
	}

	@media (min-width: 768px) {
		.mobile-menu {
			display: none;
		}
	}

	.mobile-menu__inner {
		max-width: var(--container-max);
		margin: 0 auto;
		padding: 1rem 1rem 1.75rem;
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
		gap: 0.35rem;
	}

	.mobile-course-item {
		display: flex;
		align-items: center;
		gap: 0.75rem;
		border-radius: var(--radius-xl);
		padding: 0.75rem;
		border: 1px solid transparent;
		transition:
			background 0.22s ease,
			border-color 0.22s ease,
			transform 0.22s ease;
	}

	.mobile-course-item:hover {
		background-color: rgba(255, 255, 255, 0.06);
		border-color: rgba(255, 255, 255, 0.06);
	}

	.mobile-course-item:active {
		transform: scale(0.99);
	}

	.mobile-course-item__icon {
		display: flex;
		align-items: center;
		justify-content: center;
		width: 2.5rem;
		height: 2.5rem;
		flex-shrink: 0;
		border-radius: var(--radius-lg);
		box-shadow: 0 4px 10px rgba(0, 0, 0, 0.25);
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
		transition: transform 0.22s ease !important;
	}

	.mobile-course-item:hover :global(.mobile-course-item__arrow) {
		transform: translateX(3px) !important;
		color: var(--color-teal-light) !important;
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
		transition:
			background 0.22s ease,
			color 0.22s ease,
			padding-left 0.22s ease;
	}

	.mobile-menu__link:hover {
		background-color: rgba(255, 255, 255, 0.06);
		color: var(--color-white);
		padding-left: 0.95rem;
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
		color: var(--color-navy);
		background: linear-gradient(135deg, var(--color-teal) 0%, var(--color-teal-light) 100%);
		box-shadow:
			var(--shadow-lg),
			0 4px 20px rgba(15, 164, 175, 0.35);
		transition:
			transform 0.22s ease,
			box-shadow 0.22s ease,
			filter 0.22s ease;
	}

	.mobile-menu__cta-btn:hover {
		filter: brightness(1.08);
		box-shadow:
			var(--shadow-xl),
			0 8px 28px rgba(15, 164, 175, 0.4);
	}

	.mobile-menu__cta-btn:active {
		transform: scale(0.98);
	}

	@media (prefers-reduced-motion: reduce) {
		.nav__logo,
		.nav__link,
		.nav__hamburger,
		.dropdown-item,
		.mobile-course-item,
		.mobile-menu__cta-btn {
			transition-duration: 0.01ms !important;
		}

		.dropdown-item {
			animation: none !important;
		}
	}
</style>
