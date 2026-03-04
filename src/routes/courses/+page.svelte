<script lang="ts">
	import { onMount } from 'svelte';
	import { gsap } from 'gsap';
	import { courses } from '$lib/data/courses';
	import SectionHeader from '$lib/components/ui/SectionHeader.svelte';
	import ScrollReveal from '$lib/components/ui/ScrollReveal.svelte';
	import BookOpen from 'phosphor-svelte/lib/BookOpen';
	import Pulse from 'phosphor-svelte/lib/Pulse';
	import ArrowRight from 'phosphor-svelte/lib/ArrowRight';
	import Clock from 'phosphor-svelte/lib/Clock';
	import GraduationCap from 'phosphor-svelte/lib/GraduationCap';
	import CheckCircle from 'phosphor-svelte/lib/CheckCircle';

	const iconMap: Record<string, typeof BookOpen> = { BookOpen, Pulse };

	let heroRef: HTMLElement | undefined = $state();

	onMount(() => {
		if (!heroRef) return;

		const els = ['.courses-badge', '.courses-title', '.courses-subtitle'];

		gsap.set(els, { opacity: 0, y: 24, willChange: 'transform, opacity', force3D: true });

		const ctx = gsap.context(() => {
			const tl = gsap.timeline({
				defaults: { ease: 'expo.out', duration: 1.4, force3D: true },
				delay: 0.2
			});
			tl.to('.courses-badge', { opacity: 1, y: 0, duration: 1.0 })
				.to('.courses-title', { opacity: 1, y: 0 }, '-=0.9')
				.to('.courses-subtitle', { opacity: 1, y: 0 }, '-=1.0')
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
	<title>Options Trading Courses — Explosive Swings</title>
	<meta
		name="description"
		content="Structured courses designed to take you from the basics to confidently trading options — at your own pace."
	/>
</svelte:head>

<!-- Hero Section -->
<section
	bind:this={heroRef}
	class="from-navy via-navy-mid to-deep-blue relative overflow-hidden bg-linear-to-br pt-16"
>
	<!-- Grid Overlay -->
	<div
		class="absolute inset-0 opacity-[0.02]"
		style="background-image: linear-gradient(to right, white 1px, transparent 1px), linear-gradient(to bottom, white 1px, transparent 1px); background-size: 60px 60px;"
	></div>

	<div class="relative z-10 mx-auto max-w-[1200px] px-4 py-20 text-center sm:px-6 lg:px-8 lg:py-28">
		<div
			class="courses-badge border-teal/30 bg-teal/10 mb-6 inline-flex items-center gap-2 rounded-full border px-4 py-2"
		>
			<GraduationCap size={18} weight="duotone" color="#15C5D1" />
			<span class="text-teal-light text-xs font-semibold tracking-wide uppercase">Education</span>
		</div>

		<h1
			class="courses-title font-heading mb-6 text-3xl leading-tight font-bold text-white sm:text-4xl md:text-5xl lg:text-6xl"
		>
			Level Up Your Options Game
		</h1>

		<p
			class="courses-subtitle text-grey-300 mx-auto max-w-2xl text-base leading-relaxed sm:text-lg lg:text-xl"
		>
			Structured courses designed to take you from the basics to confidently trading options — at
			your own pace.
		</p>
	</div>
</section>

<!-- Courses Grid -->
<section class="bg-off-white py-16 sm:py-20 lg:py-28">
	<div class="mx-auto max-w-[960px] px-4 sm:px-6 lg:px-8">
		<ScrollReveal>
			<div class="grid gap-6 sm:gap-8 md:grid-cols-2">
				{#each courses as course, i}
					{@const Icon = iconMap[course.icon]}
					<a
						href="/courses/{course.slug}"
						class="reveal-item group ring-grey-200/80 hover:ring-teal/30 relative flex flex-col overflow-hidden rounded-2xl bg-white shadow-sm ring-1 transition-all duration-500 ease-out hover:-translate-y-1 hover:shadow-xl"
						style="transition-delay: {i * 0.08}s"
					>
						<!-- Visual Header -->
						<div
							class="relative flex h-44 items-center justify-center overflow-hidden sm:h-52"
							style="background: linear-gradient(145deg, {course.gradient.from} 0%, {course.gradient
								.to} 100%);"
						>
							<!-- Grid Pattern -->
							<div
								class="absolute inset-0 opacity-[0.06]"
								style="background-image: linear-gradient(to right, white 1px, transparent 1px), linear-gradient(to bottom, white 1px, transparent 1px); background-size: 32px 32px;"
							></div>

							<!-- Level Badge -->
							<div class="absolute top-4 left-4">
								<span
									class="rounded-full border border-white/25 bg-white/15 px-3 py-1 text-[11px] font-semibold text-white backdrop-blur-sm"
								>
									{course.level}
								</span>
							</div>

							<!-- Icon -->
							<div
								class="relative z-10 flex h-20 w-20 items-center justify-center rounded-2xl border border-white/20 bg-white/10 backdrop-blur-sm transition-transform duration-500 ease-out group-hover:scale-105"
							>
								{#if Icon}
									<Icon size={40} weight="duotone" color="white" />
								{:else}
									<BookOpen size={40} weight="duotone" color="white" />
								{/if}
							</div>
						</div>

						<!-- Body -->
						<div class="flex flex-1 flex-col p-6 sm:p-8">
							<h3 class="text-navy font-heading mb-2 text-xl leading-snug font-bold sm:text-2xl">
								{course.title}
							</h3>
							<p class="text-grey-600 mb-6 text-sm leading-relaxed sm:text-base">
								{course.description}
							</p>

							<!-- Meta Row -->
							<div class="text-grey-500 mb-6 flex flex-wrap items-center gap-4 text-[13px]">
								<div class="flex items-center gap-1.5">
									<Clock size={15} weight="bold" class="text-grey-400" />
									<span>{course.duration}</span>
								</div>
								<div class="flex items-center gap-1.5">
									<GraduationCap size={15} weight="bold" class="text-grey-400" />
									<span>{course.modules} modules</span>
								</div>
							</div>

							<!-- Price + CTA row -->
							<div class="border-grey-100 mt-auto flex items-center justify-between border-t pt-6">
								<div class="flex items-baseline gap-1.5">
									<span class="text-navy font-heading text-3xl font-bold">${course.price}</span>
									<span class="text-grey-500 text-sm">one-time</span>
								</div>
								<span
									class="bg-teal/10 text-teal group-hover:bg-teal inline-flex items-center gap-1.5 rounded-lg px-4 py-2 text-sm font-semibold transition-all duration-300 group-hover:text-white"
								>
									Learn More
									<ArrowRight
										size={16}
										weight="bold"
										class="transition-transform duration-300 group-hover:translate-x-0.5"
									/>
								</span>
							</div>
						</div>
					</a>
				{/each}
			</div>
		</ScrollReveal>
	</div>
</section>

<!-- Why Take a Course -->
<section class="bg-white py-16 sm:py-20 lg:py-28">
	<div class="mx-auto max-w-[1200px] px-4 sm:px-6 lg:px-8">
		<ScrollReveal>
			<SectionHeader
				eyebrow="Why Learn Options?"
				title="Build Skills That Last a Lifetime"
				subtitle="Options trading gives you leverage, flexibility, and the ability to profit in any market condition — but only if you know what you're doing."
			/>

			<div class="mx-auto grid max-w-4xl gap-6 sm:grid-cols-3 sm:gap-8">
				<div class="reveal-item text-center">
					<div
						class="bg-teal/10 mx-auto mb-5 flex h-14 w-14 items-center justify-center rounded-2xl"
					>
						<BookOpen size={28} weight="duotone" color="#0FA4AF" />
					</div>
					<h3 class="text-navy font-heading mb-2 text-base font-bold sm:text-lg">
						Learn at Your Pace
					</h3>
					<p class="text-grey-600 text-sm leading-relaxed">
						Lifetime access means you can revisit lessons whenever you need a refresher.
					</p>
				</div>

				<div class="reveal-item text-center">
					<div
						class="bg-teal/10 mx-auto mb-5 flex h-14 w-14 items-center justify-center rounded-2xl"
					>
						<Pulse size={28} weight="duotone" color="#0FA4AF" />
					</div>
					<h3 class="text-navy font-heading mb-2 text-base font-bold sm:text-lg">
						Real-World Examples
					</h3>
					<p class="text-grey-600 text-sm leading-relaxed">
						Every lesson includes actual chart examples and trade setups you can apply immediately.
					</p>
				</div>

				<div class="reveal-item text-center">
					<div
						class="bg-teal/10 mx-auto mb-5 flex h-14 w-14 items-center justify-center rounded-2xl"
					>
						<GraduationCap size={28} weight="duotone" color="#0FA4AF" />
					</div>
					<h3 class="text-navy font-heading mb-2 text-base font-bold sm:text-lg">
						Community Support
					</h3>
					<p class="text-grey-600 text-sm leading-relaxed">
						Join a private community of traders learning the same strategies.
					</p>
				</div>
			</div>
		</ScrollReveal>
	</div>
</section>
