<script lang="ts">
	import { browser } from '$app/environment';
	import GearIcon from 'phosphor-svelte/lib/GearIcon';
	import GlobeIcon from 'phosphor-svelte/lib/GlobeIcon';
	import EnvelopeIcon from 'phosphor-svelte/lib/EnvelopeIcon';
	import CreditCardIcon from 'phosphor-svelte/lib/CreditCardIcon';
	import MagnifyingGlassIcon from 'phosphor-svelte/lib/MagnifyingGlassIcon';
	import BellIcon from 'phosphor-svelte/lib/BellIcon';
	import PaperPlaneTiltIcon from 'phosphor-svelte/lib/PaperPlaneTiltIcon';
	import FloppyDiskIcon from 'phosphor-svelte/lib/FloppyDiskIcon';
	import InfoIcon from 'phosphor-svelte/lib/InfoIcon';
	import CheckCircleIcon from 'phosphor-svelte/lib/CheckCircleIcon';

	const SETTINGS_KEY = 'swings_admin_settings';

	interface SiteSettings {
		siteName: string;
		siteDescription: string;
		logoUrl: string;
		faviconUrl: string;
	}

	interface EmailSettings {
		smtpHost: string;
		smtpPort: string;
		smtpUsername: string;
		smtpPassword: string;
		fromEmail: string;
	}

	interface SeoSettings {
		defaultMetaTitle: string;
		defaultMetaDescription: string;
		defaultOgImageUrl: string;
	}

	interface NotificationSettings {
		emailOnNewSignup: boolean;
		emailOnNewSubscription: boolean;
		emailOnSubscriptionCancelled: boolean;
	}

	interface AllSettings {
		site: SiteSettings;
		email: EmailSettings;
		seo: SeoSettings;
		notifications: NotificationSettings;
	}

	const defaults: AllSettings = {
		site: {
			siteName: 'Precision Options Signals',
			siteDescription: '',
			logoUrl: '',
			faviconUrl: ''
		},
		email: {
			smtpHost: '',
			smtpPort: '587',
			smtpUsername: '',
			smtpPassword: '',
			fromEmail: ''
		},
		seo: {
			defaultMetaTitle: '',
			defaultMetaDescription: '',
			defaultOgImageUrl: ''
		},
		notifications: {
			emailOnNewSignup: true,
			emailOnNewSubscription: true,
			emailOnSubscriptionCancelled: true
		}
	};

	function loadSettings(): AllSettings {
		if (!browser) return structuredClone(defaults);
		try {
			const stored = localStorage.getItem(SETTINGS_KEY);
			if (stored) {
				const parsed = JSON.parse(stored);
				return {
					site: { ...defaults.site, ...parsed.site },
					email: { ...defaults.email, ...parsed.email },
					seo: { ...defaults.seo, ...parsed.seo },
					notifications: { ...defaults.notifications, ...parsed.notifications }
				};
			}
		} catch {
			// fall through
		}
		return structuredClone(defaults);
	}

	let settings = $state<AllSettings>(loadSettings());
	let saveMessage = $state('');
	let testEmailSending = $state(false);
	let testEmailMessage = $state('');

	function saveSettings() {
		if (!browser) return;
		localStorage.setItem(SETTINGS_KEY, JSON.stringify(settings));
		saveMessage = 'Settings saved successfully';
		setTimeout(() => {
			saveMessage = '';
		}, 3000);
	}

	async function sendTestEmail() {
		testEmailSending = true;
		testEmailMessage = '';

		// Simulate sending test email since this connects to localStorage for now
		await new Promise((resolve) => setTimeout(resolve, 1500));
		testEmailSending = false;
		testEmailMessage = 'Test email functionality will be available when connected to the backend settings API.';
		setTimeout(() => {
			testEmailMessage = '';
		}, 5000);
	}

	const stripeWebhookUrl = $derived(
		browser ? `${window.location.origin}/api/webhooks/stripe` : '/api/webhooks/stripe'
	);
</script>

<svelte:head>
	<title>Settings - Admin - Precision Options Signals</title>
</svelte:head>

