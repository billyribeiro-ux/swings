<script lang="ts">
  import { onMount } from 'svelte';
  import { gsap } from 'gsap';
  import ScrollReveal from '$lib/components/ui/ScrollReveal.svelte';
  import SectionHeader from '$lib/components/ui/SectionHeader.svelte';
  import CheckCircle from 'phosphor-svelte/lib/CheckCircle';
  import TrendUp from 'phosphor-svelte/lib/TrendUp';
  import Users from 'phosphor-svelte/lib/Users';
  import ChartLineUp from 'phosphor-svelte/lib/ChartLineUp';
  import Target from 'phosphor-svelte/lib/Target';

  let heroRef: HTMLElement | undefined = $state();

  onMount(() => {
    if (!heroRef) return;

    const els = ['.about-badge', '.about-title', '.about-subtitle'];
    gsap.set(els, { opacity: 0, y: 24, willChange: 'transform, opacity', force3D: true });

    const ctx = gsap.context(() => {
      const tl = gsap.timeline({ defaults: { ease: 'expo.out', duration: 1.4, force3D: true }, delay: 0.2 });
      tl.to('.about-badge', { opacity: 1, y: 0, duration: 1.0 })
        .to('.about-title', { opacity: 1, y: 0 }, '-=0.9')
        .to('.about-subtitle', { opacity: 1, y: 0 }, '-=1.0')
        .call(() => { gsap.set(els, { willChange: 'auto' }); });
    }, heroRef as HTMLElement);

    return () => { ctx.revert(); gsap.set(els, { clearProps: 'all' }); };
  });

  const values = [
    { icon: Target, title: 'Precision', desc: 'Every trade alert is backed by technical analysis and risk management.' },
    { icon: Users, title: 'Community', desc: 'Join a network of traders learning and growing together.' },
    { icon: TrendUp, title: 'Results', desc: 'We focus on consistent, repeatable strategies that work.' },
  ];
</script>

<svelte:head>
  <title>About — Explosive Swings</title>
  <meta name="description" content="Learn about Explosive Swings, our mission to empower options traders, and the team behind the weekly watchlists." />
</svelte:head>

<!-- Hero -->
<section bind:this={heroRef} class="relative overflow-hidden bg-linear-to-br from-navy via-navy-mid to-deep-blue pt-16">
  <div
    class="absolute inset-0 opacity-[0.02]"
    style="background-image: linear-gradient(to right, white 1px, transparent 1px), linear-gradient(to bottom, white 1px, transparent 1px); background-size: 60px 60px;"
  ></div>

  <div class="relative z-10 mx-auto max-w-[1200px] px-4 py-20 sm:px-6 lg:px-8 lg:py-28 text-center">
    <div class="about-badge inline-flex items-center gap-2 rounded-full border border-teal/30 bg-teal/10 px-4 py-2 mb-6">
      <Users size={18} weight="duotone" color="#15C5D1" />
      <span class="text-xs font-semibold text-teal-light tracking-wide uppercase">About Us</span>
    </div>

    <h1 class="about-title text-3xl sm:text-4xl md:text-5xl lg:text-6xl font-bold text-white mb-6 font-heading leading-tight">
      Built by Traders, for Traders
    </h1>

    <p class="about-subtitle text-base sm:text-lg lg:text-xl text-grey-300 max-w-2xl mx-auto leading-relaxed">
      Explosive Swings was created to cut through the noise and deliver actionable options alerts you can trust.
    </p>
  </div>
</section>

<!-- Story -->
<section class="py-16 sm:py-20 lg:py-28 bg-white">
  <div class="mx-auto max-w-[1200px] px-4 sm:px-6 lg:px-8">
    <ScrollReveal>
      <div class="mx-auto max-w-3xl">
        <h2 class="reveal-item mb-8 text-2xl font-bold text-navy sm:text-3xl md:text-4xl font-heading text-center">
          Our Story
        </h2>

        <div class="reveal-item space-y-6 text-grey-700 leading-relaxed">
          <p>
            Explosive Swings started with a simple observation: most trading services overwhelm you with alerts, but very few teach you <em>why</em> a trade makes sense or <em>how</em> to manage it.
          </p>

          <p>
            Founded by <strong>Billy Ribeiro</strong>, a former lead trader at Simpler Trading who was mentored by Goldman Sachs' Mark McGoldrick, Explosive Swings focuses on quality over quantity. Every Sunday night, you get a curated watchlist of 5–7 high-probability options setups — complete with entry zones, profit targets, and stop losses.
          </p>

          <p>
            We're not here to spam you with 50 alerts a day. We're here to help you build a repeatable, disciplined approach to options trading that fits your schedule and risk tolerance.
          </p>
        </div>
      </div>
    </ScrollReveal>
  </div>
</section>

