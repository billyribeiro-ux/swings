<script lang="ts">
	import { onMount } from 'svelte';
	import { gsap } from 'gsap';
	import { createCinematicCascade, EASE, DURATION } from '$lib/utils/animations';
	import ScrollReveal from '$lib/components/ui/ScrollReveal.svelte';
	import SectionHeader from '$lib/components/ui/SectionHeader.svelte';
	import Seo from '$lib/seo/Seo.svelte';
	import { webPageSchema, buildJsonLd } from '$lib/seo/jsonld';
	import CheckCircle from 'phosphor-svelte/lib/CheckCircle';
	import TrendUp from 'phosphor-svelte/lib/TrendUp';
	import Users from 'phosphor-svelte/lib/Users';
	import ChartLineUp from 'phosphor-svelte/lib/ChartLineUp';
	import Target from 'phosphor-svelte/lib/Target';

	const jsonLd = buildJsonLd([
		webPageSchema({
			path: '/about',
			title: 'About Precision Options Signals - Team, Method, and Trading Philosophy',
			description:
				'Meet the team behind Precision Options Signals and learn the risk-first process we use to build weekly watchlists and options education.',
			speakable: '.about-title, .about-subtitle, .founder-card__name, .founder-card__bio'
		})
	]);

	let heroRef: HTMLElement | undefined = $state();

	onMount(() => {
		if (!heroRef) return;

		const ctx = gsap.context(() => {
			createCinematicCascade(heroRef!, [
				{
					selector: '.about-badge',
					duration: DURATION.fast,
					ease: EASE.snappy,
					y: 20,
					blur: 6,
					scale: 0.9,
					overlap: 0
				},
				{
					selector: '.about-title',
					duration: DURATION.cinematic,
					ease: EASE.cinematic,
					y: 36,
					blur: 10,
					scale: 0.95,
					overlap: 0.6
				},
				{
					selector: '.about-subtitle',
					duration: DURATION.slow,
					ease: EASE.soft,
					y: 28,
					blur: 8,
					scale: 0.98,
					overlap: 0.6
				}
			]);
		}, heroRef as HTMLElement);

		return () => ctx.revert();
	});
</script>

<Seo
	title="About - Precision Options Signals"
	description="Meet the team behind Precision Options Signals and learn the risk-first process we use to build weekly watchlists and options education."
	ogTitle="About Precision Options Signals - Team and Trading Philosophy"
	{jsonLd}
/>

<!-- Hero -->
<section bind:this={heroRef} class="page-hero">
	<div class="page-hero__grid-overlay"></div>

	<div class="page-hero__inner page-hero__inner--center">
		<div class="about-badge page-badge">
			<Users size={18} weight="duotone" color="#15C5D1" />
			<span class="page-badge__text">About Us</span>
		</div>

		<h1 class="about-title page-hero__title">
			Most Trading Services Give You Fish.<br />We Teach You to Read the Water.
		</h1>

		<p class="about-subtitle page-hero__subtitle page-hero__subtitle--wide">
			Founded by Billy Ribeiro and Freddie Ferber, Precision Options Signals was built on a single
			conviction: traders don't fail because of bad alerts -- they fail because no one ever
			explained the <em>why</em> behind the trade.
		</p>
	</div>
</section>

<!-- Story -->
<section class="page-section page-section--white">
	<div class="page-container">
		<ScrollReveal>
			<div class="page-narrow">
				<h2 class="reveal-item page-section__heading page-section__heading--center">
					This Isn't a Signal Service. It's a Methodology.
				</h2>

				<div class="reveal-item page-prose">
					<p>
						Billy brings over a decade of institutional-grade market experience, including his time
						as head trader at ZMC Capital and mentorship under <strong>Mark McGoldrick</strong>,
						Goldman Sachs' former Global Head of Proprietary Trading. Freddie Ferber, a longtime
						student who became a world-class trader in his own right, brings a practitioner's
						perspective -- the kind of clarity that only comes from learning the hard lessons
						firsthand and building real, consistent edge from them.
					</p>

					<p>
						Together, they've built something that didn't exist before: <strong
							>a service where quality replaces volume</strong
						>.
					</p>

					<p>
						Every Sunday night, members receive a curated watchlist of 5-7 high-probability options
						setups -- each one hand-selected and fully structured with entry zones, profit targets,
						and defined stop losses. No noise. No filler. Just setups that meet an
						institutional-grade threshold before they ever reach your inbox.
					</p>

					<p>
						Precision Options Signals serves <strong>18,000+ active traders</strong> across all experience levels,
						from professionals refining their edge to disciplined beginners building their first repeatable
						framework.
					</p>
				</div>
			</div>
		</ScrollReveal>
	</div>
