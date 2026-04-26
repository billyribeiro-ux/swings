<!--
  ConsentPreferencesModal — fine-grained toggle surface for each consent
  category. Delegates focus-trap, ESC-close, and backdrop handling to the
  canonical `Dialog` primitive so behaviour is identical to every other modal
  in the app.

  A11y:
  - Each row is a fieldset-free label-with-switch pair: a labelled `<button
    role="switch">` with `aria-checked` carries the state. This is the WAI
    pattern for binary toggles and is the only implementation screen readers
    universally announce as "on/off".
  - `necessary` is a button with `aria-checked="true"` + `aria-disabled="true"`
    and a visible "Required" tag so both sighted and AT users know it cannot
    be changed.
  - Description text is linked via `aria-describedby` so it is read out
    together with the control name.
-->
<script lang="ts" module>
	import type { BannerCopy, ConsentCategoryDef } from '$lib/api/consent';

	export interface ConsentPreferencesModalProps {
		open: boolean;
		categories: ReadonlyArray<ConsentCategoryDef>;
		copy: BannerCopy;
		onclose: () => void;
	}
</script>

<script lang="ts">
	import { untrack } from 'svelte';
	import CheckIcon from 'phosphor-svelte/lib/CheckIcon';
	import { consent } from '$lib/stores/consent.svelte';
	import Button from '$lib/components/shared/Button.svelte';
	import Dialog from '$lib/components/shared/Dialog.svelte';
	import { translateOrFallback } from '$lib/i18n/paraglide';

	const { open, categories, copy, onclose }: ConsentPreferencesModalProps = $props();

	// CONSENT-06: translation-with-fallback. Server copy is authoritative.
	const modalTitle = $derived(translateOrFallback('consent_preferences_title', copy.customize));
	const modalDescription = $derived(
		translateOrFallback(
			'consent_preferences_description',
			"Choose exactly which categories we're allowed to use. Your choice is saved locally and applies to every page on this site."
		)
	);
	const cancelLabel = $derived(translateOrFallback('consent_preferences_cancel', 'Cancel'));
	const requiredTag = $derived(
		translateOrFallback('consent_preferences_required_tag', 'Required')
	);
	const saveLabel = $derived(
		translateOrFallback('consent_banner_save_preferences', copy.savePreferences)
	);

	// Local draft — the user can toggle freely and only commit on "Save".
	// Reseed only on `open` transitions; changes to `categories` or the consent
	// store mid-session must NOT clobber the user's in-progress edits.
	let draft = $state<Record<string, boolean>>({});

	$effect(() => {
		if (!open) return;
		untrack(() => {
			const seed: Record<string, boolean> = {};
			for (const cat of categories) {
				seed[cat.key] = cat.required ? true : consent.hasCategory(cat.key);
			}
			draft = seed;
		});
	});

	function toggle(key: string, required: boolean) {
		if (required) return;
		draft = { ...draft, [key]: !draft[key] };
	}

	function handleSave() {
		for (const cat of categories) {
			if (cat.required) continue;
			consent.updateCategory(cat.key, draft[cat.key] === true);
		}
		onclose();
	}
</script>

<Dialog {open} {onclose} title={modalTitle} description={modalDescription} size="md">
	<ul class="category-list" aria-label="Consent categories">
		{#each categories as cat (cat.key)}
			{@const descId = `consent-cat-${cat.key}-desc`}
			{@const checked = draft[cat.key] === true}
			<li class="category-row">
				<div class="category-copy">
					<p class="category-label">
						{cat.label}
						{#if cat.required}
							<span class="required-tag" aria-hidden="true">{requiredTag}</span>
							<span class="sr-only"> (always on)</span>
						{/if}
					</p>
					<p id={descId} class="category-desc">{cat.description}</p>
				</div>
				<button
					type="button"
					class="switch"
					role="switch"
					aria-checked={checked}
					aria-disabled={cat.required ? 'true' : undefined}
					aria-describedby={descId}
					aria-label={cat.label}
					disabled={cat.required}
					onclick={() => toggle(cat.key, cat.required)}
				>
					<span class="switch-thumb" aria-hidden="true">
						{#if checked}
							<CheckIcon size="0.75rem" weight="bold" />
						{/if}
					</span>
				</button>
			</li>
		{/each}
	</ul>
	{#snippet footer()}
		<Button variant="tertiary" onclick={onclose}>{cancelLabel}</Button>
		<Button variant="primary" onclick={handleSave}>{saveLabel}</Button>
	{/snippet}
</Dialog>

<style>
	.category-list {
		list-style: none;
		padding: 0;
		margin: 0;
		display: flex;
		flex-direction: column;
		gap: var(--space-4);
	}

	.category-row {
		display: flex;
		align-items: flex-start;
		justify-content: space-between;
		gap: var(--space-4);
		padding-block: var(--space-3);
		border-block-end: 1px solid var(--surface-border-subtle);
	}
	.category-row:last-child {
		border-block-end: 0;
	}

	.category-copy {
		flex: 1 1 auto;
		min-inline-size: 0;
	}

	.category-label {
		margin: 0;
		font-weight: var(--w-semibold);
		font-size: var(--fs-sm);
		color: var(--surface-fg-default);
		display: inline-flex;
		align-items: center;
		gap: var(--space-2);
	}

	.required-tag {
		font-size: var(--fs-2xs);
		font-weight: var(--w-medium);
		letter-spacing: var(--ls-wider);
		text-transform: uppercase;
		color: var(--brand-teal-700);
		background-color: var(--brand-teal-50);
		border-radius: var(--radius-full);
		padding-block: var(--space-0-5);
		padding-inline: var(--space-2);
	}

	.category-desc {
		margin-block-start: var(--space-1);
		margin-block-end: 0;
		font-size: var(--fs-xs);
		color: var(--surface-fg-muted);
		line-height: var(--lh-normal);
	}

	/* Switch — WAI-ARIA button-with-role pattern. */
	.switch {
		appearance: none;
		inline-size: 2.75rem;
		block-size: 1.5rem;
		padding: 2px;
		border-radius: var(--radius-full);
		border: 1px solid var(--surface-border-default);
		background-color: var(--surface-bg-muted);
		cursor: pointer;
		flex-shrink: 0;
		display: inline-flex;
		align-items: center;
		transition:
			background-color var(--duration-150) var(--ease-out),
			border-color var(--duration-150) var(--ease-out);
	}
	.switch[aria-checked='true'] {
		background-color: var(--brand-teal-500);
		border-color: var(--brand-teal-500);
	}
	.switch[aria-disabled='true'],
	.switch:disabled {
		cursor: not-allowed;
		opacity: 0.65;
	}

	.switch-thumb {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		inline-size: 1.125rem;
		block-size: 1.125rem;
		border-radius: var(--radius-full);
		background-color: var(--surface-bg-canvas);
		color: var(--brand-teal-700);
		box-shadow: var(--shadow-sm);
		transform: translateX(0);
		transition: transform var(--duration-150) var(--ease-out);
	}
	.switch[aria-checked='true'] .switch-thumb {
		transform: translateX(1.25rem);
	}

	@media (prefers-reduced-motion: reduce) {
		.switch,
		.switch-thumb {
			transition: none;
		}
	}

	.sr-only {
		position: absolute;
		inline-size: 1px;
		block-size: 1px;
		padding: 0;
		margin: -1px;
		overflow: hidden;
		clip-path: inset(50%);
		white-space: nowrap;
		border: 0;
	}
</style>
