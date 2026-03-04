<script lang="ts">
	import { onMount } from 'svelte';
	import { gsap } from 'gsap';
	import { ScrollTrigger } from 'gsap/ScrollTrigger';
	import DownloadSimple from 'phosphor-svelte/lib/DownloadSimple';
	import EnvelopeSimple from 'phosphor-svelte/lib/EnvelopeSimple';
	import CheckCircle from 'phosphor-svelte/lib/CheckCircle';
	import FilePdf from 'phosphor-svelte/lib/FilePdf';

	let email = $state('');
	let isSubmitting = $state(false);
	let isSuccess = $state(false);
	let error = $state('');
	let containerRef: HTMLElement | undefined = $state();

	onMount(() => {
		if (!containerRef) return;

		gsap.registerPlugin(ScrollTrigger);

		const els = containerRef.querySelectorAll(
			'.greeks-icon, .greeks-title, .greeks-desc, .greeks-form'
		);
		gsap.set(els, { opacity: 0, y: 24, willChange: 'transform, opacity' });

		const ctx = gsap.context(() => {
			gsap.to(els, {
				opacity: 1,
				y: 0,
				duration: 0.8,
				stagger: 0.1,
				ease: 'power3.out',
				scrollTrigger: {
					trigger: containerRef,
					start: 'top 80%',
					once: true
				},
				onComplete: () => {
					gsap.set(els, { willChange: 'auto', clearProps: 'transform' });
				}
			});
		}, containerRef as HTMLElement);

		return () => {
			ctx.revert();
			gsap.set(els, { clearProps: 'all' });
		};
	});

	async function handleSubmit(e: Event) {
		e.preventDefault();
		error = '';

		if (!email || !email.includes('@')) {
			error = 'Please enter a valid email address';
			return;
		}

		isSubmitting = true;

		try {
			const response = await fetch('/api/greeks-pdf', {
				method: 'POST',
				headers: { 'Content-Type': 'application/json' },
				body: JSON.stringify({ email })
			});

			if (!response.ok) {
				const data = await response.json();
				throw new Error(data.error || 'Failed to send PDF');
			}

			isSuccess = true;
			email = '';
		} catch (err) {
			error = err instanceof Error ? err.message : 'Something went wrong. Please try again.';
		} finally {
			isSubmitting = false;
		}
	}
</script>

<section
	bind:this={containerRef}
	class="from-navy via-navy-mid to-deep-blue bg-linear-to-br py-16 sm:py-20 lg:py-28"
>
	<div class="mx-auto max-w-[1200px] px-4 sm:px-6 lg:px-8">
		<div
			class="mx-auto max-w-3xl overflow-hidden rounded-3xl border border-white/10 bg-white/5 backdrop-blur-sm"
		>
			<div class="grid gap-8 p-8 sm:p-10 lg:grid-cols-[auto_1fr] lg:gap-12 lg:p-12">
				<!-- Icon -->
				<div class="greeks-icon flex justify-center lg:justify-start">
					<div
						class="bg-teal/15 ring-teal/10 flex h-20 w-20 items-center justify-center rounded-2xl ring-4"
					>
						<FilePdf size={40} weight="duotone" color="#15C5D1" />
					</div>
				</div>

				<!-- Content -->
				<div>
					<h2 class="greeks-title font-heading mb-3 text-2xl font-bold text-white sm:text-3xl">
						Free Guide: The Ultimate Guide to Options Greeks
					</h2>

					<p class="greeks-desc text-grey-300 mb-6 text-base leading-relaxed">
						Master Delta, Gamma, Theta, and Vega with our comprehensive 20-page PDF guide. Learn how
						to use Greeks to make smarter trading decisions and manage risk like a pro.
					</p>

					{#if isSuccess}
						<div class="border-green/30 bg-green/10 rounded-xl border p-4">
							<div class="flex items-start gap-3">
								<CheckCircle size={24} weight="fill" color="#22B573" class="shrink-0" />
								<div>
									<h3 class="mb-1 text-sm font-semibold text-white">Check Your Inbox!</h3>
									<p class="text-grey-300 text-xs">
										We've sent the Greeks PDF guide to your email. Check your spam folder if you
										don't see it within a few minutes.
									</p>
								</div>
							</div>
						</div>
					{:else}
						<form onsubmit={handleSubmit} class="greeks-form">
							<div class="flex flex-col gap-3 sm:flex-row">
								<div class="relative flex-1">
									<EnvelopeSimple
										size={18}
										weight="bold"
										class="text-grey-400 absolute top-1/2 left-4 -translate-y-1/2"
									/>
									<input
										type="email"
										bind:value={email}
										placeholder="Enter your email"
										required
										disabled={isSubmitting}
										class="placeholder-grey-400 focus:border-teal/50 focus:ring-teal/30 w-full rounded-xl border border-white/20 bg-white/10 py-3.5 pr-4 pl-12 text-sm text-white backdrop-blur-sm transition-all duration-200 focus:bg-white/15 focus:ring-2 focus:outline-none disabled:opacity-50"
									/>
								</div>
								<button
									type="submit"
									disabled={isSubmitting}
									class="bg-teal shadow-teal/25 hover:bg-teal-light hover:shadow-teal/30 inline-flex items-center justify-center gap-2 rounded-xl px-6 py-3.5 text-sm font-semibold text-white shadow-lg transition-all duration-300 hover:-translate-y-px hover:shadow-xl active:scale-[0.97] disabled:pointer-events-none disabled:opacity-50 sm:w-auto"
								>
									{#if isSubmitting}
										Sending...
									{:else}
										<DownloadSimple size={18} weight="bold" />
										Get Free PDF
									{/if}
								</button>
							</div>

							{#if error}
								<p class="text-red mt-3 text-xs">{error}</p>
							{/if}

							<p class="text-grey-400 mt-3 text-xs">
								No spam, ever. Unsubscribe anytime. We respect your privacy.
							</p>
						</form>
					{/if}
				</div>
			</div>
		</div>
	</div>
</section>
