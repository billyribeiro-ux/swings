<script lang="ts">
  interface Props {
    eyebrow: string;
    title: string;
    subtitle?: string;
    dark?: boolean;
  }

  let { eyebrow, title, subtitle, dark = false }: Props = $props();

  const textColor = $derived(dark ? 'text-white' : 'text-navy');
  const subtitleColor = $derived(dark ? 'text-grey-400' : 'text-grey-600');
</script>

<div class="text-center max-w-3xl mx-auto mb-12">
  <span class="hero-eyebrow text-teal block mb-4">{eyebrow}</span>
  <h2 class={`text-2xl md:text-3xl lg:text-4xl font-bold ${textColor} mb-4`}>
    {@html title}
  </h2>
  {#if subtitle}
    <p class={`text-base md:text-lg ${subtitleColor} max-w-2xl mx-auto`}>
      {subtitle}
    </p>
  {/if}
</div>