<!-- Values -->
<section class="py-16 sm:py-20 lg:py-28 bg-off-white">
  <div class="mx-auto max-w-[1200px] px-4 sm:px-6 lg:px-8">
    <ScrollReveal>
      <SectionHeader
        eyebrow="Our Values"
        title="What We Stand For"
        subtitle="These principles guide every trade alert, every course, and every interaction with our community."
      />

      <div class="mx-auto grid max-w-4xl gap-6 sm:gap-8 sm:grid-cols-3">
        {#each values as value, i}
          <div
            class="reveal-item flex flex-col items-center text-center rounded-2xl bg-white p-6 sm:p-8 ring-1 ring-grey-200/80"
            style="transition-delay: {i * 0.08}s"
          >
            <div class="mb-5 flex h-14 w-14 items-center justify-center rounded-2xl bg-teal/10">
              <value.icon size={28} weight="duotone" color="#0FA4AF" />
            </div>
            <h3 class="mb-2 text-lg font-bold text-navy font-heading">{value.title}</h3>
            <p class="text-sm text-grey-600 leading-relaxed">{value.desc}</p>
          </div>
        {/each}
      </div>
    </ScrollReveal>
  </div>
</section>

<!-- Team -->
<section class="py-16 sm:py-20 lg:py-28 bg-white">
  <div class="mx-auto max-w-[1200px] px-4 sm:px-6 lg:px-8">
    <ScrollReveal>
      <h2 class="reveal-item mb-12 text-2xl font-bold text-navy text-center sm:text-3xl md:text-4xl font-heading">
        Meet the Founder
      </h2>

      <div class="reveal-item mx-auto max-w-2xl rounded-2xl bg-off-white p-8 sm:p-10 ring-1 ring-grey-200/80">
        <div class="mb-6 flex items-center gap-4">
          <div
            class="flex h-16 w-16 shrink-0 items-center justify-center rounded-full text-xl font-bold text-white"
            style="background: linear-gradient(135deg, #0FA4AF 0%, #1A3A6B 100%);"
          >
            BR
          </div>
          <div>
            <h3 class="text-xl font-bold text-navy font-heading">Billy Ribeiro</h3>
            <p class="text-sm text-grey-600">Founder & Lead Trader</p>
          </div>
        </div>

        <div class="space-y-4 text-grey-700 leading-relaxed">
          <p>
            Billy spent years as a lead trader at Simpler Trading, where he was mentored by Mark McGoldrick, a former Goldman Sachs trader. He's seen thousands of trades, taught hundreds of students, and knows what separates winning strategies from losing ones.
          </p>

          <p>
            His approach is simple: focus on high-probability setups, manage risk religiously, and never chase trades. That philosophy is the foundation of Explosive Swings.
          </p>
        </div>

        <div class="mt-6 flex flex-wrap gap-3">
          <span class="inline-flex items-center gap-1.5 rounded-lg bg-teal/10 px-3 py-1.5 text-xs font-semibold text-teal">
            <CheckCircle size={14} weight="fill" />
            10+ Years Trading
          </span>
          <span class="inline-flex items-center gap-1.5 rounded-lg bg-teal/10 px-3 py-1.5 text-xs font-semibold text-teal">
            <CheckCircle size={14} weight="fill" />
            Former Simpler Trading
          </span>
          <span class="inline-flex items-center gap-1.5 rounded-lg bg-teal/10 px-3 py-1.5 text-xs font-semibold text-teal">
            <CheckCircle size={14} weight="fill" />
            Goldman Sachs Mentored
          </span>
        </div>
      </div>
    </ScrollReveal>
  </div>
</section>

<!-- CTA -->
<section class="py-16 sm:py-20 lg:py-28 bg-linear-to-br from-navy via-navy-mid to-deep-blue">
  <div class="mx-auto max-w-[1200px] px-4 sm:px-6 lg:px-8 text-center">
    <ScrollReveal>
      <h2 class="reveal-item mb-5 text-2xl font-bold text-white sm:text-3xl md:text-4xl lg:text-5xl font-heading">
        Ready to Join Us?
      </h2>

      <p class="reveal-item mx-auto mb-10 max-w-2xl text-base text-grey-300 sm:text-lg leading-relaxed">
        Get weekly watchlists, structured courses, and join a community of traders who are serious about results.
      </p>

      <div class="reveal-item">
        <a
          href="/#pricing"
          class="inline-flex items-center justify-center gap-2 rounded-xl bg-teal px-8 py-4 text-sm font-semibold text-white shadow-lg shadow-teal/25 transition-all duration-300 hover:bg-teal-light hover:shadow-xl hover:shadow-teal/30 hover:-translate-y-px active:scale-[0.97]"
        >
          Get Instant Access
          <ChartLineUp size={18} weight="bold" />
        </a>
      </div>
    </ScrollReveal>
  </div>
</section>
