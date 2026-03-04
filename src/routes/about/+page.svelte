<script lang="ts">
	import { onMount } from 'svelte';
	import { gsap } from 'gsap';
	import ScrollReveal from '$lib/components/ui/ScrollReveal.svelte';
	import SectionHeader from '$lib/components/ui/SectionHeader.svelte';
	import CheckCircle from 'phosphor-svelte/lib/CheckCircle';
	import TrendUp from 'phosphor-svelte/lib/TrendUp';
	import Users from 'phosphor-svelte/lib/Users';
	import ChartLineUp from 'phosphor-svelte/lib/ChartLineUp';
	import Target from 'phosphor-svelte/lib/Target';

	let heroRef: HTMLElement | undefined = $state();

	onMount(() => {
		if (!heroRef) return;

		const els = ['.about-badge', '.about-title', '.about-subtitle'];
		gsap.set(els, { opacity: 0, y: 24, willChange: 'transform, opacity', force3D: true });

		const ctx = gsap.context(() => {
			const tl = gsap.timeline({
				defaults: { ease: 'expo.out', duration: 1.4, force3D: true },
				delay: 0.2
			});
			tl.to('.about-badge', { opacity: 1, y: 0, duration: 1.0 })
				.to('.about-title', { opacity: 1, y: 0 }, '-=0.9')
				.to('.about-subtitle', { opacity: 1, y: 0 }, '-=1.0')
				.call(() => {
					gsap.set(els, { willChange: 'auto' });
				});
		}, heroRef as HTMLElement);

		return () => {
			ctx.revert();
			gsap.set(els, { clearProps: 'all' });
		};
	});
</script>

<svelte:head>
	<title>About — Explosive Swings</title>
	<meta
		name="description"
		content="Learn about Explosive Swings, our mission to empower options traders, and the team behind the weekly watchlists."
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
			class="about-badge border-teal/30 bg-teal/10 mb-6 inline-flex items-center gap-2 rounded-full border px-4 py-2"
		>
			<Users size={18} weight="duotone" color="#15C5D1" />
			<span class="text-teal-light text-xs font-semibold tracking-wide uppercase">About Us</span>
		</div>

		<h1
			class="about-title font-heading mb-6 text-3xl leading-tight font-bold text-white sm:text-4xl md:text-5xl lg:text-6xl"
		>
			Most Trading Services Give You Fish.<br />We Teach You to Read the Water.
		</h1>

		<p
			class="about-subtitle text-grey-300 mx-auto max-w-3xl text-base leading-relaxed sm:text-lg lg:text-xl"
		>
			Founded by Billy Ribeiro and Freddie Ferber, Explosive Swings was built on a single
			conviction: traders don't fail because of bad alerts — they fail because no one ever explained
			the <em>why</em> behind the trade.
		</p>
	</div>
</section>

<!-- Story -->
<section class="bg-white py-16 sm:py-20 lg:py-28">
	<div class="mx-auto max-w-[1200px] px-4 sm:px-6 lg:px-8">
		<ScrollReveal>
			<div class="mx-auto max-w-3xl">
				<h2
					class="reveal-item text-navy font-heading mb-8 text-center text-2xl font-bold sm:text-3xl md:text-4xl"
				>
					This Isn't a Signal Service. It's a Methodology.
				</h2>

				<div class="reveal-item text-grey-700 space-y-6 leading-relaxed">
					<p>
						Billy brings over a decade of institutional-grade market experience, including his time
						as head trader at ZMC Capital and mentorship under <strong>Mark McGoldrick</strong>,
						Goldman Sachs' former Global Head of Proprietary Trading. Freddie Ferber, a longtime
						student who became a world-class trader in his own right, brings a practitioner's
						perspective — the kind of clarity that only comes from learning the hard lessons
						firsthand and building real, consistent edge from them.
					</p>

					<p>
						Together, they've built something that didn't exist before: <strong
							>a service where quality replaces volume</strong
						>.
					</p>

					<p>
						Every Sunday night, members receive a curated watchlist of 5–7 high-probability options
						setups — each one hand-selected and fully structured with entry zones, profit targets,
						and defined stop losses. No noise. No filler. Just setups that meet an
						institutional-grade threshold before they ever reach your inbox.
					</p>

					<p>
						Explosive Swings serves <strong>18,000+ active traders</strong> across all experience levels,
						from professionals refining their edge to disciplined beginners building their first repeatable
						framework.
					</p>
				</div>
			</div>
		</ScrollReveal>
	</div>
</section>

