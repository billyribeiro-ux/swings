<!--
  ConsentBanner — the top-level surface that prompts the subject for a
  consent decision. Composes PE7 primitives (`Button`, `Dialog`) and the
  runes-backed consent store; no third-party dependency beyond phosphor icons.

  A11y:
  - Rendered as a `<section role="region" aria-label="Cookie consent">` so it
    appears in the page-landmarks list and screen readers can jump to it.
  - Headline is an `<h2>` with `aria-labelledby` wiring on the region.
  - Action cluster is a `<div role="group">` with a descriptive label.
  - `bar` and `box` layouts do NOT trap focus — they're persistent landmarks,
    not modals. `popup` variant delegates focus trapping to `Dialog`.
  - Reduced motion: every animation is disabled under the media query.

  GPC:
  - If the store reports `gpc === true` AND `hasDecided === true`, the banner
    does NOT render. The consent event was already dispatched by the store
    during hydration. The admin's audit log records the GPC-driven denial.
-->
<script lang="ts" module>
	import type { BannerConfig, BannerLayout, BannerPosition } from '$lib/api/consent';

	export interface ConsentBannerProps {
		config: BannerConfig;
		/** Override the layout inherited from `config.layout`. */
		layout?: BannerLayout;
		/** Override the position inherited from `config.position`. */
		position?: BannerPosition;
		/** If true, forces the banner open even when the store reports hasDecided. */
		forceOpen?: boolean;
	}
</script>

<script lang="ts">
	import { consent } from '$lib/stores/consent.svelte';
	import Button from '$lib/components/shared/Button.svelte';
	import Dialog from '$lib/components/shared/Dialog.svelte';
	import ConsentPreferencesModal from './ConsentPreferencesModal.svelte';
	import { translateOrFallback } from '$lib/i18n/paraglide';

	const { config, layout, position, forceOpen = false }: ConsentBannerProps = $props();

	const effectiveLayout = $derived<BannerLayout>(layout ?? config.layout);
	const effectivePosition = $derived<BannerPosition>(position ?? config.position);

	// CONSENT-06: translation-with-fallback. The server-provided `config.copy`
	// is authoritative — translations merely override. A missing catalogue key
	// therefore never shows a raw `consent_banner_title` identifier to the
	// user; it always degrades to the English copy from the admin payload.
	const title = $derived(translateOrFallback('consent_banner_title', config.copy.title));
	const body = $derived(translateOrFallback('consent_banner_body', config.copy.body));
	const acceptAllLabel = $derived(
		translateOrFallback('consent_banner_accept_all', config.copy.acceptAll)
	);
	const rejectAllLabel = $derived(
		translateOrFallback('consent_banner_reject_all', config.copy.rejectAll)
	);
	const customizeLabel = $derived(
		translateOrFallback('consent_banner_customize', config.copy.customize)
	);
	const privacyPolicyLabel = $derived(
		config.copy.privacyPolicyLabel
			? translateOrFallback(
					'consent_banner_privacy_policy_label',
					config.copy.privacyPolicyLabel
				)
			: undefined
	);

	let preferencesOpen = $state(false);

	const isVisible = $derived(forceOpen || (!consent.state.hasDecided && !consent.state.gpc));

	function handleAcceptAll() {
		consent.acceptAll();
	}

	function handleRejectAll() {
		consent.rejectAll();
	}

	function handleCustomize() {
		preferencesOpen = true;
	}

	function handlePreferencesClose() {
		preferencesOpen = false;
	}
</script>

