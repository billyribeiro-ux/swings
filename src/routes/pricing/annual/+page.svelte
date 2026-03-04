<script lang="ts">
	import { onMount } from 'svelte';
	import { gsap } from 'gsap';
	import { env } from '$env/dynamic/public';
	import { createCheckoutSession } from '$lib/utils/stripe';
	import Button from '$lib/components/ui/Button.svelte';
	import ScrollReveal from '$lib/components/ui/ScrollReveal.svelte';
	import CheckCircle from 'phosphor-svelte/lib/CheckCircle';
	import ArrowRight from 'phosphor-svelte/lib/ArrowRight';
	import CurrencyDollar from 'phosphor-svelte/lib/CurrencyDollar';
	import Sparkle from 'phosphor-svelte/lib/Sparkle';

	let heroRef: HTMLElement | undefined = $state();
	let isLoading = $state(false);

	const annualPriceId = env.PUBLIC_STRIPE_ANNUAL_PRICE_ID || '';

	onMount(() => {
		if (!heroRef) return;

		const els = ['.price-badge', '.price-title', '.price-amount', '.price-features', '.price-cta'];
		gsap.set(els, { opacity: 0, y: 24, willChange: 'transform, opacity' });

		const ctx = gsap.context(() => {
			const tl = gsap.timeline({ delay: 0.15 });
			tl.to('.price-badge', { opacity: 1, y: 0, duration: 0.6, ease: 'power3.out' })
				.to('.price-title', { opacity: 1, y: 0, duration: 0.7, ease: 'power3.out' }, '-=0.35')
				.to('.price-amount', { opacity: 1, y: 0, duration: 0.7, ease: 'power3.out' }, '-=0.4')
				.to('.price-features', { opacity: 1, y: 0, duration: 0.7, ease: 'power3.out' }, '-=0.4')
				.to('.price-cta', { opacity: 1, y: 0, duration: 0.6, ease: 'power3.out' }, '-=0.35')
				.call(() => {
					gsap.set(els, { willChange: 'auto', clearProps: 'transform' });
				});
		}, heroRef as HTMLElement);

		return () => {
			ctx.revert();
			gsap.set(els, { clearProps: 'all' });
		};
	});

	async function handleCheckout() {
		isLoading = true;
		try {
			await createCheckoutSession(annualPriceId);
		} catch (error) {
			console.error('Checkout failed:', error);
			alert('Failed to start checkout. Please try again.');
			isLoading = false;
		}
	}

	const features = [
		'Weekly Sunday night watchlist (5–7 setups)',
		'Entry zones, profit targets, and stop losses',
		'SMS & email alerts for every trade',
		'Access to private Discord community',
		'Exclusive educational content & webinars',
		'Priority support & early access to new features',
		'Save $232/year vs monthly plan'
	];
</script>

<svelte:head>
	<title>Annual Plan — Explosive Swings</title>
	<meta
		name="description"
		content="Get weekly options watchlists for $932/year. Save 20% vs monthly."
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

	<div class="relative z-10 mx-auto max-w-[1200px] px-4 py-20 sm:px-6 lg:px-8 lg:py-32">
		<div class="mx-auto max-w-3xl text-center">
			<div
				class="price-badge border-gold/30 bg-gold/10 mb-6 inline-flex items-center gap-2 rounded-full border px-4 py-2"
			>
				<Sparkle size={18} weight="duotone" color="#D4A843" />
				<span class="text-gold-light text-xs font-semibold tracking-wide uppercase">Best Value</span
				>
			</div>

			<h1
				class="price-title font-heading mb-6 text-3xl leading-tight font-bold text-white sm:text-4xl md:text-5xl lg:text-6xl"
			>
				Annual Plan — Save 20%
			</h1>

			<div class="price-amount mb-8">
				<div class="flex items-baseline justify-center gap-2">
					<span class="font-heading text-5xl font-bold text-white sm:text-6xl">$932</span>
					<span class="text-grey-300 text-xl">/year</span>
				</div>
				<div class="bg-green/10 mt-3 inline-flex items-center gap-2 rounded-lg px-4 py-2">
					<CheckCircle size={16} weight="fill" color="#22B573" />
					<span class="text-green text-sm font-semibold">Save $232 vs monthly</span>
				</div>
			</div>
		</div>
	</div>
