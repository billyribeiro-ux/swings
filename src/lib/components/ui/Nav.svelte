<script lang="ts">
  import Button from './Button.svelte';
</script>

<nav class="fixed top-0 left-0 right-0 z-50 bg-navy/92 backdrop-blur-md border-b border-white/10">
  <div class="max-w-[1200px] mx-auto px-4 sm:px-6 lg:px-8 h-16 flex items-center justify-between">
    <!-- Logo -->
    <a href="/" class="flex items-center gap-0 font-heading font-bold text-xl">
      <span class="text-white">Explosive</span>
      <span class="text-teal-light">Swings</span>
    </a>

    <!-- CTA -->
    <Button variant="primary" href="#pricing">
      Get Instant Access
    </Button>
  </div>
</nav>
