<!--
  Toast — single non-modal announcement.

  A11y:
  - `role="status"` (polite) for info / success / warning, `role="alert"`
    (assertive) for danger, matching WAI-ARIA live-region guidance.
  - `aria-live` + `aria-atomic` ensure the full content is re-announced on
    change, rather than diffing.
  - Close control has an explicit accessible name; never icon-only without label.
  - Auto-dismiss timer is paused on hover / focus-within so slow readers and
    keyboard users are not penalised.
  - Reduced motion: removes enter animation; timer keeps running but without
    a fade-out transform.
-->
<script lang="ts" module>
	export type { ToastKind, ToastProps } from './Toast.types';
</script>

<script lang="ts">
	import type { Attachment } from 'svelte/attachments';
	import type { ToastProps as Props } from './Toast.types';
	import XIcon from 'phosphor-svelte/lib/XIcon';
	import InfoIcon from 'phosphor-svelte/lib/InfoIcon';
	import CheckCircleIcon from 'phosphor-svelte/lib/CheckCircleIcon';
	import WarningIcon from 'phosphor-svelte/lib/WarningIcon';
	import WarningOctagonIcon from 'phosphor-svelte/lib/WarningOctagonIcon';

	const {
		id,
		kind = 'info',
		title,
		description,
		duration = 5000,
		onclose,
		icon
	}: Props = $props();

	const liveRole = $derived(kind === 'danger' ? 'alert' : 'status');
	const livePoliteness = $derived(kind === 'danger' ? 'assertive' : 'polite');

	let paused = $state(false);

	/**
	 * Auto-dismiss attachment. Only runs when `duration > 0` and `paused` is
	 * false. The attachment reads `paused` (reactive), so Svelte re-runs the
	 * factory on pause/resume — clearing the old timer via the returned
	 * cleanup and re-arming a fresh one. Remaining-time accounting is therefore
	 * not preserved across pauses; a full `duration` is granted on resume,
	 * which is the friendlier behaviour for screen-reader + keyboard users.
	 */
	const autoDismiss: Attachment<HTMLElement> = () => {
		if (duration <= 0 || paused) return;
		const timer = setTimeout(() => onclose?.(id), duration);
		return () => clearTimeout(timer);
	};
</script>

<div
	class="toast"
	data-kind={kind}
	role={liveRole}
	aria-live={livePoliteness}
	aria-atomic="true"
	onmouseenter={() => (paused = true)}
	onmouseleave={() => (paused = false)}
	onfocusin={() => (paused = true)}
	onfocusout={() => (paused = false)}
	{@attach autoDismiss}
>
	<span class="icon" aria-hidden="true">
		{#if icon}
			{@render icon()}
		{:else if kind === 'success'}
			<CheckCircleIcon size="1.25rem" weight="fill" />
		{:else if kind === 'warning'}
			<WarningIcon size="1.25rem" weight="fill" />
		{:else if kind === 'danger'}
			<WarningOctagonIcon size="1.25rem" weight="fill" />
		{:else}
			<InfoIcon size="1.25rem" weight="fill" />
		{/if}
	</span>
	<div class="body">
		<p class="title">{title}</p>
		{#if description}<p class="description">{description}</p>{/if}
	</div>
	<button
		type="button"
		class="close"
		aria-label="Dismiss notification"
		onclick={() => onclose?.(id)}
	>
		<XIcon size="1rem" weight="bold" aria-hidden="true" />
	</button>
</div>

<style>
	.toast {
		display: grid;
		grid-template-columns: auto 1fr auto;
		align-items: flex-start;
		gap: var(--space-3);
		padding-block: var(--space-3);
		padding-inline: var(--space-4);
		inline-size: min(24rem, calc(100vw - var(--space-8)));
		background-color: var(--surface-bg-canvas);
		color: var(--surface-fg-default);
		border: 1px solid var(--surface-border-subtle);
		border-radius: var(--radius-lg);
		box-shadow: var(--shadow-lg);
		animation: toast-enter var(--duration-200) var(--ease-spring);
		pointer-events: auto;
	}

	.toast[data-kind='info'] {
		border-inline-start: 3px solid var(--status-info-500);
	}
	.toast[data-kind='success'] {
		border-inline-start: 3px solid var(--status-success-500);
	}
	.toast[data-kind='warning'] {
		border-inline-start: 3px solid var(--status-warning-500);
	}
	.toast[data-kind='danger'] {
		border-inline-start: 3px solid var(--status-danger-500);
	}

	.icon {
		line-height: 0;
		padding-block-start: var(--space-0-5);
	}
	.toast[data-kind='info'] .icon {
		color: var(--status-info-700);
	}
	.toast[data-kind='success'] .icon {
		color: var(--status-success-700);
	}
	.toast[data-kind='warning'] .icon {
		color: var(--status-warning-700);
	}
	.toast[data-kind='danger'] .icon {
		color: var(--status-danger-700);
	}

	.body {
		min-inline-size: 0;
	}
	.title {
		margin: 0;
		font-weight: var(--w-semibold);
		font-size: var(--fs-sm);
		line-height: var(--lh-snug);
	}
	.description {
		margin-block-start: var(--space-1);
		margin-block-end: 0;
		font-size: var(--fs-xs);
		color: var(--surface-fg-muted);
		line-height: var(--lh-normal);
	}

	.close {
		appearance: none;
		background: transparent;
		border: 0;
		padding: var(--space-1);
		margin-block-start: calc(-1 * var(--space-1));
		border-radius: var(--radius-default);
		color: var(--surface-fg-muted);
		cursor: pointer;
		display: inline-flex;
		align-items: center;
		justify-content: center;
	}
	.close:hover {
		color: var(--surface-fg-default);
		background-color: var(--surface-bg-muted);
	}

	@keyframes toast-enter {
		from {
			opacity: 0;
			transform: translateY(8px);
		}
		to {
			opacity: 1;
			transform: translateY(0);
		}
	}
	@media (prefers-reduced-motion: reduce) {
		.toast {
			animation: none;
		}
	}
</style>
