<script lang="ts">
	interface Props {
		open: boolean;
		title?: string;
		message?: string;
		confirmText?: string;
		cancelText?: string;
		variant?: 'danger' | 'warning' | 'info';
		onconfirm?: () => void;
		oncancel?: () => void;
	}

	let {
		open = $bindable(false),
		title = 'Are you sure?',
		message = '',
		confirmText = 'Confirm',
		cancelText = 'Cancel',
		variant = 'info',
		onconfirm,
		oncancel
	}: Props = $props();

	let dialogRef: HTMLDialogElement | undefined = $state();
	let previouslyFocused: HTMLElement | null = null;

	function handleConfirm() {
		open = false;
		onconfirm?.();
	}

	function handleCancel() {
		open = false;
		oncancel?.();
	}

	function handleKeydown(e: KeyboardEvent) {
		if (e.key === 'Escape') {
			e.preventDefault();
			handleCancel();
			return;
		}

		if (e.key === 'Tab' && dialogRef) {
			const focusable = dialogRef.querySelectorAll<HTMLElement>(
				'button, [href], input, select, textarea, [tabindex]:not([tabindex="-1"])'
			);
			const first = focusable[0];
			const last = focusable[focusable.length - 1];

			if (e.shiftKey) {
				if (document.activeElement === first) {
					e.preventDefault();
					last?.focus();
				}
			} else {
				if (document.activeElement === last) {
					e.preventDefault();
					first?.focus();
				}
			}
		}
	}

	function handleBackdropClick(e: MouseEvent) {
		if (e.target === dialogRef) {
			handleCancel();
		}
	}

	$effect(() => {
		if (open && dialogRef) {
			previouslyFocused = document.activeElement as HTMLElement;
			dialogRef.showModal();
			const confirmBtn = dialogRef.querySelector<HTMLElement>('.confirm-dialog__btn--cancel');
			confirmBtn?.focus();
		} else if (!open && dialogRef) {
			dialogRef.close();
			previouslyFocused?.focus();
			previouslyFocused = null;
		}
	});
</script>

