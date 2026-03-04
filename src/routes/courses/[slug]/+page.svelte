<script lang="ts">
	import { page } from '$app/stores';
	import { onMount } from 'svelte';
	import { gsap } from 'gsap';
	import { courses } from '$lib/data/courses';
	import { error } from '@sveltejs/kit';
	import Button from '$lib/components/ui/Button.svelte';
	import ScrollReveal from '$lib/components/ui/ScrollReveal.svelte';
	import CheckCircle from 'phosphor-svelte/lib/CheckCircle';
	import ArrowRight from 'phosphor-svelte/lib/ArrowRight';
	import ArrowLeft from 'phosphor-svelte/lib/ArrowLeft';
	import Clock from 'phosphor-svelte/lib/Clock';
	import GraduationCap from 'phosphor-svelte/lib/GraduationCap';
	import PlayCircle from 'phosphor-svelte/lib/PlayCircle';
	import BookOpen from 'phosphor-svelte/lib/BookOpen';
	import Pulse from 'phosphor-svelte/lib/Pulse';

	const iconMap: Record<string, typeof BookOpen> = { BookOpen, Pulse };

	const slug = $page.params.slug;
	const course = courses.find((c) => c.slug === slug);

	if (!course) {
		error(404, 'Course not found');
	}

	const Icon = iconMap[course.icon];

	let heroRef: HTMLElement | undefined = $state();

	onMount(() => {
		if (!heroRef) return;

		const els = [
			'.cd-back',
			'.cd-badge',
			'.cd-title',
			'.cd-desc',
			'.cd-meta',
			'.cd-price',
			'.cd-cta',
			'.cd-icon-box'
		];
		gsap.set(els, { opacity: 0, y: 20, willChange: 'transform, opacity', force3D: true });

		const ctx = gsap.context(() => {
			const tl = gsap.timeline({
				defaults: { ease: 'expo.out', duration: 1.4, force3D: true },
				delay: 0.15
			});
			tl.to('.cd-back', { opacity: 1, y: 0, duration: 0.8 })
				.to('.cd-badge', { opacity: 1, y: 0, duration: 1.0 }, '-=0.6')
				.to('.cd-title', { opacity: 1, y: 0 }, '-=0.9')
				.to('.cd-desc', { opacity: 1, y: 0 }, '-=1.0')
				.to('.cd-meta', { opacity: 1, y: 0 }, '-=1.1')
				.to('.cd-price', { opacity: 1, y: 0 }, '-=1.1')
				.to('.cd-cta', { opacity: 1, y: 0 }, '-=1.1')
				.to('.cd-icon-box', { opacity: 1, y: 0, scale: 1, duration: 1.6 }, '-=1.4')
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
	<title>{course.title} — Explosive Swings</title>
	<meta name="description" content={course.description} />
</svelte:head>

<!-- Hero Section -->
<section
	bind:this={heroRef}
	class="relative overflow-hidden pt-16"
	style="background: linear-gradient(145deg, {course.gradient.from} 0%, {course.gradient.to} 100%);"
>
	<!-- Grid Pattern -->
	<div
		class="absolute inset-0 opacity-[0.05]"
		style="background-image: linear-gradient(to right, white 1px, transparent 1px), linear-gradient(to bottom, white 1px, transparent 1px); background-size: 60px 60px;"
	></div>

	<div class="relative z-10 mx-auto max-w-[1200px] px-4 py-12 sm:px-6 sm:py-16 lg:px-8 lg:py-24">
		<!-- Back Link -->
		<a
			href="/courses"
			class="cd-back mb-8 inline-flex items-center gap-2 rounded-lg px-3 py-2 text-sm font-medium text-white/70 transition-all duration-200 hover:bg-white/10 hover:text-white"
		>
			<ArrowLeft size={16} weight="bold" />
			All Courses
		</a>

		<div class="grid items-center gap-10 lg:grid-cols-[1fr_auto] lg:gap-16">
			<!-- Left Column -->
			<div>
				<div
					class="cd-badge mb-5 inline-flex items-center gap-2 rounded-full border border-white/25 bg-white/15 px-4 py-1.5 backdrop-blur-sm"
				>
					<GraduationCap size={16} weight="bold" color="white" />
					<span class="text-[11px] font-semibold tracking-wide text-white uppercase"
						>{course.level}</span
					>
				</div>

				<h1
					class="cd-title font-heading mb-5 text-3xl leading-tight font-bold text-white sm:text-4xl md:text-5xl lg:text-[3.5rem]"
				>
					{course.title}
				</h1>

				<p class="cd-desc mb-8 max-w-xl text-base leading-relaxed text-white/85 sm:text-lg">
					{course.description}
				</p>

				<div class="cd-meta mb-8 flex flex-wrap items-center gap-5 text-sm text-white/70">
					<div class="flex items-center gap-2">
						<Clock size={18} weight="bold" />
						<span>{course.duration}</span>
					</div>
					<div class="flex items-center gap-2">
						<PlayCircle size={18} weight="bold" />
						<span>{course.modules} modules</span>
					</div>
					<div class="flex items-center gap-2">
						{#if Icon}
							<Icon size={18} weight="bold" />
						{/if}
						<span>Self-paced</span>
					</div>
				</div>

				<div class="cd-price mb-8 flex items-baseline gap-2">
					<span class="font-heading text-4xl font-bold text-white sm:text-5xl">${course.price}</span
					>
					<span class="text-base text-white/60">one-time</span>
				</div>

				<div class="cd-cta">
					<Button variant="primary" href="#enroll">
						Enroll Now
						<ArrowRight size={18} weight="bold" />
					</Button>
				</div>
			</div>

			<!-- Right Column - Icon -->
			<div class="cd-icon-box hidden lg:flex">
				<div
					class="flex h-56 w-56 items-center justify-center rounded-3xl border border-white/15 bg-white/8 backdrop-blur-sm xl:h-64 xl:w-64"
				>
					{#if Icon}
						<Icon size={100} weight="duotone" color="white" />
					{:else}
						<BookOpen size={100} weight="duotone" color="white" />
					{/if}
				</div>
			</div>
		</div>
	</div>
</section>

<!-- What You'll Learn -->
<section class="bg-white py-16 sm:py-20 lg:py-28">
	<div class="mx-auto max-w-[1200px] px-4 sm:px-6 lg:px-8">
		<ScrollReveal>
			<h2
				class="text-navy font-heading mb-10 text-center text-2xl font-bold sm:text-3xl md:text-4xl"
			>
				What You'll Learn
			</h2>

			<div class="mx-auto grid max-w-4xl gap-4 sm:grid-cols-2 sm:gap-5">
				{#each course.whatYouLearn as item, i}
					<div
						class="reveal-item border-grey-200/80 bg-off-white flex gap-4 rounded-xl border p-5 sm:p-6"
						style="transition-delay: {i * 0.06}s"
					>
						<CheckCircle size={22} weight="fill" color="#0FA4AF" class="mt-0.5 shrink-0" />
						<p class="text-grey-800 text-sm leading-relaxed sm:text-base">{item}</p>
					</div>
				{/each}
			</div>
		</ScrollReveal>
	</div>
</section>

<!-- Curriculum -->
<section class="bg-off-white py-16 sm:py-20 lg:py-28">
	<div class="mx-auto max-w-[1200px] px-4 sm:px-6 lg:px-8">
		<ScrollReveal>
			<h2
				class="text-navy font-heading mb-10 text-center text-2xl font-bold sm:text-3xl md:text-4xl"
			>
				Course Curriculum
			</h2>

			<div class="mx-auto max-w-3xl space-y-4 sm:space-y-5">
				{#each course.curriculum as module, i}
					<div
						class="reveal-item border-grey-200/80 overflow-hidden rounded-xl border bg-white"
						style="transition-delay: {i * 0.06}s"
					>
						<div class="border-grey-100 border-b px-6 py-5 sm:px-8 sm:py-6">
							<h3 class="text-navy font-heading text-base font-bold sm:text-lg">{module.title}</h3>
						</div>
						<ul class="divide-grey-100 divide-y">
							{#each module.lessons as lesson}
								<li
									class="text-grey-700 flex items-center gap-3 px-6 py-3.5 text-sm sm:px-8 sm:text-base"
								>
									<PlayCircle size={18} weight="fill" color="#0FA4AF" class="shrink-0" />
									<span>{lesson}</span>
								</li>
							{/each}
						</ul>
					</div>
				{/each}
			</div>
		</ScrollReveal>
	</div>
</section>

<!-- Features -->
<section class="bg-white py-16 sm:py-20 lg:py-28">
	<div class="mx-auto max-w-[1200px] px-4 sm:px-6 lg:px-8">
		<ScrollReveal>
			<h2
				class="text-navy font-heading mb-10 text-center text-2xl font-bold sm:text-3xl md:text-4xl"
			>
				What's Included
			</h2>

			<div class="mx-auto grid max-w-4xl gap-4 sm:grid-cols-2 sm:gap-5 lg:grid-cols-3">
				{#each course.features as feature, i}
					<div
						class="reveal-item border-grey-200/80 bg-off-white flex gap-3 rounded-xl border p-5"
						style="transition-delay: {i * 0.06}s"
					>
						<CheckCircle size={22} weight="fill" color="#0FA4AF" class="shrink-0" />
						<p class="text-grey-800 text-sm leading-relaxed">{feature}</p>
					</div>
				{/each}
			</div>
		</ScrollReveal>
	</div>
</section>

<!-- Enroll CTA -->
<section
	id="enroll"
	class="from-navy via-navy-mid to-deep-blue bg-linear-to-br py-16 sm:py-20 lg:py-28"
>
	<div class="mx-auto max-w-[1200px] px-4 text-center sm:px-6 lg:px-8">
		<ScrollReveal>
			<h2
				class="reveal-item font-heading mb-5 text-2xl font-bold text-white sm:text-3xl md:text-4xl lg:text-5xl"
			>
				Ready to Start Learning?
			</h2>

			<p
				class="reveal-item text-grey-300 mx-auto mb-10 max-w-2xl text-base leading-relaxed sm:text-lg"
			>
				Get lifetime access to {course.title} for a one-time payment of ${course.price}.
			</p>

			<div class="reveal-item flex flex-col items-center gap-4 sm:flex-row sm:justify-center">
				<Button variant="primary" href="#">
					Enroll Now — ${course.price}
					<ArrowRight size={18} weight="bold" />
				</Button>
				<Button variant="ghost" href="/courses">View All Courses</Button>
			</div>
		</ScrollReveal>
	</div>
</section>
