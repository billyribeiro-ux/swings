<script lang="ts">
	import { browser } from '$app/environment';
	import { onMount } from 'svelte';
	import { fade } from 'svelte/transition';
	import { quintOut, cubicOut } from 'svelte/easing';
	import { courses } from '$lib/data/courses';
	import EnvelopeSimple from 'phosphor-svelte/lib/EnvelopeSimple';
	import TrendUp from 'phosphor-svelte/lib/TrendUp';
	import ArrowUpRight from 'phosphor-svelte/lib/ArrowUpRight';

	const currentYear = new Date().getFullYear();

	let root: HTMLElement | undefined = $state();
	/** When true, optional motion layers (decorative) may run */
	let motionOk = $state(false);
	/** Footer entered viewport — drives CSS stagger + decorative transitions */
	let inView = $state(false);

	onMount(() => {
		if (!browser) return;
		motionOk = !window.matchMedia('(prefers-reduced-motion: reduce)').matches;
		if (!motionOk) {
			inView = true;
			return;
		}
		const el = root;
		if (!el) {
			inView = true;
			return;
		}
		const io = new IntersectionObserver(
			(entries) => {
				const hit = entries.some((e) => e.isIntersecting);
				if (hit) {
					inView = true;
					io.disconnect();
				}
			},
			{ rootMargin: '12% 0px -8%', threshold: 0.05 }
		);
		io.observe(el);
		return () => io.disconnect();
	});
</script>

<footer
	bind:this={root}
	id="site-footer"
	class="footer"
	class:footer--in-view={inView}
	data-motion={motionOk ? 'on' : 'off'}
