// Imperative confirm-dialog store.
//
// Pairs with `$lib/components/ui/ConfirmDialogHost.svelte` (mounted once in
// the root layout) and `$lib/components/ui/ConfirmDialog.svelte` (the
// presentation primitive). Consumers call `confirmDialog({ ... })` and
// `await` a `Promise<boolean>` instead of wiring `bind:open` on every page.
//
// Multiple concurrent requests queue and present FIFO so a destructive
// action triggered while another modal is open does not race.

export type ConfirmVariant = 'danger' | 'warning' | 'info';

export interface ConfirmOptions {
	title: string;
	message?: string | undefined;
	confirmLabel?: string | undefined;
	cancelLabel?: string | undefined;
	variant?: ConfirmVariant | undefined;
}

export interface ConfirmRequest extends ConfirmOptions {
	id: string;
	resolve: (ok: boolean) => void;
	trigger: HTMLElement | null;
}

class ConfirmStore {
	queue = $state<ConfirmRequest[]>([]);

	get current(): ConfirmRequest | null {
		return this.queue[0] ?? null;
	}

	ask(opts: ConfirmOptions): Promise<boolean> {
		return new Promise<boolean>((resolve) => {
			const trigger =
				typeof document !== 'undefined'
					? (document.activeElement as HTMLElement | null)
					: null;
			this.queue.push({
				id:
					typeof crypto !== 'undefined' && 'randomUUID' in crypto
						? crypto.randomUUID()
						: `confirm-${Date.now()}-${Math.random().toString(36).slice(2)}`,
				title: opts.title,
				message: opts.message,
				confirmLabel: opts.confirmLabel,
				cancelLabel: opts.cancelLabel,
				variant: opts.variant ?? 'info',
				trigger,
				resolve
			});
		});
	}

	resolveCurrent(value: boolean): void {
		const c = this.queue[0];
		if (!c) return;
		c.resolve(value);
		// Restore focus to the element that opened this dialog (if still in
		// the DOM and focusable). Defer one frame so the dialog has time to
		// unmount and any post-resolve work can refocus its own target.
		const trigger = c.trigger;
		this.queue = this.queue.slice(1);
		if (trigger && typeof queueMicrotask === 'function') {
			queueMicrotask(() => {
				if (trigger.isConnected) {
					try {
						trigger.focus({ preventScroll: true });
					} catch {
						/* ignore */
					}
				}
			});
		}
	}
}

export const confirms = new ConfirmStore();

/**
 * Open an in-app confirmation modal and await the user's choice.
 *
 * Resolves `true` when the user clicks the confirm button, `false` for
 * cancel / backdrop / Escape / X close. Never rejects.
 */
export function confirmDialog(opts: ConfirmOptions): Promise<boolean> {
	return confirms.ask(opts);
}
