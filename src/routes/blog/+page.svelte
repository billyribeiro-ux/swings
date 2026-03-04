<script lang="ts">
  import { onMount } from 'svelte';
  import { gsap } from 'gsap';
  import ScrollReveal from '$lib/components/ui/ScrollReveal.svelte';
  import ArrowRight from 'phosphor-svelte/lib/ArrowRight';
  import Article from 'phosphor-svelte/lib/Article';
  import CalendarBlank from 'phosphor-svelte/lib/CalendarBlank';
  import Clock from 'phosphor-svelte/lib/Clock';

  let heroRef: HTMLElement | undefined = $state();

  onMount(() => {
    if (!heroRef) return;

    const els = ['.blog-badge', '.blog-title', '.blog-subtitle'];
    gsap.set(els, { opacity: 0, y: 24, willChange: 'transform, opacity', force3D: true });

    const ctx = gsap.context(() => {
      const tl = gsap.timeline({ defaults: { ease: 'expo.out', duration: 1.4, force3D: true }, delay: 0.2 });
      tl.to('.blog-badge', { opacity: 1, y: 0, duration: 1.0 })
        .to('.blog-title', { opacity: 1, y: 0 }, '-=0.9')
        .to('.blog-subtitle', { opacity: 1, y: 0 }, '-=1.0')
        .call(() => { gsap.set(els, { willChange: 'auto' }); });
    }, heroRef as HTMLElement);

    return () => { ctx.revert(); gsap.set(els, { clearProps: 'all' }); };
  });

  const posts = [
    {
      slug: 'understanding-options-greeks',
      title: 'Understanding Options Greeks: A Complete Guide',
      excerpt: 'Master Delta, Gamma, Theta, and Vega to make smarter options trading decisions.',
      date: '2026-03-01',
      readTime: '8 min',
      category: 'Education'
    },
    {
      slug: 'weekly-watchlist-breakdown',
      title: 'How We Build Our Weekly Watchlists',
      excerpt: 'A behind-the-scenes look at our process for identifying high-probability options setups.',
      date: '2026-02-28',
      readTime: '6 min',
      category: 'Strategy'
    },
    {
      slug: 'risk-management-essentials',
      title: 'Risk Management Essentials for Options Traders',
      excerpt: 'Learn how to protect your capital and survive the inevitable losing streaks.',
      date: '2026-02-25',
      readTime: '10 min',
      category: 'Risk Management'
    }
  ];
</script>

<svelte:head>
  <title>Blog — Explosive Swings</title>
  <meta name="description" content="Options trading insights, strategies, and education from the Explosive Swings team." />
</svelte:head>

<!-- Hero -->
<section bind:this={heroRef} class="relative overflow-hidden bg-linear-to-br from-navy via-navy-mid to-deep-blue pt-16">
  <div
    class="absolute inset-0 opacity-[0.02]"
    style="background-image: linear-gradient(to right, white 1px, transparent 1px), linear-gradient(to bottom, white 1px, transparent 1px); background-size: 60px 60px;"
  ></div>

  <div class="relative z-10 mx-auto max-w-[1200px] px-4 py-20 sm:px-6 lg:px-8 lg:py-28 text-center">
    <div class="blog-badge inline-flex items-center gap-2 rounded-full border border-teal/30 bg-teal/10 px-4 py-2 mb-6">
      <Article size={18} weight="duotone" color="#15C5D1" />
      <span class="text-xs font-semibold text-teal-light tracking-wide uppercase">Blog</span>
    </div>

    <h1 class="blog-title text-3xl sm:text-4xl md:text-5xl lg:text-6xl font-bold text-white mb-6 font-heading leading-tight">
      Trading Insights & Education
    </h1>

    <p class="blog-subtitle text-base sm:text-lg lg:text-xl text-grey-300 max-w-2xl mx-auto leading-relaxed">
      Strategies, analysis, and lessons from the trading desk to help you level up your options game.
    </p>
  </div>
</section>

<!-- Posts Grid -->
<section class="py-16 sm:py-20 lg:py-28 bg-off-white">
  <div class="mx-auto max-w-[1200px] px-4 sm:px-6 lg:px-8">
    <ScrollReveal>
      <div class="grid gap-6 sm:gap-8 md:grid-cols-2 lg:grid-cols-3">
        {#each posts as post, i}
          <article
            class="reveal-item group flex flex-col overflow-hidden rounded-2xl bg-white shadow-sm ring-1 ring-grey-200/80 transition-all duration-500 ease-out hover:-translate-y-1 hover:shadow-xl hover:ring-teal/30"
            style="transition-delay: {i * 0.08}s"
          >
            <div class="flex-1 p-6 sm:p-8">
              <div class="mb-4 flex items-center gap-3 text-xs text-grey-500">
                <span class="inline-flex items-center gap-1">
                  <CalendarBlank size={14} weight="bold" class="text-grey-400" />
                  {new Date(post.date).toLocaleDateString('en-US', { month: 'short', day: 'numeric', year: 'numeric' })}
                </span>
                <span>•</span>
                <span class="inline-flex items-center gap-1">
                  <Clock size={14} weight="bold" class="text-grey-400" />
                  {post.readTime}
                </span>
              </div>

              <span class="mb-3 inline-block rounded-lg bg-teal/10 px-3 py-1 text-xs font-semibold text-teal">
                {post.category}
              </span>

              <h2 class="mb-3 text-xl font-bold text-navy font-heading leading-snug group-hover:text-teal transition-colors duration-300">
                {post.title}
              </h2>

              <p class="mb-6 text-sm text-grey-600 leading-relaxed">
                {post.excerpt}
              </p>

              <a
                href="/blog/{post.slug}"
                class="inline-flex items-center gap-1.5 text-sm font-semibold text-teal transition-all duration-300 group-hover:text-teal-light group-hover:translate-x-0.5"
              >
                Read More
                <ArrowRight size={14} weight="bold" />
              </a>
            </div>
          </article>
        {/each}
      </div>
    </ScrollReveal>
  </div>
</section>

<!-- Coming Soon -->
<section class="py-16 sm:py-20 lg:py-28 bg-white">
  <div class="mx-auto max-w-[1200px] px-4 sm:px-6 lg:px-8 text-center">
    <ScrollReveal>
      <div class="reveal-item mx-auto max-w-2xl rounded-2xl border border-grey-200 bg-off-white p-8 sm:p-10">
        <Article size={48} weight="duotone" color="#0FA4AF" class="mx-auto mb-4" />
        <h3 class="mb-3 text-2xl font-bold text-navy font-heading">More Content Coming Soon</h3>
        <p class="text-grey-600 leading-relaxed">
          We're constantly publishing new insights, strategies, and educational content. Check back regularly or subscribe to our newsletter to stay updated.
        </p>
      </div>
    </ScrollReveal>
  </div>
</section>
