<script lang="ts">
	import { type Snippet } from 'svelte';
	import { onMount } from 'svelte';
	import { gsap } from 'gsap';
	import { ScrollTrigger } from 'gsap/ScrollTrigger';

	interface Props {
		children: Snippet;
		selector?: string;
		y?: number;
		scale?: number;
		rotation?: number;
		duration?: number;
		delay?: number;
		stagger?: number;
		start?: string;
		parallax?: boolean;
	}

	let {
		children,
		selector = '.reveal-item',
		y = 40,
		scale = 0.95,
		rotation = 0,
		duration = 1.4,
		delay = 0,
		stagger = 0.08,
		start = 'top 80%',
		parallax = false
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
			scale,
			rotation,
			willChange: 'transform, opacity',
			force3D: true
		});

		const ctx = gsap.context(() => {
			gsap.to(animTargets, {
				opacity: 1,
				y: 0,
				scale: 1,
				rotation: 0,
				duration,
				delay,
				stagger: {
					each: stagger,
					ease: 'power2.out'
				},
				ease: 'expo.out',
				force3D: true,
				scrollTrigger: {
					trigger: container,
					start,
					once: true
				},
				onComplete: () => {
					gsap.set(animTargets, { willChange: 'auto' });
				}
			});

			// Optional parallax effect
			if (parallax) {
				animTargets.forEach((target, i) => {
					gsap.to(target, {
						y: i % 2 === 0 ? -20 : 20,
						scrollTrigger: {
							trigger: target,
							start: 'top bottom',
							end: 'bottom top',
							scrub: 1.5
						}
					});
				});
			}
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
