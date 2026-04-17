/**
 * Canonical toast store for PE7 UI primitives.
 *
 * Runes-backed (`$state`) — consume from any `.svelte` / `.svelte.ts` module.
 * Paired with `<ToastRegion />` from `$lib/components/shared`.
 *
 * Scope: intentionally separate from the legacy `toast.svelte.ts` (singular)
 * which targeted the older `Toast.svelte` in `$lib/components/ui`. This module
 * belongs to the PE7 shared primitive tier and consumers (CONSENT, EC, FORM)
 * should import exclusively from here.
 */

export type ToastKind = 'info' | 'success' | 'warning' | 'danger';

export interface ToastItem {
	readonly id: string;
	readonly kind: ToastKind;
	readonly title: string;
	readonly description?: string;
	/** Milliseconds before auto-dismiss. `0` = persistent (dismiss manually). */
	readonly duration: number;
	readonly createdAt: number;
}

export interface ToastInput {
	kind?: ToastKind;
	title: string;
	description?: string;
	duration?: number;
}

const DEFAULT_DURATION_MS = 5_000;
/** Safety cap so runaway producers cannot OOM the UI. */
const MAX_TOASTS = 10;

function generateId(): string {
	if (typeof crypto !== 'undefined' && typeof crypto.randomUUID === 'function') {
		return crypto.randomUUID();
	}
	return `toast_${Math.random().toString(36).slice(2)}_${Date.now().toString(36)}`;
}

class ToastStore {
	readonly items = $state<ToastItem[]>([]);
	readonly maxToasts: number;

	constructor(maxToasts: number = MAX_TOASTS) {
		this.maxToasts = maxToasts;
	}

	/** Push a new toast. Oldest is evicted when capacity is exceeded. */
	push(input: ToastInput): string {
		const id = generateId();
		const item: ToastItem = {
			id,
			kind: input.kind ?? 'info',
			title: input.title,
			description: input.description,
			duration: input.duration ?? DEFAULT_DURATION_MS,
			createdAt: Date.now()
		};
		this.items.push(item);
		while (this.items.length > this.maxToasts) {
			this.items.shift();
		}
		return id;
	}

	/** Remove a toast by id. Returns `true` when an entry was removed. */
	remove(id: string): boolean {
		const idx = this.items.findIndex((t) => t.id === id);
		if (idx === -1) return false;
		this.items.splice(idx, 1);
		return true;
	}

	/** Clear every toast (e.g. on route transitions). */
	clear(): void {
		this.items.length = 0;
	}
}

/**
 * Shared default instance. Most consumers want this.
 * Create a fresh `new ToastStore()` only for tests or isolated regions.
 */
export const toasts = new ToastStore();
export { ToastStore };
