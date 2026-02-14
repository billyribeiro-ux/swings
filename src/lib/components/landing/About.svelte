<script lang="ts">
  import SectionHeader from '$lib/components/ui/SectionHeader.svelte';
  import ScrollReveal from '$lib/components/ui/ScrollReveal.svelte';

  const stats = [
    { value: '18K+', label: 'Active Traders' },
    { value: '600%', label: 'OXY Overnight Return' },
    { value: '573x', label: '0DTE SPX Return' },
    { value: '5-7', label: 'Picks Per Week' },
  ];
</script>

<section class="py-20 lg:py-32 bg-navy">
  <div class="max-w-[1200px] mx-auto px-4 sm:px-6 lg:px-8">
    <ScrollReveal>
      <div class="grid lg:grid-cols-2 gap-12 lg:gap-16 items-start">
        <!-- Left Column -->
        <div class="reveal-item space-y-6">
          <span class="hero-eyebrow text-teal-light block">About Billy Ribeiro</span>
          <h2 class="text-3xl md:text-4xl font-bold text-white font-heading">The Trader Behind the Alerts</h2>
          
          <div class="space-y-4 text-grey-400 leading-relaxed">
            <p>
              Billy Ribeiro is a <strong class="text-white">high-performance options trader</strong> known for his precision, discipline, and consistency. During his time at <strong class="text-white">Simpler Trading</strong>, he quickly became their <strong class="text-white">lead trader</strong> — at one point generating more winning trades in a single week than the entire staff combined.
            </p>
            <p>
              Mentored by <strong class="text-white">Mark McGoldrick of Goldman Sachs</strong>, Billy developed his proprietary <strong class="text-white">"Move Prior to The Move"</strong> methodology — a framework for identifying institutional-quality setups before the crowd catches on.
            </p>
            <p>
              After recovering from cancer, Billy shifted his focus toward sustainable, high-impact trading education. This alert service allows him to share his expertise without the stress of daily live trading — and gives members access to <strong class="text-white">clean, early, high-probability setups</strong>.
            </p>
          </div>
        </div>

        <!-- Right Column - Stats Grid -->
        <div class="reveal-item grid grid-cols-2 gap-4">
          {#each stats as stat, i}
            <div 
              class="bg-navy-mid border border-white/10 rounded-xl p-6 text-center"
              style="transition-delay: {i * 0.1}s"
            >
              <div class="kpi-value text-teal-light mb-1">{stat.value}</div>
              <div class="kpi-label text-grey-400">{stat.label}</div>
            </div>
          {/each}
        </div>
      </div>
    </ScrollReveal>
  </div>
</section>
