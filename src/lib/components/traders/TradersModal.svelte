<script lang="ts">
  import { isOpen, activeView, activeTrader, closeModal, backToGrid } from '$lib/stores/modal.svelte';
  import { traders } from '$lib/data/traders';
  import TraderCard from './TraderCard.svelte';
  import TraderProfile from './TraderProfile.svelte';
  import { fade, fly } from 'svelte/transition';
  import { cubicOut } from 'svelte/easing';
  import X from 'phosphor-svelte/lib/X';

  $effect(() => {
    if (!$isOpen) return;

    function handleKeydown(event: KeyboardEvent) {
      if (event.key === 'Escape') {
        if ($activeView === 'profile') {
          backToGrid();
        } else {
          closeModal();
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
    $activeTrader ? traders.find(t => t.id === $activeTrader) : null
  );
</script>

{#if $isOpen}
  <!-- Overlay -->
  <div 
    class="fixed inset-0 z-50 bg-navy/85 backdrop-blur-lg"
    transition:fade={{ duration: 300 }}
    onclick={() => closeModal()}
    role="dialog"
    aria-modal="true"
    aria-label="Meet the traders"
  >
    <!-- Modal Container -->
    <div 
      class="absolute inset-4 md:inset-8 lg:inset-16 bg-navy rounded-2xl overflow-hidden"
      transition:fly={{ y: 30, duration: 400, easing: cubicOut }}
      onclick={(e) => e.stopPropagation()}
    >
      <!-- Close Button -->
      <button 
        onclick={closeModal}
        class="absolute top-4 right-4 z-10 w-10 h-10 bg-white/10 hover:bg-white/20 rounded-full flex items-center justify-center text-white transition-colors"
        aria-label="Close modal"
      >
        <X size={24} />
      </button>

      <!-- Content -->
      <div class="h-full overflow-y-auto p-6 md:p-12">
        {#if $activeView === 'grid'}
          <div in:fly={{ y: 20, duration: 300, delay: 100 }}>
            <h2 class="text-3xl md:text-4xl font-bold text-white text-center mb-4 font-heading">Meet The Traders</h2>
            <p class="text-grey-400 text-center max-w-2xl mx-auto mb-12">
              Get to know the experts behind Explosive Swings and their trading methodologies.
            </p>

            <div class="grid md:grid-cols-2 gap-6 max-w-4xl mx-auto">
              {#each traders as trader}
                <TraderCard {trader} />
              {/each}
            </div>
          </div>
        {:else if $activeView === 'profile' && activeTraderData}
          <div in:fly={{ y: 20, duration: 300 }}>
            <TraderProfile trader={activeTraderData} />
          </div>
        {/if}
      </div>
    </div>
  </div>
{/if}
