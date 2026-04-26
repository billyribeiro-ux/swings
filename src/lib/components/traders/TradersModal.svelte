<script lang="ts">
	import { modal } from '$lib/stores/modal.svelte';
	import { traders } from '$lib/data/traders';
	import TraderCard from './TraderCard.svelte';
	import TraderProfile from './TraderProfile.svelte';
	import { blur, fly } from 'svelte/transition';
	import { quintOut, expoOut } from 'svelte/easing';
	import XIcon from 'phosphor-svelte/lib/XIcon';

	$effect(() => {
		if (!modal.isOpen) return;

		function handleKeydown(event: KeyboardEvent) {
			if (event.key === 'Escape') {
				if (modal.activeView === 'profile') {
					modal.backToGrid();
				} else {
					modal.close();
				}
			}
		}

		document.addEventListener('keydown', handleKeydown);
		document.body.style.overflow = 'hidden';

		return () => {
			document.removeEventListener('keydown', handleKeydown);
			document.body.style.overflow = '';
		};
	});

	const activeTraderData = $derived(
		modal.activeTrader ? traders.find((t) => t.id === modal.activeTrader) : null
	);
</script>

{#if modal.isOpen}
	<!-- Overlay -->
	<div
		class="modal-overlay"
		transition:blur={{ duration: 400, amount: 8 }}
		onclick={() => modal.close()}
		onkeydown={(e) => e.key === 'Enter' && modal.close()}
		tabindex="0"
		role="button"
		aria-label="Close modal overlay"
	>
		<!-- Modal Container -->
		<div
			class="modal-container"
			transition:fly={{ y: 40, duration: 500, easing: quintOut }}
			onclick={(e) => e.stopPropagation()}
			onkeydown={(e) => e.stopPropagation()}
			role="presentation"
			tabindex="-1"
		>
			<!-- Close Button -->
			<button onclick={modal.close} class="modal-close" aria-label="Close modal">
				<XIcon size={24} />
			</button>

			<!-- Content -->
			<div class="modal-content">
				{#if modal.activeView === 'grid'}
					<div in:fly={{ y: 24, duration: 450, delay: 120, easing: expoOut }}>
						<h2 class="modal-title">Meet The Traders</h2>
						<p class="modal-subtitle">
							Get to know the experts behind Precision Options Signals and their
							trading methodologies.
						</p>

						<div class="modal-grid">
							{#each traders as trader (trader.id)}
								<TraderCard {trader} />
							{/each}
						</div>
					</div>
				{:else if modal.activeView === 'profile' && activeTraderData}
					<div in:fly={{ y: 24, duration: 400, easing: expoOut }}>
						<TraderProfile trader={activeTraderData} />
					</div>
				{/if}
			</div>
		</div>
	</div>
{/if}

<style>
	.modal-overlay {
		position: fixed;
		inset: 0;
		z-index: var(--z-50);
		background-color: rgba(11, 29, 58, 0.85);
		backdrop-filter: blur(12px);
	}

	.modal-container {
		position: absolute;
		inset: 1rem;
		background-color: var(--color-navy);
		border-radius: var(--radius-2xl);
		overflow: hidden;
	}

	@media (min-width: 768px) {
		.modal-container {
			inset: 2rem;
		}
	}
	@media (min-width: 1024px) {
		.modal-container {
			inset: 4rem;
		}
	}

	.modal-close {
		position: absolute;
		top: 1rem;
		right: 1rem;
		z-index: var(--z-10);
		width: 2.5rem;
		height: 2.5rem;
		background-color: rgba(255, 255, 255, 0.1);
		border-radius: var(--radius-full);
		display: flex;
		align-items: center;
		justify-content: center;
		color: var(--color-white);
		transition: background-color 200ms var(--ease-out);
	}

	.modal-close:hover {
		background-color: rgba(255, 255, 255, 0.2);
	}

	.modal-content {
		height: 100%;
		overflow-y: auto;
		padding: 1.5rem;
	}

	@media (min-width: 768px) {
		.modal-content {
			padding: 3rem;
		}
	}

	.modal-title {
		font-size: var(--fs-3xl);
		font-weight: var(--w-bold);
		color: var(--color-white);
		text-align: center;
		margin-bottom: 1rem;
		font-family: var(--font-heading);
	}

	@media (min-width: 768px) {
		.modal-title {
			font-size: var(--fs-4xl);
		}
	}

	.modal-subtitle {
		color: var(--color-grey-400);
		text-align: center;
		max-width: 42rem;
		margin: 0 auto 3rem;
	}

	.modal-grid {
		display: grid;
		gap: 1.5rem;
		max-width: 56rem;
		margin: 0 auto;
	}

	@media (min-width: 768px) {
		.modal-grid {
			grid-template-columns: repeat(2, 1fr);
		}
	}
</style>
