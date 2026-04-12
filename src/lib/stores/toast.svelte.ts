// Svelte 5 runes-based toast store
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
