<!--
  ConsentGate — declarative wrapper that only renders children when the given
  consent category has been granted.

  A common pattern for CONSENT-02: wrap an analytics widget, a marketing
  pixel container, a personalization hero, or a YouTube embed iframe so it
  remains invisible (and inert) until the user grants the matching category.

  When the fallback snippet is provided and rendered (i.e. the category is
  denied or undecided), the component emits a one-shot `impression` DOM event
  on the root element — but gated itself by the `analytics` category so we
  don't create a consent-bypass via "missed impression" telemetry.
-->
<script lang="ts" module>
	import type { Snippet } from 'svelte';

	export interface ConsentGateProps {
		category: string;
		children: Snippet;
		/** Optional fallback shown when the category is not granted. */
		fallback?: Snippet;
	}
</script>

<script lang="ts">
	import type { Attachment } from 'svelte/attachments';
	import { consent } from '$lib/stores/consent.svelte';

	const { category, children, fallback }: ConsentGateProps = $props();

	const granted = $derived(consent.hasCategory(category));

	let reported = false;

	/**
	 * Emit a single `impression` event the first time the fallback renders,
	 * provided the observer itself is allowed by the user (analytics granted).
	 * We deliberately do NOT emit when the category under gate IS analytics —
	 * that would be self-referential and leak a signal the user rejected.
	 *
	 * Implemented as an `{@attach}` so the dispatch is tied directly to the
	 * fallback's lifecycle — the event only ever fires from a live element.
	 */
	const impressionAttach: Attachment<HTMLElement> = (node) => {
		if (reported) return;
		if (category === 'analytics') return;
		if (!consent.hasCategory('analytics')) return;
		reported = true;
		node.dispatchEvent(
			new CustomEvent('impression', {
				bubbles: true,
				detail: { category }
			})
		);
	};
</script>

{#if granted}
	{@render children()}
{:else if fallback}
	<div class="consent-fallback" {@attach impressionAttach}>
		{@render fallback()}
	</div>
{/if}

<style>
	.consent-fallback {
		display: contents;
	}
</style>
