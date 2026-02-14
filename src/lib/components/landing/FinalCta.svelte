<script lang="ts">
  import { onMount } from 'svelte';
  import Button from '$lib/components/ui/Button.svelte';
  import ArrowRight from 'phosphor-svelte/lib/ArrowRight';
  import { gsap } from 'gsap';
  import { ScrollTrigger } from 'gsap/ScrollTrigger';

  let sectionRef: HTMLElement | undefined = $state();
  let glowRef: HTMLElement | undefined = $state();

  onMount(() => {
    if (!sectionRef || !glowRef) return;
    const section = sectionRef;
    const glow = glowRef;

    gsap.registerPlugin(ScrollTrigger);

    const contentEls = section.querySelectorAll('.final-cta-content > *');
    gsap.set(contentEls, {
      opacity: 0,
      y: 24,
      willChange: 'transform, opacity',
      force3D: true,
    });

    const ctx = gsap.context(() => {
      // Ultra-slow cinematic glow breathing
      gsap.to(glow, {
        scale: 1.1,
        opacity: 0.65,
        duration: 7,
        ease: 'sine.inOut',
        yoyo: true,
        repeat: -1,
        force3D: true,
      });

      // Silky staggered content reveal
      gsap.to(contentEls, {
        opacity: 1,
        y: 0,
        duration: 1.4,
        stagger: 0.1,
        ease: 'expo.out',
        force3D: true,
        scrollTrigger: {
          trigger: section,
          start: 'top 80%',
          once: true,
        },
        onComplete() {
          gsap.set(contentEls, { willChange: 'auto' });
        },
      });
    }, section);

    return () => {
      ctx.revert();
      gsap.set(contentEls, { clearProps: 'all' });
    };
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

  <div class="final-cta-content relative z-10 max-w-[1200px] mx-auto px-4 sm:px-6 lg:px-8 text-center">
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
