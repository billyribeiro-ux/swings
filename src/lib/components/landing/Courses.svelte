<script lang="ts">
  import SectionHeader from '$lib/components/ui/SectionHeader.svelte';
  import ScrollReveal from '$lib/components/ui/ScrollReveal.svelte';
  import BookOpen from 'phosphor-svelte/lib/BookOpen';
  import Pulse from 'phosphor-svelte/lib/Pulse';
  import ArrowRight from 'phosphor-svelte/lib/ArrowRight';
  import { courses } from '$lib/data/courses';

  const iconMap = {
    BookOpen,
    Pulse,
  };
</script>

<section class="py-20 lg:py-32 bg-off-white">
  <div class="max-w-[1200px] mx-auto px-4 sm:px-6 lg:px-8">
    <ScrollReveal>
      <SectionHeader
        eyebrow="Education"
        title="Level Up Your Options Game"
        subtitle="Structured courses designed to take you from the basics to confidently trading options — at your own pace."
      />

      <div class="grid md:grid-cols-2 gap-8">
        {#each courses as course, i}
          {@const IconComponent = iconMap[course.icon as keyof typeof iconMap]}
          <div 
            class="reveal-item group bg-white rounded-2xl overflow-hidden shadow-sm hover:shadow-lg transition-all duration-300 border border-grey-100"
            style="transition-delay: {i * 0.15}s"
          >
            <!-- Visual Header -->
            <div 
              class="h-40 relative overflow-hidden"
              style="background: linear-gradient(135deg, {course.gradient.from} 0%, {course.gradient.to} 100%);"
            >
              <!-- Grid Pattern -->
              <div 
                class="absolute inset-0 opacity-10"
                style="background-image: linear-gradient(to right, white 1px, transparent 1px), linear-gradient(to bottom, white 1px, transparent 1px); background-size: 20px 20px;"
              ></div>
              
              <!-- Level Badge -->
              <div class="absolute top-4 left-4">
                <span class="px-3 py-1 bg-white/20 backdrop-blur-sm text-white text-xs font-semibold rounded-full border border-white/20">
                  {course.level}
                </span>
              </div>

              <!-- Icon -->
              <div class="absolute inset-0 flex items-center justify-center">
                <div class="w-16 h-16 bg-white/10 backdrop-blur-sm rounded-xl flex items-center justify-center border border-white/20">
                  <IconComponent size={32} weight="duotone" color="white" />
                </div>
              </div>
            </div>

            <!-- Body -->
            <div class="p-6">
              <h3 class="text-xl font-bold text-navy mb-2 font-heading">{course.title}</h3>
              <p class="text-grey-600 text-sm leading-relaxed mb-4">{course.description}</p>
              
              <div class="flex items-center justify-between">
                <span class="text-xs text-grey-500">{course.meta}</span>
                <a href="#" class="inline-flex items-center gap-1 text-teal font-semibold text-sm hover:text-teal-light transition-colors">
                  Learn More
                  <ArrowRight size={16} />
                </a>
              </div>
            </div>
          </div>
        {/each}
      </div>
    </ScrollReveal>
  </div>
</section>
