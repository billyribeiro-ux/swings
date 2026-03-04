<script lang="ts">
	import { onMount } from 'svelte';
	import { sampleAlert } from '$lib/data/alerts';
	import { gsap } from 'gsap';

	interface Props {
		delay?: number;
	}

	let { delay = 0.6 }: Props = $props();

	let cardRef: HTMLElement | undefined = $state();

	onMount(() => {
		if (!cardRef) return;
		const element = cardRef;

		gsap.set(element, {
			opacity: 0,
			x: 30,
			willChange: 'transform, opacity'
		});

		const ctx = gsap.context(() => {
			gsap.to(element, {
				opacity: 1,
				x: 0,
				duration: 0.9,
				delay,
				ease: 'power3.out',
				onComplete() {
					gsap.set(element, { willChange: 'auto', clearProps: 'transform' });
				}
			});
		}, element);

		return () => {
			ctx.revert();
			gsap.set(element, { clearProps: 'all' });
		};
	});
</script>

<div
	bind:this={cardRef}
	class="max-w-sm rounded-2xl border border-white/10 bg-white/5 p-6 backdrop-blur-sm"
>
	<!-- Header -->
	<div class="mb-4 flex items-center justify-between">
		<span class="text-grey-400 text-xs font-semibold tracking-wider uppercase">Sample Alert</span>
		<div class="flex items-center gap-2">
			<span class="bg-green h-2 w-2 animate-pulse rounded-full"></span>
			<span class="text-green text-xs font-medium">Live Format</span>
		</div>
	</div>

	<!-- Ticker -->
	<div class="mb-4">
		<span
			class="font-ui text-3xl font-bold tracking-tight text-white"
			style="font-variant-numeric: tabular-nums;"
		>
			{sampleAlert.ticker}
		</span>
	</div>

	<!-- Data Rows -->
	<div class="space-y-3">
		<div class="flex items-center justify-between">
			<span class="text-grey-400 text-sm">Entry Zone</span>
			<span
				class="text-teal-light font-ui text-sm font-semibold"
				style="font-variant-numeric: tabular-nums;"
			>
				{sampleAlert.entryZone}
			</span>
		</div>
		<div class="flex items-center justify-between">
			<span class="text-grey-400 text-sm">Invalidation</span>
			<span
				class="text-red font-ui text-sm font-semibold"
				style="font-variant-numeric: tabular-nums;"
			>
				{sampleAlert.invalidation}
			</span>
		</div>
		<div class="flex items-center justify-between">
			<span class="text-grey-400 text-sm">Profit Zone 1</span>
			<span
				class="text-green font-ui text-sm font-semibold"
				style="font-variant-numeric: tabular-nums;"
			>
				{sampleAlert.profitZones[0]}
			</span>
		</div>
		<div class="flex items-center justify-between">
			<span class="text-grey-400 text-sm">Profit Zone 2</span>
			<span
				class="text-green font-ui text-sm font-semibold"
				style="font-variant-numeric: tabular-nums;"
			>
				{sampleAlert.profitZones[1]}
			</span>
		</div>
		<div class="flex items-center justify-between">
			<span class="text-grey-400 text-sm">Profit Zone 3</span>
			<span
				class="text-green font-ui text-sm font-semibold"
				style="font-variant-numeric: tabular-nums;"
			>
				{sampleAlert.profitZones[2]}
			</span>
		</div>
	</div>

	<!-- Notes -->
	<div class="bg-teal/10 border-teal mt-4 rounded-r-lg border-l-2 p-3">
		<p class="text-grey-200 text-xs leading-relaxed">{sampleAlert.notes}</p>
	</div>
</div>
