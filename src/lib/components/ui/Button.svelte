<script lang="ts">
  import { type Snippet } from 'svelte';

  interface Props {
    variant?: 'primary' | 'ghost' | 'outline';
    href?: string;
    onclick?: () => void;
    children: Snippet;
  }

  let { variant = 'primary', href, onclick, children }: Props = $props();

  const baseClasses = 'inline-flex items-center justify-center gap-2 px-6 py-3 rounded-lg font-ui font-semibold text-sm transition-all duration-200 focus:outline-none focus:ring-2 focus:ring-teal focus:ring-offset-2';
  
  const variantClasses = {
    primary: 'bg-teal text-white hover:bg-teal-light shadow-lg shadow-teal/25',
    ghost: 'bg-white/10 text-white border border-white/20 hover:bg-white/20 backdrop-blur-sm',
    outline: 'bg-transparent text-navy border-2 border-navy hover:bg-navy hover:text-white',
  };

  const classes = `${baseClasses} ${variantClasses[variant]}`;
</script>

{#if href}
  <a {href} class={classes}>
    {@render children()}
  </a>
{:else}
  <button {onclick} class={classes}>
    {@render children()}
  </button>
{/if}
