<script lang="ts">
	import { page } from '$app/stores';
	import { onMount } from 'svelte';
	import { gsap } from 'gsap';
	import CheckCircle from 'phosphor-svelte/lib/CheckCircle';
	import Envelope from 'phosphor-svelte/lib/Envelope';
	import BrowsersIcon from 'phosphor-svelte/lib/Browsers';
	import CalendarCheck from 'phosphor-svelte/lib/CalendarCheck';
	import ArrowRight from 'phosphor-svelte/lib/ArrowRight';
	import Button from '$lib/components/ui/Button.svelte';

	let sessionId = $state('');
	let containerRef: HTMLElement | undefined = $state();

	onMount(() => {
		sessionId = $page.url.searchParams.get('session_id') || '';

		if (!containerRef) return;

		const els = [
			'.success-icon',
			'.success-title',
			'.success-subtitle',
			'.success-steps',
			'.success-cta'
		];
		gsap.set(els, { opacity: 0, y: 24, willChange: 'transform, opacity', force3D: true });

		const ctx = gsap.context(() => {
			const tl = gsap.timeline({
				defaults: { ease: 'expo.out', duration: 1.4, force3D: true },
				delay: 0.2
			});
			tl.to('.success-icon', { opacity: 1, y: 0, scale: 1, duration: 1.0 })
				.to('.success-title', { opacity: 1, y: 0 }, '-=0.8')
				.to('.success-subtitle', { opacity: 1, y: 0 }, '-=1.0')
				.to('.success-steps', { opacity: 1, y: 0 }, '-=1.0')
				.to('.success-cta', { opacity: 1, y: 0 }, '-=1.0')
				.call(() => {
					gsap.set(els, { willChange: 'auto' });
				});
		}, containerRef as HTMLElement);

		return () => {
			ctx.revert();
			gsap.set(els, { clearProps: 'all' });
		};
	});

	const steps = [
		{
			icon: Envelope,
			title: 'Check Your Email',
			desc: "You'll receive login credentials and onboarding instructions within the next few minutes."
		},
		{
			icon: BrowsersIcon,
			title: 'Access Your Dashboard',
			desc: 'Log in to view your member area, past watchlists, and educational resources.'
		},
		{
			icon: CalendarCheck,
			title: 'Get Your First Watchlist',
			desc: 'Every Sunday night at 8 PM ET, your weekly watchlist will be delivered via email and SMS.'
		}
	];
</script>

<svelte:head>
	<title>Welcome to Explosive Swings!</title>
	<meta name="robots" content="noindex" />
</svelte:head>

<div
	bind:this={containerRef}
	class="from-navy via-navy-mid to-deep-blue flex min-h-screen items-center justify-center bg-linear-to-br px-4 py-24 pt-28"
>
	<div class="mx-auto w-full max-w-2xl text-center">
		<!-- Success Icon -->
		<div class="success-icon mb-8 flex justify-center">
			<div
				class="bg-green/15 ring-green/10 flex h-20 w-20 items-center justify-center rounded-full ring-4 sm:h-24 sm:w-24"
			>
				<CheckCircle size={48} weight="fill" color="#22B573" />
			</div>
		</div>

		<!-- Title -->
		<h1
			class="success-title font-heading mb-4 text-3xl font-bold text-white sm:text-4xl md:text-5xl"
		>
			Welcome to Explosive Swings!
		</h1>

		<!-- Subtitle -->
		<p class="success-subtitle text-grey-300 mb-10 text-base leading-relaxed sm:text-lg">
			Your subscription is confirmed. Check your email for access details and your first Sunday
			night watchlist.
		</p>

		<!-- Steps -->
		<div
			class="success-steps mb-10 overflow-hidden rounded-2xl border border-white/10 bg-white/4 text-left backdrop-blur-sm"
		>
			<div class="border-b border-white/8 px-6 py-5 sm:px-8">
				<h2 class="font-heading text-lg font-bold text-white sm:text-xl">What Happens Next?</h2>
			</div>

			<div class="divide-y divide-white/8">
				{#each steps as step, i}
					<div class="flex gap-4 px-6 py-5 sm:px-8 sm:py-6">
						<div class="bg-teal/15 flex h-10 w-10 shrink-0 items-center justify-center rounded-xl">
							<step.icon size={20} weight="duotone" color="#15C5D1" />
						</div>
						<div>
							<h3 class="mb-1 text-sm font-semibold text-white sm:text-base">{step.title}</h3>
							<p class="text-grey-400 text-xs leading-relaxed sm:text-sm">{step.desc}</p>
						</div>
					</div>
				{/each}
			</div>
		</div>

		<!-- CTA -->
		<div class="success-cta flex flex-col items-center gap-4 sm:flex-row sm:justify-center">
			<Button variant="primary" href="/">
				Back to Home
				<ArrowRight size={18} weight="bold" />
			</Button>
		</div>

		{#if sessionId}
			<p class="text-grey-600 mt-10 text-xs">
				Session: {sessionId}
			</p>
		{/if}
	</div>
</div>
