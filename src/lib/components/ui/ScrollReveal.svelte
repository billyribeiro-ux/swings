<script lang="ts">
	import { type Snippet } from 'svelte';
	import { onMount } from 'svelte';
	import { gsap } from 'gsap';
	import { ScrollTrigger } from 'gsap/ScrollTrigger';

	interface Props {
		children: Snippet;
		selector?: string;
		y?: number;
		duration?: number;
		delay?: number;
		stagger?: number;
		start?: string;
	}

	let {
		children,
		selector = '.reveal-item',
		y = 32,
		duration = 0.9,
		delay = 0,
		stagger = 0.1,
		start = 'top 85%'
	}: Props = $props();

	let container: HTMLElement | undefined = $state();

	onMount(() => {
		if (!container) return;

		gsap.registerPlugin(ScrollTrigger);

		const targets = container.querySelectorAll(selector);
		const animTargets = targets.length > 0 ? targets : container.children;
		if (!animTargets.length) return;

		gsap.set(animTargets, {
			opacity: 0,
			y,
			willChange: 'transform, opacity'
		});

		const ctx = gsap.context(() => {
			gsap.to(animTargets, {
				opacity: 1,
				y: 0,
				duration,
				delay,
				stagger,
				ease: 'power3.out',
				scrollTrigger: {
					trigger: container,
					start,
					once: true
				},
				onComplete: () => {
					gsap.set(animTargets, { willChange: 'auto', clearProps: 'transform' });
				}
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
