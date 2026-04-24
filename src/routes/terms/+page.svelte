<script lang="ts">
	import Seo from '$lib/seo/Seo.svelte';
	import { webPageSchema, buildJsonLd } from '$lib/seo/jsonld';
	import LegalPage, { type LegalSection } from '$lib/components/legal/LegalPage.svelte';
	import WarningIcon from 'phosphor-svelte/lib/WarningIcon';

	const sections: LegalSection[] = [
		{ id: 'acceptance', label: 'Acceptance of terms' },
		{ id: 'description', label: 'Description of service' },
		{ id: 'disclaimer', label: 'No financial advice' },
		{ id: 'account', label: 'Account registration' },
		{ id: 'subscription', label: 'Subscription & payments' },
		{ id: 'ip', label: 'Intellectual property' },
		{ id: 'conduct', label: 'Prohibited conduct' },
		{ id: 'risk', label: 'Risk acknowledgment' },
		{ id: 'liability', label: 'Limitation of liability' },
		{ id: 'termination', label: 'Termination' },
		{ id: 'changes', label: 'Changes to terms' },
		{ id: 'contact', label: 'Contact' }
	];

	const jsonLd = buildJsonLd([
		webPageSchema({
			path: '/terms',
			title: 'Terms of Service - Precision Options Signals',
			description:
				'Terms of Service for Precision Options Signals. Read our terms and conditions for using our trading education platform and weekly watchlist service.'
		})
	]);
</script>

<Seo
	title="Terms of Service - Precision Options Signals"
	description="Terms of Service for Precision Options Signals. Read our terms and conditions for using our trading education platform and weekly watchlist service."
	{jsonLd}
/>

<LegalPage
	breadcrumb="Terms of Service"
	title="Terms of Service"
	intro="These Terms govern your access to and use of Precision Options Signals — our website, weekly watchlists, courses, and related services. Please read them carefully."
	lastUpdated="April 2, 2026"
	effectiveDate="April 2, 2026"
	{sections}
>
	<section id="acceptance">
		<h2>Acceptance of terms</h2>
		<p>
			By accessing or using the Precision Options Signals website, services, content, or any
			applications (collectively, the "Service") made available by Precision Options Signals ("we,"
			"us," or "our"), you agree to be bound by these Terms of Service. If you do not agree, do not
			use the Service.
		</p>
	</section>

	<section id="description">
		<h2>Description of service</h2>
		<p>
			Precision Options Signals provides trading education content, weekly stock watchlists, options
			trading courses, and related educational materials. Our Service is for informational and
			educational purposes only.
		</p>
		<aside class="legal-callout" role="note">
			<span class="legal-callout__icon" aria-hidden="true">
				<WarningIcon size={16} weight="fill" />
			</span>
			<div class="legal-callout__body">
				<strong class="legal-callout__title">Not financial advice</strong>
				<p>
					Nothing on this site constitutes financial advice, investment advice, trading advice, or
					any other sort of advice. You are solely responsible for your trading decisions.
				</p>
			</div>
		</aside>
	</section>

	<section id="disclaimer">
		<h2>No financial advice disclaimer</h2>
		<p>
			All content provided by Precision Options Signals, including but not limited to watchlists,
			alerts, courses, articles, and any other materials, is for educational and informational
			purposes only. We are not registered investment advisors, broker-dealers, or financial
			planners.
		</p>
		<p>
			Trading stocks and options involves substantial risk of loss and is not suitable for every
			investor. Past performance is not indicative of future results. You should consult with a
			qualified financial advisor before making any investment decisions.
		</p>
	</section>

	<section id="account">
		<h2>Account registration</h2>
		<p>
			To access certain features, you may need to create an account. You agree to provide accurate,
			current, and complete information during registration and to keep your account information
			updated. You are responsible for maintaining the confidentiality of your account credentials
			and for all activities that occur under your account.
		</p>
	</section>

	<section id="subscription">
		<h2>Subscription &amp; payments</h2>
		<p>
			Certain features of the Service require a paid subscription. By subscribing, you agree to pay
			the applicable fees. Subscriptions automatically renew unless canceled before the renewal
			date. All fees are non-refundable except as required by law or as explicitly stated in our
			refund policy.
		</p>
	</section>

	<section id="ip">
		<h2>Intellectual property</h2>
		<p>
			All content, features, and functionality of the Service — including text, graphics, logos,
			icons, images, audio, video, courses, watchlists, and software — are owned by Precision
			Options Signals and are protected by copyright, trademark, and other intellectual property
			laws. You may not reproduce, distribute, modify, or create derivative works without our prior
			written consent.
		</p>
	</section>

	<section id="conduct">
		<h2>Prohibited conduct</h2>
		<p>You agree not to:</p>
		<ul>
			<li>Share, redistribute, or resell any subscription content or watchlists</li>
			<li>Use the Service for any unlawful purpose</li>
			<li>Attempt to gain unauthorized access to any part of the Service</li>
			<li>Interfere with or disrupt the Service or its infrastructure</li>
			<li>Scrape, data-mine, or use automated tools to access the Service</li>
		</ul>
	</section>

	<section id="risk">
		<h2>Risk acknowledgment</h2>
		<p>
			You acknowledge that trading in stocks, options, and other financial instruments carries a
			high level of risk and may not be suitable for all investors. You could sustain a loss of
			some or all of your investment.
			<strong>You should not trade with money you cannot afford to lose.</strong>
		</p>
	</section>

	<section id="liability">
		<h2>Limitation of liability</h2>
		<p>
			To the fullest extent permitted by law, Precision Options Signals shall not be liable for any
			indirect, incidental, special, consequential, or punitive damages, including but not limited
			to loss of profits, data, or trading losses, arising out of or related to your use of the
			Service.
		</p>
	</section>

	<section id="termination">
		<h2>Termination</h2>
		<p>
			We reserve the right to suspend or terminate your account and access to the Service at our
			sole discretion, without notice, for conduct that we believe violates these Terms or is
			harmful to other users, us, or third parties.
		</p>
	</section>

	<section id="changes">
		<h2>Changes to terms</h2>
		<p>
			We may update these Terms from time to time. We will notify you of material changes by
			posting the new Terms on this page with an updated date. Your continued use of the Service
			after changes constitutes acceptance of the revised Terms.
		</p>
	</section>

	<section id="contact">
		<h2>Contact</h2>
		<p>
			If you have questions about these Terms, please contact us at
			<a href="mailto:support@precisionoptionsignals.com">support@precisionoptionsignals.com</a>.
		</p>
	</section>
</LegalPage>
