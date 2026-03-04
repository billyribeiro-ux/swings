<script lang="ts">
	import { onMount } from 'svelte';
	import { gsap } from 'gsap';
	import ScrollReveal from '$lib/components/ui/ScrollReveal.svelte';
	import ArrowRight from 'phosphor-svelte/lib/ArrowRight';
	import Article from 'phosphor-svelte/lib/Article';
	import CalendarBlank from 'phosphor-svelte/lib/CalendarBlank';
	import Clock from 'phosphor-svelte/lib/Clock';

	let heroRef: HTMLElement | undefined = $state();

	onMount(() => {
		if (!heroRef) return;

		const els = ['.blog-badge', '.blog-title', '.blog-subtitle'];
		gsap.set(els, { opacity: 0, y: 24, willChange: 'transform, opacity' });

		const ctx = gsap.context(() => {
			const tl = gsap.timeline({ delay: 0.15 });
			tl.to('.blog-badge', { opacity: 1, y: 0, duration: 0.6, ease: 'power3.out' })
				.to('.blog-title', { opacity: 1, y: 0, duration: 0.7, ease: 'power3.out' }, '-=0.35')
				.to('.blog-subtitle', { opacity: 1, y: 0, duration: 0.7, ease: 'power3.out' }, '-=0.4')
				.call(() => {
					gsap.set(els, { willChange: 'auto', clearProps: 'transform' });
				});
		}, heroRef as HTMLElement);

		return () => {
			ctx.revert();
			gsap.set(els, { clearProps: 'all' });
		};
	});

	const posts = [
		{
			slug: 'understanding-options-greeks',
			title: 'Understanding Options Greeks: A Complete Guide',
			excerpt: 'Master Delta, Gamma, Theta, and Vega to make smarter options trading decisions.',
			date: '2026-03-01',
			readTime: '8 min',
			category: 'Education'
		},
		{
			slug: 'weekly-watchlist-breakdown',
			title: 'How We Build Our Weekly Watchlists',
			excerpt:
				'A behind-the-scenes look at our process for identifying high-probability options setups.',
			date: '2026-02-28',
			readTime: '6 min',
			category: 'Strategy'
		},
		{
			slug: 'risk-management-essentials',
			title: 'Risk Management Essentials for Options Traders',
			excerpt: 'Learn how to protect your capital and survive the inevitable losing streaks.',
			date: '2026-02-25',
			readTime: '10 min',
			category: 'Risk Management'
		}
	];
</script>

<svelte:head>
	<title>Blog — Explosive Swings</title>
	<meta
		name="description"
		content="Options trading insights, strategies, and education from the Explosive Swings team."
	/>
</svelte:head>

<!-- Hero -->
<section
	bind:this={heroRef}
	class="from-navy via-navy-mid to-deep-blue relative overflow-hidden bg-linear-to-br pt-16"
>
	<div
		class="absolute inset-0 opacity-[0.02]"
		style="background-image: linear-gradient(to right, white 1px, transparent 1px), linear-gradient(to bottom, white 1px, transparent 1px); background-size: 60px 60px;"
	></div>

	<div class="relative z-10 mx-auto max-w-[1200px] px-4 py-20 text-center sm:px-6 lg:px-8 lg:py-28">
		<div
			class="blog-badge border-teal/30 bg-teal/10 mb-6 inline-flex items-center gap-2 rounded-full border px-4 py-2"
		>
			<Article size={18} weight="duotone" color="#15C5D1" />
			<span class="text-teal-light text-xs font-semibold tracking-wide uppercase">Blog</span>
		</div>

		<h1
			class="blog-title font-heading mb-6 text-3xl leading-tight font-bold text-white sm:text-4xl md:text-5xl lg:text-6xl"
		>
			Trading Insights & Education
		</h1>

		<p
			class="blog-subtitle text-grey-300 mx-auto max-w-2xl text-base leading-relaxed sm:text-lg lg:text-xl"
		>
			Strategies, analysis, and lessons from the trading desk to help you level up your options
			game.
		</p>
	</div>
</section>

<!-- Posts Grid -->
<section class="bg-off-white py-16 sm:py-20 lg:py-28">
	<div class="mx-auto max-w-[1200px] px-4 sm:px-6 lg:px-8">
		<ScrollReveal>
			<div class="grid gap-6 sm:gap-8 md:grid-cols-2 lg:grid-cols-3">
				{#each posts as post, i}
					<article
						class="reveal-item group ring-grey-200/80 hover:ring-teal/30 flex flex-col overflow-hidden rounded-2xl bg-white shadow-sm ring-1 transition-all duration-500 ease-out hover:-translate-y-1 hover:shadow-xl"
						style="transition-delay: {i * 0.08}s"
					>
						<div class="flex-1 p-6 sm:p-8">
							<div class="text-grey-500 mb-4 flex items-center gap-3 text-xs">
								<span class="inline-flex items-center gap-1">
									<CalendarBlank size={14} weight="bold" class="text-grey-400" />
									{new Date(post.date).toLocaleDateString('en-US', {
										month: 'short',
										day: 'numeric',
										year: 'numeric'
									})}
								</span>
								<span>•</span>
								<span class="inline-flex items-center gap-1">
									<Clock size={14} weight="bold" class="text-grey-400" />
									{post.readTime}
								</span>
							</div>

							<span
								class="bg-teal/10 text-teal mb-3 inline-block rounded-lg px-3 py-1 text-xs font-semibold"
							>
								{post.category}
							</span>

							<h2
								class="text-navy font-heading group-hover:text-teal mb-3 text-xl leading-snug font-bold transition-colors duration-300"
							>
								{post.title}
							</h2>

							<p class="text-grey-600 mb-6 text-sm leading-relaxed">
								{post.excerpt}
							</p>

							<a
								href="/blog/{post.slug}"
								class="text-teal group-hover:text-teal-light inline-flex items-center gap-1.5 text-sm font-semibold transition-all duration-300 group-hover:translate-x-0.5"
							>
								Read More
								<ArrowRight size={14} weight="bold" />
							</a>
						</div>
					</article>
				{/each}
			</div>
		</ScrollReveal>
	</div>
</section>

<!-- Coming Soon -->
<section class="bg-white py-16 sm:py-20 lg:py-28">
	<div class="mx-auto max-w-[1200px] px-4 text-center sm:px-6 lg:px-8">
		<ScrollReveal>
			<div
				class="reveal-item border-grey-200 bg-off-white mx-auto max-w-2xl rounded-2xl border p-8 sm:p-10"
			>
				<Article size={48} weight="duotone" color="#0FA4AF" class="mx-auto mb-4" />
				<h3 class="text-navy font-heading mb-3 text-2xl font-bold">More Content Coming Soon</h3>
				<p class="text-grey-600 leading-relaxed">
					We're constantly publishing new insights, strategies, and educational content. Check back
					regularly or subscribe to our newsletter to stay updated.
				</p>
			</div>
		</ScrollReveal>
	</div>
</section>