>
	<!-- Ambient layers (decorative) -->
	<div class="footer__bg" aria-hidden="true">
		<div class="footer__bg-gradient"></div>
		<div class="footer__bg-grid"></div>
		{#if inView && motionOk}
			<div class="footer__glow footer__glow--a" in:fade={{ duration: 900, easing: cubicOut }}></div>
			<div class="footer__glow footer__glow--b" in:fade={{ duration: 1100, delay: 80, easing: cubicOut }}></div>
		{/if}
	</div>

	<div class="footer__container">
		<div class="footer__top-line" aria-hidden="true">
			{#if inView && motionOk}
				<span class="footer__top-line-fill" in:fade={{ duration: 700, easing: quintOut }}></span>
			{:else}
				<span class="footer__top-line-fill footer__top-line-fill--static"></span>
			{/if}
		</div>

		<div class="footer__grid">
			<!-- Brand -->
			<div class="footer__col footer__col--brand">
				<div class="footer__brand-inner">
					<a href="/" class="footer__logo">
						<span class="footer__logo-brand">Explosive</span>
						<span class="footer__logo-accent">Swings</span>
					</a>
					<p class="footer__tagline">
						Weekly options watchlists and structured trading courses, built by Billy Ribeiro.
					</p>
				</div>
			</div>

			<!-- Courses -->
			<nav class="footer__col" aria-labelledby="footer-courses-heading">
				<h4 id="footer-courses-heading" class="footer__heading">Courses</h4>
				<ul class="footer__list">
					{#each courses as course, i (course.id)}
						<li
							class="footer__li"
							style={motionOk && inView
								? `--stagger: ${80 + i * 42}ms`
								: undefined}
						>
							<a href="/courses/{course.slug}" class="footer__link">
								<span class="footer__link-text">{course.title}</span>
								<ArrowUpRight class="footer__link-icon" size={14} weight="bold" aria-hidden="true" />
							</a>
						</li>
					{/each}
					<li
						class="footer__li footer__li--cta"
						style={motionOk && inView ? '--stagger: 420ms' : undefined}
					>
						<a href="/courses" class="footer__link footer__link--emphasis">
							<span class="footer__link-text">View all courses</span>
							<ArrowUpRight class="footer__link-icon" size={14} weight="bold" aria-hidden="true" />
						</a>
					</li>
				</ul>
			</nav>

			<!-- Company -->
			<nav class="footer__col" aria-labelledby="footer-company-heading">
				<h4 id="footer-company-heading" class="footer__heading">Explore</h4>
				<ul class="footer__list">
					<li class="footer__li" style={motionOk && inView ? '--stagger: 100ms' : undefined}>
						<a href="/blog" class="footer__link">
							<span class="footer__link-text">Blog</span>
						</a>
					</li>
					<li class="footer__li" style={motionOk && inView ? '--stagger: 145ms' : undefined}>
						<a href="/#pricing" class="footer__link">
							<span class="footer__link-text">Pricing</span>
						</a>
					</li>
					<li class="footer__li" style={motionOk && inView ? '--stagger: 190ms' : undefined}>
						<a href="/terms" class="footer__link">
							<span class="footer__link-text">Terms</span>
						</a>
					</li>
					<li class="footer__li" style={motionOk && inView ? '--stagger: 235ms' : undefined}>
						<a href="/privacy" class="footer__link">
							<span class="footer__link-text">Privacy</span>
						</a>
					</li>
				</ul>
			</nav>

			<!-- Contact -->
			<div class="footer__col footer__col--contact">
				<h4 class="footer__heading">Contact</h4>
				<a href="mailto:support@explosiveswings.com" class="footer__contact-card">
					<span class="footer__contact-icon-wrap" aria-hidden="true">
						<EnvelopeSimple size={20} weight="bold" />
					</span>
					<span class="footer__contact-body">
						<span class="footer__contact-label">Email us</span>
						<span class="footer__contact-email">support@explosiveswings.com</span>
					</span>
				</a>
			</div>
		</div>

		<div class="footer__bottom">
			<div class="footer__bottom-inner">
				<p class="footer__copyright">© {currentYear} Explosive Swings. All rights reserved.</p>
				<div class="footer__disclaimer">
					<TrendUp size={14} weight="bold" class="footer__disclaimer-icon" aria-hidden="true" />
					<span>Trading involves risk. Past performance ≠ future results.</span>
				</div>
			</div>
		</div>
	</div>
</footer>

<style>
	.footer {
		position: relative;
		isolation: isolate;
		overflow: hidden;
		background-color: var(--color-navy);
		color: var(--color-grey-300);
		padding: clamp(2.5rem, 6vw, 4rem) 0 clamp(1.75rem, 4vw, 2.25rem);
	}

	.footer__bg {
		position: absolute;
		inset: 0;
		z-index: 0;
		pointer-events: none;
	}

	.footer__bg-gradient {
		position: absolute;
		inset: 0;
		background:
			radial-gradient(ellipse 80% 55% at 50% -20%, rgba(15, 164, 175, 0.18), transparent 55%),
			radial-gradient(ellipse 60% 40% at 100% 100%, rgba(212, 168, 67, 0.06), transparent 45%),
			linear-gradient(180deg, rgba(19, 43, 80, 0.65) 0%, var(--color-navy) 38%);
	}

	.footer__bg-grid {
		position: absolute;
		inset: 0;
		opacity: 0.22;
		background-image:
			linear-gradient(rgba(255, 255, 255, 0.04) 1px, transparent 1px),
			linear-gradient(90deg, rgba(255, 255, 255, 0.04) 1px, transparent 1px);
		background-size: 48px 48px;
		mask-image: linear-gradient(180deg, black 0%, transparent 85%);
	}

	.footer__glow {
		position: absolute;
		border-radius: 50%;
		filter: blur(64px);
		opacity: 0.45;
	}

	.footer__glow--a {
		width: min(55vw, 420px);
		height: min(55vw, 420px);
		top: -18%;
		left: -8%;
		background: radial-gradient(circle, rgba(15, 164, 175, 0.35) 0%, transparent 70%);
	}

	.footer__glow--b {
		width: min(45vw, 360px);
		height: min(45vw, 360px);
		bottom: -25%;
		right: -12%;
		background: radial-gradient(circle, rgba(26, 58, 107, 0.55) 0%, transparent 72%);
	}

	.footer__container {
		position: relative;
		z-index: 1;
		max-width: var(--container-max);
		margin: 0 auto;
		padding: 0 clamp(1rem, 4vw, 2rem);
	}

	.footer__top-line {
		position: relative;
		height: 2px;
		margin-bottom: clamp(1.75rem, 4vw, 2.5rem);
		border-radius: var(--radius-full);
		background: rgba(255, 255, 255, 0.06);
		overflow: hidden;
	}

	.footer__top-line-fill {
		position: absolute;
		inset: 0;
		border-radius: inherit;
		background: linear-gradient(
			90deg,
			transparent 0%,
			var(--color-teal) 35%,
			var(--color-gold-light) 65%,
			transparent 100%
		);
		background-size: 200% 100%;
		animation: footer-shimmer 4.5s ease-in-out infinite;
	}

	.footer__top-line-fill--static {
		animation: none;
		opacity: 0.85;
		background-position: 50% 0;
	}

	@keyframes footer-shimmer {
		0%,
		100% {
			background-position: 0% 50%;
		}
		50% {
			background-position: 100% 50%;
		}
	}

	@media (prefers-reduced-motion: reduce) {
		.footer__top-line-fill {
			animation: none;
			opacity: 0.9;
		}
	}

	.footer__grid {
		display: grid;
		gap: clamp(2rem, 5vw, 3rem);
		margin-bottom: clamp(2rem, 5vw, 2.75rem);
	}

	@media (min-width: 640px) {
		.footer__grid {
			grid-template-columns: repeat(2, minmax(0, 1fr));
			align-items: start;
		}

		.footer__col--brand {
			grid-column: span 2;
		}
	}

	@media (min-width: 1024px) {
		.footer__grid {
			grid-template-columns: minmax(220px, 1.15fr) repeat(3, minmax(0, 1fr));
			gap: 2rem 2.5rem;
		}

		.footer__col--brand {
			grid-column: auto;
		}
	}

	.footer__logo {
		display: inline-flex;
		align-items: center;
		gap: 0.125rem;
		font-family: var(--font-heading);
		font-size: clamp(1.125rem, 2.5vw, 1.25rem);
		font-weight: var(--w-bold);
		letter-spacing: var(--ls-tight);
		margin-bottom: 0.75rem;
		transition: transform var(--duration-300) var(--ease-out);
	}

	.footer__logo:hover {
		transform: translateY(-2px);
	}

	.footer__logo-brand {
		color: var(--color-white);
	}
	.footer__logo-accent {
		color: var(--color-teal-light);
	}

	.footer__tagline {
		color: var(--color-grey-500);
		max-width: 22rem;
		font-size: var(--fs-xs);
		line-height: var(--lh-relaxed);
	}

	.footer[data-motion='on'].footer--in-view .footer__brand-inner {
		animation: footer-li-in 0.6s cubic-bezier(0.22, 1, 0.36, 1) both;
		animation-delay: 0ms;
	}

	@media (prefers-reduced-motion: reduce) {
		.footer[data-motion='on'].footer--in-view .footer__brand-inner {
			animation: none;
		}
	}

	.footer__heading {
		color: var(--color-grey-400);
		margin: 0 0 1rem;
		font-size: var(--fs-2xs);
		font-weight: var(--w-semibold);
		letter-spacing: 0.14em;
		text-transform: uppercase;
	}

	.footer__list {
		list-style: none;
		margin: 0;
		padding: 0;
		display: flex;
		flex-direction: column;
		gap: 0.25rem;
	}

	.footer__li {
		opacity: 1;
		transform: none;
	}

	.footer[data-motion='on'].footer--in-view .footer__li {
		animation: footer-li-in 0.55s cubic-bezier(0.22, 1, 0.36, 1) both;
		animation-delay: var(--stagger, 0ms);
	}

	@keyframes footer-li-in {
		from {
			opacity: 0;
			transform: translateY(10px);
		}
		to {
			opacity: 1;
			transform: translateY(0);
		}
	}

	@media (prefers-reduced-motion: reduce) {
		.footer[data-motion='on'].footer--in-view .footer__li {
			animation: none;
		}
	}

	.footer__link {
		display: inline-flex;
		align-items: center;
		justify-content: flex-start;
		gap: 0.35rem;
		min-height: 44px;
		padding: 0.35rem 0;
		color: var(--color-grey-300);
		font-size: var(--fs-sm);
		text-decoration: none;
		border-radius: var(--radius-md);
		transition:
			color var(--duration-200) var(--ease-out),
			transform var(--duration-200) var(--ease-out);
	}

	@media (min-width: 1024px) {
		.footer__link {
			min-height: 0;
			padding: 0.2rem 0;
		}
	}

	.footer__link-text {
		position: relative;
	}

	.footer__link-text::after {
		content: '';
		position: absolute;
		left: 0;
		bottom: -2px;
		width: 100%;
		height: 1px;
		background: linear-gradient(90deg, var(--color-teal-light), transparent);
		transform: scaleX(0);
		transform-origin: left;
		transition: transform var(--duration-300) var(--ease-out);
	}

	.footer__link:hover .footer__link-text::after,
	.footer__link:focus-visible .footer__link-text::after {
		transform: scaleX(1);
	}

	.footer__link:hover,
	.footer__link:focus-visible {
		color: var(--color-teal-light);
	}

	.footer__link:focus-visible {
		outline: 2px solid var(--color-teal);
		outline-offset: 3px;
	}

	:global(.footer__link-icon) {
		flex-shrink: 0;
		opacity: 0.55;
		transition:
			opacity var(--duration-200) var(--ease-out),
			transform var(--duration-200) var(--ease-out);
	}

	.footer__link:hover :global(.footer__link-icon) {
		opacity: 1;
		transform: translate(2px, -2px);
	}

	.footer__link--emphasis {
		color: var(--color-teal-light);
		font-weight: var(--w-medium);
	}

	.footer__link--emphasis .footer__link-text::after {
		background: linear-gradient(90deg, var(--color-gold-light), transparent);
	}

	.footer__col--contact {
		display: flex;
		flex-direction: column;
	}

	.footer__contact-card {
		display: flex;
		align-items: flex-start;
		gap: 0.75rem;
		padding: 0.9rem 1rem;
		margin-top: 0.15rem;
		border-radius: var(--radius-xl);
		background: rgba(255, 255, 255, 0.04);
		border: 1px solid rgba(255, 255, 255, 0.08);
		text-decoration: none;
		color: inherit;
		transition:
			background var(--duration-300) var(--ease-out),
			border-color var(--duration-300) var(--ease-out),
			transform var(--duration-300) var(--ease-out),
			box-shadow var(--duration-300) var(--ease-out);
	}

	.footer__contact-card:hover,
	.footer__contact-card:focus-visible {
		background: rgba(15, 164, 175, 0.1);
		border-color: rgba(15, 164, 175, 0.35);
		transform: translateY(-2px);
		box-shadow: 0 12px 40px -20px rgba(0, 0, 0, 0.45);
	}

	.footer__contact-card:focus-visible {
		outline: 2px solid var(--color-teal-light);
		outline-offset: 2px;
	}

	.footer__contact-icon-wrap {
		display: flex;
		align-items: center;
		justify-content: center;
		width: 2.5rem;
		height: 2.5rem;
		border-radius: var(--radius-lg);
		background: rgba(15, 164, 175, 0.15);
		color: var(--color-teal-light);
		flex-shrink: 0;
	}

	.footer__contact-body {
		display: flex;
		flex-direction: column;
		gap: 0.15rem;
		min-width: 0;
	}

	.footer__contact-label {
		font-size: var(--fs-2xs);
		text-transform: uppercase;
		letter-spacing: 0.08em;
		color: var(--color-grey-500);
		font-weight: var(--w-semibold);
	}

	.footer__contact-email {
		font-size: var(--fs-sm);
		color: var(--color-grey-200);
		word-break: break-word;
	}

	.footer__bottom {
		border-top: 1px solid rgba(255, 255, 255, 0.08);
		padding-top: clamp(1.25rem, 3vw, 1.75rem);
	}

	.footer__bottom-inner {
		display: flex;
		flex-direction: column;
		align-items: flex-start;
		gap: 1rem;
	}

	@media (min-width: 640px) {
		.footer__bottom-inner {
			flex-direction: row;
			flex-wrap: wrap;
			align-items: center;
			justify-content: space-between;
			gap: 1rem 1.5rem;
		}
	}

	.footer__copyright {
		margin: 0;
		color: var(--color-grey-500);
		font-size: var(--fs-xs);
	}

	.footer__disclaimer {
		display: flex;
		align-items: flex-start;
		gap: 0.4rem;
		color: var(--color-grey-600);
		font-size: var(--fs-xs);
		line-height: 1.5;
		max-width: 42ch;
	}

	:global(.footer__disclaimer-icon) {
		flex-shrink: 0;
		margin-top: 0.1rem;
		color: var(--color-grey-500) !important;
	}
</style>
