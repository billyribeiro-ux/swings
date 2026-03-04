<script lang="ts">
	import SectionHeader from '$lib/components/ui/SectionHeader.svelte';
	import ScrollReveal from '$lib/components/ui/ScrollReveal.svelte';
	import BookOpen from 'phosphor-svelte/lib/BookOpen';
	import Pulse from 'phosphor-svelte/lib/Pulse';
	import ArrowRight from 'phosphor-svelte/lib/ArrowRight';
	import Clock from 'phosphor-svelte/lib/Clock';
	import GraduationCap from 'phosphor-svelte/lib/GraduationCap';
	import { courses } from '$lib/data/courses';

	const iconMap: Record<string, typeof BookOpen> = { BookOpen, Pulse };
</script>

<section class="bg-off-white py-16 sm:py-20 lg:py-28">
	<div class="mx-auto max-w-[1200px] px-4 sm:px-6 lg:px-8">
		<ScrollReveal>
			<SectionHeader
				eyebrow="Education"
				title="Level Up Your Options Game"
				subtitle="Structured courses designed to take you from the basics to confidently trading options — at your own pace."
			/>

			<div class="mx-auto grid max-w-[960px] gap-6 sm:gap-8 md:grid-cols-2">
				{#each courses as course, i}
					{@const Icon = iconMap[course.icon]}
					<a
						href="/courses/{course.slug}"
						class="reveal-item group ring-grey-200/80 hover:ring-teal/30 flex flex-col overflow-hidden rounded-2xl bg-white shadow-sm ring-1 transition-all duration-500 ease-out hover:-translate-y-1 hover:shadow-xl"
						style="transition-delay: {i * 0.08}s"
					>
						<!-- Visual Header -->
						<div
							class="relative flex h-40 items-center justify-center overflow-hidden sm:h-44"
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
								class="relative z-10 flex h-16 w-16 items-center justify-center rounded-2xl border border-white/20 bg-white/10 backdrop-blur-sm transition-transform duration-500 ease-out group-hover:scale-105"
							>
								{#if Icon}
									<Icon size={32} weight="duotone" color="white" />
								{:else}
									<BookOpen size={32} weight="duotone" color="white" />
								{/if}
							</div>
						</div>

						<!-- Body -->
						<div class="flex flex-1 flex-col p-6">
							<h3 class="text-navy font-heading mb-2 text-lg font-bold sm:text-xl">
								{course.title}
							</h3>
							<p class="text-grey-600 mb-5 text-sm leading-relaxed">{course.description}</p>

							<div class="mt-auto flex items-center justify-between">
								<div class="text-grey-500 flex items-center gap-3 text-xs">
									<span class="flex items-center gap-1">
										<Clock size={13} weight="bold" class="text-grey-400" />
										{course.duration}
									</span>
									<span class="flex items-center gap-1">
										<GraduationCap size={13} weight="bold" class="text-grey-400" />
										{course.level}
									</span>
								</div>
								<span
									class="text-teal group-hover:text-teal-light inline-flex items-center gap-1 text-sm font-semibold transition-all duration-300 group-hover:translate-x-0.5"
								>
									Learn More
									<ArrowRight size={14} weight="bold" />
								</span>
							</div>
						</div>
					</a>
				{/each}
			</div>
		</ScrollReveal>
	</div>
</section>
