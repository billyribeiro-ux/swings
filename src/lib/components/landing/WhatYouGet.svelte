<script lang="ts">
  import SectionHeader from '$lib/components/ui/SectionHeader.svelte';
  import ScrollReveal from '$lib/components/ui/ScrollReveal.svelte';
  import CheckCircle from 'phosphor-svelte/lib/CheckCircle';

  const features = [
    {
      title: '5–7 Top Picks Every Sunday Night',
      description: 'A detailed watchlist delivered before the week starts — complete with entries, targets, exits, and stops so you\'re fully prepared before Monday\'s open.',
    },
    {
      title: 'Charts with Marked Zones',
      description: 'Every alert comes with annotated charts showing entry, invalidation, and profit targets visually.',
    },
    {
      title: 'Entries, Targets, Exits & Stops',
      description: 'Every pick comes with precise levels — no ambiguity. You know exactly where to get in, where to take profit, and where to cut it.',
    },
    {
      title: 'Detailed Weekly Video Walkthrough',
      description: 'A thorough breakdown of every pick on the watchlist — so you understand the setup, the levels, and can set your alerts ahead of time with confidence.',
    },
    {
      title: 'Multi-Channel Delivery',
      description: 'Watchlist delivered via email, SMS, and your members-only dashboard every Sunday night — so you never miss a week.',
      fullWidth: true,
    },
  ];
</script>

<section class="py-20 lg:py-32 bg-white">
  <div class="max-w-[1200px] mx-auto px-4 sm:px-6 lg:px-8">
    <ScrollReveal>
      <SectionHeader
        eyebrow="What's Included"
        title="Your Sunday Night Watchlist"
        subtitle="Every Sunday night, your watchlist drops with everything you need to trade the week ahead — no guessing, no scrambling Monday morning."
      />

      <div class="grid md:grid-cols-2 gap-6 max-w-5xl mx-auto">
        {#each features as feature, i}
          <div 
            class="reveal-item flex gap-4 p-6 rounded-lg border border-grey-200 bg-off-white {feature.fullWidth ? 'md:col-span-2' : ''}"
            style="transition-delay: {i * 0.12}s"
          >
            <div class="flex-shrink-0 w-6 h-6 flex items-center justify-center mt-0.5">
              <CheckCircle size={24} weight="fill" color="#0FA4AF" />
            </div>
            <div>
              <h3 class="text-base font-bold text-navy mb-2 font-heading">{feature.title}</h3>
              <p class="text-grey-600 text-sm leading-relaxed">{feature.description}</p>
            </div>
          </div>
        {/each}
      </div>
    </ScrollReveal>
  </div>
</section>
