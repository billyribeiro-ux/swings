<script lang="ts">
  const currentYear = new Date().getFullYear();
</script>

<footer class="bg-navy border-t border-white/10 py-12">
  <div class="max-w-[1200px] mx-auto px-4 sm:px-6 lg:px-8">
    <!-- Copyright & Links -->
    <div class="flex flex-col md:flex-row items-center justify-between gap-4 mb-8">
      <p class="text-grey-400 text-sm">
        © {currentYear} Explosive Swings. All rights reserved.
      </p>
      <div class="flex items-center gap-6">
        <a href="#" class="text-sm text-teal-light hover:text-teal transition-colors">Terms</a>
        <a href="#" class="text-sm text-teal-light hover:text-teal transition-colors">Privacy</a>
        <a href="#" class="text-sm text-teal-light hover:text-teal transition-colors">Contact</a>
      </div>
    </div>

    <!-- Risk Disclaimer -->
    <div class="border-t border-white/10 pt-8">
      <p class="text-xs text-grey-500 text-center max-w-3xl mx-auto">
        Trading involves risk. Past performance does not guarantee future results. Always do your own due diligence.
      </p>
    </div>
  </div>
</footer>