</section>

<!-- Features -->
<section class="bg-white py-16 sm:py-20 lg:py-28">
	<div class="mx-auto max-w-[1200px] px-4 sm:px-6 lg:px-8">
		<ScrollReveal>
			<div class="price-features mx-auto max-w-2xl">
				<h2 class="text-navy font-heading mb-10 text-center text-2xl font-bold sm:text-3xl">
					Everything Included
				</h2>

				<div class="space-y-4">
					{#each features as feature, i}
						<div
							class="reveal-item border-grey-200/80 bg-off-white flex items-start gap-4 rounded-xl border p-5"
							style="transition-delay: {i * 0.06}s"
						>
							<CheckCircle size={24} weight="fill" color="#0FA4AF" class="mt-0.5 shrink-0" />
							<p class="text-grey-800 leading-relaxed">{feature}</p>
						</div>
					{/each}
				</div>
			</div>
		</ScrollReveal>
	</div>
</section>

<!-- Comparison -->
<section class="bg-off-white py-16 sm:py-20 lg:py-28">
	<div class="mx-auto max-w-[1200px] px-4 sm:px-6 lg:px-8">
		<ScrollReveal>
			<div
				class="reveal-item border-grey-200 mx-auto max-w-2xl overflow-hidden rounded-2xl border bg-white"
			>
				<div class="from-teal to-teal-light bg-linear-to-r p-6 text-center">
					<h3 class="font-heading text-xl font-bold text-white">Why Annual is Better</h3>
				</div>
				<div class="divide-grey-100 divide-y p-6">
					<div class="flex items-center justify-between py-4">
						<span class="text-grey-700">Monthly Plan (12 months)</span>
						<span class="text-grey-800 text-lg font-bold">$1,164</span>
					</div>
					<div class="flex items-center justify-between py-4">
						<span class="text-grey-700">Annual Plan</span>
						<span class="text-teal text-lg font-bold">$932</span>
					</div>
					<div class="flex items-center justify-between py-4">
						<span class="text-green font-semibold">You Save</span>
						<span class="text-green text-2xl font-bold">$232</span>
					</div>
				</div>
			</div>
		</ScrollReveal>
	</div>
</section>

<!-- CTA -->
<section class="bg-white py-16 sm:py-20 lg:py-28">
	<div class="mx-auto max-w-[1200px] px-4 text-center sm:px-6 lg:px-8">
		<ScrollReveal>
			<div class="price-cta mx-auto max-w-2xl">
				<h2
					class="reveal-item text-navy font-heading mb-5 text-2xl font-bold sm:text-3xl md:text-4xl"
				>
					Lock In Your Savings Today
				</h2>

				<p class="reveal-item text-grey-700 mb-10 leading-relaxed">
					Get 12 months of weekly watchlists and save over $230 compared to paying monthly.
				</p>

				<div class="reveal-item flex flex-col items-center gap-4 sm:flex-row sm:justify-center">
					<button
						onclick={handleCheckout}
						disabled={isLoading}
						class="bg-teal shadow-teal/25 hover:bg-teal-light hover:shadow-teal/30 inline-flex items-center justify-center gap-2 rounded-xl px-8 py-4 text-sm font-semibold text-white shadow-lg transition-all duration-300 hover:-translate-y-px hover:shadow-xl active:scale-[0.97] disabled:pointer-events-none disabled:opacity-50"
					>
						{#if isLoading}
							Processing...
						{:else}
							Start Annual Plan — $932/year
							<ArrowRight size={18} weight="bold" />
						{/if}
					</button>

					<a
						href="/pricing/monthly"
						class="text-teal hover:text-teal-light inline-flex items-center gap-2 text-sm font-semibold transition-colors duration-200"
					>
						View Monthly Plan
						<ArrowRight size={14} weight="bold" />
					</a>
				</div>

				<p class="reveal-item text-grey-500 mt-8 text-xs">
					Billed annually. All plans include a 30-day money-back guarantee.
				</p>
			</div>
		</ScrollReveal>
	</div>
</section>
