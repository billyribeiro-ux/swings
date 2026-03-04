<script lang="ts">
  import { page } from '$app/stores';
  import { courses } from '$lib/data/courses';
  import { error } from '@sveltejs/kit';
  import Button from '$lib/components/ui/Button.svelte';
  import ScrollReveal from '$lib/components/ui/ScrollReveal.svelte';
  import CheckCircle from 'phosphor-svelte/lib/CheckCircle';
  import ArrowRight from 'phosphor-svelte/lib/ArrowRight';
  import Clock from 'phosphor-svelte/lib/Clock';
  import GraduationCap from 'phosphor-svelte/lib/GraduationCap';
  import BookOpen from 'phosphor-svelte/lib/BookOpen';
  import Pulse from 'phosphor-svelte/lib/Pulse';

  const iconMap = {
    BookOpen,
    Pulse
  };

  const slug = $page.params.slug;
  const course = courses.find(c => c.slug === slug);

  if (!course) {
    error(404, 'Course not found');
  }

  const IconComponent = iconMap[course.icon as keyof typeof iconMap];
</script>

<svelte:head>
  <title>{course.title} — Explosive Swings</title>
  <meta name="description" content={course.description} />
</svelte:head>

<!-- Hero Section -->
<section 
  class="relative py-20 lg:py-32 overflow-hidden"
  style="background: linear-gradient(135deg, {course.gradient.from} 0%, {course.gradient.to} 100%);"
>
  <!-- Grid Pattern Overlay -->
  <div 
    class="absolute inset-0 opacity-10"
    style="background-image: linear-gradient(to right, white 1px, transparent 1px), linear-gradient(to bottom, white 1px, transparent 1px); background-size: 60px 60px;"
  ></div>

  <div class="relative z-10 max-w-[1200px] mx-auto px-4 sm:px-6 lg:px-8">
    <div class="grid lg:grid-cols-2 gap-12 items-center">
      <!-- Left Column -->
      <div>
        <div class="inline-flex items-center gap-2 px-4 py-2 rounded-full bg-white/20 backdrop-blur-sm border border-white/30 mb-6">
          <span class="text-xs font-semibold text-white tracking-wide uppercase">{course.level}</span>
        </div>

        <h1 class="text-4xl md:text-5xl lg:text-6xl font-bold text-white mb-6 font-heading">
          {course.title}
        </h1>

        <p class="text-xl text-white/90 mb-8 leading-relaxed">
          {course.description}
        </p>

        <div class="flex flex-wrap items-center gap-6 mb-8 text-white/80">
          <div class="flex items-center gap-2">
            <Clock size={20} weight="bold" />
            <span>{course.duration}</span>
          </div>
          <div class="flex items-center gap-2">
            <GraduationCap size={20} weight="bold" />
            <span>{course.modules} modules</span>
          </div>
          <div class="flex items-center gap-2">
            <IconComponent size={20} weight="bold" />
            <span>Self-paced</span>
          </div>
        </div>

        <div class="flex items-baseline gap-3 mb-8">
          <span class="text-5xl font-bold text-white font-heading">${course.price}</span>
          <span class="text-white/70">one-time payment</span>
        </div>

        <Button variant="primary" href="#enroll" class="bg-white text-navy hover:bg-grey-100">
          Enroll Now
          <ArrowRight size={20} weight="bold" />
        </Button>
      </div>

      <!-- Right Column - Icon -->
      <div class="flex justify-center lg:justify-end">
        <div class="w-64 h-64 bg-white/10 backdrop-blur-sm rounded-3xl flex items-center justify-center border border-white/20">
          <IconComponent size={120} weight="duotone" color="white" />
        </div>
      </div>
    </div>
  </div>
</section>

