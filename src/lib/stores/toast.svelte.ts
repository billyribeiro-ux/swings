// Svelte 5 runes-based toast store.
//
// @deprecated Use `toasts` from `$lib/stores/toasts.svelte.ts` instead — that
// is the canonical PE7 toast store paired with `ToastRegion` from
// `$lib/components/shared`. This legacy store pairs with the orphaned
// `$lib/components/ui/Toast.svelte` and has no active consumers; kept until
// the next minor release to avoid breaking any out-of-tree imports.
export type ToastType = 'success' | 'error' | 'warning' | 'info';

export interface Toast {
	id: string;
	type: ToastType;
	message: string;
	duration: number;
	createdAt: number;
}

class ToastState {
	toasts = $state<Toast[]>([]);
	private timers = new Map<string, ReturnType<typeof setTimeout>>();

	add(type: ToastType, message: string, duration = 4000) {
		const id = crypto.randomUUID();
		const newToast: Toast = { id, type, message, duration, createdAt: Date.now() };
		this.toasts.push(newToast);

		const timer = setTimeout(() => {
			this.remove(id);
		}, duration);
		this.timers.set(id, timer);
	}

	success(message: string) {
		this.add('success', message);
	}

	error(message: string) {
		this.add('error', message);
	}

	warning(message: string) {
		this.add('warning', message);
	}

	info(message: string) {
		this.add('info', message);
	}

	remove(id: string) {
		const timer = this.timers.get(id);
		if (timer) {
			clearTimeout(timer);
			this.timers.delete(id);
		}
		this.toasts = this.toasts.filter((t) => t.id !== id);
	}
}

export const toast = new ToastState();
