<script lang="ts">
	import { onMount, tick } from 'svelte';
	import XCircleIcon from 'phosphor-svelte/lib/XCircleIcon';
	import WarningIcon from 'phosphor-svelte/lib/WarningIcon';
	import InfoIcon from 'phosphor-svelte/lib/InfoIcon';
	import XIcon from 'phosphor-svelte/lib/XIcon';

	type Variant = 'danger' | 'warning' | 'info';

	interface Props {
		title: string;
		message?: string;
		confirmLabel?: string;
		cancelLabel?: string;
		variant?: Variant;
		onresolve: (ok: boolean) => void;
	}

	let {
		title,
		message = '',
		confirmLabel = 'Confirm',
		cancelLabel = 'Cancel',
		variant = 'info',
		onresolve
	}: Props = $props();

	let panelRef: HTMLDivElement | undefined = $state();
	let cancelBtnRef: HTMLButtonElement | undefined = $state();
	let closing = $state(false);

	function close(value: boolean): void {
		if (closing) return;
		closing = true;
		const reduced =
			typeof window !== 'undefined' &&
			window.matchMedia?.('(prefers-reduced-motion: reduce)').matches;
		const delay = reduced ? 0 : 150;
		window.setTimeout(() => onresolve(value), delay);
	}

	function handleKeydown(e: KeyboardEvent): void {
		if (e.key === 'Escape') {
			e.preventDefault();
			e.stopPropagation();
			close(false);
			return;
		}

		if (e.key === 'Tab' && panelRef) {
			const focusable = panelRef.querySelectorAll<HTMLElement>(
				'button:not([disabled]), [href], input:not([disabled]), select:not([disabled]), textarea:not([disabled]), [tabindex]:not([tabindex="-1"])'
			);
			if (focusable.length === 0) return;
			const first = focusable[0];
			const last = focusable[focusable.length - 1];
			const active = document.activeElement as HTMLElement | null;

			if (e.shiftKey) {
				if (active === first || !panelRef.contains(active)) {
					e.preventDefault();
					last.focus();
				}
			} else {
				if (active === last) {
					e.preventDefault();
					first.focus();
				}
			}
		}
	}

	function handleBackdropPointerDown(e: PointerEvent): void {
		// Only close when the gesture actually started on the backdrop, not
		// when the user clicks inside the panel and drags onto the backdrop.
		if (e.target === e.currentTarget) {
			close(false);
		}
	}

	onMount(() => {
		const previouslyFocused = document.activeElement as HTMLElement | null;
		const prevOverflow = document.body.style.overflow;
		document.body.style.overflow = 'hidden';

		void tick().then(() => {
			cancelBtnRef?.focus({ preventScroll: true });
		});

		return () => {
			document.body.style.overflow = prevOverflow;
			// Focus restoration is handled by the store after resolve so it
			// can target the original trigger element rather than whatever
			// happened to hold focus when the host unmounted us.
			void previouslyFocused;
		};
	});
</script>

<svelte:window onkeydown={handleKeydown} />

<div
	class="confirm-backdrop"
	class:confirm-backdrop--closing={closing}
	role="presentation"
	onpointerdown={handleBackdropPointerDown}
