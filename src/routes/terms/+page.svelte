<script lang="ts">
	import Seo from '$lib/seo/Seo.svelte';
	import { webPageSchema, buildJsonLd } from '$lib/seo/jsonld';

	const jsonLd = buildJsonLd([
		webPageSchema({
			path: '/terms',
			title: 'Terms of Service - Explosive Swings',
			description:
				'Terms of Service for Explosive Swings. Read our terms and conditions for using our trading education platform and weekly watchlist service.'
		})
	]);
</script>

<Seo
	title="Terms of Service - Explosive Swings"
	description="Terms of Service for Explosive Swings. Read our terms and conditions for using our trading education platform and weekly watchlist service."
	canonical="/terms"
	{jsonLd}
/>

<div class="legal">
	<div class="legal__container">
		<h1 class="legal__title">Terms of Service</h1>
		<p class="legal__updated">Last updated: April 2, 2026</p>

		<section class="legal__section">
			<h2>1. Acceptance of Terms</h2>
			<p>
				By accessing or using the Explosive Swings website, services, content, or any applications
				(collectively, the "Service") made available by Explosive Swings ("we," "us," or "our"), you
				agree to be bound by these Terms of Service. If you do not agree, do not use the Service.
			</p>
		</section>

		<section class="legal__section">
			<h2>2. Description of Service</h2>
			<p>
				Explosive Swings provides trading education content, weekly stock watchlists, options
				trading courses, and related educational materials. Our Service is for informational and
				educational purposes only. <strong
					>Nothing on this site constitutes financial advice, investment advice, trading advice, or
					any other sort of advice.</strong
				>
			</p>
		</section>

		<section class="legal__section">
			<h2>3. No Financial Advice Disclaimer</h2>
			<p>
				All content provided by Explosive Swings, including but not limited to watchlists, alerts,
				courses, articles, and any other materials, is for educational and informational purposes
				only. We are not registered investment advisors, broker-dealers, or financial planners.
			</p>
			<p>
				Trading stocks and options involves substantial risk of loss and is not suitable for every
				investor. Past performance is not indicative of future results. You should consult with a
				qualified financial advisor before making any investment decisions.
			</p>
		</section>

		<section class="legal__section">
			<h2>4. Account Registration</h2>
			<p>
				To access certain features, you may need to create an account. You agree to provide
				accurate, current, and complete information during registration and to keep your account
				information updated. You are responsible for maintaining the confidentiality of your account
				credentials and for all activities that occur under your account.
			</p>
		</section>

		<section class="legal__section">
			<h2>5. Subscription & Payments</h2>
			<p>
				Certain features of the Service require a paid subscription. By subscribing, you agree to
				pay the applicable fees. Subscriptions automatically renew unless canceled before the
				renewal date. All fees are non-refundable except as required by law or as explicitly stated
				in our refund policy.
			</p>
		</section>

		<section class="legal__section">
			<h2>6. Intellectual Property</h2>
			<p>
				All content, features, and functionality of the Service — including text, graphics, logos,
				icons, images, audio, video, courses, watchlists, and software — are owned by Explosive
				Swings and are protected by copyright, trademark, and other intellectual property laws. You
				may not reproduce, distribute, modify, or create derivative works without our prior written
				consent.
			</p>
		</section>

		<section class="legal__section">
			<h2>7. Prohibited Conduct</h2>
			<p>You agree not to:</p>
			<ul>
				<li>Share, redistribute, or resell any subscription content or watchlists</li>
				<li>Use the Service for any unlawful purpose</li>
				<li>Attempt to gain unauthorized access to any part of the Service</li>
				<li>Interfere with or disrupt the Service or its infrastructure</li>
				<li>Scrape, data-mine, or use automated tools to access the Service</li>
			</ul>
		</section>

		<section class="legal__section">
			<h2>8. Risk Acknowledgment</h2>
			<p>
				You acknowledge that trading in stocks, options, and other financial instruments carries a
				high level of risk and may not be suitable for all investors. You could sustain a loss of
				some or all of your investment. You should not trade with money you cannot afford to lose.
			</p>
		</section>

		<section class="legal__section">
			<h2>9. Limitation of Liability</h2>
			<p>
				To the fullest extent permitted by law, Explosive Swings shall not be liable for any
				indirect, incidental, special, consequential, or punitive damages, including but not limited
				to loss of profits, data, or trading losses, arising out of or related to your use of the
				Service.
			</p>
		</section>

		<section class="legal__section">
			<h2>10. Termination</h2>
			<p>
				We reserve the right to suspend or terminate your account and access to the Service at our
				sole discretion, without notice, for conduct that we believe violates these Terms or is
				harmful to other users, us, or third parties.
			</p>
		</section>

		<section class="legal__section">
			<h2>11. Changes to Terms</h2>
			<p>
				We may update these Terms from time to time. We will notify you of material changes by
				posting the new Terms on this page with an updated date. Your continued use of the Service
				after changes constitutes acceptance of the revised Terms.
			</p>
		</section>

		<section class="legal__section">
			<h2>12. Contact</h2>
			<p>
				If you have questions about these Terms, please contact us at
				<a href="mailto:support@explosiveswings.com">support@explosiveswings.com</a>.
			</p>
		</section>
	</div>
</div>

<style>
	.legal {
		min-height: 100vh;
		padding: 6rem 1.5rem 4rem;
		background-color: var(--color-navy-deep);
	}

	.legal__container {
		max-width: 48rem;
		margin: 0 auto;
	}

	.legal__title {
		font-size: var(--fs-4xl);
		font-weight: var(--w-bold);
		font-family: var(--font-heading);
		color: var(--color-white);
		margin-bottom: 0.5rem;
	}

	.legal__updated {
		color: var(--color-grey-400);
		font-size: var(--fs-sm);
		margin-bottom: 3rem;
	}

	.legal__section {
		margin-bottom: 2.5rem;
	}

	.legal__section h2 {
		font-size: var(--fs-xl);
		font-weight: var(--w-semibold);
		font-family: var(--font-heading);
		color: var(--color-white);
		margin-bottom: 0.75rem;
	}

	.legal__section p {
		color: var(--color-grey-300);
		font-size: var(--fs-base);
		line-height: 1.75;
		margin-bottom: 0.75rem;
	}

	.legal__section ul {
		color: var(--color-grey-300);
		font-size: var(--fs-base);
		line-height: 1.75;
		padding-left: 1.5rem;
		margin-top: 0.5rem;
	}

	.legal__section li {
		margin-bottom: 0.35rem;
	}

	.legal__section a {
		color: var(--color-teal);
		text-decoration: none;
		transition: opacity 200ms;
	}

	.legal__section a:hover {
		opacity: 0.8;
	}

	.legal__section strong {
		color: var(--color-white);
	}
</style>
