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
    stagger = 0.08,
    y = 32,
    duration = 1.4,
    delay = 0,
    start = 'top 88%',
  }: Props = $props();

  let container: HTMLElement | undefined = $state();

  onMount(() => {
    if (!container) return;

    gsap.registerPlugin(ScrollTrigger);

    const targets = container.querySelectorAll(selector);
    const animTargets = targets.length > 0 ? targets : container.children;
    if (!animTargets.length) return;

    // GPU-accelerated initial state
    gsap.set(animTargets, {
      opacity: 0,
      y,
      willChange: 'transform, opacity',
      force3D: true,
    });

    const ctx = gsap.context(() => {
      gsap.to(animTargets, {
        opacity: 1,
        y: 0,
        duration,
        delay,
        stagger,
        ease: 'expo.out',
        force3D: true,
        scrollTrigger: {
          trigger: container,
          start,
          once: true,
        },
        onComplete() {
          // Release GPU layers after animation settles
          gsap.set(animTargets, { willChange: 'auto' });
        },
      });
    }, container as HTMLElement);

    return () => {
      ctx.revert();
      gsap.set(animTargets, { clearProps: 'all' });
    };
  });
</script>

<div bind:this={container}>
  {@render children()}
</div>