>
	<div
		bind:this={panelRef}
		class="confirm-panel confirm-panel--{variant}"
		class:confirm-panel--closing={closing}
		role="alertdialog"
		aria-modal="true"
		aria-labelledby="confirm-title-{variant}"
		aria-describedby={message ? `confirm-message-${variant}` : undefined}
	>
		<button
			type="button"
			class="confirm-close"
			aria-label="Close dialog"
			onclick={() => close(false)}
		>
			<XIcon size={20} weight="bold" />
		</button>

		<div class="confirm-body">
			<div class="confirm-icon-chip" aria-hidden="true">
				{#if variant === 'danger'}
					<XCircleIcon size={28} weight="duotone" />
				{:else if variant === 'warning'}
					<WarningIcon size={28} weight="duotone" />
				{:else}
					<InfoIcon size={28} weight="duotone" />
				{/if}
			</div>

			<div class="confirm-text">
				<h2 id="confirm-title-{variant}" class="confirm-title">{title}</h2>
				{#if message}
					<p id="confirm-message-{variant}" class="confirm-message">{message}</p>
				{/if}
			</div>
		</div>

		<div class="confirm-footer">
			<button
				bind:this={cancelBtnRef}
				type="button"
				class="confirm-btn confirm-btn--cancel"
				onclick={() => close(false)}
			>
				{cancelLabel}
			</button>
			<button
				type="button"
				class="confirm-btn confirm-btn--confirm confirm-btn--{variant}"
				onclick={() => close(true)}
			>
				{confirmLabel}
			</button>
		</div>
	</div>
</div>

<style>
	.confirm-backdrop {
		position: fixed;
		inset: 0;
		z-index: 9000;
		display: flex;
		align-items: center;
		justify-content: center;
		padding: 1rem;
		background: rgba(0, 0, 0, 0.6);
		backdrop-filter: blur(8px);
		-webkit-backdrop-filter: blur(8px);
		animation: confirm-backdrop-in 200ms ease-out both;
	}

	.confirm-backdrop--closing {
		animation: confirm-backdrop-out 150ms ease-in both;
	}

	.confirm-panel {
		position: relative;
		width: clamp(320px, 92vw, 440px);
		padding: 1.5rem;
		background: var(--color-navy-deep, #060e1f);
		border: 1px solid rgba(255, 255, 255, 0.12);
		border-radius: var(--radius-xl, 1rem);
		box-shadow:
			0 24px 64px rgba(0, 0, 0, 0.5),
			0 1px 0 rgba(255, 255, 255, 0.06) inset;
		animation: confirm-panel-in 200ms cubic-bezier(0.16, 1, 0.3, 1) both;
	}

	.confirm-panel--closing {
		animation: confirm-panel-out 150ms ease-in both;
	}

	@keyframes confirm-backdrop-in {
		from {
			opacity: 0;
		}
		to {
			opacity: 1;
		}
	}

	@keyframes confirm-backdrop-out {
		from {
			opacity: 1;
		}
		to {
			opacity: 0;
		}
	}

	@keyframes confirm-panel-in {
		from {
			opacity: 0;
			transform: scale(0.96);
		}
		to {
			opacity: 1;
			transform: scale(1);
		}
	}

	@keyframes confirm-panel-out {
		from {
			opacity: 1;
			transform: scale(1);
		}
		to {
			opacity: 0;
			transform: scale(0.97);
		}
	}

	@media (prefers-reduced-motion: reduce) {
		.confirm-backdrop,
		.confirm-backdrop--closing,
		.confirm-panel,
		.confirm-panel--closing {
			animation: none !important;
		}
	}

	.confirm-close {
		position: absolute;
		top: 0.75rem;
		right: 0.75rem;
		display: inline-flex;
		align-items: center;
		justify-content: center;
		width: 2rem;
		height: 2rem;
		border: none;
		border-radius: 0.5rem;
		background: transparent;
		color: var(--color-grey-400, #9ca3af);
		cursor: pointer;
		transition:
			background-color 150ms ease,
			color 150ms ease;
	}

	.confirm-close:hover {
		background: rgba(255, 255, 255, 0.06);
		color: var(--color-grey-200, #e5e7eb);
	}

	.confirm-close:focus-visible {
		outline: none;
		box-shadow: 0 0 0 2px rgba(125, 211, 252, 0.7);
		color: var(--color-grey-200, #e5e7eb);
	}

	.confirm-body {
		display: flex;
		gap: 1rem;
		align-items: flex-start;
		padding-right: 1.75rem; /* leave room for the X */
		margin-bottom: 1.25rem;
	}

	.confirm-icon-chip {
		flex: 0 0 auto;
		display: inline-flex;
		align-items: center;
		justify-content: center;
		width: 48px;
		height: 48px;
		border-radius: 12px;
	}

	.confirm-panel--danger .confirm-icon-chip {
		background: rgba(239, 68, 68, 0.12);
		color: #fca5a5;
	}

	.confirm-panel--warning .confirm-icon-chip {
		background: rgba(245, 158, 11, 0.12);
		color: #fcd34d;
	}

	.confirm-panel--info .confirm-icon-chip {
		background: rgba(125, 211, 252, 0.12);
		color: #7dd3fc;
	}

	.confirm-text {
		flex: 1 1 auto;
		min-width: 0;
	}

	.confirm-title {
		margin: 0 0 0.375rem 0;
		font-family: var(--font-heading, inherit);
		font-size: 16px;
		font-weight: var(--w-semibold, 600);
		line-height: 1.3;
		color: var(--color-white, #fff);
	}

	.confirm-message {
		margin: 0;
		font-family: var(--font-ui, inherit);
		font-size: 14px;
		line-height: 1.5;
		color: var(--color-grey-300, #d1d5db);
	}

	.confirm-footer {
		display: flex;
		justify-content: flex-end;
		gap: 0.5rem;
	}

	.confirm-btn {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		min-height: 2.5rem;
		padding: 0 1rem;
		font-family: var(--font-ui, inherit);
		font-size: var(--fs-sm, 0.875rem);
		font-weight: var(--w-semibold, 600);
		line-height: 1.2;
		border-radius: 0.625rem;
		border: 1px solid transparent;
		cursor: pointer;
		transition:
			background-color 150ms ease,
			border-color 150ms ease,
			color 150ms ease,
			box-shadow 150ms ease,
			transform 80ms ease;
	}

	.confirm-btn:active {
		transform: translateY(1px);
	}

	.confirm-btn:focus-visible {
		outline: none;
		box-shadow:
			0 0 0 2px var(--color-navy-deep, #060e1f),
			0 0 0 4px rgba(125, 211, 252, 0.7);
	}

	.confirm-btn--cancel {
		background: rgba(255, 255, 255, 0.05);
		border-color: rgba(255, 255, 255, 0.1);
		color: var(--color-grey-200, #e5e7eb);
	}

	.confirm-btn--cancel:hover {
		background: rgba(255, 255, 255, 0.09);
		border-color: rgba(255, 255, 255, 0.16);
		color: var(--color-white, #fff);
	}

	.confirm-btn--danger {
		background: rgba(239, 68, 68, 0.15);
		border-color: rgba(239, 68, 68, 0.4);
		color: #fca5a5;
	}

	.confirm-btn--danger:hover {
		background: rgba(239, 68, 68, 0.28);
		border-color: rgba(239, 68, 68, 0.6);
		color: #fee2e2;
	}

	.confirm-btn--warning {
		background: rgba(245, 158, 11, 0.15);
		border-color: rgba(245, 158, 11, 0.4);
		color: #fcd34d;
	}

	.confirm-btn--warning:hover {
		background: rgba(245, 158, 11, 0.28);
		border-color: rgba(245, 158, 11, 0.6);
		color: #fef3c7;
	}

	.confirm-btn--info {
		background: linear-gradient(180deg, rgba(125, 211, 252, 0.22), rgba(20, 184, 166, 0.22));
		border-color: rgba(125, 211, 252, 0.4);
		color: #ecfeff;
	}

	.confirm-btn--info:hover {
		background: linear-gradient(180deg, rgba(125, 211, 252, 0.34), rgba(20, 184, 166, 0.34));
		border-color: rgba(125, 211, 252, 0.6);
	}

	@media (max-width: 480px) {
		.confirm-footer {
			flex-direction: column-reverse;
		}

		.confirm-btn {
			width: 100%;
		}
	}
</style>
