<script lang="ts">
  import { sampleAlert } from '$lib/data/alerts';
  import { gsap } from 'gsap';

  interface Props {
    delay?: number;
  }

  let { delay = 0.4 }: Props = $props();

  let cardRef: HTMLElement | undefined = $state();

  $effect(() => {
    if (!cardRef) return;

    const ctx = gsap.context(() => {
      gsap.from(cardRef, {
        x: 60,
        opacity: 0,
        duration: 0.8,
        delay,
        ease: 'power3.out',
      });
    }, cardRef);

    return () => ctx.revert();
  });
</script>

<div bind:this={cardRef} class="bg-white/5 border border-white/10 backdrop-blur-sm rounded-2xl p-6 max-w-sm">
  <!-- Header -->
  <div class="flex items-center justify-between mb-4">
    <span class="text-xs font-semibold tracking-wider text-grey-400 uppercase">Sample Alert</span>
    <div class="flex items-center gap-2">
      <span class="w-2 h-2 bg-green rounded-full animate-pulse"></span>
      <span class="text-xs text-green font-medium">Live Format</span>
    </div>
  </div>

  <!-- Ticker -->
  <div class="mb-4">
    <span class="text-3xl font-bold text-white font-ui tracking-tight" style="font-variant-numeric: tabular-nums;">
      {sampleAlert.ticker}
    </span>
  </div>

  <!-- Data Rows -->
  <div class="space-y-3">
    <div class="flex justify-between items-center">
      <span class="text-sm text-grey-400">Entry Zone</span>
      <span class="text-sm font-semibold text-teal-light font-ui" style="font-variant-numeric: tabular-nums;">
        {sampleAlert.entryZone}
      </span>
    </div>
    <div class="flex justify-between items-center">
      <span class="text-sm text-grey-400">Invalidation</span>
      <span class="text-sm font-semibold text-red font-ui" style="font-variant-numeric: tabular-nums;">
        {sampleAlert.invalidation}
      </span>
    </div>
    <div class="flex justify-between items-center">
      <span class="text-sm text-grey-400">Profit Zone 1</span>
      <span class="text-sm font-semibold text-green font-ui" style="font-variant-numeric: tabular-nums;">
        {sampleAlert.profitZones[0]}
      </span>
    </div>
    <div class="flex justify-between items-center">
      <span class="text-sm text-grey-400">Profit Zone 2</span>
      <span class="text-sm font-semibold text-green font-ui" style="font-variant-numeric: tabular-nums;">
        {sampleAlert.profitZones[1]}
      </span>
    </div>
    <div class="flex justify-between items-center">
      <span class="text-sm text-grey-400">Profit Zone 3</span>
      <span class="text-sm font-semibold text-green font-ui" style="font-variant-numeric: tabular-nums;">
        {sampleAlert.profitZones[2]}
      </span>
    </div>
  </div>

  <!-- Notes -->
  <div class="mt-4 p-3 bg-teal/10 border-l-2 border-teal rounded-r-lg">
    <p class="text-xs text-grey-200 leading-relaxed">{sampleAlert.notes}</p>
  </div>
</div>
