/**
 * Global toast notification store.
 *
 * Svelte 5 runes-backed (`$state`) singleton consumed via `toast.success()`,
 * `toast.error()`, `toast.warning()`, `toast.info()`. Paired with
 * `<Toaster />` (`$lib/components/ui/Toaster.svelte`) which is mounted once
 * in the root layout and renders the live stack.
 *
 * Replaces the legacy ad-hoc per-page `flash()` + local `toast` string state,
 * `alert()` calls, and silent `console.error` failures across the admin app.
 *
 * Sister store note: a separate `toasts` (plural) store at
 * `$lib/stores/toasts.svelte.ts` powers the older PE7 `<ToastRegion />`
 * primitive used in a couple of public-shell consumers (forms,
 * `_ui-kit`). Both can coexist; new code should prefer this one.
 */

export type ToastVariant = 'success' | 'error' | 'warning' | 'info';

export interface ToastAction {
	readonly label: string;
	readonly onClick: () => void;
}

export interface ToastOptions {
	/** Optional second-line description rendered under the title. */
	description?: string;
	/** Milliseconds before auto-dismiss. `0` = sticky (no auto-dismiss). */
	duration?: number;
	/** Inline action button. The handler does NOT auto-dismiss the toast. */
	action?: ToastAction;
}

export interface ToastItem {
	readonly id: string;
	readonly variant: ToastVariant;
	readonly title: string;
	readonly description?: string;
	readonly duration: number;
	readonly action?: ToastAction;
	readonly createdAt: number;
}

const DEFAULT_DURATION_BY_VARIANT: Record<ToastVariant, number> = {
	success: 3500,
	info: 3500,
	warning: 5000,
	error: 6000
};

/** Maximum number of toasts visible at once before the oldest is evicted. */
const MAX_VISIBLE = 5;

function generateId(): string {
	if (typeof crypto !== 'undefined' && typeof crypto.randomUUID === 'function') {
		return crypto.randomUUID();
	}
	return `t_${Math.random().toString(36).slice(2)}_${Date.now().toString(36)}`;
}

class ToastStore {
	/**
	 * Reactive list of currently visible toasts. Newest at the end of the
	 * array; the renderer reverses for display so the freshest sits on top.
	 */
	readonly items = $state<ToastItem[]>([]);

	private push(variant: ToastVariant, title: string, options: ToastOptions = {}): string {
		const id = generateId();
		const duration =
			typeof options.duration === 'number'
				? options.duration
				: DEFAULT_DURATION_BY_VARIANT[variant];
		const item: ToastItem = {
			id,
			variant,
			title,
			description: options.description,
			duration,
			action: options.action,
			createdAt: Date.now()
		};
		this.items.push(item);
		// Evict the oldest (front of the queue) if we're past capacity. Keeps
		// the renderer simple: it just iterates `items` and animates exits via
		// keyed `{#each}`.
		while (this.items.length > MAX_VISIBLE) {
			this.items.shift();
		}
		return id;
	}

	success(title: string, options?: ToastOptions): string {
		return this.push('success', title, options);
	}

	error(title: string, options?: ToastOptions): string {
		return this.push('error', title, options);
	}

	warning(title: string, options?: ToastOptions): string {
		return this.push('warning', title, options);
	}

	info(title: string, options?: ToastOptions): string {
		return this.push('info', title, options);
	}

	/** Remove a toast by id. Returns `true` if a row was removed. */
	dismiss(id: string): boolean {
		const idx = this.items.findIndex((t) => t.id === id);
		if (idx === -1) return false;
		this.items.splice(idx, 1);
		return true;
	}

	/** Clear every toast (e.g. on auth sign-out, route resets). */
	clear(): void {
		this.items.length = 0;
	}
}

export const toast = new ToastStore();
