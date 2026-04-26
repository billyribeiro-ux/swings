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
	import CopyIcon from 'phosphor-svelte/lib/CopyIcon';

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

		await new Promise((resolve) => setTimeout(resolve, 1500));
		testEmailSending = false;
		testEmailMessage =
			'Test email functionality will be available when connected to the backend settings API.';
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
	<header class="settings-page__header">
		<div class="settings-page__title-row">
			<GearIcon size={28} weight="duotone" />
			<div class="settings-page__copy">
				<h1 class="settings-page__title">Settings</h1>
				<p class="settings-page__subtitle">
					Manage site configuration, email, payments, and SEO defaults.
				</p>
			</div>
		</div>
		<p class="settings-page__hint">
			Looking for runtime kill-switches like <code>system.maintenance_mode</code> or encrypted
			secrets? Use the
			<a href="/admin/settings/system" class="settings-page__link">typed system catalogue →</a>
		</p>
	</header>

	{#if saveMessage}
		<div class="toast" role="status">
			<CheckCircleIcon size={16} weight="fill" />
			<span>{saveMessage}</span>
		</div>
	{/if}

	<div class="settings-page__sections">
		<!-- Site Settings -->
		<section class="card">
			<header class="card__head">
				<div class="card__icon card__icon--teal">
					<GlobeIcon size={18} weight="duotone" />
				</div>
				<div class="card__heading-block">
					<span class="card__eyebrow">General</span>
					<h2 class="card__title">Site settings</h2>
					<p class="card__desc">General site information and branding.</p>
				</div>
			</header>

			<div class="card__body">
				<div class="field">
					<label class="field__label" for="site-name">Site name</label>
					<input
						id="site-name"
						name="site-name"
						type="text"
						class="field__input"
						bind:value={settings.site.siteName}
						placeholder="Your Site Name"
					/>
				</div>

				<div class="field">
					<label class="field__label" for="site-desc">Site description</label>
					<textarea
						id="site-desc"
						name="site-desc"
						class="field__textarea"
						bind:value={settings.site.siteDescription}
						placeholder="A brief description of your site"
						rows={3}
					></textarea>
				</div>

				<div class="field">
					<label class="field__label" for="logo-url">Logo URL</label>
					<input
						id="logo-url"
						name="logo-url"
						type="url"
						class="field__input"
						bind:value={settings.site.logoUrl}
						placeholder="https://example.com/logo.svg"
					/>
				</div>

				<div class="field">
					<label class="field__label" for="favicon-url">Favicon URL</label>
					<input
						id="favicon-url"
						name="favicon-url"
						type="url"
						class="field__input"
						bind:value={settings.site.faviconUrl}
						placeholder="https://example.com/favicon.ico"
					/>
				</div>
			</div>
		</section>

		<!-- Email Configuration -->
		<section class="card">
			<header class="card__head">
				<div class="card__icon card__icon--blue">
					<EnvelopeIcon size={18} weight="duotone" />
				</div>
				<div class="card__heading-block">
					<span class="card__eyebrow">Transactional</span>
					<h2 class="card__title">Email configuration</h2>
					<p class="card__desc">SMTP settings for transactional emails.</p>
				</div>
			</header>

			<div class="card__body">
				<div class="field-row">
					<div class="field">
						<label class="field__label" for="smtp-host">SMTP host</label>
						<input
							id="smtp-host"
							name="smtp-host"
							type="text"
							class="field__input"
							bind:value={settings.email.smtpHost}
							placeholder="smtp.example.com"
						/>
					</div>

					<div class="field field--narrow">
						<label class="field__label" for="smtp-port">Port</label>
						<input
							id="smtp-port"
							name="smtp-port"
							type="text"
							class="field__input"
							bind:value={settings.email.smtpPort}
							placeholder="587"
						/>
					</div>
				</div>

				<div class="field-row">
					<div class="field">
						<label class="field__label" for="smtp-user">Username</label>
						<input
							id="smtp-user"
							name="smtp-user"
							type="text"
							autocomplete="username"
							class="field__input"
							bind:value={settings.email.smtpUsername}
							placeholder="your-smtp-username"
						/>
					</div>

					<div class="field">
						<label class="field__label" for="smtp-pass">Password</label>
						<input
							id="smtp-pass"
							name="smtp-pass"
							type="password"
							autocomplete="new-password"
							class="field__input"
							bind:value={settings.email.smtpPassword}
							placeholder="••••••••"
						/>
					</div>
				</div>

				<div class="field">
					<label class="field__label" for="from-email">From email</label>
					<input
						id="from-email"
						name="from-email"
						type="email"
						autocomplete="email"
						class="field__input"
						bind:value={settings.email.fromEmail}
						placeholder="noreply@example.com"
					/>
				</div>

				<div class="card__action">
					<button
						class="btn btn--secondary"
						onclick={sendTestEmail}
						disabled={testEmailSending}
						type="button"
					>
						<PaperPlaneTiltIcon size={16} weight="bold" />
						<span>{testEmailSending ? 'Sending…' : 'Send test email'}</span>
					</button>
					{#if testEmailMessage}
						<p class="card__info-text">{testEmailMessage}</p>
					{/if}
				</div>
			</div>
		</section>

		<!-- Payment Settings -->
		<section class="card">
			<header class="card__head">
				<div class="card__icon card__icon--green">
					<CreditCardIcon size={18} weight="duotone" />
				</div>
				<div class="card__heading-block">
					<span class="card__eyebrow">Billing</span>
					<h2 class="card__title">Payment settings</h2>
					<p class="card__desc">Stripe integration and webhook configuration.</p>
				</div>
			</header>

			<div class="card__body">
				<div class="field">
					<label class="field__label" for="webhook-url">Stripe webhook URL</label>
					<div class="field__readonly-wrap">
						<input
							id="webhook-url"
							name="webhook-url"
							type="text"
							class="field__input field__input--readonly"
							value={stripeWebhookUrl}
							readonly
						/>
						<button
							class="btn btn--secondary btn--small"
							type="button"
							onclick={() => {
								if (browser) {
									navigator.clipboard.writeText(stripeWebhookUrl);
								}
							}}
							title="Copy to clipboard"
						>
							<CopyIcon size={14} weight="bold" />
							<span>Copy</span>
						</button>
					</div>
				</div>

				<div class="field">
					<label class="field__label" for="stripe-key">Stripe public key</label>
					<input
						id="stripe-key"
						name="stripe-key"
						type="text"
						class="field__input field__input--readonly"
						value="pk_live_••••••••••••••••••••••••"
						readonly
					/>
				</div>

				<div class="card__notice">
					<InfoIcon size={16} weight="fill" />
					<p>
						Stripe API keys are configured via environment variables (<code
							>STRIPE_SECRET_KEY</code
						>, <code>STRIPE_PUBLIC_KEY</code>, <code>STRIPE_WEBHOOK_SECRET</code>) in your
						<code>.env</code> file. Changes require a server restart.
					</p>
				</div>
			</div>
		</section>

		<!-- SEO Defaults -->
		<section class="card">
			<header class="card__head">
				<div class="card__icon card__icon--purple">
					<MagnifyingGlassIcon size={18} weight="duotone" />
				</div>
				<div class="card__heading-block">
					<span class="card__eyebrow">Discovery</span>
					<h2 class="card__title">SEO defaults</h2>
					<p class="card__desc">Fallback meta tags for pages without custom SEO.</p>
				</div>
			</header>

			<div class="card__body">
				<div class="field">
					<label class="field__label" for="meta-title">Default meta title</label>
					<input
						id="meta-title"
						name="meta-title"
						type="text"
						class="field__input"
						bind:value={settings.seo.defaultMetaTitle}
						placeholder="Precision Options Signals - Options Trading Alerts"
					/>
				</div>

				<div class="field">
					<label class="field__label" for="meta-desc">Default meta description</label>
					<textarea
						id="meta-desc"
						name="meta-desc"
						class="field__textarea"
						bind:value={settings.seo.defaultMetaDescription}
						placeholder="Get expert swing trading alerts and options analysis…"
						rows={3}
					></textarea>
				</div>

				<div class="field">
					<label class="field__label" for="og-image">Default OG image URL</label>
					<input
						id="og-image"
						name="og-image"
						type="url"
						class="field__input"
						bind:value={settings.seo.defaultOgImageUrl}
						placeholder="https://example.com/og-image.jpg"
					/>
				</div>
			</div>
		</section>

		<!-- Notifications -->
		<section class="card">
			<header class="card__head">
				<div class="card__icon card__icon--amber">
					<BellIcon size={18} weight="duotone" />
				</div>
				<div class="card__heading-block">
					<span class="card__eyebrow">Alerts</span>
					<h2 class="card__title">Notifications</h2>
					<p class="card__desc">Email alerts for key events.</p>
				</div>
			</header>

			<div class="card__body">
				<div class="toggle-row">
					<div class="toggle-row__info">
						<span class="toggle-row__label">Email on new signup</span>
						<span class="toggle-row__desc">Receive an email when a new user registers.</span>
					</div>
					<button
						type="button"
						class="toggle"
						class:toggle--on={settings.notifications.emailOnNewSignup}
						onclick={() =>
							(settings.notifications.emailOnNewSignup =
								!settings.notifications.emailOnNewSignup)}
						role="switch"
						aria-checked={settings.notifications.emailOnNewSignup}
						aria-label="Email on new signup"
					>
						<span class="toggle__track">
							<span class="toggle__thumb"></span>
						</span>
					</button>
				</div>

				<div class="toggle-row">
					<div class="toggle-row__info">
						<span class="toggle-row__label">Email on new subscription</span>
						<span class="toggle-row__desc">Get notified when someone subscribes to a plan.</span>
					</div>
					<button
						type="button"
						class="toggle"
						class:toggle--on={settings.notifications.emailOnNewSubscription}
						onclick={() =>
							(settings.notifications.emailOnNewSubscription =
								!settings.notifications.emailOnNewSubscription)}
						role="switch"
						aria-checked={settings.notifications.emailOnNewSubscription}
						aria-label="Email on new subscription"
					>
						<span class="toggle__track">
							<span class="toggle__thumb"></span>
						</span>
					</button>
				</div>

				<div class="toggle-row">
					<div class="toggle-row__info">
						<span class="toggle-row__label">Email on subscription cancelled</span>
						<span class="toggle-row__desc">Get notified when a subscription is cancelled.</span>
					</div>
					<button
						type="button"
						class="toggle"
						class:toggle--on={settings.notifications.emailOnSubscriptionCancelled}
						onclick={() =>
							(settings.notifications.emailOnSubscriptionCancelled =
								!settings.notifications.emailOnSubscriptionCancelled)}
						role="switch"
						aria-checked={settings.notifications.emailOnSubscriptionCancelled}
						aria-label="Email on subscription cancelled"
					>
						<span class="toggle__track">
							<span class="toggle__thumb"></span>
						</span>
					</button>
				</div>
			</div>
		</section>
	</div>

	<!-- Save Button Bar -->
	<div class="save-bar">
		<div class="save-bar__note">
			<InfoIcon size={14} weight="fill" />
			<span>Settings are stored in localStorage. Connect to a backend settings API for production.</span>
		</div>
		<button class="btn btn--primary" type="button" onclick={saveSettings}>
			<FloppyDiskIcon size={16} weight="bold" />
			<span>Save all settings</span>
		</button>
	</div>
</div>

<style>
	.settings-page {
		max-width: 56rem;
		padding: 0 0 6rem;
	}

	.settings-page__header {
		margin-bottom: 1.5rem;
	}
	.settings-page__title-row {
		display: flex;
		align-items: flex-start;
		gap: 0.85rem;
		color: var(--color-white);
	}
	.settings-page__copy {
		min-width: 0;
	}
	.settings-page__title {
		margin: 0;
		font-family: var(--font-heading);
		font-size: 1.5rem;
		font-weight: 700;
		color: var(--color-white);
		letter-spacing: -0.01em;
		line-height: 1.15;
	}
	.settings-page__subtitle {
		margin: 0.35rem 0 0;
		font-size: 0.875rem;
		color: var(--color-grey-400);
		max-width: 60ch;
		line-height: 1.55;
	}
	.settings-page__hint {
		margin: 0.85rem 0 0;
		font-size: 0.75rem;
		color: var(--color-grey-500);
	}
	.settings-page__hint code {
		font-family: var(--font-mono);
		background: rgba(255, 255, 255, 0.06);
		padding: 0.1em 0.35em;
		border-radius: var(--radius-default);
	}
	.settings-page__link {
		color: var(--color-teal-light);
		text-decoration: underline;
		text-underline-offset: 2px;
	}

	/* Toast */
	.toast {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		padding: 0.75rem 1rem;
		background: rgba(15, 164, 175, 0.12);
		border: 1px solid rgba(15, 164, 175, 0.25);
		border-radius: var(--radius-lg);
		color: #5eead4;
		font-size: 0.875rem;
		font-weight: 500;
		margin-bottom: 1.25rem;
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
		gap: 1.25rem;
		margin-bottom: 1.5rem;
	}

	.card {
		background: var(--color-navy-mid);
		border: 1px solid rgba(255, 255, 255, 0.06);
		border-radius: var(--radius-xl);
		overflow: hidden;
		box-shadow:
			0 1px 0 rgba(255, 255, 255, 0.03) inset,
			0 12px 32px rgba(0, 0, 0, 0.18);
	}
	.card__head {
		display: flex;
		align-items: flex-start;
		gap: 0.85rem;
		padding: 1.25rem 1.25rem 0;
	}
	.card__icon {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		width: 2.25rem;
		height: 2.25rem;
		border-radius: var(--radius-lg);
		flex-shrink: 0;
	}
	.card__icon--teal {
		background: rgba(15, 164, 175, 0.15);
		color: var(--color-teal);
	}
	.card__icon--blue {
		background: rgba(59, 130, 246, 0.15);
		color: #60a5fa;
	}
	.card__icon--green {
		background: rgba(34, 197, 94, 0.15);
		color: #4ade80;
	}
	.card__icon--purple {
		background: rgba(168, 85, 247, 0.15);
		color: #c4b5fd;
	}
	.card__icon--amber {
		background: rgba(245, 158, 11, 0.15);
		color: #fbbf24;
	}
	.card__heading-block {
		min-width: 0;
	}
	.card__eyebrow {
		display: block;
		font-size: 0.6875rem;
		font-weight: 700;
		color: var(--color-grey-500);
		text-transform: uppercase;
		letter-spacing: 0.06em;
		margin-bottom: 0.25rem;
	}
	.card__title {
		margin: 0;
		font-family: var(--font-heading);
		font-size: 1rem;
		font-weight: 700;
		color: var(--color-white);
		letter-spacing: -0.01em;
	}
	.card__desc {
		margin: 0.25rem 0 0;
		font-size: 0.75rem;
		color: var(--color-grey-400);
	}
	.card__body {
		padding: 1.25rem;
		display: flex;
		flex-direction: column;
		gap: 1rem;
	}
	.card__action {
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
		align-items: flex-start;
	}
	.card__info-text {
		margin: 0;
		font-size: 0.75rem;
		color: var(--color-grey-400);
	}
	.card__notice {
		display: flex;
		align-items: flex-start;
		gap: 0.5rem;
		padding: 0.85rem 1rem;
		background: rgba(15, 164, 175, 0.08);
		border: 1px solid rgba(15, 164, 175, 0.18);
		border-radius: var(--radius-lg);
		color: var(--color-teal-light);
		font-size: 0.75rem;
		line-height: 1.55;
	}
	.card__notice p {
		margin: 0;
	}
	.card__notice code {
		background: rgba(255, 255, 255, 0.1);
		padding: 0.05em 0.35em;
		border-radius: var(--radius-default);
		font-size: 0.85em;
	}

	/* Fields */
	.field {
		display: flex;
		flex-direction: column;
		gap: 0.4rem;
		flex: 1;
	}
	.field--narrow {
		flex: 0 0 6.5rem;
		max-width: 6.5rem;
	}
	.field__label {
		font-size: 0.75rem;
		font-weight: 500;
		color: var(--color-grey-300);
	}
	.field__input,
	.field__textarea {
		width: 100%;
		min-height: 2.5rem;
		padding: 0.65rem 0.875rem;
		background: rgba(255, 255, 255, 0.05);
		border: 1px solid rgba(255, 255, 255, 0.1);
		border-radius: var(--radius-lg);
		color: var(--color-white);
		font-size: 0.875rem;
		font-family: var(--font-ui);
		color-scheme: dark;
		transition:
			border-color 150ms,
			box-shadow 150ms;
	}
	.field__input::placeholder,
	.field__textarea::placeholder {
		color: var(--color-grey-500);
	}
	.field__input:focus,
	.field__textarea:focus {
		outline: none;
		border-color: var(--color-teal);
		box-shadow: 0 0 0 3px rgba(15, 164, 175, 0.15);
	}
	.field__input--readonly {
		opacity: 0.75;
		cursor: default;
	}
	.field__textarea {
		resize: vertical;
		min-height: 4.5rem;
		padding: 0.65rem 0.875rem;
	}
	.field__readonly-wrap {
		display: flex;
		gap: 0.5rem;
	}
	.field__readonly-wrap .field__input {
		flex: 1;
	}
	.field-row {
		display: flex;
		flex-direction: column;
		gap: 1rem;
	}

	/* Toggle rows */
	.toggle-row {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 1rem;
		padding: 0.75rem 0;
		border-bottom: 1px solid rgba(255, 255, 255, 0.04);
	}
	.toggle-row:last-child {
		border-bottom: none;
		padding-bottom: 0;
	}
	.toggle-row:first-child {
		padding-top: 0;
	}
	.toggle-row__info {
		display: flex;
		flex-direction: column;
		gap: 0.15rem;
		min-width: 0;
	}
	.toggle-row__label {
		font-size: 0.875rem;
		font-weight: 500;
		color: var(--color-white);
	}
	.toggle-row__desc {
		font-size: 0.75rem;
		color: var(--color-grey-400);
	}

	/* Toggle switch */
	.toggle {
		position: relative;
		width: 2.75rem;
		height: 1.5rem;
		border-radius: var(--radius-full);
		border: none;
		background: transparent;
		cursor: pointer;
		flex-shrink: 0;
		padding: 0;
	}
	.toggle__track {
		display: block;
		width: 100%;
		height: 100%;
		border-radius: var(--radius-full);
		background: rgba(255, 255, 255, 0.12);
		position: relative;
		transition: background 200ms var(--ease-out);
	}
	.toggle--on .toggle__track {
		background: var(--color-teal);
	}
	.toggle__thumb {
		position: absolute;
		top: 3px;
		left: 3px;
		width: 1.125rem;
		height: 1.125rem;
		border-radius: var(--radius-full);
		background: var(--color-white);
		transition: transform 200ms var(--ease-out);
		box-shadow: 0 1px 2px rgba(0, 0, 0, 0.25);
	}
	.toggle--on .toggle__thumb {
		transform: translateX(1.25rem);
	}

	/* Buttons */
	.btn {
		display: inline-flex;
		align-items: center;
		gap: 0.5rem;
		min-height: 2.5rem;
		padding: 0 0.875rem;
		border: 1px solid transparent;
		border-radius: var(--radius-lg);
		font-size: 0.875rem;
		font-weight: 600;
		font-family: var(--font-ui);
		background: transparent;
		color: var(--color-grey-300);
		cursor: pointer;
		text-decoration: none;
		transition:
			background-color 150ms,
			border-color 150ms,
			color 150ms,
			box-shadow 150ms,
			transform 150ms;
	}
	.btn:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}
	.btn--primary {
		background: linear-gradient(135deg, var(--color-teal), var(--color-teal-dark, #0d8a94));
		color: var(--color-white);
		box-shadow: 0 6px 16px -4px rgba(15, 164, 175, 0.45);
	}
	.btn--primary:hover:not(:disabled) {
		transform: translateY(-1px);
		box-shadow: 0 8px 18px -4px rgba(15, 164, 175, 0.55);
	}
	.btn--secondary {
		background: rgba(255, 255, 255, 0.05);
		border-color: rgba(255, 255, 255, 0.1);
		color: var(--color-grey-200);
	}
	.btn--secondary:hover:not(:disabled) {
		background: rgba(255, 255, 255, 0.1);
		border-color: rgba(255, 255, 255, 0.18);
		color: var(--color-white);
	}
	.btn--small {
		min-height: 2rem;
		padding: 0 0.65rem;
		font-size: 0.75rem;
	}

	/* Save bar */
	.save-bar {
		display: flex;
		flex-direction: column;
		gap: 0.85rem;
		padding: 1rem 1.25rem;
		background: var(--color-navy-mid);
		border: 1px solid rgba(255, 255, 255, 0.06);
		border-radius: var(--radius-xl);
		position: sticky;
		bottom: 1rem;
		box-shadow:
			0 1px 0 rgba(255, 255, 255, 0.03) inset,
			0 12px 32px rgba(0, 0, 0, 0.18);
	}
	.save-bar__note {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		font-size: 0.75rem;
		color: var(--color-grey-500);
	}

	/* Tablet+ */
	@media (min-width: 768px) {
		.settings-page__title {
			font-size: 1.5rem;
		}
		.card__head {
			padding: 1.75rem 1.75rem 0;
		}
		.card__body {
			padding: 1.75rem;
		}
		.card {
			border-radius: var(--radius-2xl);
		}
		.field-row {
			flex-direction: row;
		}
		.field--narrow {
			flex: 0 0 7.5rem;
			max-width: 7.5rem;
		}
		.save-bar {
			flex-direction: row;
			align-items: center;
			justify-content: space-between;
		}
	}
</style>
