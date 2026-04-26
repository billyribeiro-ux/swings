<script lang="ts">
	import { onMount, type Snippet } from 'svelte';
	import { resolve } from '$app/paths';
	import HouseIcon from 'phosphor-svelte/lib/HouseIcon';
	import CaretRightIcon from 'phosphor-svelte/lib/CaretRightIcon';
	import ScalesIcon from 'phosphor-svelte/lib/ScalesIcon';
	import CalendarBlankIcon from 'phosphor-svelte/lib/CalendarBlankIcon';
	import ClockIcon from 'phosphor-svelte/lib/ClockIcon';
	import ArrowUpIcon from 'phosphor-svelte/lib/ArrowUpIcon';
	import PaperPlaneTiltIcon from 'phosphor-svelte/lib/PaperPlaneTiltIcon';

	export interface LegalSection {
		id: string;
		label: string;
	}

	interface Props {
		breadcrumb: string;
		title: string;
		intro: string;
		lastUpdated: string;
		effectiveDate?: string;
		sections: LegalSection[];
		contactEmail?: string;
		children: Snippet;
	}

	let {
		breadcrumb,
		title,
		intro,
		lastUpdated,
		effectiveDate,
		sections,
		contactEmail = 'support@precisionoptionsignals.com',
		children
	}: Props = $props();

	let activeId = $state<string>('');
	let showBackToTop = $state(false);

	onMount(() => {
		activeId = sections[0]?.id ?? '';

		const targets = sections
			.map((s) => document.getElementById(s.id))
			.filter((el): el is HTMLElement => el !== null);

		const spy = new IntersectionObserver(
			(entries) => {
				// Pick the topmost intersecting section to mark active
				const visible = entries
					.filter((e) => e.isIntersecting)
					.sort((a, b) => a.boundingClientRect.top - b.boundingClientRect.top);
				if (visible[0]) {
					activeId = (visible[0].target as HTMLElement).id;
				}
			},
			{ rootMargin: '-20% 0px -70% 0px', threshold: 0 }
		);
		targets.forEach((el) => spy.observe(el));

		const onScroll = () => {
			showBackToTop = window.scrollY > 640;
		};
		onScroll();
		window.addEventListener('scroll', onScroll, { passive: true });

		return () => {
			spy.disconnect();
			window.removeEventListener('scroll', onScroll);
		};
	});

	function scrollToTop() {
		window.scrollTo({ top: 0, behavior: 'smooth' });
	}
</script>