</section>

<!-- What Separates Us -->
<section class="page-section page-section--off-white">
	<div class="page-container">
		<ScrollReveal>
			<SectionHeader
				eyebrow="Our Approach"
				title="What Separates Us from Every Other Alert Service"
				subtitle="Quality over quantity. Education over noise. Methodology over signals."
			/>

			<div class="about-features-grid">
				<div class="reveal-item about-feature-card">
					<div class="about-feature-card__icon">
						<Target size={28} weight="duotone" color="#0FA4AF" />
					</div>
					<h3 class="about-feature-card__title">Fewer, Better Setups</h3>
					<p class="about-feature-card__desc">
						5–7 per week, not 50 per day. Every setup meets institutional-grade criteria before it
						reaches you.
					</p>
				</div>

				<div class="reveal-item about-feature-card">
					<div class="about-feature-card__icon">
						<ChartLineUp size={28} weight="duotone" color="#0FA4AF" />
					</div>
					<h3 class="about-feature-card__title">Full Trade Structure</h3>
					<p class="about-feature-card__desc">
						Entry zones, profit targets, and defined stop losses on every setup. No guesswork.
					</p>
				</div>

				<div class="reveal-item about-feature-card">
					<div class="about-feature-card__icon">
						<TrendUp size={28} weight="duotone" color="#0FA4AF" />
					</div>
					<h3 class="about-feature-card__title">Institutional Pedigree</h3>
					<p class="about-feature-card__desc">
						Built by traders who've operated at the highest levels of the market -- Goldman Sachs,
						ZMC Capital.
					</p>
				</div>

				<div class="reveal-item about-feature-card">
					<div class="about-feature-card__icon">
						<Users size={28} weight="duotone" color="#0FA4AF" />
					</div>
					<h3 class="about-feature-card__title">Education Embedded</h3>
					<p class="about-feature-card__desc">
						You'll understand <em>why</em> the setup qualifies, not just what to click. Learn the methodology.
					</p>
				</div>

				<div class="reveal-item about-feature-card about-feature-card--full">
					<div class="about-feature-card__icon">
						<CheckCircle size={28} weight="duotone" color="#0FA4AF" />
					</div>
					<h3 class="about-feature-card__title">Designed for Real Schedules</h3>
					<p class="about-feature-card__desc">
						Sunday delivery means you're prepared before the week begins, not chasing it. Trade on
						your terms.
					</p>
				</div>
			</div>
		</ScrollReveal>
	</div>
</section>

