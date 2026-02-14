<script lang="ts">
  import { type Snippet } from 'svelte';
  import { onMount } from 'svelte';
  import { gsap } from 'gsap';
  import { ScrollTrigger } from 'gsap/ScrollTrigger';

  interface Props {
    children: Snippet;
    selector?: string;
    stagger?: number;
    y?: number;
    duration?: number;
    delay?: number;
    start?: string;
  }

  let {
    children,
    selector = '.reveal-item',
    stagger = 0.15,
    y = 60,
    duration = 1,
    delay = 0,
    start = 'top 85%',
  }: Props = $props();

  let container: HTMLElement | undefined = $state();

  onMount(() => {
    if (!container) return;

    gsap.registerPlugin(ScrollTrigger);

    const targets = container.querySelectorAll(selector);

    // If no .reveal-item children found, animate the container's direct children
    const animTargets = targets.length > 0 ? targets : container.children;
    if (!animTargets.length) return;

    // Set initial state explicitly — elements start visible in DOM,
    // GSAP sets them invisible only after JS loads (progressive enhancement)
    gsap.set(animTargets, { opacity: 0, y });

    const ctx = gsap.context(() => {
      gsap.to(animTargets, {
        opacity: 1,
        y: 0,
        duration,
        delay,
        stagger,
        ease: 'power2.out',
        scrollTrigger: {
          trigger: container,
          start,
          once: true,
        },
      });
    }, container as HTMLElement);

    return () => {
      ctx.revert();
      // Ensure elements are visible after cleanup (navigation, HMR)
      gsap.set(animTargets, { clearProps: 'all' });
    };
  });
</script>

<div bind:this={container}>
  {@render children()}
</div>
