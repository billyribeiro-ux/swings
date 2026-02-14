// Svelte 5 reactive module state
let isOpen = $state(false);
let activeView = $state<'grid' | 'profile'>('grid');
let activeTrader = $state<string | null>(null);

export function openModal(): void {
  isOpen = true;
  activeView = 'grid';
  activeTrader = null;
}

export function closeModal(): void {
  isOpen = false;
}

export function showProfile(traderId: string): void {
  activeTrader = traderId;
  activeView = 'profile';
}

export function backToGrid(): void {
  activeView = 'grid';
  activeTrader = null;
}

export { isOpen, activeView, activeTrader };