<!-- Team -->
<section class="page-section page-section--white">
	<div class="page-container">
		<ScrollReveal>
			<h2
				class="reveal-item page-section__heading page-section__heading--center page-section__heading--mb-lg"
			>
				Meet the Founders
			</h2>

			<div class="founders-grid">
				<!-- Billy Ribeiro -->
				<div class="reveal-item founder-card">
					<div class="founder-card__header">
						<div class="founder-card__avatar founder-card__avatar--teal">BR</div>
						<div>
							<h3 class="founder-card__name">Billy Ribeiro</h3>
							<p class="founder-card__role">Co-Founder</p>
						</div>
					</div>

					<div class="founder-card__bio">
						<p>
							Over a decade of institutional-grade market experience, including head trader at ZMC
							Capital. Mentored by Mark McGoldrick, Goldman Sachs' former Global Head of Proprietary
							Trading.
						</p>
						<p>
							Billy's approach: focus on high-probability setups, manage risk religiously, and never
							chase trades. That philosophy is the foundation of Precision Options Signals.
						</p>
					</div>

					<div class="founder-card__tags">
						<span class="founder-tag founder-tag--teal">
							<CheckCircle size={14} weight="fill" />
							10+ Years Trading
						</span>
						<span class="founder-tag founder-tag--teal">
							<CheckCircle size={14} weight="fill" />
							ZMC Capital
						</span>
						<span class="founder-tag founder-tag--teal">
							<CheckCircle size={14} weight="fill" />
							Goldman Sachs Mentored
						</span>
					</div>
				</div>

				<!-- Freddie Ferber -->
				<div class="reveal-item founder-card">
					<div class="founder-card__header">
						<img
							src="/images/freddie-ferber.jpg"
							alt="Freddie Ferber"
							class="founder-card__photo"
							loading="lazy"
							width="64"
							height="64"
						/>
						<div>
							<h3 class="founder-card__name">Freddie Ferber</h3>
							<p class="founder-card__role">Co-Founder</p>
						</div>
					</div>

					<div class="founder-card__bio">
						<p>
							A longtime student who became a world-class trader in his own right. Freddie brings a
							practitioner's perspective -- the kind of clarity that only comes from learning the
							hard lessons firsthand.
						</p>
						<p>
							His focus on building real, consistent edge from experience complements Billy's
							institutional approach, creating a complete methodology for traders at every level.
						</p>
					</div>

					<div class="founder-card__tags">
						<span class="founder-tag founder-tag--gold">
							<CheckCircle size={14} weight="fill" />
							World-Class Trader
						</span>
						<span class="founder-tag founder-tag--gold">
							<CheckCircle size={14} weight="fill" />
							Practitioner's Edge
						</span>
						<span class="founder-tag founder-tag--gold">
							<CheckCircle size={14} weight="fill" />
							Educator
						</span>
					</div>
				</div>
			</div>
		</ScrollReveal>
	</div>
</section>

<!-- CTA -->
<section class="page-section page-section--dark">
	<div class="page-container page-container--center">
		<ScrollReveal>
			<h2 class="reveal-item page-dark-heading">Ready to Join Us?</h2>

			<p class="reveal-item page-dark-subtitle">
				Get weekly watchlists, structured courses, and join a community of traders who are serious
				about results.
			</p>

			<div class="reveal-item">
				<a href="/pricing/monthly" class="page-cta-btn">
					Get Instant Access
					<ChartLineUp size={18} weight="bold" />
				</a>
			</div>
		</ScrollReveal>
	</div>
</section>

