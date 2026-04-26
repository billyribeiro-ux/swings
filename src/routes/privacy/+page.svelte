<script lang="ts">
	import Seo from '$lib/seo/Seo.svelte';
	import { webPageSchema, buildJsonLd } from '$lib/seo/jsonld';
	import LegalPage, { type LegalSection } from '$lib/components/legal/LegalPage.svelte';
	import InfoIcon from 'phosphor-svelte/lib/InfoIcon';

	const sections: LegalSection[] = [
		{ id: 'introduction', label: 'Introduction' },
		{ id: 'information-we-collect', label: 'Information we collect' },
		{ id: 'how-we-use', label: 'How we use your information' },
		{ id: 'payments', label: 'Payment processing' },
		{ id: 'sharing', label: 'Data sharing' },
		{ id: 'cookies', label: 'Cookies' },
		{ id: 'security', label: 'Data security' },
		{ id: 'retention', label: 'Data retention' },
		{ id: 'rights', label: 'Your rights' },
		{ id: 'children', label: "Children's privacy" },
		{ id: 'changes', label: 'Changes to this policy' },
		{ id: 'contact', label: 'Contact' }
	];

	const jsonLd = buildJsonLd([
		webPageSchema({
			path: '/privacy',
			title: 'Privacy Policy - Precision Options Signals',
			description:
				'Privacy Policy for Precision Options Signals. Learn how we collect, use, and protect your personal information.'
		})
	]);
</script>

<Seo
	title="Privacy Policy - Precision Options Signals"
	description="Privacy Policy for Precision Options Signals. Learn how we collect, use, and protect your personal information."
	{jsonLd}
/>

<LegalPage
	breadcrumb="Privacy Policy"
	title="Privacy Policy"
	intro="We respect your privacy and take the protection of your personal data seriously. This policy explains what we collect, why we collect it, and the controls you have over your data."
	lastUpdated="April 2, 2026"
	effectiveDate="April 2, 2026"
	{sections}
>
	<section id="introduction">
		<h2>Introduction</h2>
		<p>
			Precision Options Signals ("we," "us," or "our") respects your privacy and is committed
			to protecting your personal data. This Privacy Policy explains how we collect, use,
			disclose, and safeguard your information when you visit our website and use our
			services.
		</p>
		<aside class="legal-callout legal-callout--info" role="note">
			<span class="legal-callout__icon" aria-hidden="true">
				<InfoIcon size={16} weight="fill" />
			</span>
			<div class="legal-callout__body">
				<strong class="legal-callout__title">At a glance</strong>
				<p>
					We never sell your data, we process payments through Stripe (no card details
					touch our servers), and you can request deletion of your account at any time.
				</p>
			</div>
		</aside>
	</section>

	<section id="information-we-collect">
		<h2>Information we collect</h2>
		<h3>Information you provide</h3>
		<ul>
			<li>Name and email address when you create an account</li>
			<li>Payment information when you subscribe (processed securely by Stripe)</li>
			<li>Any information you submit through contact forms</li>
		</ul>
		<h3>Information collected automatically</h3>
		<ul>
			<li>IP address, browser type, and device information</li>
			<li>Pages visited, time spent, and navigation patterns</li>
			<li>Cookies and similar tracking technologies</li>
		</ul>
		<p>
			We also use first-party usage analytics (page views and optional CTA impressions/clicks)
			to improve the site. You can turn this off anytime by setting
			<code>localStorage.setItem('swings_analytics_opt_out', '1')</code> in your browser console,
			or by enabling "Do Not Track" where supported.
		</p>
	</section>

	<section id="how-we-use">
		<h2>How we use your information</h2>
		<p>We use your information to:</p>
		<ul>
			<li>Provide and maintain our services, including watchlists and courses</li>
			<li>Process subscriptions and payments</li>
			<li>Send you service-related communications and updates</li>
			<li>Improve our website and user experience</li>
			<li>Comply with legal obligations</li>
		</ul>
	</section>

	<section id="payments">
		<h2>Payment processing</h2>
		<p>
			All payment transactions are processed through Stripe. We do not store your credit card
			information on our servers. Stripe's privacy policy governs the collection and use of
			your payment information. Visit
			<a href="https://stripe.com/privacy" target="_blank" rel="noopener noreferrer"
				>stripe.com/privacy</a
			>
			for more details.
		</p>
	</section>

	<section id="sharing">
		<h2>Data sharing</h2>
		<p>We do not sell your personal data. We may share information with:</p>
		<ul>
			<li>
				<strong>Service providers</strong> — payment processors (Stripe), hosting, and analytics
			</li>
			<li>
				<strong>Legal requirements</strong> — when required by law, regulation, or legal process
			</li>
			<li>
				<strong>Business transfers</strong> — in connection with a merger, acquisition, or sale
				of assets
			</li>
		</ul>
	</section>

	<section id="cookies">
		<h2>Cookies</h2>
		<p>
			We use cookies and similar technologies to maintain your session, remember your
			preferences, and analyze site usage. You can control cookies through your browser
			settings, but disabling them may affect your experience.
		</p>
	</section>

	<section id="security">
		<h2>Data security</h2>
		<p>
			We implement industry-standard security measures including encryption, secure password
			hashing, and access controls to protect your personal data. However, no method of
			electronic transmission or storage is 100% secure, and we cannot guarantee absolute
			security.
		</p>
	</section>

	<section id="retention">
		<h2>Data retention</h2>
		<p>
			We retain your personal data for as long as your account is active or as needed to
			provide our services. You may request deletion of your account and associated data by
			contacting us.
		</p>
	</section>

	<section id="rights">
		<h2>Your rights</h2>
		<p>Depending on your location, you may have the right to:</p>
		<ul>
			<li>Access, update, or delete your personal information</li>
			<li>Opt out of marketing communications</li>
			<li>Request a copy of your data</li>
			<li>Withdraw consent where processing is based on consent</li>
		</ul>
		<p>
			To exercise these rights, contact us at
			<a href="mailto:support@precisionoptionsignals.com"
				>support@precisionoptionsignals.com</a
			>.
		</p>
	</section>

	<section id="children">
		<h2>Children's privacy</h2>
		<p>
			Our Service is not intended for individuals under the age of 18. We do not knowingly
			collect personal data from children.
		</p>
	</section>

	<section id="changes">
		<h2>Changes to this policy</h2>
		<p>
			We may update this Privacy Policy from time to time. We will notify you of material
			changes by posting the updated policy on this page with a new date. Your continued use
			of the Service constitutes acceptance of the revised policy.
		</p>
	</section>

	<section id="contact">
		<h2>Contact</h2>
		<p>
			If you have questions about this Privacy Policy, please contact us at
			<a href="mailto:support@precisionoptionsignals.com"
				>support@precisionoptionsignals.com</a
			>.
		</p>
	</section>
</LegalPage>
