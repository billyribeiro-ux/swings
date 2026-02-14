<script lang="ts">
  import type { Trader } from '$lib/data/traders';
  import { backToGrid } from '$lib/stores/modal.svelte';
  import ArrowLeft from 'phosphor-svelte/lib/ArrowLeft';
  import Star from 'phosphor-svelte/lib/Star';
  import Pulse from 'phosphor-svelte/lib/Pulse';
  import BookOpen from 'phosphor-svelte/lib/BookOpen';

  interface Props {
    trader: Trader;
  }

  let { trader }: Props = $props();

  const iconMap = {
    Star,
    Pulse,
    BookOpen,
  };
</script>

<div class="max-w-4xl mx-auto">
  <!-- Back Button -->
  <button 
    onclick={backToGrid}
    class="inline-flex items-center gap-2 text-grey-400 hover:text-white transition-colors mb-6"
  >
    <ArrowLeft size={20} />
    <span>All Traders</span>
  </button>

  <!-- Profile Header -->
  <div class="flex items-center gap-6 mb-8">
    <div 
      class="w-24 h-24 rounded-full flex items-center justify-center text-2xl font-bold text-white"
      style="background: linear-gradient(135deg, {trader.avatarGradient.from} 0%, {trader.avatarGradient.to} 100%);"
    >
      {trader.initials}
    </div>
    <div>
      <h2 class="text-3xl font-bold text-white font-heading">{trader.name}</h2>
      <p class="text-teal-light text-lg">{trader.role}</p>
    </div>
  </div>

  <!-- Bio -->
  <div class="space-y-4 mb-8">
    {#each trader.bio as paragraph, i (i)}
      <p class="text-grey-300 leading-relaxed">{@html paragraph}</p>
    {/each}
  </div>

  <!-- Highlights -->
  <div class="grid grid-cols-3 gap-4 mb-8">
    {#each trader.highlights as highlight, i (i)}
      <div class="bg-navy-mid border border-white/10 rounded-xl p-4 text-center">
        <div class="kpi-value text-teal-light mb-1">{highlight.value}</div>
        <div class="kpi-label text-grey-400">{highlight.label}</div>
      </div>
    {/each}
  </div>

  <!-- Action Buttons -->
  <div class="flex flex-wrap gap-4">
    {#each trader.actions as action, i (i)}
      {@const IconComponent = iconMap[action.icon as keyof typeof iconMap]}
      <button 
        class="inline-flex items-center gap-2 px-6 py-3 rounded-lg font-semibold text-sm transition-all {action.variant === 'primary' ? 'bg-teal text-white hover:bg-teal-light' : 'bg-transparent border border-white/20 text-white hover:bg-white/10'}"
      >
        <IconComponent size={18} weight={action.icon === 'Star' ? 'fill' : 'regular'} />
        {action.label}
      </button>
    {/each}
  </div>
</div>