<div class="settings-page">
	<div class="settings-page__header">
		<div class="settings-page__title-row">
			<GearIcon size={28} weight="duotone" />
			<h1 class="settings-page__title">Settings</h1>
		</div>
		<p class="settings-page__subtitle">Manage your site configuration, email, payments, and SEO defaults.</p>
		<p class="settings-page__hint">
			Looking for runtime kill-switches like <code>system.maintenance_mode</code> or
			encrypted secrets? Use the
			<a href="/admin/settings/system" class="settings-page__link">typed system catalogue →</a>
		</p>
	</div>

	{#if saveMessage}
		<div class="settings-page__toast">
			<CheckCircleIcon size={18} weight="fill" />
			<span>{saveMessage}</span>
		</div>
	{/if}

	<div class="settings-page__sections">
		<!-- Site Settings -->
		<section class="settings-card">
			<div class="settings-card__header">
				<div class="settings-card__icon settings-card__icon--teal">
					<GlobeIcon size={22} weight="duotone" />
				</div>
				<div>
					<h2 class="settings-card__title">Site Settings</h2>
					<p class="settings-card__desc">General site information and branding</p>
				</div>
			</div>

			<div class="settings-card__body">
				<div class="settings-field">
					<label class="settings-field__label" for="site-name">Site Name</label>
					<input
						id="site-name"
						type="text"
						class="settings-field__input"
						bind:value={settings.site.siteName}
						placeholder="Your Site Name"
					/>
				</div>

				<div class="settings-field">
					<label class="settings-field__label" for="site-desc">Site Description</label>
					<textarea
						id="site-desc"
						class="settings-field__textarea"
						bind:value={settings.site.siteDescription}
						placeholder="A brief description of your site"
						rows={3}
					></textarea>
				</div>

				<div class="settings-field">
					<label class="settings-field__label" for="logo-url">Logo URL</label>
					<input
						id="logo-url"
						type="url"
						class="settings-field__input"
						bind:value={settings.site.logoUrl}
						placeholder="https://example.com/logo.svg"
					/>
				</div>

				<div class="settings-field">
					<label class="settings-field__label" for="favicon-url">Favicon URL</label>
					<input
						id="favicon-url"
						type="url"
						class="settings-field__input"
						bind:value={settings.site.faviconUrl}
						placeholder="https://example.com/favicon.ico"
					/>
				</div>
			</div>
		</section>

		<!-- Email Configuration -->
		<section class="settings-card">
			<div class="settings-card__header">
				<div class="settings-card__icon settings-card__icon--blue">
					<EnvelopeIcon size={22} weight="duotone" />
				</div>
				<div>
					<h2 class="settings-card__title">Email Configuration</h2>
					<p class="settings-card__desc">SMTP settings for transactional emails</p>
				</div>
			</div>

			<div class="settings-card__body">
				<div class="settings-field-row">
					<div class="settings-field">
						<label class="settings-field__label" for="smtp-host">SMTP Host</label>
						<input
							id="smtp-host"
							type="text"
							class="settings-field__input"
							bind:value={settings.email.smtpHost}
							placeholder="smtp.example.com"
						/>
					</div>

					<div class="settings-field settings-field--narrow">
						<label class="settings-field__label" for="smtp-port">Port</label>
						<input
							id="smtp-port"
							type="text"
							class="settings-field__input"
							bind:value={settings.email.smtpPort}
							placeholder="587"
						/>
					</div>
				</div>

				<div class="settings-field-row">
					<div class="settings-field">
						<label class="settings-field__label" for="smtp-user">Username</label>
						<input
							id="smtp-user"
							type="text"
							class="settings-field__input"
							bind:value={settings.email.smtpUsername}
							placeholder="your-smtp-username"
						/>
					</div>

					<div class="settings-field">
						<label class="settings-field__label" for="smtp-pass">Password</label>
						<input
							id="smtp-pass"
							type="password"
							class="settings-field__input"
							bind:value={settings.email.smtpPassword}
							placeholder="••••••••"
						/>
					</div>
				</div>

				<div class="settings-field">
					<label class="settings-field__label" for="from-email">From Email</label>
					<input
						id="from-email"
						type="email"
						class="settings-field__input"
						bind:value={settings.email.fromEmail}
						placeholder="noreply@example.com"
					/>
				</div>

				<div class="settings-card__action">
					<button
						class="settings-btn settings-btn--secondary"
						onclick={sendTestEmail}
						disabled={testEmailSending}
					>
						<PaperPlaneTiltIcon size={16} weight="bold" />
						{testEmailSending ? 'Sending...' : 'Send Test Email'}
					</button>
					{#if testEmailMessage}
						<p class="settings-card__info-text">{testEmailMessage}</p>
					{/if}
				</div>
			</div>
		</section>

		<!-- Payment Settings -->
		<section class="settings-card">
			<div class="settings-card__header">
				<div class="settings-card__icon settings-card__icon--green">
					<CreditCardIcon size={22} weight="duotone" />
				</div>
				<div>
					<h2 class="settings-card__title">Payment Settings</h2>
					<p class="settings-card__desc">Stripe integration and webhook configuration</p>
				</div>
			</div>

			<div class="settings-card__body">
				<div class="settings-field">
					<label class="settings-field__label" for="webhook-url">Stripe Webhook URL</label>
					<div class="settings-field__readonly-wrap">
						<input
							id="webhook-url"
							type="text"
							class="settings-field__input settings-field__input--readonly"
							value={stripeWebhookUrl}
							readonly
						/>
						<button
							class="settings-field__copy-btn"
							onclick={() => {
								if (browser) {
									navigator.clipboard.writeText(stripeWebhookUrl);
								}
							}}
							title="Copy to clipboard"
						>
							Copy
						</button>
					</div>
				</div>

				<div class="settings-field">
					<label class="settings-field__label" for="stripe-key">Stripe Public Key</label>
					<input
						id="stripe-key"
						type="text"
						class="settings-field__input settings-field__input--readonly"
						value="pk_live_••••••••••••••••••••••••"
						readonly
					/>
				</div>

				<div class="settings-card__notice">
					<InfoIcon size={16} weight="fill" />
					<p>Stripe API keys are configured via environment variables (<code>STRIPE_SECRET_KEY</code>, <code>STRIPE_PUBLIC_KEY</code>, <code>STRIPE_WEBHOOK_SECRET</code>) in your <code>.env</code> file. Changes require a server restart.</p>
				</div>
			</div>
		</section>

		<!-- SEO Defaults -->
		<section class="settings-card">
			<div class="settings-card__header">
				<div class="settings-card__icon settings-card__icon--purple">
					<MagnifyingGlassIcon size={22} weight="duotone" />
				</div>
				<div>
					<h2 class="settings-card__title">SEO Defaults</h2>
					<p class="settings-card__desc">Fallback meta tags for pages without custom SEO</p>
				</div>
			</div>

			<div class="settings-card__body">
				<div class="settings-field">
					<label class="settings-field__label" for="meta-title">Default Meta Title</label>
					<input
						id="meta-title"
						type="text"
						class="settings-field__input"
						bind:value={settings.seo.defaultMetaTitle}
						placeholder="Precision Options Signals - Options Trading Alerts"
					/>
				</div>

				<div class="settings-field">
					<label class="settings-field__label" for="meta-desc">Default Meta Description</label>
					<textarea
						id="meta-desc"
						class="settings-field__textarea"
						bind:value={settings.seo.defaultMetaDescription}
						placeholder="Get expert swing trading alerts and options analysis..."
						rows={3}
					></textarea>
				</div>

				<div class="settings-field">
					<label class="settings-field__label" for="og-image">Default OG Image URL</label>
					<input
						id="og-image"
						type="url"
						class="settings-field__input"
						bind:value={settings.seo.defaultOgImageUrl}
						placeholder="https://example.com/og-image.jpg"
					/>
				</div>
			</div>
		</section>

		<!-- Notifications -->
		<section class="settings-card">
			<div class="settings-card__header">
				<div class="settings-card__icon settings-card__icon--amber">
					<BellIcon size={22} weight="duotone" />
				</div>
				<div>
					<h2 class="settings-card__title">Notifications</h2>
					<p class="settings-card__desc">Email alerts for key events</p>
				</div>
			</div>

			<div class="settings-card__body">
				<div class="settings-toggle">
					<div class="settings-toggle__info">
						<span class="settings-toggle__label">Email on new signup</span>
						<span class="settings-toggle__desc">Receive an email when a new user registers</span>
					</div>
					<button
						class="settings-toggle__switch"
						class:settings-toggle__switch--on={settings.notifications.emailOnNewSignup}
						onclick={() => (settings.notifications.emailOnNewSignup = !settings.notifications.emailOnNewSignup)}
						role="switch"
						aria-checked={settings.notifications.emailOnNewSignup}
						aria-label="Email on new signup"
					>
						<span class="settings-toggle__knob"></span>
					</button>
				</div>

				<div class="settings-toggle">
					<div class="settings-toggle__info">
						<span class="settings-toggle__label">Email on new subscription</span>
						<span class="settings-toggle__desc">Get notified when someone subscribes to a plan</span>
					</div>
					<button
						class="settings-toggle__switch"
						class:settings-toggle__switch--on={settings.notifications.emailOnNewSubscription}
						onclick={() => (settings.notifications.emailOnNewSubscription = !settings.notifications.emailOnNewSubscription)}
						role="switch"
						aria-checked={settings.notifications.emailOnNewSubscription}
						aria-label="Email on new subscription"
					>
						<span class="settings-toggle__knob"></span>
					</button>
				</div>

				<div class="settings-toggle">
					<div class="settings-toggle__info">
						<span class="settings-toggle__label">Email on subscription cancelled</span>
						<span class="settings-toggle__desc">Get notified when a subscription is cancelled</span>
					</div>
					<button
						class="settings-toggle__switch"
						class:settings-toggle__switch--on={settings.notifications.emailOnSubscriptionCancelled}
						onclick={() => (settings.notifications.emailOnSubscriptionCancelled = !settings.notifications.emailOnSubscriptionCancelled)}
						role="switch"
						aria-checked={settings.notifications.emailOnSubscriptionCancelled}
						aria-label="Email on subscription cancelled"
					>
						<span class="settings-toggle__knob"></span>
					</button>
				</div>
			</div>
		</section>
	</div>

	<!-- Save Button Bar -->
	<div class="settings-page__save-bar">
		<div class="settings-page__save-note">
			<InfoIcon size={14} weight="fill" />
			<span>Settings are stored in localStorage. Connect to a backend settings API for production use.</span>
		</div>
		<button class="settings-btn settings-btn--primary" onclick={saveSettings}>
			<FloppyDiskIcon size={18} weight="bold" />
			Save All Settings
		</button>
	</div>
</div>

<style>
	.settings-page {
		max-width: 800px;
	}

	.settings-page__header {
		margin-bottom: 1.5rem;
	}

	.settings-page__title-row {
		display: flex;
		align-items: center;
		gap: var(--space-3);
		color: var(--color-white);
	}

	.settings-page__title {
		font-size: var(--fs-xl);
		font-weight: var(--w-bold);
		color: var(--color-white);
		font-family: var(--font-heading);
		margin: 0;
	}

	.settings-page__subtitle {
		font-size: var(--fs-sm);
		color: var(--color-grey-400);
		margin-top: var(--space-2);
	}

	.settings-page__hint {
		font-size: var(--fs-xs);
		color: var(--color-grey-500);
		margin-top: var(--space-2);
	}

	.settings-page__link {
		color: var(--color-teal-light);
		text-decoration: underline;
		text-underline-offset: 2px;
	}

	/* Toast */
	.settings-page__toast {
		display: flex;
		align-items: center;
		gap: var(--space-2);
		padding: var(--space-3) var(--space-4);
		background: rgba(34, 181, 115, 0.12);
		border: 1px solid rgba(34, 181, 115, 0.25);
		border-radius: var(--radius-lg);
		color: var(--color-green);
		font-size: var(--fs-sm);
		font-weight: var(--w-medium);
		margin-bottom: var(--space-6);
		animation: toast-in 300ms ease-out;
	}

	@keyframes toast-in {
		from {
			opacity: 0;
			transform: translateY(-8px);
		}
		to {
			opacity: 1;
			transform: translateY(0);
		}
	}

	/* Section cards */
	.settings-page__sections {
		display: flex;
		flex-direction: column;
		gap: var(--space-6);
		margin-bottom: var(--space-6);
	}

	.settings-card {
		background: var(--color-navy-mid);
		border: 1px solid rgba(255, 255, 255, 0.06);
		border-radius: var(--radius-xl);
		overflow: hidden;
	}

	.settings-card__header {
		display: flex;
		align-items: center;
		gap: var(--space-3);
		padding: var(--space-5) var(--space-5) 0;
	}

	.settings-card__icon {
		width: 2.5rem;
		height: 2.5rem;
		border-radius: var(--radius-lg);
		display: flex;
		align-items: center;
		justify-content: center;
		flex-shrink: 0;
	}

	.settings-card__icon--teal {
		background: rgba(15, 164, 175, 0.15);
		color: var(--color-teal);
	}

	.settings-card__icon--blue {
		background: rgba(59, 130, 246, 0.15);
		color: #3b82f6;
	}

	.settings-card__icon--green {
		background: rgba(34, 197, 94, 0.15);
		color: #22c55e;
	}

	.settings-card__icon--purple {
		background: rgba(168, 85, 247, 0.15);
		color: #a855f7;
	}

	.settings-card__icon--amber {
		background: rgba(245, 158, 11, 0.15);
		color: #f59e0b;
	}

	.settings-card__title {
		font-size: var(--fs-md);
		font-weight: var(--w-bold);
		color: var(--color-white);
		font-family: var(--font-heading);
		margin: 0;
	}

	.settings-card__desc {
		font-size: var(--fs-xs);
		color: var(--color-grey-400);
		margin: var(--space-0-5) 0 0;
	}

	.settings-card__body {
		padding: var(--space-5);
		display: flex;
		flex-direction: column;
		gap: var(--space-4);
	}

	.settings-card__action {
		display: flex;
		flex-direction: column;
		gap: var(--space-2);
		align-items: flex-start;
	}

	.settings-card__info-text {
		font-size: var(--fs-xs);
		color: var(--color-grey-400);
		margin: 0;
	}

	.settings-card__notice {
		display: flex;
		align-items: flex-start;
		gap: var(--space-2);
		padding: var(--space-3) var(--space-4);
		background: rgba(15, 164, 175, 0.08);
		border: 1px solid rgba(15, 164, 175, 0.15);
		border-radius: var(--radius-lg);
		color: var(--color-teal-light);
		font-size: var(--fs-xs);
		line-height: var(--lh-relaxed);
	}

	.settings-card__notice p {
		margin: 0;
	}

	.settings-card__notice code {
		background: rgba(255, 255, 255, 0.1);
		padding: 1px 5px;
		border-radius: var(--radius-default);
		font-size: 0.8em;
	}

	/* Fields */
	.settings-field {
		display: flex;
		flex-direction: column;
		gap: var(--space-1-5);
		flex: 1;
	}

	.settings-field--narrow {
		flex: 0 0 100px;
		max-width: 100px;
	}

	.settings-field__label {
		font-size: var(--fs-sm);
		font-weight: var(--w-medium);
		color: var(--color-grey-300);
	}

	.settings-field__input,
	.settings-field__textarea {
		width: 100%;
		padding: var(--space-2-5) var(--space-3);
		background: rgba(255, 255, 255, 0.05);
		border: 1px solid rgba(255, 255, 255, 0.1);
		border-radius: var(--radius-lg);
		color: var(--color-white);
		font-size: var(--fs-sm);
		font-family: var(--font-ui);
		transition: border-color 200ms var(--ease-out);
	}

	.settings-field__input::placeholder,
	.settings-field__textarea::placeholder {
		color: var(--color-grey-500);
	}

	.settings-field__input:focus,
	.settings-field__textarea:focus {
		outline: none;
		border-color: var(--color-teal);
	}

	.settings-field__input--readonly {
		opacity: 0.7;
		cursor: default;
	}

	.settings-field__textarea {
		resize: vertical;
		min-height: 60px;
	}

	.settings-field__readonly-wrap {
		display: flex;
		gap: var(--space-2);
	}

	.settings-field__readonly-wrap .settings-field__input {
		flex: 1;
	}

	.settings-field__copy-btn {
		padding: var(--space-2) var(--space-3);
		background: rgba(255, 255, 255, 0.08);
		border: 1px solid rgba(255, 255, 255, 0.1);
		border-radius: var(--radius-lg);
		color: var(--color-teal);
		font-size: var(--fs-xs);
		font-weight: var(--w-semibold);
		cursor: pointer;
		white-space: nowrap;
		transition: background 200ms var(--ease-out);
	}

	.settings-field__copy-btn:hover {
		background: rgba(255, 255, 255, 0.14);
	}

	.settings-field-row {
		display: flex;
		flex-direction: column;
		gap: var(--space-4);
	}

	/* Toggles */
	.settings-toggle {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: var(--space-4);
		padding: var(--space-3) 0;
		border-bottom: 1px solid rgba(255, 255, 255, 0.04);
	}

	.settings-toggle:last-child {
		border-bottom: none;
		padding-bottom: 0;
	}

	.settings-toggle:first-child {
		padding-top: 0;
	}

	.settings-toggle__info {
		display: flex;
		flex-direction: column;
		gap: var(--space-0-5);
		min-width: 0;
	}

	.settings-toggle__label {
		font-size: var(--fs-sm);
		font-weight: var(--w-medium);
		color: var(--color-white);
	}

	.settings-toggle__desc {
		font-size: var(--fs-xs);
		color: var(--color-grey-400);
	}

	.settings-toggle__switch {
		position: relative;
		width: 44px;
		height: 24px;
		border-radius: var(--radius-full);
		border: none;
		background: rgba(255, 255, 255, 0.12);
		cursor: pointer;
		flex-shrink: 0;
		transition: background 200ms var(--ease-out);
		padding: 0;
	}

	.settings-toggle__switch--on {
		background: var(--color-teal);
	}

	.settings-toggle__knob {
		position: absolute;
		top: 3px;
		left: 3px;
		width: 18px;
		height: 18px;
		border-radius: var(--radius-full);
		background: var(--color-white);
		transition: transform 200ms var(--ease-out);
		box-shadow: var(--shadow-sm);
	}

	.settings-toggle__switch--on .settings-toggle__knob {
		transform: translateX(20px);
	}

	/* Buttons */
	.settings-btn {
		display: inline-flex;
		align-items: center;
		gap: var(--space-2);
		padding: var(--space-2-5) var(--space-5);
		border: none;
		border-radius: var(--radius-xl);
		font-size: var(--fs-sm);
		font-weight: var(--w-semibold);
		font-family: var(--font-ui);
		cursor: pointer;
		transition: all 200ms var(--ease-out);
	}

	.settings-btn:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}

	.settings-btn--primary {
		background: linear-gradient(135deg, var(--color-teal), var(--color-teal-light));
		color: var(--color-white);
		box-shadow: 0 4px 14px rgba(15, 164, 175, 0.25);
	}

	.settings-btn--primary:hover:not(:disabled) {
		transform: translateY(-1px);
		box-shadow: 0 6px 18px rgba(15, 164, 175, 0.35);
	}

	.settings-btn--secondary {
		background: rgba(255, 255, 255, 0.08);
		color: var(--color-grey-300);
		border: 1px solid rgba(255, 255, 255, 0.1);
	}

	.settings-btn--secondary:hover:not(:disabled) {
		background: rgba(255, 255, 255, 0.14);
		color: var(--color-white);
	}

	/* Save bar */
	.settings-page__save-bar {
		display: flex;
		flex-direction: column;
		gap: var(--space-3);
		padding: var(--space-5);
		background: var(--color-navy-mid);
		border: 1px solid rgba(255, 255, 255, 0.06);
		border-radius: var(--radius-xl);
		position: sticky;
		bottom: var(--space-4);
	}

	.settings-page__save-note {
		display: flex;
		align-items: center;
		gap: var(--space-2);
		font-size: var(--fs-xs);
		color: var(--color-grey-500);
	}

	/* Tablet+ */
	@media (min-width: 768px) {
		.settings-page__header {
			margin-bottom: 2rem;
		}

		.settings-page__title {
			font-size: var(--fs-2xl);
		}

		.settings-field-row {
			flex-direction: row;
		}

		.settings-field--narrow {
			flex: 0 0 120px;
			max-width: 120px;
		}

		.settings-page__save-bar {
			flex-direction: row;
			align-items: center;
			justify-content: space-between;
		}
	}
</style>
