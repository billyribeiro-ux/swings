<script lang="ts">
  import SectionHeader from '$lib/components/ui/SectionHeader.svelte';
  import ScrollReveal from '$lib/components/ui/ScrollReveal.svelte';
  import CaretRight from 'phosphor-svelte/lib/CaretRight';

  const audience = [
    'Traders who want <strong>simple, followable alerts</strong>',
    'People who don\'t have time to learn advanced strategies',
    'Traders who want <strong>early entries</strong>, not late chases',
    'Anyone who wants guidance without sitting in a live room',
  ];
</script>

<section class="py-20 lg:py-32 bg-off-white">
  <div class="max-w-[1200px] mx-auto px-4 sm:px-6 lg:px-8">
    <ScrollReveal>
      <SectionHeader
        eyebrow="Who This Is For"
        title="If You Want Clarity Without Complexity,<br/>This Was Built for You"
      />

      <div class="grid md:grid-cols-2 gap-4 max-w-4xl mx-auto">
        {#each audience as item, i}
          <div 
            class="reveal-item flex items-start gap-3 p-5 rounded-lg border border-grey-200 bg-white"
            style="transition-delay: {i * 0.1}s"
          >
            <CaretRight size={20} weight="bold" color="#0FA4AF" class="flex-shrink-0 mt-0.5" />
            <p class="text-grey-800 text-left">{@html item}</p>
          </div>
        {/each}
      </div>
    </ScrollReveal>
  </div>
</section>