<!-- What You'll Learn -->
<section class="py-20 lg:py-32 bg-white">
  <div class="max-w-[1200px] mx-auto px-4 sm:px-6 lg:px-8">
    <ScrollReveal>
      <h2 class="text-3xl md:text-4xl font-bold text-navy mb-12 text-center font-heading">
        What You'll Learn
      </h2>

      <div class="grid md:grid-cols-2 gap-6 max-w-4xl mx-auto">
        {#each course.whatYouLearn as item, i}
          <div 
            class="reveal-item flex gap-4 p-6 rounded-lg border border-grey-200 bg-off-white"
            style="transition-delay: {i * 0.1}s"
          >
            <div class="shrink-0 w-6 h-6 flex items-center justify-center mt-0.5">
              <CheckCircle size={24} weight="fill" color="#0FA4AF" />
            </div>
            <p class="text-grey-800">{item}</p>
          </div>
        {/each}
      </div>
    </ScrollReveal>
  </div>
</section>

<!-- Curriculum -->
<section class="py-20 lg:py-32 bg-off-white">
  <div class="max-w-[1200px] mx-auto px-4 sm:px-6 lg:px-8">
    <ScrollReveal>
      <h2 class="text-3xl md:text-4xl font-bold text-navy mb-12 text-center font-heading">
        Course Curriculum
      </h2>

      <div class="max-w-3xl mx-auto space-y-6">
        {#each course.curriculum as module, i}
          <div 
            class="reveal-item bg-white rounded-xl p-8 border border-grey-200"
            style="transition-delay: {i * 0.1}s"
          >
            <h3 class="text-xl font-bold text-navy mb-4 font-heading">{module.title}</h3>
            <ul class="space-y-3">
              {#each module.lessons as lesson}
                <li class="flex items-start gap-3 text-grey-700">
                  <CheckCircle size={20} weight="fill" color="#0FA4AF" class="shrink-0 mt-0.5" />
                  <span>{lesson}</span>
                </li>
              {/each}
            </ul>
          </div>
        {/each}
      </div>
    </ScrollReveal>
  </div>
</section>

<!-- Features -->
<section class="py-20 lg:py-32 bg-white">
  <div class="max-w-[1200px] mx-auto px-4 sm:px-6 lg:px-8">
    <ScrollReveal>
      <h2 class="text-3xl md:text-4xl font-bold text-navy mb-12 text-center font-heading">
        What's Included
      </h2>

      <div class="grid md:grid-cols-2 lg:grid-cols-3 gap-6 max-w-5xl mx-auto">
        {#each course.features as feature, i}
          <div 
            class="reveal-item flex gap-4 p-6 rounded-lg border border-grey-200 bg-off-white"
            style="transition-delay: {i * 0.1}s"
          >
            <CheckCircle size={24} weight="fill" color="#0FA4AF" class="shrink-0" />
            <p class="text-grey-800">{feature}</p>
          </div>
        {/each}
      </div>
    </ScrollReveal>
  </div>
</section>

<!-- Enroll CTA -->
<section id="enroll" class="py-20 lg:py-32 bg-linear-to-br from-navy via-navy-mid to-deep-blue">
  <div class="max-w-[1200px] mx-auto px-4 sm:px-6 lg:px-8 text-center">
    <h2 class="text-3xl md:text-4xl lg:text-5xl font-bold text-white mb-6 font-heading">
      Ready to Start Learning?
    </h2>
    
    <p class="text-xl text-grey-300 max-w-2xl mx-auto mb-8">
      Get lifetime access to {course.title} for a one-time payment of ${course.price}.
    </p>

    <div class="flex flex-col sm:flex-row gap-4 justify-center">
      <Button variant="primary" href="#" class="bg-teal text-white hover:bg-teal-light">
        Enroll Now — ${course.price}
        <ArrowRight size={20} weight="bold" />
      </Button>
      <Button variant="ghost" href="/courses" class="text-white border-white/30 hover:bg-white/10">
        View All Courses
      </Button>
    </div>
  </div>
</section>