<style>
	/* ---- Shared page patterns ---- */
	.page-hero {
		position: relative;
		overflow: hidden;
		padding-top: 4rem;
		background: linear-gradient(
			to bottom right,
			var(--color-navy),
			var(--color-navy-mid),
			var(--color-deep-blue)
		);
	}

	.page-hero__grid-overlay {
		position: absolute;
		inset: 0;
		opacity: 0.02;
		background-image:
			linear-gradient(to right, white 1px, transparent 1px),
			linear-gradient(to bottom, white 1px, transparent 1px);
		background-size: 60px 60px;
	}

	.page-hero__inner {
		position: relative;
		z-index: var(--z-10);
		max-width: var(--container-max);
		margin: 0 auto;
		padding: 5rem 1rem;
	}

	@media (min-width: 640px) {
		.page-hero__inner {
			padding: 5rem 1.5rem;
		}
	}
	@media (min-width: 1024px) {
		.page-hero__inner {
			padding: 7rem 2rem;
		}
	}

	.page-hero__inner--center {
		text-align: center;
	}

	.page-badge {
		display: inline-flex;
		align-items: center;
		gap: 0.5rem;
		border-radius: var(--radius-full);
		border: 1px solid rgba(15, 164, 175, 0.3);
		background-color: rgba(15, 164, 175, 0.1);
		padding: 0.5rem 1rem;
		margin-bottom: 1.5rem;
	}

	.page-badge__text {
		color: var(--color-teal-light);
		font-size: var(--fs-xs);
		font-weight: var(--w-semibold);
		letter-spacing: 0.05em;
		text-transform: uppercase;
	}

	.page-hero__title {
		font-family: var(--font-heading);
		font-size: var(--fs-3xl);
		font-weight: var(--w-bold);
		color: var(--color-white);
		line-height: 1.15;
		margin-bottom: 1.5rem;
	}

	@media (min-width: 640px) {
		.page-hero__title {
			font-size: var(--fs-4xl);
		}
	}
	@media (min-width: 768px) {
		.page-hero__title {
			font-size: clamp(2.5rem, 5vw, 3rem);
		}
	}
	@media (min-width: 1024px) {
		.page-hero__title {
			font-size: clamp(3rem, 5vw, 3.75rem);
		}
	}

	.page-hero__subtitle {
		color: var(--color-grey-300);
		font-size: 1rem;
		line-height: 1.65;
		max-width: 42rem;
		margin: 0 auto;
	}

	@media (min-width: 640px) {
		.page-hero__subtitle {
			font-size: var(--fs-lg);
		}
	}
	@media (min-width: 1024px) {
		.page-hero__subtitle {
			font-size: var(--fs-xl);
		}
	}

	.page-hero__subtitle--wide {
		max-width: 48rem;
	}

	/* ---- Page sections ---- */
	.page-section {
		padding: 4rem 0;
	}

	@media (min-width: 640px) {
		.page-section {
			padding: 5rem 0;
		}
	}
	@media (min-width: 1024px) {
		.page-section {
			padding: 7rem 0;
		}
	}

	.page-section--white {
		background-color: var(--color-white);
	}
	.page-section--off-white {
		background-color: var(--color-off-white);
	}
	.page-section--dark {
		background: linear-gradient(
			to bottom right,
			var(--color-navy),
			var(--color-navy-mid),
			var(--color-deep-blue)
		);
	}

	.page-container {
		max-width: var(--container-max);
		margin: 0 auto;
		padding: 0 1rem;
	}

	@media (min-width: 640px) {
		.page-container {
			padding: 0 1.5rem;
		}
	}
	@media (min-width: 1024px) {
		.page-container {
			padding: 0 2rem;
		}
	}

	.page-container--center {
		text-align: center;
	}

	.page-narrow {
		max-width: 48rem;
		margin: 0 auto;
	}

	.page-section__heading {
		color: var(--color-navy);
		font-family: var(--font-heading);
		font-size: var(--fs-2xl);
		font-weight: var(--w-bold);
		margin-bottom: 2rem;
	}

	@media (min-width: 640px) {
		.page-section__heading {
			font-size: var(--fs-3xl);
		}
	}
	@media (min-width: 768px) {
		.page-section__heading {
			font-size: var(--fs-4xl);
		}
	}

	.page-section__heading--center {
		text-align: center;
	}
	.page-section__heading--mb-lg {
		margin-bottom: 3rem;
	}

	.page-prose {
		color: var(--color-grey-700);
		line-height: 1.65;
		display: flex;
		flex-direction: column;
		gap: 1.5rem;
	}

	/* ---- About feature cards ---- */
	.about-features-grid {
		max-width: 56rem;
		margin: 0 auto;
		display: grid;
		gap: 1.5rem;
	}

	@media (min-width: 640px) {
		.about-features-grid {
			gap: 2rem;
		}
	}
	@media (min-width: 768px) {
		.about-features-grid {
			grid-template-columns: repeat(2, 1fr);
		}
	}

	.about-feature-card {
		display: flex;
		flex-direction: column;
		border-radius: var(--radius-2xl);
		background-color: var(--color-white);
		padding: 1.5rem;
		outline: 1px solid rgba(216, 220, 228, 0.8);
		outline-offset: -1px;
	}

	@media (min-width: 640px) {
		.about-feature-card {
			padding: 2rem;
		}
	}

	@media (min-width: 768px) {
		.about-feature-card--full {
			grid-column: span 2;
		}
	}

	.about-feature-card__icon {
		display: flex;
		align-items: center;
		justify-content: center;
		width: 3.5rem;
		height: 3.5rem;
		border-radius: var(--radius-2xl);
		background-color: rgba(15, 164, 175, 0.1);
		margin-bottom: 1.25rem;
	}

	.about-feature-card__title {
		color: var(--color-navy);
		font-family: var(--font-heading);
		font-size: var(--fs-lg);
		font-weight: var(--w-bold);
		margin-bottom: 0.5rem;
	}

	.about-feature-card__desc {
		color: var(--color-grey-600);
		line-height: 1.65;
	}

	/* ---- Founder cards ---- */
	.founders-grid {
		max-width: 64rem;
		margin: 0 auto;
		display: grid;
		gap: 2rem;
	}

	@media (min-width: 768px) {
		.founders-grid {
			grid-template-columns: repeat(2, 1fr);
		}
	}

	.founder-card {
		background-color: var(--color-off-white);
		border-radius: var(--radius-2xl);
		padding: 2rem;
		outline: 1px solid rgba(216, 220, 228, 0.8);
		outline-offset: -1px;
	}

	@media (min-width: 640px) {
		.founder-card {
			padding: 2.5rem;
		}
	}

	.founder-card__header {
		display: flex;
		align-items: center;
		gap: 1rem;
		margin-bottom: 1.5rem;
	}

	.founder-card__avatar {
		width: 4rem;
		height: 4rem;
		flex-shrink: 0;
		border-radius: var(--radius-full);
		display: flex;
		align-items: center;
		justify-content: center;
		font-size: var(--fs-xl);
		font-weight: var(--w-bold);
		color: var(--color-white);
	}

	.founder-card__avatar--teal {
		background: linear-gradient(135deg, #0fa4af 0%, #1a3a6b 100%);
	}

	.founder-card__photo {
		width: 4rem;
		height: 4rem;
		flex-shrink: 0;
		border-radius: var(--radius-full);
		object-fit: cover;
		outline: 2px solid rgba(212, 168, 67, 0.3);
		outline-offset: -2px;
	}

	.founder-card__name {
		color: var(--color-navy);
		font-family: var(--font-heading);
		font-size: var(--fs-xl);
		font-weight: var(--w-bold);
	}

	.founder-card__role {
		color: var(--color-grey-600);
		font-size: var(--fs-sm);
	}

	.founder-card__bio {
		color: var(--color-grey-700);
		line-height: 1.65;
		display: flex;
		flex-direction: column;
		gap: 1rem;
	}

	.founder-card__tags {
		margin-top: 1.5rem;
		display: flex;
		flex-wrap: wrap;
		gap: 0.75rem;
	}

	.founder-tag {
		display: inline-flex;
		align-items: center;
		gap: 0.375rem;
		border-radius: var(--radius-lg);
		padding: 0.375rem 0.75rem;
		font-size: var(--fs-xs);
		font-weight: var(--w-semibold);
	}

	.founder-tag--teal {
		background-color: rgba(15, 164, 175, 0.1);
		color: var(--color-teal);
	}

	.founder-tag--gold {
		background-color: rgba(212, 168, 67, 0.1);
		color: var(--color-gold);
	}

	/* ---- Dark section CTA ---- */
	.page-dark-heading {
		font-family: var(--font-heading);
		font-size: var(--fs-2xl);
		font-weight: var(--w-bold);
		color: var(--color-white);
		margin-bottom: 1.25rem;
	}

	@media (min-width: 640px) {
		.page-dark-heading {
			font-size: var(--fs-3xl);
		}
	}
	@media (min-width: 768px) {
		.page-dark-heading {
			font-size: var(--fs-4xl);
		}
	}
	@media (min-width: 1024px) {
		.page-dark-heading {
			font-size: clamp(2.5rem, 4vw, 3rem);
		}
	}

	.page-dark-subtitle {
		color: var(--color-grey-300);
		max-width: 42rem;
		margin: 0 auto 2.5rem;
		font-size: 1rem;
		line-height: 1.65;
	}

	@media (min-width: 640px) {
		.page-dark-subtitle {
			font-size: var(--fs-lg);
		}
	}

	.page-cta-btn {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		gap: 0.5rem;
		border-radius: var(--radius-xl);
		padding: 1rem 2rem;
		font-size: var(--fs-sm);
		font-weight: var(--w-semibold);
		color: var(--color-white);
		background-color: var(--color-teal);
		box-shadow:
			var(--shadow-lg),
			0 4px 14px rgba(15, 164, 175, 0.25);
		transition: all 300ms var(--ease-out);
	}

	.page-cta-btn:hover {
		background-color: var(--color-teal-light);
		transform: translateY(-1px);
		box-shadow:
			var(--shadow-xl),
			0 8px 20px rgba(15, 164, 175, 0.3);
	}

	.page-cta-btn:active {
		transform: scale(0.97);
	}
</style>