{#snippet bannerBody()}
	<div class="content">
		<div class="copy">
			<h2 id="consent-banner-title" class="title">{title}</h2>
			<p class="body">
				{body}
				{#if config.copy.privacyPolicyHref && privacyPolicyLabel}
					<!-- eslint-disable-next-line svelte/no-navigation-without-resolve -- privacyPolicyHref comes from admin-managed BannerConfig and is typically an external URL; resolve() does not apply -->
					<a class="policy-link" href={config.copy.privacyPolicyHref}>
						{privacyPolicyLabel}
					</a>
				{/if}
			</p>
		</div>
		<div class="actions" role="group" aria-label="Consent actions">
			<Button variant="tertiary" size="md" onclick={handleCustomize}>
				{customizeLabel}
			</Button>
			<Button variant="secondary" size="md" onclick={handleRejectAll}>
				{rejectAllLabel}
			</Button>
			<Button variant="primary" size="md" onclick={handleAcceptAll}>
				{acceptAllLabel}
			</Button>
		</div>
	</div>
{/snippet}

{#if isVisible && effectiveLayout === 'popup'}
	<Dialog open {title} description={body} size="md" closeOnBackdrop={false} closeOnEscape={false}>
		<div class="popup-inner" role="region" aria-label="Cookie consent">
			<div class="actions" role="group" aria-label="Consent actions">
				<Button variant="tertiary" size="md" fullWidth onclick={handleCustomize}>
					{customizeLabel}
				</Button>
				<Button variant="secondary" size="md" fullWidth onclick={handleRejectAll}>
					{rejectAllLabel}
				</Button>
				<Button variant="primary" size="md" fullWidth onclick={handleAcceptAll}>
					{acceptAllLabel}
				</Button>
			</div>
			{#if config.copy.privacyPolicyHref && privacyPolicyLabel}
				<p class="policy-note">
					<!-- eslint-disable-next-line svelte/no-navigation-without-resolve -- privacyPolicyHref comes from admin-managed BannerConfig and is typically an external URL; resolve() does not apply -->
					<a class="policy-link" href={config.copy.privacyPolicyHref}>
						{privacyPolicyLabel}
					</a>
				</p>
			{/if}
		</div>
	</Dialog>
{:else if isVisible}
	<section
		class="banner"
		data-testid="consent-banner"
		data-layout={effectiveLayout}
		data-position={effectivePosition}
		aria-labelledby="consent-banner-title"
	>
		{@render bannerBody()}
	</section>
{/if}

<ConsentPreferencesModal
	open={preferencesOpen}
	categories={config.categories}
	copy={config.copy}
	onclose={handlePreferencesClose}
/>

<style>
	.banner {
		position: fixed;
		/* Stack ABOVE the dashboard's mobile bottom tab bar (z=50). Without
		 * this lift the banner gets visually cropped at its bottom on
		 * `/dashboard/*` and `/admin/*` mobile viewports. Tooltips
		 * (z=11000) and Dialog (manages its own focus-trap layer) still
		 * float over the banner. */
		z-index: 60;
		background-color: var(--surface-bg-canvas);
		color: var(--surface-fg-default);
		border: 1px solid var(--surface-border-subtle);
		box-shadow: var(--shadow-xl);
		animation: banner-enter var(--duration-300) var(--ease-spring);
	}

	/* --- Layouts ---------------------------------------------------------- */
	.banner[data-layout='bar'] {
		inset-inline: 0;
		inline-size: 100%;
		border-radius: 0;
		border-inline: 0;
	}
	.banner[data-layout='bar'][data-position='bottom'],
	.banner[data-layout='bar'][data-position='bottom-start'],
	.banner[data-layout='bar'][data-position='bottom-end'] {
		inset-block-end: 0;
		border-block-end: 0;
	}

	/* On mobile (< 768px), the dashboard mounts a 56px bottom tab bar.
	 * Push the banner up by that height + a comfortable gap so tap targets
	 * don't overlap and the banner is fully visible — not just stacked on
	 * top of the bar. Desktop is unaffected. */
	@media (max-width: 768px) {
		.banner[data-layout='bar'][data-position='bottom'],
		.banner[data-layout='bar'][data-position='bottom-start'],
		.banner[data-layout='bar'][data-position='bottom-end'] {
			inset-block-end: calc(56px + env(safe-area-inset-bottom, 0px));
		}
		.banner[data-layout='box'][data-position='bottom'],
		.banner[data-layout='box'][data-position='bottom-start'],
		.banner[data-layout='box'][data-position='bottom-end'] {
			inset-block-end: calc(var(--space-4) + 56px + env(safe-area-inset-bottom, 0px));
		}
	}
	.banner[data-layout='bar'][data-position='top'] {
		inset-block-start: 0;
		border-block-start: 0;
	}

	.banner[data-layout='box'] {
		inline-size: min(32rem, calc(100vw - var(--space-8)));
		margin-inline: var(--space-4);
		border-radius: var(--radius-xl);
	}
	.banner[data-layout='box'][data-position='bottom'] {
		inset-block-end: var(--space-4);
		inset-inline-start: 50%;
		transform: translateX(-50%);
	}
	.banner[data-layout='box'][data-position='bottom-start'] {
		inset-block-end: var(--space-4);
		inset-inline-start: var(--space-4);
	}
	.banner[data-layout='box'][data-position='bottom-end'] {
		inset-block-end: var(--space-4);
		inset-inline-end: var(--space-4);
	}
	.banner[data-layout='box'][data-position='top'] {
		inset-block-start: var(--space-4);
		inset-inline-start: 50%;
		transform: translateX(-50%);
	}
	.banner[data-layout='box'][data-position='center'] {
		inset-block-start: 50%;
		inset-inline-start: 50%;
		transform: translate(-50%, -50%);
	}

	.content {
		display: flex;
		flex-direction: column;
		gap: var(--space-4);
		padding-block: var(--space-5);
		padding-inline: var(--space-6);
	}

	.banner[data-layout='bar'] .content {
		max-inline-size: var(--container-max);
		margin-inline: auto;
	}

	@media (min-width: 48rem) {
		.banner[data-layout='bar'] .content {
			flex-direction: row;
			align-items: center;
			gap: var(--space-6);
		}
	}

	.copy {
		flex: 1 1 auto;
		min-inline-size: 0;
	}

	.title {
		margin: 0;
		font-family: var(--font-heading);
		font-size: var(--fs-md);
		font-weight: var(--w-semibold);
		line-height: var(--lh-snug);
		color: var(--surface-fg-default);
	}

	.body {
		margin-block-start: var(--space-1-5);
		margin-block-end: 0;
		font-size: var(--fs-sm);
		line-height: var(--lh-normal);
		color: var(--surface-fg-muted);
	}

	.policy-link {
		color: var(--brand-teal-600);
		text-decoration: underline;
		text-underline-offset: 0.2em;
	}
	.policy-link:hover {
		color: var(--brand-teal-700);
	}

	.actions {
		display: flex;
		flex-wrap: wrap;
		gap: var(--space-2);
		flex-shrink: 0;
	}

	@media (max-width: 40rem) {
		.actions {
			flex-direction: column;
			align-items: stretch;
		}
	}

	.popup-inner {
		display: flex;
		flex-direction: column;
		gap: var(--space-4);
	}

	.popup-inner .actions {
		flex-direction: column;
	}

	.policy-note {
		margin: 0;
		font-size: var(--fs-xs);
		color: var(--surface-fg-muted);
		text-align: center;
	}

	@keyframes banner-enter {
		from {
			opacity: 0;
			transform: translateY(var(--banner-enter-offset, 12px));
		}
		to {
			opacity: 1;
			transform: translateY(0);
		}
	}
	.banner[data-position='top'] {
		--banner-enter-offset: -12px;
	}

	@media (prefers-reduced-motion: reduce) {
		.banner {
			animation: none;
		}
	}
</style>
