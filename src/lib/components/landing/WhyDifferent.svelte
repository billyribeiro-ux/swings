<script lang="ts">
  import SectionHeader from '$lib/components/ui/SectionHeader.svelte';
  import ScrollReveal from '$lib/components/ui/ScrollReveal.svelte';
  import Clock from 'phosphor-svelte/lib/Clock';
  import ShieldCheck from 'phosphor-svelte/lib/ShieldCheck';
  import CurrencyDollar from 'phosphor-svelte/lib/CurrencyDollar';

  const features = [
    {
      icon: Clock,
      title: 'Early Entry Alerts',
      description: 'Most services notify you after the move has already happened. We send alerts before the breakout, with the exact price level we\'re watching — giving you time to plan, not chase.',
    },
    {
      icon: ShieldCheck,
      title: 'Clear Invalidation Levels',
      description: 'Every alert includes a simple \'no longer valid\' level. If price breaks below it, the setup is done — no confusion, no guessing, no hoping. You always know where you stand.',
    },
    {
      icon: CurrencyDollar,
      title: 'Simple Profit-Taking Guidance',
      description: 'We highlight logical profit zones so you can scale out with confidence, even if you\'re not an experienced trader. No complicated calculations — just clear, actionable targets.',
    },
  ];
</script>

<section id="how-it-works" class="py-20 lg:py-32 bg-off-white">
  <div class="max-w-[1200px] mx-auto px-4 sm:px-6 lg:px-8">
    <ScrollReveal>
      <SectionHeader
        eyebrow="Why We're Different"
        title="Most Alert Services Tell You <em>After</em> the Move. We Tell You <em>Before</em>."
        subtitle="Built on the proprietary 'Move Prior to The Move' methodology — your watchlist arrives Sunday night so you can set alerts and be positioned before the action starts."
      />

      <div class="grid md:grid-cols-3 gap-8">
        {#each features as feature, i}
          <div 
            class="reveal-item group bg-white rounded-xl p-8 shadow-sm hover:shadow-lg transition-all duration-300 border border-grey-100"
            style="transition-delay: {i * 0.15}s"
          >
            <!-- Icon -->
            <div class="w-12 h-12 bg-teal/10 rounded-full flex items-center justify-center mb-6">
              <feature.icon size={24} weight="duotone" color="#0FA4AF" />
            </div>
            
            <!-- Content -->
            <h3 class="text-lg font-bold text-navy mb-3 font-heading">{feature.title}</h3>
            <p class="text-grey-600 text-sm leading-relaxed">{feature.description}</p>
          </div>
        {/each}
      </div>
    </ScrollReveal>
  </div>
</section>
