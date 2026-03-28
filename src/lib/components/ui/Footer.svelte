<script lang="ts">
	import { courses } from '$lib/data/courses';
	import EnvelopeSimple from 'phosphor-svelte/lib/EnvelopeSimple';
	import TrendUp from 'phosphor-svelte/lib/TrendUp';

	const currentYear = new Date().getFullYear();
</script>

<footer class="footer">
	<div class="footer__container">
		<!-- Top: Logo + Nav grid -->
		<div class="footer__grid">
			<!-- Brand -->
			<div class="footer__brand">
				<a href="/" class="footer__logo">
					<span class="footer__logo-brand">Explosive</span>
					<span class="footer__logo-accent">Swings</span>
				</a>
				<p class="footer__tagline">
					Weekly options watchlists and structured trading courses, built by Billy Ribeiro.
				</p>
			</div>

			<!-- Courses -->
			<div>
				<h4 class="footer__heading">Courses</h4>
				<ul class="footer__list">
					{#each courses as course (course.id)}
						<li>
							<a href="/courses/{course.slug}" class="footer__link">{course.title}</a>
						</li>
					{/each}
					<li>
						<a href="/courses" class="footer__link">View All →</a>
					</li>
				</ul>
			</div>

			<!-- Company -->
			<div>
				<h4 class="footer__heading">Company</h4>
				<ul class="footer__list">
					<li><a href="/#pricing" class="footer__link">Pricing</a></li>
					<li><a href="/terms" class="footer__link">Terms</a></li>
					<li><a href="/privacy" class="footer__link">Privacy</a></li>
				</ul>
			</div>

			<!-- Contact -->
			<div>
				<h4 class="footer__heading">Contact</h4>
				<a href="mailto:support@explosiveswings.com" class="footer__contact-link">
					<EnvelopeSimple size={16} weight="bold" class="footer__contact-icon" />
					support@explosiveswings.com
				</a>
			</div>
		</div>

		<!-- Bottom: Copyright + Disclaimer -->
		<div class="footer__bottom">
			<div class="footer__bottom-inner">
				<p class="footer__copyright">
					© {currentYear} Explosive Swings. All rights reserved.
				</p>
				<div class="footer__disclaimer">
					<TrendUp size={14} weight="bold" class="footer__disclaimer-icon" />
					<span>Trading involves risk. Past performance ≠ future results.</span>
				</div>
			</div>
		</div>
	</div>
</footer>

<style>
	.footer {
		background-color: var(--color-navy);
		border-top: 1px solid rgba(255, 255, 255, 0.1);
		padding: 3rem 0;
	}

	@media (min-width: 640px) {
		.footer {
			padding: 4rem 0;
		}
	}

	.footer__container {
		max-width: var(--container-max);
		margin: 0 auto;
		padding: 0 1rem;
	}

	@media (min-width: 640px) {
		.footer__container {
			padding: 0 1.5rem;
		}
	}

	@media (min-width: 1024px) {
		.footer__container {
			padding: 0 2rem;
		}
	}

	.footer__grid {
		display: grid;
		gap: 2.5rem;
		margin-bottom: 2.5rem;
	}

	@media (min-width: 640px) {
		.footer__grid {
			grid-template-columns: repeat(2, 1fr);
		}
	}

	@media (min-width: 1024px) {
		.footer__grid {
			grid-template-columns: repeat(4, 1fr);
		}
	}

	@media (min-width: 640px) {
		.footer__brand {
			grid-column: span 2;
		}
	}

	@media (min-width: 1024px) {
		.footer__brand {
			grid-column: span 1;
		}
	}

	.footer__logo {
		display: inline-flex;
		align-items: center;
		gap: 0.125rem;
		font-family: var(--font-heading);
		font-size: 1.125rem;
		font-weight: var(--w-bold);
		letter-spacing: -0.025em;
		margin-bottom: 0.75rem;
	}

	.footer__logo-brand {
		color: var(--color-white);
	}
	.footer__logo-accent {
		color: var(--color-teal-light);
	}

	.footer__tagline {
		color: var(--color-grey-500);
		max-width: 20rem;
		font-size: var(--fs-xs);
		line-height: 1.65;
	}

	.footer__heading {
		color: var(--color-grey-400);
		margin-bottom: 1rem;
		font-size: 11px;
		font-weight: var(--w-semibold);
		letter-spacing: 0.1em;
		text-transform: uppercase;
	}

	.footer__list {
		display: flex;
		flex-direction: column;
		gap: 0.625rem;
	}

	.footer__link {
		color: var(--color-grey-300);
		font-size: var(--fs-sm);
		transition: color 200ms var(--ease-out);
	}

	.footer__link:hover {
		color: var(--color-teal-light);
	}

	.footer__contact-link {
		display: inline-flex;
		align-items: center;
		gap: 0.5rem;
		color: var(--color-grey-300);
		font-size: var(--fs-sm);
		transition: color 200ms var(--ease-out);
	}

	.footer__contact-link:hover {
		color: var(--color-teal-light);
	}

	:global(.footer__contact-icon) {
		color: var(--color-grey-500) !important;
	}

	.footer__bottom {
		border-top: 1px solid rgba(255, 255, 255, 0.08);
		padding-top: 2rem;
	}

	.footer__bottom-inner {
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 1rem;
	}

	@media (min-width: 640px) {
		.footer__bottom-inner {
			flex-direction: row;
			justify-content: space-between;
		}
	}

	.footer__copyright {
		color: var(--color-grey-500);
		font-size: var(--fs-xs);
	}

	.footer__disclaimer {
		display: flex;
		align-items: center;
		gap: 0.375rem;
		color: var(--color-grey-600);
		font-size: var(--fs-xs);
	}

	:global(.footer__disclaimer-icon) {
		color: var(--color-grey-500) !important;
	}
</style>
