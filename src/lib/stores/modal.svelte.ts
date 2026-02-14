import { writable } from 'svelte/store';

// Use standard Svelte stores for cross-component state
export const isOpen = writable(false);
export const activeView = writable<'grid' | 'profile'>('grid');
export const activeTrader = writable<string | null>(null);

export function openModal(): void {
  isOpen.set(true);
  activeView.set('grid');
  activeTrader.set(null);
}

export function closeModal(): void {
  isOpen.set(false);
}

export function showProfile(traderId: string): void {
  activeTrader.set(traderId);
  activeView.set('profile');
}

export function backToGrid(): void {
  activeView.set('grid');
  activeTrader.set(null);
}
