<script lang="ts">
  import { gsap } from 'gsap';
  import Button from '$lib/components/ui/Button.svelte';
  import SampleAlertCard from './SampleAlertCard.svelte';
  import ArrowRight from 'phosphor-svelte/lib/ArrowRight';

  let heroRef: HTMLElement | undefined = $state();
  let glowRef: HTMLElement | undefined = $state();

  $effect(() => {
    if (!heroRef) return;

    const ctx = gsap.context(() => {
      const tl = gsap.timeline({ defaults: { ease: 'power3.out' } });

      tl.from('.hero-badge', { y: 24, opacity: 0, duration: 0.6 })
        .from('.hero-title', { y: 24, opacity: 0, duration: 0.6 }, '-=0.4')
        .from('.hero-subtitle', { y: 24, opacity: 0, duration: 0.6 }, '-=0.4')
        .from('.hero-actions', { y: 24, opacity: 0, duration: 0.6 }, '-=0.4')
        .from('.hero-trust', { y: 24, opacity: 0, duration: 0.6 }, '-=0.4');

      // Glow orb animation
      if (glowRef) {
        gsap.to(glowRef, {
          scale: 1.08,
          opacity: 1,
          duration: 3,
          ease: 'sine.inOut',
          yoyo: true,
          repeat: -1,
        });
      }
    }, heroRef as HTMLElement);

    return () => ctx.revert();
  });

  function scrollToHowItWorks() {
    const element = document.getElementById('how-it-works');
    if (element) {
      element.scrollIntoView({ behavior: 'smooth' });
    }
  }
</script>

<section bind:this={heroRef} class="relative min-h-screen overflow-hidden pt-16">
  <!-- Background -->
  <div class="absolute inset-0 bg-gradient-to-br from-navy via-navy-mid to-deep-blue"></div>
  
  <!-- Grid Overlay -->
  <div 
    class="absolute inset-0 opacity-[0.02]"
    style="background-image: linear-gradient(to right, white 1px, transparent 1px), linear-gradient(to bottom, white 1px, transparent 1px); background-size: 60px 60px;"
  ></div>
  
  <!-- Glow Orb -->
  <div 
    bind:this={glowRef}
    class="absolute top-20 right-10 w-[700px] h-[700px] rounded-full opacity-60 pointer-events-none"
    style="background: radial-gradient(circle, rgba(15, 164, 175, 0.3) 0%, transparent 70%);"
  ></div>

  <div class="relative z-10 max-w-[1200px] mx-auto px-4 sm:px-6 lg:px-8 py-20 lg:py-32">
    <div class="grid lg:grid-cols-2 gap-12 lg:gap-16 items-center">
      <!-- Left Column -->
      <div class="space-y-8">
        <!-- Eyebrow Badge -->
        <div class="hero-badge inline-flex items-center gap-2 px-4 py-2 rounded-full border border-teal/30 bg-teal/10">
          <span class="w-2 h-2 bg-teal rounded-full animate-pulse"></span>
          <span class="text-xs font-semibold text-teal-light tracking-wide uppercase">
            Weekly watchlist delivered every Sunday night
          </span>
        </div>

        <!-- Title -->
        <h1 class="hero-title text-white">
          Simple, Early Stock Alerts <span class="text-teal-light">You Can Actually Use</span>
        </h1>

        <!-- Subtitle -->
        <p class="hero-subtitle text-grey-300">
          Every Sunday night, get a detailed watchlist of 5–7 top stock picks with defined entries, targets, exits, and stops — so you're ready before the market opens.
        </p>

        <!-- Actions -->
        <div class="hero-actions flex flex-wrap gap-4">
          <Button variant="primary" href="#pricing">
            Get Instant Access
            <ArrowRight size={20} weight="bold" class="w-5 h-5" />
          </Button>
          <Button variant="ghost" onclick={scrollToHowItWorks}>
            See How It Works
          </Button>
        </div>

        <!-- Trust Line -->
        <div class="hero-trust flex items-center gap-3 pt-4">
          <div 
            class="w-10 h-10 rounded-full flex items-center justify-center text-sm font-bold text-white"
            style="background: linear-gradient(135deg, #0FA4AF 0%, #1A3A6B 100%);"
          >
            BR
          </div>
          <p class="text-sm text-grey-400">
            Created by <span class="text-white font-semibold">Billy Ribeiro</span> — former lead trader at Simpler Trading, mentored by Goldman Sachs' Mark McGoldrick
          </p>
        </div>
      </div>

      <!-- Right Column -->
      <div class="flex justify-center lg:justify-end">
        <SampleAlertCard delay={0.6} />
      </div>
    </div>
  </div>
</section>
