<script lang="ts">
  import { openModal } from '$lib/stores/modal.svelte';
  import UsersThree from 'phosphor-svelte/lib/UsersThree';
</script>

<button
  onclick={openModal}
  aria-label="Meet the traders"
  class="fixed bottom-8 right-8 z-40 group flex items-center gap-2 px-4 py-3 bg-gradient-to-r from-navy to-navy-mid border border-teal/30 rounded-full shadow-lg hover:shadow-xl transition-all duration-300 hover:-translate-y-1"
>
  <!-- Pulse Animation -->
  <span class="absolute inset-0 rounded-full bg-teal/20 animate-ping opacity-0 group-hover:opacity-100"></span>
  
  <UsersThree size={24} weight="duotone" color="#15C5D1" />
  <span class="hidden sm:inline text-sm font-semibold text-white">Meet The Traders</span>
</button>
