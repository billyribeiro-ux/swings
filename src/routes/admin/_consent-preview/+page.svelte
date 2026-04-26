<!--
  Consent primitives — dev preview.

  Intended for development use only. Reachable at `/admin/_consent-preview`.
  Matches the pattern set by `/admin/_ui-kit`.

  Authz: gated by the admin shell in `src/routes/admin/+layout.svelte` —
  the layout's `{:else}` branch only renders children when
  `auth.isAuthenticated && auth.isAdmin && adminSessionReady`, so non-admin
  visitors see the admin login form instead of this preview surface. The
  in-page `$effect` below adds a defense-in-depth redirect in case the
  layout gate is ever lifted.
-->
<script lang="ts">
	import { goto } from '$app/navigation';
	import { auth } from '$lib/stores/auth.svelte';
	import { consent } from '$lib/stores/consent.svelte';
	import ConsentBanner from '$lib/components/consent/ConsentBanner.svelte';
	import ConsentGate from '$lib/components/consent/ConsentGate.svelte';
	import Button from '$lib/components/shared/Button.svelte';
	import { STUB_BANNER_CONFIG, type BannerConfig } from '$lib/api/consent';

	// Defense-in-depth: if a future refactor lifts the admin-shell gate,
	// this redirect still keeps the dev preview off the public surface.
	$effect(() => {
		if (!auth.loading && !auth.isAdmin) {
			void goto('/admin', { replaceState: true });
		}
	});

	// One config per layout variant so all three render at once.
	const barConfig = $derived<BannerConfig>({ ...STUB_BANNER_CONFIG, layout: 'bar' });
	const boxConfig = $derived<BannerConfig>({ ...STUB_BANNER_CONFIG, layout: 'box' });
	const popupConfig = $derived<BannerConfig>({ ...STUB_BANNER_CONFIG, layout: 'popup' });

	function handleReset() {
		consent.revokeAll();
	}
</script>

<svelte:head>
	<title>Consent preview (dev)</title>
</svelte:head>

<div class="kit">
	<header class="kit-header">
		<h1>Consent primitives</h1>
		<p class="lede">
			Banner + preferences modal + gate, rendered against stub config. Live until CONSENT-01
			backend replaces the stubs. <code>/admin/_ui-kit</code> is the companion page for PE7 shared
			primitives.
		</p>
	</header>

	<section class="tile">
		<h2>Store state</h2>
		<pre class="state">{JSON.stringify(consent.decision, null, 2)}</pre>
		<div class="cluster">
			<Button variant="tertiary" onclick={handleReset}>Reset decision</Button>
			<Button variant="ghost" onclick={() => consent.acceptAll()}>Accept all</Button>
			<Button variant="ghost" onclick={() => consent.rejectAll()}>Reject all</Button>
		</div>
	</section>

	<section class="tile">
		<h2>Bar layout (bottom)</h2>
		<p class="note">
			Persists until the user decides. The real app shell mounts exactly one of these
			globally.
		</p>
		<ConsentBanner config={barConfig} forceOpen />
	</section>

	<section class="tile">
		<h2>Box layout (bottom-end)</h2>
		<p class="note">Floating card variant — good for brand-forward sites.</p>
		<ConsentBanner config={boxConfig} position="bottom-end" forceOpen />
	</section>

	<section class="tile">
		<h2>Popup layout</h2>
		<p class="note">
			Blocking modal. Delegates focus-trap to <code>Dialog</code>. Use sparingly — blocks
			content until decision.
		</p>
		<ConsentBanner config={popupConfig} forceOpen />
	</section>

	<section class="tile">
		<h2>Gated content</h2>
		<p class="note">
			Content inside <code>&lt;ConsentGate&gt;</code> only renders when the category is granted.
		</p>
		<ConsentGate category="analytics">
			<div class="gated">
				<strong>Analytics widget</strong>
				<p>Visible only when analytics consent is granted.</p>
			</div>
			{#snippet fallback()}
				<div class="gated gated--fallback">
					<strong>Analytics widget (blocked)</strong>
					<p>Grant analytics consent to see this widget.</p>
				</div>
			{/snippet}
		</ConsentGate>
	</section>
</div>

<style>
	.kit {
		max-inline-size: var(--container-max);
		margin-inline: auto;
		padding-block: var(--space-8);
		padding-inline: var(--space-6);
		display: flex;
		flex-direction: column;
		gap: var(--space-8);
	}
	.kit-header {
		display: flex;
		flex-direction: column;
		gap: var(--space-2);
	}
	.lede {
		margin: 0;
		color: var(--surface-fg-muted);
	}
	code {
		font-family: var(--font-mono);
		font-size: 0.95em;
	}
	.tile {
		background-color: var(--surface-bg-subtle);
		border: 1px solid var(--surface-border-subtle);
		border-radius: var(--radius-2xl);
		padding: var(--space-6);
		display: flex;
		flex-direction: column;
		gap: var(--space-4);
	}
	.tile h2 {
		margin: 0;
		font-size: var(--fs-lg);
	}
	.note {
		margin: 0;
		color: var(--surface-fg-muted);
		font-size: var(--fs-sm);
	}
	.state {
		margin: 0;
		padding: var(--space-3);
		border-radius: var(--radius-md);
		background-color: var(--brand-navy-800);
		color: var(--neutral-100);
		font-family: var(--font-mono);
		font-size: var(--fs-xs);
		overflow: auto;
	}
	.cluster {
		display: flex;
		gap: var(--space-3);
		flex-wrap: wrap;
	}
	.gated {
		padding: var(--space-4);
		border: 1px solid var(--status-info-500);
		border-radius: var(--radius-2xl);
		background-color: var(--status-info-50);
	}
	.gated--fallback {
		border-color: var(--surface-border-default);
		background-color: var(--surface-bg-muted);
	}
</style>