<div class="legal">
	<div class="legal__bg" aria-hidden="true"></div>

	<div class="legal__hero">
		<nav class="legal__breadcrumb" aria-label="Breadcrumb">
			<a href={resolve('/')} class="legal__crumb legal__crumb--link">
				<HouseIcon size={14} weight="regular" />
				<span>Home</span>
			</a>
			<CaretRightIcon size={12} weight="bold" class="legal__crumb-sep" />
			<span class="legal__crumb legal__crumb--current">Legal</span>
			<CaretRightIcon size={12} weight="bold" class="legal__crumb-sep" />
			<span class="legal__crumb legal__crumb--current">{breadcrumb}</span>
		</nav>

		<div class="legal__eyebrow">
			<ScalesIcon size={14} weight="fill" />
			<span>Legal</span>
		</div>

		<h1 class="legal__title">{title}</h1>
		<p class="legal__intro">{intro}</p>

		<dl class="legal__meta">
			<div class="legal__meta-item">
				<dt>
					<ClockIcon size={14} weight="regular" />
					<span>Last updated</span>
				</dt>
				<dd>{lastUpdated}</dd>
			</div>
			{#if effectiveDate}
				<div class="legal__meta-item">
					<dt>
						<CalendarBlankIcon size={14} weight="regular" />
						<span>Effective</span>
					</dt>
					<dd>{effectiveDate}</dd>
				</div>
			{/if}
		</dl>
	</div>

	<div class="legal__body">
		<aside class="legal__toc" aria-label="On this page">
			<div class="legal__toc-card">
				<p class="legal__toc-heading">On this page</p>
				<ol class="legal__toc-list">
					{#each sections as s, i (s.id)}
						<li>
							<a
								href={`#${s.id}`}
								class="legal__toc-link"
								class:legal__toc-link--active={activeId === s.id}
								aria-current={activeId === s.id ? 'location' : undefined}
							>
								<span class="legal__toc-index"
									>{String(i + 1).padStart(2, '0')}</span
								>
								<span class="legal__toc-label">{s.label}</span>
							</a>
						</li>
					{/each}
				</ol>
			</div>
		</aside>

		<article class="legal__article prose">
			{@render children()}

			<section class="legal__contact-card" aria-labelledby="legal-contact-heading">
				<div class="legal__contact-icon">
					<PaperPlaneTiltIcon size={20} weight="fill" />
				</div>
				<div class="legal__contact-body">
					<h3 id="legal-contact-heading" class="legal__contact-title">Questions?</h3>
					<p class="legal__contact-text">
						Our team is here to help. Reach us at
						<a href={`mailto:${contactEmail}`} class="legal__contact-link"
							>{contactEmail}</a
						>
						and we'll respond within one business day.
					</p>
				</div>
			</section>
		</article>
	</div>

	<button
		type="button"
		class="legal__to-top"
		class:legal__to-top--visible={showBackToTop}
		onclick={scrollToTop}
		aria-label="Back to top"
	>
		<ArrowUpIcon size={18} weight="bold" />
	</button>
</div>

<style>
	/* ── Page shell ─────────────────────────────────────────────────────── */
	.legal {
		position: relative;
		min-height: 100vh;
		padding: clamp(5rem, 9vw, 7.5rem) clamp(1rem, 4vw, 3rem) 6rem;
		background: linear-gradient(180deg, var(--color-navy-deep) 0%, var(--color-navy) 100%);
		isolation: isolate;
		overflow-x: clip;
	}

	.legal__bg {
		position: absolute;
		inset: 0;
		background-image:
			radial-gradient(ellipse 60% 40% at 50% 0%, rgba(15, 164, 175, 0.15), transparent 60%),
			linear-gradient(to right, rgba(255, 255, 255, 0.025) 1px, transparent 1px),
			linear-gradient(to bottom, rgba(255, 255, 255, 0.025) 1px, transparent 1px);
		background-size:
			100% 100%,
			56px 56px,
			56px 56px;
		mask-image: radial-gradient(ellipse 75% 55% at 50% 0%, black 0%, transparent 75%);
		pointer-events: none;
		z-index: -1;
	}

	/* ── Hero ───────────────────────────────────────────────────────────── */
	.legal__hero {
		max-width: 72rem;
		margin: 0 auto clamp(2rem, 5vw, 3.5rem);
		text-align: left;
	}

	.legal__breadcrumb {
		display: inline-flex;
		align-items: center;
		gap: 0.4rem;
		flex-wrap: wrap;
		color: var(--color-grey-500);
		font-size: var(--fs-xs);
		margin-bottom: 1.5rem;
	}

	.legal__crumb {
		display: inline-flex;
		align-items: center;
		gap: 0.35rem;
	}

	.legal__crumb--link {
		color: var(--color-grey-400);
		text-decoration: none;
		transition: color 200ms var(--ease-out);
	}

	.legal__crumb--link:hover {
		color: var(--color-teal-light);
	}

	.legal__crumb--current {
		color: var(--color-grey-300);
	}

	:global(.legal__crumb-sep) {
		color: var(--color-grey-600, rgba(255, 255, 255, 0.2));
		flex-shrink: 0;
	}

	.legal__eyebrow {
		display: inline-flex;
		align-items: center;
		gap: 0.4rem;
		padding: 0.35rem 0.8rem;
		border-radius: var(--radius-full);
		background: rgba(15, 164, 175, 0.12);
		border: 1px solid rgba(15, 164, 175, 0.3);
		color: var(--color-teal-light);
		font-size: 0.72rem;
		font-weight: var(--w-semibold);
		letter-spacing: 0.08em;
		text-transform: uppercase;
		margin-bottom: 1.25rem;
	}

	.legal__title {
		font-family: var(--font-heading);
		font-size: var(--fs-4xl);
		font-weight: var(--w-bold);
		color: var(--color-white);
		letter-spacing: var(--ls-tight);
		line-height: var(--lh-tight);
		margin: 0 0 1rem;
		max-width: 28ch;
	}

	.legal__intro {
		color: var(--color-grey-300);
		font-size: var(--fs-lg);
		line-height: var(--lh-relaxed);
		margin: 0 0 2rem;
		max-width: 58ch;
	}

	.legal__meta {
		display: flex;
		flex-wrap: wrap;
		gap: 0.5rem 0.75rem;
		margin: 0;
		padding: 0;
	}

	.legal__meta-item {
		display: inline-flex;
		align-items: center;
		gap: 0.5rem;
		padding: 0.4rem 0.85rem;
		border-radius: var(--radius-full);
		background: rgba(255, 255, 255, 0.035);
		border: 1px solid rgba(255, 255, 255, 0.08);
	}

	.legal__meta-item dt {
		display: inline-flex;
		align-items: center;
		gap: 0.35rem;
		color: var(--color-grey-500);
		font-size: var(--fs-xs);
		font-weight: var(--w-medium);
	}

	.legal__meta-item dt :global(svg) {
		color: var(--color-grey-400);
	}

	.legal__meta-item dd {
		margin: 0;
		color: var(--color-grey-200);
		font-size: var(--fs-xs);
		font-weight: var(--w-semibold);
		font-variant-numeric: tabular-nums;
	}

	/* ── Body grid ──────────────────────────────────────────────────────── */
	.legal__body {
		max-width: 72rem;
		margin: 0 auto;
		display: grid;
		gap: clamp(2rem, 5vw, 3.5rem);
		grid-template-columns: 1fr;
	}

	@media (min-width: 960px) {
		.legal__body {
			grid-template-columns: minmax(12rem, 16rem) minmax(0, 1fr);
			align-items: start;
		}
	}

	/* ── TOC ────────────────────────────────────────────────────────────── */
	.legal__toc {
		display: none;
	}

	@media (min-width: 960px) {
		.legal__toc {
			display: block;
			position: sticky;
			top: 6rem;
			max-height: calc(100vh - 7rem);
			overflow-y: auto;
			overscroll-behavior: contain;
		}
	}

	.legal__toc-card {
		padding: 1.25rem 1rem;
		border-radius: var(--radius-xl);
		border: 1px solid rgba(255, 255, 255, 0.06);
		background: linear-gradient(165deg, rgba(19, 43, 80, 0.55) 0%, rgba(12, 27, 46, 0.65) 100%);
		backdrop-filter: blur(12px) saturate(1.1);
	}

	.legal__toc-heading {
		margin: 0 0 0.75rem;
		padding: 0 0.4rem;
		color: var(--color-grey-500);
		font-size: 0.7rem;
		font-weight: var(--w-semibold);
		letter-spacing: 0.12em;
		text-transform: uppercase;
	}

	.legal__toc-list {
		list-style: none;
		margin: 0;
		padding: 0;
		display: flex;
		flex-direction: column;
		gap: 2px;
	}

	.legal__toc-link {
		position: relative;
		display: flex;
		align-items: flex-start;
		gap: 0.65rem;
		padding: 0.45rem 0.6rem;
		border-radius: var(--radius-md);
		color: var(--color-grey-400);
		font-size: var(--fs-sm);
		line-height: 1.45;
		text-decoration: none;
		transition:
			color 180ms var(--ease-out),
			background-color 180ms var(--ease-out);
	}

	.legal__toc-link::before {
		content: '';
		position: absolute;
		left: -1rem;
		top: 0.55rem;
		bottom: 0.55rem;
		width: 2px;
		border-radius: 2px;
		background-color: transparent;
		transition: background-color 180ms var(--ease-out);
	}

	.legal__toc-link:hover {
		color: var(--color-white);
		background-color: rgba(255, 255, 255, 0.04);
	}

	.legal__toc-link--active {
		color: var(--color-white);
		background-color: rgba(15, 164, 175, 0.08);
	}

	.legal__toc-link--active::before {
		background-color: var(--color-teal-light);
	}

	.legal__toc-index {
		color: var(--color-grey-500);
		font-variant-numeric: tabular-nums;
		font-size: 0.72rem;
		font-weight: var(--w-semibold);
		letter-spacing: 0.05em;
		margin-top: 0.1rem;
		flex-shrink: 0;
	}

	.legal__toc-link--active .legal__toc-index {
		color: var(--color-teal-light);
	}

	.legal__toc-label {
		flex: 1;
	}

	/* ── Article / prose ────────────────────────────────────────────────── */
	.legal__article {
		max-width: 68ch;
		color: var(--color-grey-300);
		font-size: var(--fs-md);
		line-height: 1.75;
	}

	/* Scoped prose so only consumers who opt in get these styles */
	:global(.legal__article.prose > section) {
		scroll-margin-top: 6rem;
		margin-bottom: clamp(2.25rem, 4vw, 3rem);
	}

	:global(.legal__article.prose > section + section) {
		padding-top: clamp(2rem, 4vw, 2.75rem);
		border-top: 1px solid rgba(255, 255, 255, 0.05);
	}

	:global(.legal__article.prose h2) {
		display: flex;
		align-items: baseline;
		gap: 0.75rem;
		font-family: var(--font-heading);
		font-size: var(--fs-2xl);
		font-weight: var(--w-bold);
		color: var(--color-white);
		letter-spacing: var(--ls-tight);
		line-height: var(--lh-snug);
		margin: 0 0 1rem;
	}

	:global(.legal__article.prose h2::before) {
		content: counter(legal-section, decimal-leading-zero);
		counter-increment: legal-section;
		font-family: var(--font-mono);
		font-size: 0.72rem;
		font-weight: var(--w-semibold);
		color: var(--color-teal-light);
		background: rgba(15, 164, 175, 0.1);
		border: 1px solid rgba(15, 164, 175, 0.25);
		padding: 0.25rem 0.55rem;
		border-radius: var(--radius-full);
		letter-spacing: 0.08em;
		line-height: 1.1;
		flex-shrink: 0;
	}

	:global(.legal__article.prose) {
		counter-reset: legal-section;
	}

	:global(.legal__article.prose h3) {
		font-family: var(--font-heading);
		font-size: var(--fs-lg);
		font-weight: var(--w-semibold);
		color: var(--color-white);
		letter-spacing: var(--ls-tight);
		line-height: var(--lh-snug);
		margin: 2rem 0 0.75rem;
	}

	:global(.legal__article.prose p) {
		margin: 0 0 1.1rem;
		color: var(--color-grey-300);
	}

	:global(.legal__article.prose p:last-child) {
		margin-bottom: 0;
	}

	:global(.legal__article.prose strong) {
		color: var(--color-white);
		font-weight: var(--w-semibold);
	}

	:global(.legal__article.prose a) {
		color: var(--color-teal-light);
		text-decoration: underline;
		text-decoration-color: rgba(15, 164, 175, 0.4);
		text-underline-offset: 3px;
		transition:
			color 180ms var(--ease-out),
			text-decoration-color 180ms var(--ease-out);
	}

	:global(.legal__article.prose a:hover) {
		color: var(--color-white);
		text-decoration-color: currentColor;
	}

	:global(.legal__article.prose code) {
		font-family: var(--font-mono);
		font-size: 0.875em;
		padding: 0.1em 0.4em;
		border-radius: var(--radius-md);
		background: rgba(255, 255, 255, 0.06);
		border: 1px solid rgba(255, 255, 255, 0.08);
		color: var(--color-white);
	}

	:global(.legal__article.prose ul),
	:global(.legal__article.prose ol) {
		margin: 0 0 1.25rem;
		padding-left: 1.5rem;
		color: var(--color-grey-300);
	}

	:global(.legal__article.prose li) {
		margin-bottom: 0.5rem;
		line-height: 1.7;
	}

	:global(.legal__article.prose li::marker) {
		color: var(--color-teal-light);
	}

	:global(.legal__article.prose ul li::marker) {
		content: '— ';
	}

	/* Callout block */
	:global(.legal__article.prose .legal-callout) {
		display: flex;
		gap: 0.85rem;
		padding: 1rem 1.15rem;
		margin: 1.25rem 0;
		border-radius: var(--radius-lg);
		border: 1px solid rgba(212, 168, 67, 0.35);
		background: linear-gradient(
			135deg,
			rgba(212, 168, 67, 0.08) 0%,
			rgba(212, 168, 67, 0.03) 100%
		);
		color: var(--color-grey-200);
		font-size: var(--fs-sm);
		line-height: 1.65;
	}

	:global(.legal__article.prose .legal-callout--info) {
		border-color: rgba(15, 164, 175, 0.35);
		background: linear-gradient(
			135deg,
			rgba(15, 164, 175, 0.08) 0%,
			rgba(15, 164, 175, 0.03) 100%
		);
	}

	:global(.legal__article.prose .legal-callout__icon) {
		flex-shrink: 0;
		width: 1.75rem;
		height: 1.75rem;
		display: inline-flex;
		align-items: center;
		justify-content: center;
		border-radius: var(--radius-full);
		background: rgba(212, 168, 67, 0.15);
		color: #fbd38b;
	}

	:global(.legal__article.prose .legal-callout--info .legal-callout__icon) {
		background: rgba(15, 164, 175, 0.15);
		color: var(--color-teal-light);
	}

	:global(.legal__article.prose .legal-callout__body) {
		flex: 1;
	}

	:global(.legal__article.prose .legal-callout__title) {
		display: block;
		margin: 0 0 0.25rem;
		color: var(--color-white);
		font-size: var(--fs-sm);
		font-weight: var(--w-semibold);
		letter-spacing: 0.01em;
	}

	:global(.legal__article.prose .legal-callout p) {
		margin: 0;
		color: inherit;
	}

	/* ── Contact card ───────────────────────────────────────────────────── */
	.legal__contact-card {
		display: flex;
		gap: 1rem;
		align-items: flex-start;
		margin-top: 3rem;
		padding: 1.5rem;
		border-radius: var(--radius-xl);
		border: 1px solid rgba(255, 255, 255, 0.08);
		background: linear-gradient(
			135deg,
			rgba(15, 164, 175, 0.08) 0%,
			rgba(19, 43, 80, 0.4) 100%
		);
	}

	.legal__contact-icon {
		flex-shrink: 0;
		width: 2.5rem;
		height: 2.5rem;
		display: inline-flex;
		align-items: center;
		justify-content: center;
		border-radius: var(--radius-lg);
		background: rgba(15, 164, 175, 0.15);
		color: var(--color-teal-light);
	}

	.legal__contact-body {
		flex: 1;
		min-width: 0;
	}

	.legal__contact-title {
		margin: 0 0 0.35rem;
		color: var(--color-white);
		font-size: var(--fs-lg);
		font-weight: var(--w-semibold);
		font-family: var(--font-heading);
		letter-spacing: var(--ls-tight);
	}

	.legal__contact-text {
		margin: 0;
		color: var(--color-grey-300);
		font-size: var(--fs-sm);
		line-height: 1.65;
	}

	.legal__contact-link {
		color: var(--color-teal-light);
		text-decoration: underline;
		text-decoration-color: rgba(15, 164, 175, 0.4);
		text-underline-offset: 3px;
		font-weight: var(--w-semibold);
		transition: color 180ms var(--ease-out);
	}

	.legal__contact-link:hover {
		color: var(--color-white);
	}

	/* ── Back to top ────────────────────────────────────────────────────── */
	.legal__to-top {
		position: fixed;
		right: clamp(1rem, 3vw, 2rem);
		bottom: clamp(1rem, 3vw, 2rem);
		width: 2.75rem;
		height: 2.75rem;
		display: inline-flex;
		align-items: center;
		justify-content: center;
		border-radius: var(--radius-full);
		background: linear-gradient(135deg, var(--color-teal-light), var(--color-teal));
		color: var(--color-white);
		border: none;
		cursor: pointer;
		opacity: 0;
		visibility: hidden;
		transform: translateY(8px);
		transition:
			opacity 200ms var(--ease-out),
			transform 200ms var(--ease-out),
			visibility 200ms;
		box-shadow:
			0 1px 0 rgba(255, 255, 255, 0.15) inset,
			0 10px 30px -10px rgba(15, 164, 175, 0.7);
		z-index: var(--z-30);
	}

	.legal__to-top--visible {
		opacity: 1;
		visibility: visible;
		transform: translateY(0);
	}

	.legal__to-top:hover {
		filter: brightness(1.1);
	}

	.legal__to-top:focus-visible {
		outline: 2px solid var(--color-white);
		outline-offset: 3px;
	}

	/* ── Print ──────────────────────────────────────────────────────────── */
	@media print {
		.legal {
			background: white;
			color: black;
			padding: 2rem;
		}

		.legal__bg,
		.legal__to-top,
		.legal__toc {
			display: none !important;
		}

		.legal__body {
			display: block;
		}

		.legal__title,
		.legal__intro,
		:global(.legal__article.prose h2),
		:global(.legal__article.prose h3),
		:global(.legal__article.prose p),
		:global(.legal__article.prose li) {
			color: black !important;
		}

		:global(.legal__article.prose a) {
			color: black;
			text-decoration: underline;
		}
	}
</style>
