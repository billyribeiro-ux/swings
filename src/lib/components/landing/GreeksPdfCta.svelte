<script lang="ts">
	import { gsap } from 'gsap';
	import { createCinematicReveal, EASE, DURATION } from '$lib/utils/animations';
	import DownloadSimpleIcon from 'phosphor-svelte/lib/DownloadSimpleIcon';
	import EnvelopeSimpleIcon from 'phosphor-svelte/lib/EnvelopeSimpleIcon';
	import CheckCircleIcon from 'phosphor-svelte/lib/CheckCircleIcon';
	import FilePdfIcon from 'phosphor-svelte/lib/FilePdfIcon';

	let email = $state('');
	let isSubmitting = $state(false);
	let isSuccess = $state(false);
	let error = $state('');
	let containerRef: HTMLElement | undefined = $state();

	$effect(() => {
		if (!containerRef) return;
		const container = containerRef;

		const els = container.querySelectorAll(
			'.greeks-icon, .greeks-title, .greeks-desc, .greeks-form'
		);

		const ctx = gsap.context(() => {
			createCinematicReveal({
				targets: els,
				trigger: container,
				y: 32,
				blur: 8,
				scale: 0.96,
				duration: DURATION.slow,
				stagger: 0.12,
				ease: EASE.cinematic,
				start: 'top 78%'
			});
		}, container);

		return () => ctx.revert();
	});

	async function handleSubmit(e: SubmitEvent) {
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

<section bind:this={containerRef} class="greeks-section">
	<div class="greeks-section__container">
		<div class="greeks-section__card">
			<div class="greeks-section__layout">
				<!-- Icon -->
				<div class="greeks-icon greeks-section__icon-col">
					<div class="greeks-section__icon-box">
						<FilePdfIcon size={40} weight="duotone" color="#15C5D1" />
					</div>
				</div>

				<!-- Content -->
				<div>
					<h2 class="greeks-title greeks-section__title">
						Free Guide: The Ultimate Guide to Options Greeks
					</h2>

					<p class="greeks-desc greeks-section__desc">
						Master Delta, Gamma, Theta, and Vega with our comprehensive 20-page PDF guide. Learn how
						to use Greeks to make smarter trading decisions and manage risk like a pro.
					</p>

					{#if isSuccess}
						<div class="greeks-section__success">
							<div class="greeks-section__success-inner">
								<CheckCircleIcon
									size={24}
									weight="fill"
									color="#22B573"
									class="greeks-section__success-icon"
								/>
								<div>
									<h3 class="greeks-section__success-title">Check Your Inbox!</h3>
									<p class="greeks-section__success-text">
										We've sent the Greeks PDF guide to your email. Check your spam folder if you
										don't see it within a few minutes.
									</p>
								</div>
							</div>
						</div>
					{:else}
						<form onsubmit={handleSubmit} class="greeks-form">
							<div class="greeks-section__form-row">
								<div class="greeks-section__input-wrap">
									<EnvelopeSimpleIcon size={18} weight="bold" class="greeks-section__input-icon" />
									<input
										id="greeks-email"
										name="email"
										type="email"
										bind:value={email}
										placeholder="Enter your email"
										required
										autocomplete="email"
										disabled={isSubmitting}
										class="greeks-section__input"
									/>
								</div>
								<button type="submit" disabled={isSubmitting} class="greeks-section__submit">
									{#if isSubmitting}
										Sending...
									{:else}
										<DownloadSimpleIcon size={18} weight="bold" />
										Get Free PDF
									{/if}
								</button>
							</div>

							{#if error}
								<p class="greeks-section__error">{error}</p>
							{/if}

							<p class="greeks-section__privacy">
								No spam, ever. Unsubscribe anytime. We respect your privacy.
							</p>
						</form>
					{/if}
				</div>
			</div>
		</div>
	</div>
</section>

<style>
	.greeks-section {
		background: linear-gradient(
			to bottom right,
			var(--color-navy),
			var(--color-navy-mid),
			var(--color-deep-blue)
		);
		padding: 4rem 0;
	}

	@media (min-width: 640px) {
		.greeks-section {
			padding: 5rem 0;
		}
	}
	@media (min-width: 1024px) {
		.greeks-section {
			padding: 7rem 0;
		}
	}

	.greeks-section__container {
		max-width: var(--container-max);
		margin: 0 auto;
		padding: 0 1rem;
	}

	@media (min-width: 640px) {
		.greeks-section__container {
			padding: 0 1.5rem;
		}
	}
	@media (min-width: 1024px) {
		.greeks-section__container {
			padding: 0 2rem;
		}
	}

	.greeks-section__card {
		max-width: 48rem;
		margin: 0 auto;
		overflow: hidden;
		border-radius: var(--radius-3xl);
		border: 1px solid rgba(255, 255, 255, 0.1);
		background-color: rgba(255, 255, 255, 0.05);
		backdrop-filter: blur(4px);
	}

	.greeks-section__layout {
		display: grid;
		gap: 2rem;
		padding: 2rem;
	}

	@media (min-width: 640px) {
		.greeks-section__layout {
			padding: 2.5rem;
		}
	}

	@media (min-width: 1024px) {
		.greeks-section__layout {
			grid-template-columns: auto 1fr;
			gap: 3rem;
			padding: 3rem;
		}
	}

	.greeks-section__icon-col {
		display: flex;
		justify-content: center;
	}

	@media (min-width: 1024px) {
		.greeks-section__icon-col {
			justify-content: flex-start;
		}
	}

	.greeks-section__icon-box {
		display: flex;
		align-items: center;
		justify-content: center;
		width: 5rem;
		height: 5rem;
		border-radius: var(--radius-2xl);
		background-color: rgba(15, 164, 175, 0.15);
		box-shadow: 0 0 0 4px rgba(15, 164, 175, 0.1);
	}

	.greeks-section__title {
		font-family: var(--font-heading);
		font-size: var(--fs-2xl);
		font-weight: var(--w-bold);
		color: var(--color-white);
		margin-bottom: 0.75rem;
	}

	@media (min-width: 640px) {
		.greeks-section__title {
			font-size: var(--fs-3xl);
		}
	}

	.greeks-section__desc {
		color: var(--color-grey-300);
		font-size: 1rem;
		line-height: 1.65;
		margin-bottom: 1.5rem;
	}

	.greeks-section__success {
		border: 1px solid rgba(34, 181, 115, 0.3);
		background-color: rgba(34, 181, 115, 0.1);
		border-radius: var(--radius-xl);
		padding: 1rem;
	}

	.greeks-section__success-inner {
		display: flex;
		align-items: flex-start;
		gap: 0.75rem;
	}

	:global(.greeks-section__success-icon) {
		flex-shrink: 0;
	}

	.greeks-section__success-title {
		font-size: var(--fs-sm);
		font-weight: var(--w-semibold);
		color: var(--color-white);
		margin-bottom: 0.25rem;
	}

	.greeks-section__success-text {
		color: var(--color-grey-300);
		font-size: var(--fs-xs);
	}

	.greeks-section__form-row {
		display: flex;
		flex-direction: column;
		gap: 0.75rem;
	}

	@media (min-width: 640px) {
		.greeks-section__form-row {
			flex-direction: row;
		}
	}

	.greeks-section__input-wrap {
		position: relative;
		flex: 1;
	}

	:global(.greeks-section__input-icon) {
		position: absolute !important;
		top: 50%;
		left: 1rem;
		transform: translateY(-50%);
		color: var(--color-grey-400) !important;
	}

	.greeks-section__input {
		width: 100%;
		border-radius: var(--radius-xl);
		border: 1px solid rgba(255, 255, 255, 0.2);
		background-color: rgba(255, 255, 255, 0.1);
		padding: 0.875rem 1rem 0.875rem 3rem;
		font-size: var(--fs-sm);
		color: var(--color-white);
		backdrop-filter: blur(4px);
		transition: all 200ms var(--ease-out);
	}

	.greeks-section__input::placeholder {
		color: var(--color-grey-400);
	}

	.greeks-section__input:focus {
		outline: none;
		border-color: rgba(15, 164, 175, 0.5);
		background-color: rgba(255, 255, 255, 0.15);
		box-shadow: 0 0 0 2px rgba(15, 164, 175, 0.3);
	}

	.greeks-section__input:disabled {
		opacity: 0.5;
	}

	.greeks-section__submit {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		gap: 0.5rem;
		border-radius: var(--radius-xl);
		padding: 0.875rem 1.5rem;
		font-size: var(--fs-sm);
		font-weight: var(--w-semibold);
		color: var(--color-white);
		background-color: var(--color-teal);
		box-shadow:
			var(--shadow-lg),
			0 4px 14px rgba(15, 164, 175, 0.25);
		transition: all 300ms var(--ease-out);
		cursor: pointer;
		border: none;
	}

	.greeks-section__submit:hover {
		background-color: var(--color-teal-light);
		transform: translateY(-1px);
		box-shadow:
			var(--shadow-xl),
			0 8px 20px rgba(15, 164, 175, 0.3);
	}

	.greeks-section__submit:active {
		transform: scale(0.97);
	}
	.greeks-section__submit:disabled {
		pointer-events: none;
		opacity: 0.5;
	}

	@media (min-width: 640px) {
		.greeks-section__submit {
			width: auto;
		}
	}

	.greeks-section__error {
		color: var(--color-red);
		margin-top: 0.75rem;
		font-size: var(--fs-xs);
	}

	.greeks-section__privacy {
		color: var(--color-grey-400);
		margin-top: 0.75rem;
		font-size: var(--fs-xs);
	}
</style>