<!-- What Separates Us -->
<section class="bg-off-white py-16 sm:py-20 lg:py-28">
	<div class="mx-auto max-w-[1200px] px-4 sm:px-6 lg:px-8">
		<ScrollReveal>
			<SectionHeader
				eyebrow="Our Approach"
				title="What Separates Us from Every Other Alert Service"
				subtitle="Quality over quantity. Education over noise. Methodology over signals."
			/>

			<div class="mx-auto grid max-w-4xl gap-6 sm:gap-8 md:grid-cols-2">
				<div
					class="reveal-item ring-grey-200/80 flex flex-col rounded-2xl bg-white p-6 ring-1 sm:p-8"
				>
					<div class="bg-teal/10 mb-5 flex h-14 w-14 items-center justify-center rounded-2xl">
						<Target size={28} weight="duotone" color="#0FA4AF" />
					</div>
					<h3 class="text-navy font-heading mb-2 text-lg font-bold">Fewer, Better Setups</h3>
					<p class="text-grey-600 leading-relaxed">
						5–7 per week, not 50 per day. Every setup meets institutional-grade criteria before it
						reaches you.
					</p>
				</div>

				<div
					class="reveal-item ring-grey-200/80 flex flex-col rounded-2xl bg-white p-6 ring-1 sm:p-8"
				>
					<div class="bg-teal/10 mb-5 flex h-14 w-14 items-center justify-center rounded-2xl">
						<ChartLineUp size={28} weight="duotone" color="#0FA4AF" />
					</div>
					<h3 class="text-navy font-heading mb-2 text-lg font-bold">Full Trade Structure</h3>
					<p class="text-grey-600 leading-relaxed">
						Entry zones, profit targets, and defined stop losses on every setup. No guesswork.
					</p>
				</div>

				<div
					class="reveal-item ring-grey-200/80 flex flex-col rounded-2xl bg-white p-6 ring-1 sm:p-8"
				>
					<div class="bg-teal/10 mb-5 flex h-14 w-14 items-center justify-center rounded-2xl">
						<TrendUp size={28} weight="duotone" color="#0FA4AF" />
					</div>
					<h3 class="text-navy font-heading mb-2 text-lg font-bold">Institutional Pedigree</h3>
					<p class="text-grey-600 leading-relaxed">
						Built by traders who've operated at the highest levels of the market — Goldman Sachs,
						ZMC Capital.
					</p>
				</div>

				<div
					class="reveal-item ring-grey-200/80 flex flex-col rounded-2xl bg-white p-6 ring-1 sm:p-8"
				>
					<div class="bg-teal/10 mb-5 flex h-14 w-14 items-center justify-center rounded-2xl">
						<Users size={28} weight="duotone" color="#0FA4AF" />
					</div>
					<h3 class="text-navy font-heading mb-2 text-lg font-bold">Education Embedded</h3>
					<p class="text-grey-600 leading-relaxed">
						You'll understand <em>why</em> the setup qualifies, not just what to click. Learn the methodology.
					</p>
				</div>

				<div
					class="reveal-item ring-grey-200/80 flex flex-col rounded-2xl bg-white p-6 ring-1 sm:p-8 md:col-span-2"
				>
					<div class="bg-teal/10 mb-5 flex h-14 w-14 items-center justify-center rounded-2xl">
						<CheckCircle size={28} weight="duotone" color="#0FA4AF" />
					</div>
					<h3 class="text-navy font-heading mb-2 text-lg font-bold">Designed for Real Schedules</h3>
					<p class="text-grey-600 leading-relaxed">
						Sunday delivery means you're prepared before the week begins, not chasing it. Trade on
						your terms.
					</p>
				</div>
			</div>
		</ScrollReveal>
	</div>
</section>

