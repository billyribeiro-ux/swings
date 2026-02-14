<script lang="ts">
  import SectionHeader from '$lib/components/ui/SectionHeader.svelte';
  import ScrollReveal from '$lib/components/ui/ScrollReveal.svelte';
  import Button from '$lib/components/ui/Button.svelte';
  import { pricingPlans } from '$lib/data/pricing';
</script>

<section id="pricing" class="py-20 lg:py-32 bg-white">
  <div class="max-w-[1200px] mx-auto px-4 sm:px-6 lg:px-8">
    <ScrollReveal>
      <SectionHeader
        eyebrow="Pricing"
        title="Straightforward Pricing. No Contracts. Cancel Anytime."
      />

      <div class="grid md:grid-cols-2 gap-8 max-w-3xl mx-auto">
        {#each pricingPlans as plan, i}
          <div 
            class="reveal-item relative rounded-2xl p-8 {plan.featured ? 'bg-gradient-to-b from-teal/5 to-white border-2 border-teal' : 'bg-off-white border border-grey-200'}"
            style="transition-delay: {i * 0.15}s"
          >
            <!-- Badge -->
            {#if plan.badge}
              <div class="absolute -top-3 left-1/2 -translate-x-1/2">
                <span class="px-4 py-1 bg-teal text-white text-xs font-semibold rounded-full">
                  {plan.badge}
                </span>
              </div>
            {/if}

            <!-- Plan Name -->
            <h3 class="text-lg font-semibold text-grey-600 mb-2 font-ui">{plan.name}</h3>
            
            <!-- Price -->
            <div class="flex items-baseline gap-1 mb-4">
              <span class="text-5xl font-bold text-navy font-heading">${plan.amount}</span>
              <span class="text-grey-500">{plan.suffix}</span>
            </div>

            <!-- Savings Badge -->
            {#if plan.savings}
              <div class="mb-4">
                <span class="px-3 py-1 bg-green/10 text-green text-sm font-medium rounded-full">
                  {plan.savings}
                </span>
              </div>
            {/if}

            <!-- Note -->
            <p class="text-sm text-grey-500 mb-6">{plan.note}</p>

            <!-- CTA -->
            <Button variant={plan.variant === 'primary' ? 'primary' : 'outline'} href="#">
              {plan.cta}
            </Button>
          </div>
        {/each}
      </div>
    </ScrollReveal>
  </div>
</section>
