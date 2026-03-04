<script lang="ts">
  import { onMount } from 'svelte';
  import { gsap } from 'gsap';
  import { ScrollTrigger } from 'gsap/ScrollTrigger';
  import SectionHeader from '$lib/components/ui/SectionHeader.svelte';
  import Star from 'phosphor-svelte/lib/Star';
  import Quotes from 'phosphor-svelte/lib/Quotes';

  let containerRef: HTMLElement | undefined = $state();

  const testimonials = [
    {
      name: 'Michael Chen',
      role: 'Full-Time Trader',
      avatar: 'MC',
      rating: 5,
      text: "Billy's watchlists have completely transformed my trading. The entry zones are spot-on, and the risk management is exactly what I needed. I've gone from guessing to executing with confidence.",
      gradient: 'from-teal to-teal-light'
    },
    {
      name: 'Sarah Martinez',
      role: 'Portfolio Manager',
      avatar: 'SM',
      rating: 5,
      text: "I've tried dozens of trading services, and Explosive Swings is the only one I trust. No fluff, no spam—just high-quality setups delivered every Sunday night. The Discord community is gold.",
      gradient: 'from-gold to-gold-light'
    },
    {
      name: 'David Thompson',
      role: 'Options Trader',
      avatar: 'DT',
      rating: 5,
      text: "The courses alone are worth 10x the subscription price. Billy breaks down complex strategies in a way that actually makes sense. My win rate has improved dramatically since joining.",
      gradient: 'from-deep-blue to-teal'
    }
  ];

  onMount(() => {
    if (!containerRef) return;

    gsap.registerPlugin(ScrollTrigger);

    const cards = containerRef.querySelectorAll('.testimonial-card');

    // Cinematic entrance: staggered fade + scale + rotation
    gsap.set(cards, {
      opacity: 0,
      scale: 0.85,
      rotationY: -15,
      z: -100,
      willChange: 'transform, opacity',
      force3D: true
    });

    const ctx = gsap.context(() => {
      const tl = gsap.timeline({
        scrollTrigger: {
          trigger: containerRef,
          start: 'top 75%',
          end: 'top 25%',
          toggleActions: 'play none none reverse'
        }
      });

      tl.to(cards, {
        opacity: 1,
        scale: 1,
        rotationY: 0,
        z: 0,
        duration: 1.6,
        stagger: {
          each: 0.15,
          ease: 'expo.out'
        },
        ease: 'expo.out',
        onComplete: () => {
          gsap.set(cards, { willChange: 'auto' });
        }
      });

      // Parallax effect on scroll
      cards.forEach((card, i) => {
        gsap.to(card, {
          y: i % 2 === 0 ? -30 : 30,
          scrollTrigger: {
            trigger: card,
            start: 'top bottom',
            end: 'bottom top',
            scrub: 1.5
          }
        });
      });

      // Hover animations
      cards.forEach((card) => {
        const quote = card.querySelector('.quote-icon');
        const avatar = card.querySelector('.avatar');

        card.addEventListener('mouseenter', () => {
          gsap.to(card, {
            scale: 1.02,
            y: -8,
            boxShadow: '0 25px 50px -12px rgba(15, 164, 175, 0.25)',
            duration: 0.4,
            ease: 'power2.out'
          });

          gsap.to(quote, {
            scale: 1.1,
            rotation: 5,
            duration: 0.3,
            ease: 'back.out(2)'
          });

          gsap.to(avatar, {
            scale: 1.05,
            duration: 0.3,
            ease: 'back.out(2)'
          });
        });

        card.addEventListener('mouseleave', () => {
          gsap.to(card, {
            scale: 1,
            y: 0,
            boxShadow: '0 10px 15px -3px rgba(0, 0, 0, 0.1)',
            duration: 0.4,
            ease: 'power2.out'
          });

          gsap.to(quote, {
            scale: 1,
            rotation: 0,
            duration: 0.3,
            ease: 'power2.out'
          });

          gsap.to(avatar, {
            scale: 1,
            duration: 0.3,
            ease: 'power2.out'
          });
        });
      });
    }, containerRef as HTMLElement);

    return () => {
      ctx.revert();
      gsap.set(cards, { clearProps: 'all' });
    };
  });
</script>

<section bind:this={containerRef} class="relative overflow-hidden py-16 sm:py-20 lg:py-32 bg-off-white">
  <!-- Animated background gradient -->
  <div class="absolute inset-0 opacity-30">
    <div class="absolute inset-0 bg-linear-to-br from-teal/5 via-transparent to-navy/5"></div>
  </div>

  <div class="relative z-10 mx-auto max-w-[1200px] px-4 sm:px-6 lg:px-8">
    <SectionHeader
      eyebrow="Testimonials"
      title="Trusted by Hundreds of Traders"
      subtitle="See what our members are saying about their experience with Explosive Swings."
    />

    <div class="grid gap-8 md:grid-cols-2 lg:grid-cols-3">
      {#each testimonials as testimonial, i}
        <div
          class="testimonial-card group relative flex flex-col overflow-hidden rounded-3xl bg-white p-8 shadow-lg ring-1 ring-grey-200/50 transition-all duration-500"
          style="perspective: 1000px;"
        >
          <!-- Quote icon -->
          <div class="quote-icon mb-6 flex h-14 w-14 items-center justify-center rounded-2xl bg-linear-to-br {testimonial.gradient} shadow-lg">
            <Quotes size={28} weight="fill" color="white" />
          </div>

          <!-- Rating -->
          <div class="mb-4 flex gap-1">
            {#each Array(testimonial.rating) as _, j}
              <Star size={18} weight="fill" color="#D4A843" />
            {/each}
          </div>

          <!-- Text -->
          <p class="mb-6 flex-1 text-base leading-relaxed text-grey-700">
            "{testimonial.text}"
          </p>

          <!-- Author -->
          <div class="flex items-center gap-4 border-t border-grey-100 pt-6">
            <div
              class="avatar flex h-12 w-12 shrink-0 items-center justify-center rounded-full bg-linear-to-br {testimonial.gradient} text-sm font-bold text-white shadow-md"
            >
              {testimonial.avatar}
            </div>
            <div>
              <h4 class="text-sm font-bold text-navy">{testimonial.name}</h4>
              <p class="text-xs text-grey-500">{testimonial.role}</p>
            </div>
          </div>

          <!-- Subtle glow effect on hover -->
          <div class="pointer-events-none absolute inset-0 rounded-3xl opacity-0 transition-opacity duration-500 group-hover:opacity-100">
            <div class="absolute inset-0 rounded-3xl bg-linear-to-br {testimonial.gradient} opacity-5"></div>
          </div>
        </div>
      {/each}
    </div>

    <!-- Social proof stats -->
    <div class="mt-16 grid gap-6 sm:grid-cols-3">
      <div class="rounded-2xl border border-grey-200 bg-white p-6 text-center">
        <div class="mb-2 text-4xl font-bold text-teal font-heading">500+</div>
        <p class="text-sm text-grey-600">Active Members</p>
      </div>
      <div class="rounded-2xl border border-grey-200 bg-white p-6 text-center">
        <div class="mb-2 text-4xl font-bold text-teal font-heading">4.9/5</div>
        <p class="text-sm text-grey-600">Average Rating</p>
      </div>
      <div class="rounded-2xl border border-grey-200 bg-white p-6 text-center">
        <div class="mb-2 text-4xl font-bold text-teal font-heading">95%</div>
        <p class="text-sm text-grey-600">Renewal Rate</p>
      </div>
    </div>
  </div>
</section>
