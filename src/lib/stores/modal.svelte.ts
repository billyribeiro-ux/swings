// Svelte 5 reactive state class for modal management
class ModalState {
	isOpen = $state(false);
	activeView = $state<'grid' | 'profile'>('grid');
	activeTrader = $state<string | null>(null);

	open = () => {
		this.isOpen = true;
		this.activeView = 'grid';
		this.activeTrader = null;
	};

	close = () => {
		this.isOpen = false;
	};

	showProfile = (traderId: string) => {
		this.activeTrader = traderId;
		this.activeView = 'profile';
	};

	backToGrid = () => {
		this.activeView = 'grid';
		this.activeTrader = null;
	};
}

export const modal = new ModalState();
