<script lang="ts">
	import { onMount } from 'svelte';
	import { gsap } from 'gsap';
	import Button from '$lib/components/ui/Button.svelte';
	import SampleAlertCard from './SampleAlertCard.svelte';
	import ArrowRight from 'phosphor-svelte/lib/ArrowRight';

	let heroRef: HTMLElement | undefined = $state();
	let glowRef: HTMLElement | undefined = $state();

	onMount(() => {
		if (!heroRef) return;

		const elements = [
			'.hero-badge',
			'.hero-title',
			'.hero-subtitle',
			'.hero-actions',
			'.hero-trust'
		];

		gsap.set(elements, {
			opacity: 0,
			y: 24,
			willChange: 'transform, opacity',
			force3D: true
		});

		const ctx = gsap.context(() => {
			const tl = gsap.timeline({
				defaults: { ease: 'expo.out', duration: 1.6, force3D: true },
				delay: 0.3
			});

			// Flowing cascade — each element glides in with heavy overlap
			tl.to('.hero-badge', { opacity: 1, y: 0, duration: 1.2 })
				.to('.hero-title', { opacity: 1, y: 0 }, '-=1.0')
				.to('.hero-subtitle', { opacity: 1, y: 0 }, '-=1.2')
				.to('.hero-actions', { opacity: 1, y: 0 }, '-=1.2')
				.to('.hero-trust', { opacity: 1, y: 0 }, '-=1.2')
				.call(() => {
					gsap.set(elements, { willChange: 'auto' });
				});

			// Cinematic glow orb — ultra-slow breathing
			if (glowRef) {
				gsap.to(glowRef, {
					scale: 1.12,
					opacity: 0.75,
					duration: 6,
					ease: 'sine.inOut',
					yoyo: true,
					repeat: -1,
					force3D: true
				});
			}
		}, heroRef as HTMLElement);

		return () => {
			ctx.revert();
			gsap.set(elements, { clearProps: 'all' });
		};
	});

	function scrollToHowItWorks() {
		const element = document.getElementById('how-it-works');
		if (element) {
			element.scrollIntoView({ behavior: 'smooth' });
		}
	}
</script>

<section bind:this={heroRef} class="relative min-h-screen overflow-hidden pt-16">
	<!-- Background -->
	<div class="from-navy via-navy-mid to-deep-blue absolute inset-0 bg-linear-to-br"></div>

	<!-- Grid Overlay -->
	<div
		class="absolute inset-0 opacity-[0.02]"
		style="background-image: linear-gradient(to right, white 1px, transparent 1px), linear-gradient(to bottom, white 1px, transparent 1px); background-size: 60px 60px;"
	></div>

	<!-- Glow Orb -->
	<div
		bind:this={glowRef}
		class="pointer-events-none absolute top-20 right-10 h-[700px] w-[700px] rounded-full opacity-60"
		style="background: radial-gradient(circle, rgba(15, 164, 175, 0.3) 0%, transparent 70%);"
	></div>

	<div class="relative z-10 mx-auto max-w-[1200px] px-4 py-20 sm:px-6 lg:px-8 lg:py-32">
		<div class="grid items-center gap-12 lg:grid-cols-2 lg:gap-16">
			<!-- Left Column -->
			<div class="space-y-8">
				<!-- Eyebrow Badge -->
				<div
					class="hero-badge border-teal/30 bg-teal/10 inline-flex items-center gap-2 rounded-full border px-4 py-2"
				>
					<span class="bg-teal h-2 w-2 animate-pulse rounded-full"></span>
					<span class="text-teal-light text-xs font-semibold tracking-wide uppercase">
						Weekly watchlist delivered every Sunday night
					</span>
				</div>

				<!-- Title -->
				<h1 class="hero-title text-white">
					Simple, Early Stock Alerts <span class="text-teal-light">You Can Actually Use</span>
				</h1>

				<!-- Subtitle -->
				<p class="hero-subtitle text-grey-300">
					Every Sunday night, get a detailed watchlist of 5–7 top stock picks with defined entries,
					targets, exits, and stops — so you're ready before the market opens.
				</p>

				<!-- Actions -->
				<div class="hero-actions flex flex-wrap gap-4">
					<Button variant="primary" href="#pricing">
						Get Instant Access
						<ArrowRight size={20} weight="bold" class="h-5 w-5" />
					</Button>
					<Button variant="ghost" onclick={scrollToHowItWorks}>See How It Works</Button>
				</div>

				<!-- Trust Line -->
				<div class="hero-trust flex items-center gap-3 pt-4">
					<div
						class="flex h-10 w-10 items-center justify-center rounded-full text-sm font-bold text-white"
						style="background: linear-gradient(135deg, #0FA4AF 0%, #1A3A6B 100%);"
					>
						BR
					</div>
					<p class="text-grey-400 text-sm">
						Created by <span class="font-semibold text-white">Billy Ribeiro</span> — former lead trader
						at Simpler Trading, mentored by Goldman Sachs' Mark McGoldrick
					</p>
				</div>
			</div>

			<!-- Right Column -->
			<div class="flex justify-center lg:justify-end">
				<SampleAlertCard delay={0.6} />
			</div>
		</div>
	</div>
</section>
