<script lang="ts">
	import { onMount } from 'svelte';
	import Button from '$lib/components/ui/Button.svelte';
	import ArrowRight from 'phosphor-svelte/lib/ArrowRight';
	import { gsap } from 'gsap';
	import { ScrollTrigger } from 'gsap/ScrollTrigger';

	let sectionRef: HTMLElement | undefined = $state();
	let glowRef: HTMLElement | undefined = $state();

	onMount(() => {
		if (!sectionRef || !glowRef) return;
		const section = sectionRef;
		const glow = glowRef;

		gsap.registerPlugin(ScrollTrigger);

		const contentEls = section.querySelectorAll('.final-cta-content > *');
		gsap.set(contentEls, {
			opacity: 0,
			y: 24,
			willChange: 'transform, opacity'
		});

		const ctx = gsap.context(() => {
			// Glow breathing
			gsap.to(glow, {
				scale: 1.08,
				opacity: 0.6,
				duration: 6,
				ease: 'sine.inOut',
				yoyo: true,
				repeat: -1
			});

			// Staggered content reveal
			gsap.to(contentEls, {
				opacity: 1,
				y: 0,
				duration: 0.8,
				stagger: 0.1,
				ease: 'power3.out',
				scrollTrigger: {
					trigger: section,
					start: 'top 80%',
					once: true
				},
				onComplete() {
					gsap.set(contentEls, { willChange: 'auto', clearProps: 'transform' });
				}
			});
		}, section);

		return () => {
			ctx.revert();
			gsap.set(contentEls, { clearProps: 'all' });
		};
	});
</script>

<section bind:this={sectionRef} class="relative overflow-hidden py-20 lg:py-32">
	<!-- Background -->
	<div class="from-navy via-navy-mid to-deep-blue absolute inset-0 bg-linear-to-br"></div>

	<!-- Centered Glow Orb -->
	<div
		bind:this={glowRef}
		class="pointer-events-none absolute top-1/2 left-1/2 h-[600px] w-[600px] -translate-x-1/2 -translate-y-1/2 rounded-full opacity-40"
		style="background: radial-gradient(circle, rgba(15, 164, 175, 0.4) 0%, transparent 70%);"
	></div>

	<div
		class="final-cta-content relative z-10 mx-auto max-w-[1200px] px-4 text-center sm:px-6 lg:px-8"
	>
		<h2 class="font-heading mb-6 text-3xl font-bold text-white md:text-4xl lg:text-5xl">
			Trade with Clarity. Trade with Confidence.
		</h2>
		<p class="text-grey-300 mx-auto mb-8 max-w-2xl text-lg">
			Get your weekly watchlist every Sunday night — detailed entries, targets, exits, and stops so
			you're prepared before the market opens.
		</p>
		<Button variant="primary" href="#pricing">
			Get Instant Access to Alerts
			<ArrowRight size={20} weight="bold" />
		</Button>
	</div>
</section>