{#if open}
	<dialog
		bind:this={dialogRef}
		class="confirm-dialog"
		onkeydown={handleKeydown}
		onclick={handleBackdropClick}
	>
		<div class="confirm-dialog__panel confirm-dialog--{variant}">
			<div class="confirm-dialog__icon">
				{#if variant === 'danger'}
					<svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
						<circle cx="12" cy="12" r="10"/>
						<line x1="15" y1="9" x2="9" y2="15"/>
						<line x1="9" y1="9" x2="15" y2="15"/>
					</svg>
				{:else if variant === 'warning'}
					<svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
						<path d="M10.29 3.86L1.82 18a2 2 0 0 0 1.71 3h16.94a2 2 0 0 0 1.71-3L13.71 3.86a2 2 0 0 0-3.42 0z"/>
						<line x1="12" y1="9" x2="12" y2="13"/>
						<line x1="12" y1="17" x2="12.01" y2="17"/>
					</svg>
				{:else}
					<svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
						<circle cx="12" cy="12" r="10"/>
						<line x1="12" y1="16" x2="12" y2="12"/>
						<line x1="12" y1="8" x2="12.01" y2="8"/>
					</svg>
				{/if}
			</div>

			<h2 class="confirm-dialog__title">{title}</h2>

			{#if message}
				<p class="confirm-dialog__message">{message}</p>
			{/if}

			<div class="confirm-dialog__actions">
				<button
					class="confirm-dialog__btn confirm-dialog__btn--cancel"
					onclick={handleCancel}
				>
					{cancelText}
				</button>
				<button
					class="confirm-dialog__btn confirm-dialog__btn--confirm confirm-dialog__btn--{variant}"
					onclick={handleConfirm}
				>
					{confirmText}
				</button>
			</div>
		</div>
	</dialog>
{/if}

<style>
	.confirm-dialog {
		position: fixed;
		inset: 0;
		display: flex;
		align-items: center;
		justify-content: center;
		border: none;
		background: transparent;
		padding: var(--space-4);
		max-width: 100vw;
		max-height: 100vh;
		width: 100%;
		height: 100%;
	}

	.confirm-dialog::backdrop {
		background: rgba(11, 29, 58, 0.7);
		backdrop-filter: blur(8px);
		-webkit-backdrop-filter: blur(8px);
	}

	.confirm-dialog__panel {
		background: var(--color-navy-mid);
		border: 1px solid rgba(255, 255, 255, 0.1);
		border-radius: var(--radius-2xl);
		padding: var(--space-8);
		max-width: 440px;
		width: 100%;
		box-shadow: var(--shadow-2xl);
		text-align: center;
		animation: confirm-enter var(--duration-300) var(--ease-out) forwards;
	}

	@keyframes confirm-enter {
		from {
			opacity: 0;
			transform: scale(0.9);
		}
		to {
			opacity: 1;
			transform: scale(1);
		}
	}

	.confirm-dialog__icon {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		width: 48px;
		height: 48px;
		border-radius: var(--radius-full);
		margin-bottom: var(--space-4);
	}

	.confirm-dialog--danger .confirm-dialog__icon {
		background: rgba(224, 72, 72, 0.15);
		color: var(--color-red);
	}

	.confirm-dialog--warning .confirm-dialog__icon {
		background: rgba(212, 168, 67, 0.15);
		color: var(--color-gold);
	}

	.confirm-dialog--info .confirm-dialog__icon {
		background: rgba(15, 164, 175, 0.15);
		color: var(--color-teal);
	}

	.confirm-dialog__title {
		font-family: var(--font-heading);
		font-size: var(--fs-lg);
		font-weight: var(--w-semibold);
		color: var(--color-white);
		margin: 0 0 var(--space-2) 0;
		line-height: var(--lh-snug);
	}

	.confirm-dialog__message {
		font-family: var(--font-ui);
		font-size: var(--fs-sm);
		color: var(--color-grey-400);
		line-height: var(--lh-relaxed);
		margin: 0 0 var(--space-6) 0;
	}

	.confirm-dialog__actions {
		display: flex;
		gap: var(--space-3);
		justify-content: center;
	}

	.confirm-dialog__btn {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		padding: var(--space-2-5) var(--space-6);
		border-radius: var(--radius-xl);
		font-family: var(--font-ui);
		font-weight: var(--w-semibold);
		font-size: var(--fs-sm);
		cursor: pointer;
		transition: all var(--duration-200) var(--ease-out);
		border: none;
		line-height: var(--lh-normal);
	}

	.confirm-dialog__btn:focus-visible {
		outline: none;
		box-shadow:
			0 0 0 2px var(--color-navy),
			0 0 0 4px rgba(15, 164, 175, 0.7);
	}

	.confirm-dialog__btn--cancel {
		background: rgba(255, 255, 255, 0.08);
		color: var(--color-grey-300);
		border: 1px solid rgba(255, 255, 255, 0.1);
	}

	.confirm-dialog__btn--cancel:hover {
		background: rgba(255, 255, 255, 0.14);
		color: var(--color-white);
	}

	.confirm-dialog__btn--confirm {
		color: var(--color-white);
	}

	.confirm-dialog__btn--danger {
		background: var(--color-red);
		box-shadow: 0 4px 14px rgba(224, 72, 72, 0.25);
	}

	.confirm-dialog__btn--danger:hover {
		background: #c93c3c;
		box-shadow: 0 6px 18px rgba(224, 72, 72, 0.35);
	}

	.confirm-dialog__btn--warning {
		background: var(--color-gold);
		color: var(--color-navy);
		box-shadow: 0 4px 14px rgba(212, 168, 67, 0.25);
	}

	.confirm-dialog__btn--warning:hover {
		background: var(--color-gold-light);
		box-shadow: 0 6px 18px rgba(212, 168, 67, 0.35);
	}

	.confirm-dialog__btn--info {
		background: var(--color-teal);
		box-shadow: 0 4px 14px rgba(15, 164, 175, 0.25);
	}

	.confirm-dialog__btn--info:hover {
		background: var(--color-teal-light);
		box-shadow: 0 6px 18px rgba(15, 164, 175, 0.35);
	}

	@media (max-width: 480px) {
		.confirm-dialog__panel {
			padding: var(--space-6);
		}

		.confirm-dialog__actions {
			flex-direction: column-reverse;
		}

		.confirm-dialog__btn {
			width: 100%;
		}
	}
</style>
