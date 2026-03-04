<script lang="ts">
	import { courses } from '$lib/data/courses';
	import EnvelopeSimple from 'phosphor-svelte/lib/EnvelopeSimple';
	import TrendUp from 'phosphor-svelte/lib/TrendUp';

	const currentYear = new Date().getFullYear();
</script>

<footer class="bg-navy border-t border-white/10 py-12 sm:py-16">
	<div class="mx-auto max-w-[1200px] px-4 sm:px-6 lg:px-8">
		<!-- Top: Logo + Nav grid -->
		<div class="mb-10 grid gap-10 sm:grid-cols-2 lg:grid-cols-4">
			<!-- Brand -->
			<div class="sm:col-span-2 lg:col-span-1">
				<a
					href="/"
					class="font-heading mb-3 inline-flex items-center gap-0.5 text-lg font-bold tracking-tight"
				>
					<span class="text-white">Explosive</span>
					<span class="text-teal-light">Swings</span>
				</a>
				<p class="text-grey-500 max-w-xs text-xs leading-relaxed">
					Weekly options watchlists and structured trading courses, built by Billy Ribeiro.
				</p>
			</div>

			<!-- Courses -->
			<div>
				<h4 class="text-grey-400 mb-4 text-[11px] font-semibold tracking-widest uppercase">
					Courses
				</h4>
				<ul class="space-y-2.5">
					{#each courses as course}
						<li>
							<a
								href="/courses/{course.slug}"
								class="text-grey-300 hover:text-teal-light text-sm transition-colors duration-200"
							>
								{course.title}
							</a>
						</li>
					{/each}
					<li>
						<a
							href="/courses"
							class="text-grey-300 hover:text-teal-light text-sm transition-colors duration-200"
						>
							View All →
						</a>
					</li>
				</ul>
			</div>

			<!-- Company -->
			<div>
				<h4 class="text-grey-400 mb-4 text-[11px] font-semibold tracking-widest uppercase">
					Company
				</h4>
				<ul class="space-y-2.5">
					<li>
						<a
							href="/#pricing"
							class="text-grey-300 hover:text-teal-light text-sm transition-colors duration-200"
							>Pricing</a
						>
					</li>
					<li>
						<a
							href="/terms"
							class="text-grey-300 hover:text-teal-light text-sm transition-colors duration-200"
							>Terms</a
						>
					</li>
					<li>
						<a
							href="/privacy"
							class="text-grey-300 hover:text-teal-light text-sm transition-colors duration-200"
							>Privacy</a
						>
					</li>
				</ul>
			</div>

			<!-- Contact -->
			<div>
				<h4 class="text-grey-400 mb-4 text-[11px] font-semibold tracking-widest uppercase">
					Contact
				</h4>
				<a
					href="mailto:support@explosiveswings.com"
					class="text-grey-300 hover:text-teal-light inline-flex items-center gap-2 text-sm transition-colors duration-200"
				>
					<EnvelopeSimple size={16} weight="bold" class="text-grey-500" />
					support@explosiveswings.com
				</a>
			</div>
		</div>

		<!-- Bottom: Copyright + Disclaimer -->
		<div class="border-t border-white/8 pt-8">
			<div class="flex flex-col items-center gap-4 sm:flex-row sm:justify-between">
				<p class="text-grey-500 text-xs">
					© {currentYear} Explosive Swings. All rights reserved.
				</p>
				<div class="text-grey-600 flex items-center gap-1.5 text-xs">
					<TrendUp size={14} weight="bold" class="text-grey-500" />
					<span>Trading involves risk. Past performance ≠ future results.</span>
				</div>
			</div>
		</div>
	</div>
</footer>
