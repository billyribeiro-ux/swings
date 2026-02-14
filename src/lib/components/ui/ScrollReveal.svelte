<script lang="ts">
  import { type Snippet } from 'svelte';
  import { gsap } from 'gsap';
  import { ScrollTrigger } from 'gsap/ScrollTrigger';

  if (typeof window !== 'undefined') {
    gsap.registerPlugin(ScrollTrigger);
  }

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
    stagger = 0.12,
    y = 40,
    duration = 0.8,
    delay = 0,
    start = 'top 80%',
  }: Props = $props();

  let container: HTMLElement | undefined = $state();

  $effect(() => {
    if (!container) return;
    const targets = container.querySelectorAll(selector);
    if (!targets.length) return;

    const ctx = gsap.context(() => {
      gsap.from(targets, {
        y,
        opacity: 0,
        duration,
        delay,
        stagger,
        ease: 'power3.out',
        scrollTrigger: {
          trigger: container,
          start,
          once: true,
        },
      });
    }, container as HTMLElement);

    return () => ctx.revert();
  });
</script>

<div bind:this={container}>
  {@render children()}
</div>
