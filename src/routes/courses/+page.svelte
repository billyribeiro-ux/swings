<script lang="ts">
  import { courses } from '$lib/data/courses';
  import SectionHeader from '$lib/components/ui/SectionHeader.svelte';
  import ScrollReveal from '$lib/components/ui/ScrollReveal.svelte';
  import Button from '$lib/components/ui/Button.svelte';
  import BookOpen from 'phosphor-svelte/lib/BookOpen';
  import Pulse from 'phosphor-svelte/lib/Pulse';
  import ArrowRight from 'phosphor-svelte/lib/ArrowRight';
  import Clock from 'phosphor-svelte/lib/Clock';
  import GraduationCap from 'phosphor-svelte/lib/GraduationCap';

  const iconMap = {
    BookOpen,
    Pulse
  };
</script>

<svelte:head>
  <title>Options Trading Courses — Explosive Swings</title>
  <meta name="description" content="Structured courses designed to take you from the basics to confidently trading options — at your own pace." />
</svelte:head>

<!-- Hero Section -->
<section class="relative py-20 lg:py-32 bg-linear-to-br from-navy via-navy-mid to-deep-blue overflow-hidden">
  <!-- Grid Overlay -->
  <div 
    class="absolute inset-0 opacity-[0.02]"
    style="background-image: linear-gradient(to right, white 1px, transparent 1px), linear-gradient(to bottom, white 1px, transparent 1px); background-size: 60px 60px;"
  ></div>

  <div class="relative z-10 max-w-[1200px] mx-auto px-4 sm:px-6 lg:px-8 text-center">
    <div class="inline-flex items-center gap-2 px-4 py-2 rounded-full border border-teal/30 bg-teal/10 mb-6">
      <GraduationCap size={20} weight="duotone" color="#15C5D1" />
      <span class="text-xs font-semibold text-teal-light tracking-wide uppercase">Education</span>
    </div>

    <h1 class="text-4xl md:text-5xl lg:text-6xl font-bold text-white mb-6 font-heading">
      Level Up Your Options Game
    </h1>
    
    <p class="text-xl text-grey-300 max-w-3xl mx-auto mb-8 leading-relaxed">
      Structured courses designed to take you from the basics to confidently trading options — at your own pace.
    </p>
  </div>
</section>

<!-- Courses Grid -->
<section class="py-20 lg:py-32 bg-off-white">
  <div class="max-w-[1200px] mx-auto px-4 sm:px-6 lg:px-8">
    <ScrollReveal>
      <div class="grid md:grid-cols-2 gap-8">
        {#each courses as course, i}
          {@const IconComponent = iconMap[course.icon as keyof typeof iconMap]}
          <div 
            class="reveal-item group bg-white rounded-2xl overflow-hidden shadow-sm hover:shadow-xl transition-all duration-300 border border-grey-200"
            style="transition-delay: {i * 0.15}s"
          >
            <!-- Visual Header -->
            <div 
              class="relative h-48 flex items-center justify-center overflow-hidden"
              style="background: linear-gradient(135deg, {course.gradient.from} 0%, {course.gradient.to} 100%);"
            >
              <!-- Grid Pattern Overlay -->
              <div 
                class="absolute inset-0 opacity-10"
                style="background-image: linear-gradient(to right, white 1px, transparent 1px), linear-gradient(to bottom, white 1px, transparent 1px); background-size: 40px 40px;"
              ></div>

              <!-- Level Badge -->
              <div class="absolute top-4 left-4">
                <span class="px-3 py-1 bg-white/20 backdrop-blur-sm text-white text-xs font-semibold rounded-full border border-white/30">
                  {course.level}
                </span>
              </div>

              <!-- Icon -->
              <div class="relative z-10 w-20 h-20 bg-white/10 backdrop-blur-sm rounded-2xl flex items-center justify-center border border-white/20">
                <IconComponent size={40} weight="duotone" color="white" />
              </div>
            </div>

            <!-- Body -->
            <div class="p-8">
              <h3 class="text-2xl font-bold text-navy mb-3 font-heading">{course.title}</h3>
              <p class="text-grey-600 mb-6 leading-relaxed">{course.description}</p>

              <!-- Meta Row -->
              <div class="flex items-center gap-4 mb-6 text-sm text-grey-500">
                <div class="flex items-center gap-2">
                  <Clock size={16} weight="bold" />
                  <span>{course.duration}</span>
                </div>
                <div class="flex items-center gap-2">
                  <GraduationCap size={16} weight="bold" />
                  <span>{course.modules} modules</span>
                </div>
              </div>

              <!-- Price -->
              <div class="flex items-baseline gap-2 mb-6">
                <span class="text-4xl font-bold text-navy font-heading">${course.price}</span>
                <span class="text-grey-500">one-time</span>
              </div>

              <!-- CTA -->
              <Button variant="primary" href="/courses/{course.slug}" class="w-full">
                Learn More
                <ArrowRight size={20} weight="bold" />
              </Button>
            </div>
          </div>
        {/each}
      </div>
    </ScrollReveal>
  </div>
</section>

<!-- Why Take a Course -->
<section class="py-20 lg:py-32 bg-white">
  <div class="max-w-[1200px] mx-auto px-4 sm:px-6 lg:px-8">
    <ScrollReveal>
      <SectionHeader
        eyebrow="Why Learn Options?"
        title="Build Skills That Last a Lifetime"
        subtitle="Options trading gives you leverage, flexibility, and the ability to profit in any market condition — but only if you know what you're doing."
      />

      <div class="grid md:grid-cols-3 gap-8 max-w-5xl mx-auto">
        <div class="reveal-item text-center">
          <div class="w-16 h-16 bg-teal/10 rounded-full flex items-center justify-center mx-auto mb-4">
            <BookOpen size={32} weight="duotone" color="#0FA4AF" />
          </div>
          <h3 class="text-lg font-bold text-navy mb-2 font-heading">Learn at Your Pace</h3>
          <p class="text-grey-600 text-sm">Lifetime access means you can revisit lessons whenever you need a refresher.</p>
        </div>

        <div class="reveal-item text-center">
          <div class="w-16 h-16 bg-teal/10 rounded-full flex items-center justify-center mx-auto mb-4">
            <Pulse size={32} weight="duotone" color="#0FA4AF" />
          </div>
          <h3 class="text-lg font-bold text-navy mb-2 font-heading">Real-World Examples</h3>
          <p class="text-grey-600 text-sm">Every lesson includes actual chart examples and trade setups you can apply immediately.</p>
        </div>

        <div class="reveal-item text-center">
          <div class="w-16 h-16 bg-teal/10 rounded-full flex items-center justify-center mx-auto mb-4">
            <GraduationCap size={32} weight="duotone" color="#0FA4AF" />
          </div>
          <h3 class="text-lg font-bold text-navy mb-2 font-heading">Community Support</h3>
          <p class="text-grey-600 text-sm">Join a private community of traders learning the same strategies.</p>
        </div>
      </div>
    </ScrollReveal>
  </div>
</section>
