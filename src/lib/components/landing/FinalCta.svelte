<script lang="ts">
  import Button from '$lib/components/ui/Button.svelte';
  import ArrowRight from 'phosphor-svelte/lib/ArrowRight';
  import { gsap } from 'gsap';

  let sectionRef: HTMLElement | undefined = $state();
  let glowRef: HTMLElement | undefined = $state();

  $effect(() => {
    if (!sectionRef || !glowRef) return;
    const section = sectionRef;
    const glow = glowRef;

    const ctx = gsap.context(() => {
      gsap.to(glow, {
        scale: 1.08,
        opacity: 1,
        duration: 3,
        ease: 'sine.inOut',
        yoyo: true,
        repeat: -1,
      });
    }, section);

    return () => ctx.revert();
  });
</script>

<section bind:this={sectionRef} class="relative py-20 lg:py-32 overflow-hidden">
  <!-- Background -->
  <div class="absolute inset-0 bg-gradient-to-br from-navy via-navy-mid to-deep-blue"></div>
  
  <!-- Centered Glow Orb -->
  <div 
    bind:this={glowRef}
    class="absolute top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 w-[600px] h-[600px] rounded-full opacity-40 pointer-events-none"
    style="background: radial-gradient(circle, rgba(15, 164, 175, 0.4) 0%, transparent 70%);"
  ></div>

  <div class="relative z-10 max-w-[1200px] mx-auto px-4 sm:px-6 lg:px-8 text-center">
    <h2 class="text-3xl md:text-4xl lg:text-5xl font-bold text-white mb-6 font-heading">
      Trade with Clarity. Trade with Confidence.
    </h2>
    <p class="text-lg text-grey-300 max-w-2xl mx-auto mb-8">
      Get your weekly watchlist every Sunday night — detailed entries, targets, exits, and stops so you're prepared before the market opens.
    </p>
    <Button variant="primary" href="#pricing">
      Get Instant Access to Alerts
      <ArrowRight size={20} weight="bold" />
    </Button>
  </div>
</section>
