<!--
  ToastRegion — portal-style container that hosts the shared toast stack.

  A11y:
  - The region itself is a live region (aria-live="polite"), so new toasts
    are announced even if the toast itself has not yet hooked its own live
    region. Individual toasts still use role="status" / role="alert" for
    politeness control per-item.
  - Positioned with logical properties so `bottom-end` in LTR == bottom-right,
    `bottom-end` in RTL == bottom-left.
-->
<script lang="ts" module>
	export type ToastRegionPosition =
		| 'top-start'
		| 'top-end'
		| 'top-center'
		| 'bottom-start'
		| 'bottom-end'
		| 'bottom-center';

	export interface ToastRegionProps {
		position?: ToastRegionPosition;
		/** Optional custom store — defaults to the shared `toasts` instance. */
		store?: ToastStore;
		label?: string;
	}
</script>

<script lang="ts">
	import Toast from './Toast.svelte';
	import { toasts as defaultStore, ToastStore } from '$lib/stores/toasts.svelte';

	const { position = 'bottom-end', store = defaultStore, label = 'Notifications' }: ToastRegionProps =
		$props();
</script>

<div
	class="toast-region"
	data-position={position}
	role="region"
	aria-label={label}
	aria-live="polite"
>
	{#each store.items as item (item.id)}
		<Toast
			id={item.id}
			kind={item.kind}
			title={item.title}
			description={item.description}
			duration={item.duration}
			onclose={(id) => {
				if (id) store.remove(id);
			}}
		/>
	{/each}
</div>

<style>
	.toast-region {
		position: fixed;
		z-index: var(--z-50);
		display: flex;
		flex-direction: column;
		gap: var(--space-3);
		padding: var(--space-4);
		pointer-events: none;
		inline-size: max-content;
		max-inline-size: calc(100vw - var(--space-8));
	}

	.toast-region[data-position='top-start'] {
		inset-block-start: 0;
		inset-inline-start: 0;
		align-items: flex-start;
	}
	.toast-region[data-position='top-end'] {
		inset-block-start: 0;
		inset-inline-end: 0;
		align-items: flex-end;
	}
	.toast-region[data-position='top-center'] {
		inset-block-start: 0;
		inset-inline-start: 50%;
		transform: translateX(-50%);
		align-items: center;
	}
	.toast-region[data-position='bottom-start'] {
		inset-block-end: 0;
		inset-inline-start: 0;
		align-items: flex-start;
		flex-direction: column-reverse;
	}
	.toast-region[data-position='bottom-end'] {
		inset-block-end: 0;
		inset-inline-end: 0;
		align-items: flex-end;
		flex-direction: column-reverse;
	}
	.toast-region[data-position='bottom-center'] {
		inset-block-end: 0;
		inset-inline-start: 50%;
		transform: translateX(-50%);
		align-items: center;
		flex-direction: column-reverse;
	}

	.toast-region > :global(*) {
		pointer-events: auto;
	}
</style>
