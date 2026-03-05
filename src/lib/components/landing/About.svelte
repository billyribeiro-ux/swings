<script lang="ts">
	import SectionHeader from '$lib/components/ui/SectionHeader.svelte';
	import ScrollReveal from '$lib/components/ui/ScrollReveal.svelte';

	const stats = [
		{ value: '18K+', label: 'Active Traders' },
		{ value: '600%', label: 'OXY Overnight Return' },
		{ value: '573x', label: '0DTE SPX Return' },
		{ value: '5-7', label: 'Picks Per Week' }
	];
</script>

<section class="about-section">
	<div class="about-section__container">
		<ScrollReveal>
			<div class="about-section__layout">
				<!-- Left Column -->
				<div class="reveal-item about-section__content">
					<span class="hero-eyebrow about-section__eyebrow">About Billy Ribeiro</span>
					<h2 class="about-section__heading">The Trader Behind the Alerts</h2>

					<div class="about-section__body">
						<p>
							Billy Ribeiro is a <strong class="about-section__strong"
								>high-performance options trader</strong
							>
							known for his precision, discipline, and consistency. During his time at
							<strong class="about-section__strong">Simpler Trading</strong>, he quickly became
							their <strong class="about-section__strong">lead trader</strong> — at one point generating
							more winning trades in a single week than the entire staff combined.
						</p>
						<p>
							Mentored by <strong class="about-section__strong"
								>Mark McGoldrick of Goldman Sachs</strong
							>, Billy developed his proprietary
							<strong class="about-section__strong">"Move Prior to The Move"</strong> methodology — a
							framework for identifying institutional-quality setups before the crowd catches on.
						</p>
						<p>
							After recovering from cancer, Billy shifted his focus toward sustainable, high-impact
							trading education. This alert service allows him to share his expertise without the
							stress of daily live trading — and gives members access to <strong
								class="about-section__strong">clean, early, high-probability setups</strong
							>.
						</p>
					</div>
				</div>

				<!-- Right Column - Stats Grid -->
				<div class="reveal-item about-section__stats">
					{#each stats as stat, i}
						<div class="about-section__stat-card" style="transition-delay: {i * 0.1}s">
							<div class="kpi-value about-section__stat-value">{stat.value}</div>
							<div class="kpi-label about-section__stat-label">{stat.label}</div>
						</div>
					{/each}
				</div>
			</div>
		</ScrollReveal>
	</div>
</section>

<style>
	.about-section {
		padding: 5rem 0;
		background-color: var(--color-navy);
	}

	@media (min-width: 1024px) {
		.about-section {
			padding: 8rem 0;
		}
	}

	.about-section__container {
		max-width: var(--container-max);
		margin: 0 auto;
		padding: 0 1rem;
	}

	@media (min-width: 640px) {
		.about-section__container {
			padding: 0 1.5rem;
		}
	}
	@media (min-width: 1024px) {
		.about-section__container {
			padding: 0 2rem;
		}
	}

	.about-section__layout {
		display: grid;
		gap: 3rem;
		align-items: start;
	}

	@media (min-width: 1024px) {
		.about-section__layout {
			grid-template-columns: 1fr 1fr;
			gap: 4rem;
		}
	}

	.about-section__content {
		display: flex;
		flex-direction: column;
		gap: 1.5rem;
	}

	.about-section__eyebrow {
		display: block;
		color: var(--color-teal-light);
	}

	.about-section__heading {
		font-size: var(--fs-3xl);
		font-weight: var(--w-bold);
		color: var(--color-white);
		font-family: var(--font-heading);
	}

	@media (min-width: 768px) {
		.about-section__heading {
			font-size: var(--fs-4xl);
		}
	}

	.about-section__body {
		display: flex;
		flex-direction: column;
		gap: 1rem;
		color: var(--color-grey-400);
		line-height: 1.65;
	}

	.about-section__strong {
		color: var(--color-white);
	}

	.about-section__stats {
		display: grid;
		grid-template-columns: repeat(2, 1fr);
		gap: 1rem;
	}

	.about-section__stat-card {
		background-color: var(--color-navy-mid);
		border: 1px solid rgba(255, 255, 255, 0.1);
		border-radius: var(--radius-xl);
		padding: 1.5rem;
		text-align: center;
	}

	.about-section__stat-value {
		color: var(--color-teal-light);
		margin-bottom: 0.25rem;
	}

	.about-section__stat-label {
		color: var(--color-grey-400);
	}
</style>
