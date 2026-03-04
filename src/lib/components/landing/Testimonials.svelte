<script lang="ts">
	import { onMount } from 'svelte';
	import { gsap } from 'gsap';
	import { ScrollTrigger } from 'gsap/ScrollTrigger';
	import SectionHeader from '$lib/components/ui/SectionHeader.svelte';
	import Star from 'phosphor-svelte/lib/Star';
	import Quotes from 'phosphor-svelte/lib/Quotes';

	let containerRef: HTMLElement | undefined = $state();

	const testimonials = [
		{
			name: 'Michael Chen',
			role: 'Full-Time Trader',
			avatar: 'MC',
			rating: 5,
			text: "Billy's watchlists have completely transformed my trading. The entry zones are spot-on, and the risk management is exactly what I needed. I've gone from guessing to executing with confidence.",
			gradient: 'from-teal to-teal-light'
		},
		{
			name: 'Sarah Martinez',
			role: 'Portfolio Manager',
			avatar: 'SM',
			rating: 5,
			text: "I've tried dozens of trading services, and Explosive Swings is the only one I trust. No fluff, no spam—just high-quality setups delivered every Sunday night. The Discord community is gold.",
			gradient: 'from-gold to-gold-light'
		},
		{
			name: 'David Thompson',
			role: 'Options Trader',
			avatar: 'DT',
			rating: 5,
			text: 'The courses alone are worth 10x the subscription price. Billy breaks down complex strategies in a way that actually makes sense. My win rate has improved dramatically since joining.',
			gradient: 'from-deep-blue to-teal'
		}
	];

	onMount(() => {
		if (!containerRef) return;

		gsap.registerPlugin(ScrollTrigger);

		const cards = containerRef.querySelectorAll('.testimonial-card');
		const stats = containerRef.querySelectorAll('.stat-item');

		gsap.set(cards, { opacity: 0, y: 32, willChange: 'transform, opacity' });
		gsap.set(stats, { opacity: 0, y: 20, willChange: 'transform, opacity' });

		const ctx = gsap.context(() => {
			// Cards: simple staggered fade-up
			gsap.to(cards, {
				opacity: 1,
				y: 0,
				duration: 0.8,
				stagger: 0.12,
				ease: 'power3.out',
				scrollTrigger: {
					trigger: containerRef,
					start: 'top 80%',
					once: true
				},
				onComplete: () => {
					gsap.set(cards, { willChange: 'auto', clearProps: 'transform' });
				}
			});

			// Stats: fade up after cards
			gsap.to(stats, {
				opacity: 1,
				y: 0,
				duration: 0.7,
				stagger: 0.08,
				ease: 'power3.out',
				scrollTrigger: {
					trigger: stats[0]?.parentElement,
					start: 'top 90%',
					once: true
				},
				onComplete: () => {
					gsap.set(stats, { willChange: 'auto', clearProps: 'transform' });
				}
			});
		}, containerRef as HTMLElement);

		return () => {
			ctx.revert();
			gsap.set([...cards, ...stats], { clearProps: 'all' });
		};
	});
</script>

<section
	bind:this={containerRef}
	class="bg-off-white relative overflow-hidden py-16 sm:py-20 lg:py-32"
>
	<div class="relative z-10 mx-auto max-w-[1200px] px-4 sm:px-6 lg:px-8">
		<SectionHeader
			eyebrow="Testimonials"
			title="Trusted by Thousands of Traders"
			subtitle="See what our members are saying about their experience with Explosive Swings."
		/>

		<div class="grid gap-8 md:grid-cols-2 lg:grid-cols-3">
			{#each testimonials as testimonial, i}
				<div
					class="testimonial-card group ring-grey-200/60 hover:ring-teal/20 relative flex flex-col overflow-hidden rounded-2xl bg-white p-8 shadow-sm ring-1 transition-shadow duration-300 hover:shadow-lg"
				>
					<!-- Quote icon -->
					<div
						class="mb-6 flex h-12 w-12 items-center justify-center rounded-xl bg-linear-to-br {testimonial.gradient} shadow-sm"
					>
						<Quotes size={24} weight="fill" color="white" />
					</div>

					<!-- Rating -->
					<div class="mb-4 flex gap-0.5">
						{#each Array(testimonial.rating) as _, j}
							<Star size={16} weight="fill" color="#D4A843" />
						{/each}
					</div>

					<!-- Text -->
					<p class="text-grey-700 mb-6 flex-1 leading-relaxed">
						"{testimonial.text}"
					</p>

					<!-- Author -->
					<div class="border-grey-100 flex items-center gap-3 border-t pt-6">
						<div
							class="flex h-10 w-10 shrink-0 items-center justify-center rounded-full bg-linear-to-br {testimonial.gradient} text-xs font-bold text-white"
						>
							{testimonial.avatar}
						</div>
						<div>
							<h4 class="text-navy text-sm font-bold">{testimonial.name}</h4>
							<p class="text-grey-500 text-xs">{testimonial.role}</p>
						</div>
					</div>
				</div>
			{/each}
		</div>

		<!-- Social proof stats -->
		<div class="mt-16 grid gap-6 sm:grid-cols-3">
			<div class="stat-item border-grey-200 rounded-2xl border bg-white p-6 text-center">
				<div class="text-teal font-heading mb-2 text-4xl font-bold">18,000+</div>
				<p class="text-grey-600 text-sm">Active Traders</p>
			</div>
			<div class="stat-item border-grey-200 rounded-2xl border bg-white p-6 text-center">
				<div class="text-teal font-heading mb-2 text-4xl font-bold">4.9/5</div>
				<p class="text-grey-600 text-sm">Average Rating</p>
			</div>
			<div class="stat-item border-grey-200 rounded-2xl border bg-white p-6 text-center">
				<div class="text-teal font-heading mb-2 text-4xl font-bold">95%</div>
				<p class="text-grey-600 text-sm">Renewal Rate</p>
			</div>
		</div>
	</div>
</section>
