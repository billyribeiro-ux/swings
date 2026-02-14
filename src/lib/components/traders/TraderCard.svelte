<script lang="ts">
  import type { Trader } from '$lib/data/traders';
  import { showProfile } from '$lib/stores/modal.svelte';
  import ArrowRight from 'phosphor-svelte/lib/ArrowRight';

  interface Props {
    trader: Trader;
  }

  let { trader }: Props = $props();
</script>

<button 
  onclick={() => showProfile(trader.id)}
  class="group text-left w-full bg-navy-mid border border-white/10 rounded-xl p-6 hover:border-teal/50 hover:-translate-y-1 hover:shadow-lg hover:shadow-teal/10 transition-all duration-300"
>
  <!-- Avatar -->
  <div 
    class="w-16 h-16 rounded-full flex items-center justify-center text-lg font-bold text-white mb-4"
    style="background: linear-gradient(135deg, {trader.avatarGradient.from} 0%, {trader.avatarGradient.to} 100%);"
  >
    {trader.initials}
  </div>

  <!-- Name & Role -->
  <h3 class="text-xl font-bold text-white mb-1 font-heading">{trader.name}</h3>
  <p class="text-teal-light text-sm font-medium mb-3">{trader.role}</p>

  <!-- Tagline -->
  <p class="text-grey-400 text-sm leading-relaxed mb-4">{trader.tagline}</p>

  <!-- View Profile Link -->
  <span class="inline-flex items-center gap-1 text-teal text-sm font-semibold group-hover:text-teal-light transition-colors">
    View Profile
    <ArrowRight size={16} />
  </span>
</button>