<!-- Team -->
<section class="bg-white py-16 sm:py-20 lg:py-28">
	<div class="mx-auto max-w-[1200px] px-4 sm:px-6 lg:px-8">
		<ScrollReveal>
			<h2
				class="reveal-item text-navy font-heading mb-12 text-center text-2xl font-bold sm:text-3xl md:text-4xl"
			>
				Meet the Founders
			</h2>

			<div class="mx-auto grid max-w-5xl gap-8 md:grid-cols-2">
				<!-- Billy Ribeiro -->
				<div class="reveal-item bg-off-white ring-grey-200/80 rounded-2xl p-8 ring-1 sm:p-10">
					<div class="mb-6 flex items-center gap-4">
						<div
							class="flex h-16 w-16 shrink-0 items-center justify-center rounded-full text-xl font-bold text-white"
							style="background: linear-gradient(135deg, #0FA4AF 0%, #1A3A6B 100%);"
						>
							BR
						</div>
						<div>
							<h3 class="text-navy font-heading text-xl font-bold">Billy Ribeiro</h3>
							<p class="text-grey-600 text-sm">Co-Founder</p>
						</div>
					</div>

					<div class="text-grey-700 space-y-4 leading-relaxed">
						<p>
							Over a decade of institutional-grade market experience, including head trader at ZMC
							Capital. Mentored by Mark McGoldrick, Goldman Sachs' former Global Head of Proprietary
							Trading.
						</p>

						<p>
							Billy's approach: focus on high-probability setups, manage risk religiously, and never
							chase trades. That philosophy is the foundation of Explosive Swings.
						</p>
					</div>

					<div class="mt-6 flex flex-wrap gap-3">
						<span
							class="bg-teal/10 text-teal inline-flex items-center gap-1.5 rounded-lg px-3 py-1.5 text-xs font-semibold"
						>
							<CheckCircle size={14} weight="fill" />
							10+ Years Trading
						</span>
						<span
							class="bg-teal/10 text-teal inline-flex items-center gap-1.5 rounded-lg px-3 py-1.5 text-xs font-semibold"
						>
							<CheckCircle size={14} weight="fill" />
							ZMC Capital
						</span>
						<span
							class="bg-teal/10 text-teal inline-flex items-center gap-1.5 rounded-lg px-3 py-1.5 text-xs font-semibold"
						>
							<CheckCircle size={14} weight="fill" />
							Goldman Sachs Mentored
						</span>
					</div>
				</div>

				<!-- Freddie Ferber -->
				<div class="reveal-item bg-off-white ring-grey-200/80 rounded-2xl p-8 ring-1 sm:p-10">
					<div class="mb-6 flex items-center gap-4">
						<div
							class="flex h-16 w-16 shrink-0 items-center justify-center rounded-full text-xl font-bold text-white"
							style="background: linear-gradient(135deg, #D4A843 0%, #0FA4AF 100%);"
						>
							FF
						</div>
						<div>
							<h3 class="text-navy font-heading text-xl font-bold">Freddie Ferber</h3>
							<p class="text-grey-600 text-sm">Co-Founder</p>
						</div>
					</div>

					<div class="text-grey-700 space-y-4 leading-relaxed">
						<p>
							A longtime student who became a world-class trader in his own right. Freddie brings a
							practitioner's perspective — the kind of clarity that only comes from learning the
							hard lessons firsthand.
						</p>

						<p>
							His focus on building real, consistent edge from experience complements Billy's
							institutional approach, creating a complete methodology for traders at every level.
						</p>
					</div>

					<div class="mt-6 flex flex-wrap gap-3">
						<span
							class="bg-gold/10 text-gold inline-flex items-center gap-1.5 rounded-lg px-3 py-1.5 text-xs font-semibold"
						>
							<CheckCircle size={14} weight="fill" />
							World-Class Trader
						</span>
						<span
							class="bg-gold/10 text-gold inline-flex items-center gap-1.5 rounded-lg px-3 py-1.5 text-xs font-semibold"
						>
							<CheckCircle size={14} weight="fill" />
							Practitioner's Edge
						</span>
						<span
							class="bg-gold/10 text-gold inline-flex items-center gap-1.5 rounded-lg px-3 py-1.5 text-xs font-semibold"
						>
							<CheckCircle size={14} weight="fill" />
							Educator
						</span>
					</div>
				</div>
			</div>
		</ScrollReveal>
	</div>
</section>

<!-- CTA -->
<section class="from-navy via-navy-mid to-deep-blue bg-linear-to-br py-16 sm:py-20 lg:py-28">
	<div class="mx-auto max-w-[1200px] px-4 text-center sm:px-6 lg:px-8">
		<ScrollReveal>
			<h2
				class="reveal-item font-heading mb-5 text-2xl font-bold text-white sm:text-3xl md:text-4xl lg:text-5xl"
			>
				Ready to Join Us?
			</h2>

			<p
				class="reveal-item text-grey-300 mx-auto mb-10 max-w-2xl text-base leading-relaxed sm:text-lg"
			>
				Get weekly watchlists, structured courses, and join a community of traders who are serious
				about results.
			</p>

			<div class="reveal-item">
				<a
					href="/#pricing"
					class="bg-teal shadow-teal/25 hover:bg-teal-light hover:shadow-teal/30 inline-flex items-center justify-center gap-2 rounded-xl px-8 py-4 text-sm font-semibold text-white shadow-lg transition-all duration-300 hover:-translate-y-px hover:shadow-xl active:scale-[0.97]"
				>
					Get Instant Access
					<ChartLineUp size={18} weight="bold" />
				</a>
			</div>
		</ScrollReveal>
	</div>
</section>
