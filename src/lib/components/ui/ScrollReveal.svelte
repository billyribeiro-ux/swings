<script lang="ts">
	import { type Snippet } from 'svelte';
	import { onMount } from 'svelte';
	import { gsap } from 'gsap';
	import { createCinematicReveal, isReducedMotion } from '$lib/utils/animations';

	interface Props {
		children: Snippet;
		selector?: string;
		y?: number;
		blur?: number;
		scale?: number;
		duration?: number;
		delay?: number;
		stagger?: number;
		start?: string;
		ease?: string;
	}

	let {
		children,
		selector = '.reveal-item',
		y = 40,
		blur = 6,
		scale = 0.96,
		duration = 0.9,
		delay = 0,
		stagger = 0.12,
		start = 'top 82%',
		ease = 'expo.out'
	}: Props = $props();

	let container: HTMLElement | undefined = $state();

	onMount(() => {
		if (!container) return;

		const targets = container.querySelectorAll(selector);
		const animTargets = targets.length > 0 ? targets : container.children;
		if (!animTargets.length) return;

		const ctx = gsap.context(() => {
			createCinematicReveal({
				targets: animTargets,
				trigger: container!,
				y,
				blur,
				scale,
				duration,
				stagger,
				ease,
				start
			});
		}, container as HTMLElement);

		return () => {
			ctx.revert();
			if (!isReducedMotion()) {
				gsap.set(animTargets, { clearProps: 'all' });
			}
		};
	});
</script>

<div bind:this={container}>
	{@render children()}
</div>
