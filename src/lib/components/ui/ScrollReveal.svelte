<script lang="ts">
	import { type Snippet, untrack } from 'svelte';
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
		delay: _delay = 0,
		stagger = 0.12,
		start = 'top 82%',
		ease = 'expo.out'
	}: Props = $props();

	let container: HTMLElement | undefined = $state();

	$effect(() => {
		if (!container) return;
		const root = container;

		// Snapshot props once at mount so changing them post-mount doesn't tear the
		// reveal animation down (matches the legacy onMount semantics).
		const opts = untrack(() => ({ y, blur, scale, duration, stagger, start, ease, selector }));

		const targets = root.querySelectorAll(opts.selector);
		const animTargets = targets.length > 0 ? targets : root.children;
		if (!animTargets.length) return;

		const ctx = gsap.context(() => {
			createCinematicReveal({
				targets: animTargets,
				trigger: root,
				y: opts.y,
				blur: opts.blur,
				scale: opts.scale,
				duration: opts.duration,
				stagger: opts.stagger,
				ease: opts.ease,
				start: opts.start
			});
		}, root);

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
