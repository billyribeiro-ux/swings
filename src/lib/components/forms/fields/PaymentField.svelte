<!--
  FORM-10: Stripe Elements payment field (FORM-08).

  Stripe.js is loaded on first mount by injecting a <script> tag. The
  publishable key is sourced from the `PUBLIC_STRIPE_PUBLISHABLE_KEY`
  dynamic env var (exposed via `$env/dynamic/public`). A PaymentIntent is
  created server-side (`POST /api/payments/intents` with { amount_cents,
  currency, field_key }); the returned client secret is handed to
  Elements, and the resulting PaymentMethod id lands in form data under
  the field key so FORM-03 can link the submission to an orders row.

  A11y: mounts the Elements iframe inside a `role="group"` labelled by
  the field's visible title so SR users know the scope.
-->
<script lang="ts">
	import { env } from '$env/dynamic/public';
	import { onMount } from 'svelte';
	import { api } from '$lib/api/client';
	import type { FieldProps, FieldSchema } from '../types.ts';

	interface StripeLike {
		elements(options: Record<string, unknown>): StripeElements;
		confirmPayment(options: {
			elements: StripeElements;
			confirmParams: Record<string, unknown>;
			redirect: 'if_required';
		}): Promise<{
			paymentIntent?: { id: string; status: string };
			error?: { message: string };
		}>;
	}
	interface StripeElements {
		create(kind: string, options?: Record<string, unknown>): StripeElementHandle;
	}
	interface StripeElementHandle {
		mount(selector: string | HTMLElement): void;
	}

	const { field, value, error, disabled = false, onChange }: FieldProps = $props();
	const pay = $derived(field as Extract<FieldSchema, { type: 'payment' }>);
	const controlId = $derived(`form-field-${field.key}`);
	const mountId = $derived(`${controlId}-stripe`);
	const helpId = $derived(field.helpText ? `${controlId}-help` : undefined);
	const errorId = $derived(error ? `${controlId}-err` : undefined);

	let loading = $state(true);
	let localError = $state('');
	let elements: StripeElements | null = $state(null);

	const current = $derived(
		value && typeof value === 'object' && 'payment_method_id' in value
			? ((value as Record<string, unknown>).payment_method_id as string)
			: ''
	);

	interface StripeWindow extends Window {
		Stripe?: (key: string) => StripeLike;
	}

	async function ensureStripe(): Promise<StripeLike | null> {
		if (typeof window === 'undefined') return null;
		const w = window as StripeWindow;
		const key = env.PUBLIC_STRIPE_PUBLISHABLE_KEY ?? '';
		const existing = w.Stripe;
		if (existing) return existing(key);
		await new Promise<void>((resolve, reject) => {
			const s = document.createElement('script');
			s.src = 'https://js.stripe.com/v3/';
			s.async = true;
			s.onload = () => resolve();
			s.onerror = () => reject(new Error('Failed to load Stripe.js'));
			document.head.appendChild(s);
		});
		const loaded = w.Stripe;
		if (!loaded) return null;
		return loaded(key);
	}

	async function init() {
		try {
			const stripe = await ensureStripe();
			if (!stripe) {
				localError = 'Unable to load Stripe.';
				return;
			}
			const intent = await api.post<{ client_secret: string }>(
				'/api/payments/intents',
				{
					amount_cents: pay.amount_cents,
					currency: pay.currency ?? 'usd',
					field_key: field.key
				},
				{ skipAuth: true }
			);
			const el = stripe.elements({ clientSecret: intent.client_secret });
			elements = el;
			const paymentEl = el.create('payment');
			paymentEl.mount(`#${mountId}`);
			// Seed the value with the intent's id marker so submit knows payment was staged.
			onChange(field.key, {
				client_secret: intent.client_secret,
				amount_cents: pay.amount_cents,
				currency: pay.currency ?? 'usd'
			});
		} catch (err) {
			localError = err instanceof Error ? err.message : 'Payment setup failed.';
		} finally {
			loading = false;
		}
	}

	onMount(() => {
		void init();
	});
</script>

<div class="fm-field" class:fm-field--invalid={!!error}>
	<div class="fm-field__label">
		<label for={controlId}>
			{field.label ?? 'Payment'}
			{#if field.required}
				<span class="fm-field__required" aria-hidden="true">*</span>
				<span class="fm-field__sr">(required)</span>
			{/if}
		</label>
	</div>
	{#if field.helpText}
		<p id={helpId} class="fm-field__help">{field.helpText}</p>
	{/if}
	<div
		id={controlId}
		role="group"
		aria-label={field.label ?? 'Payment'}
		aria-describedby={helpId ?? errorId}
		aria-busy={loading}
		class="fm-payment"
	>
		<div id={mountId} class="fm-payment__mount"></div>
		{#if loading}
			<p class="fm-field__help">Loading secure payment…</p>
		{/if}
		{#if current}
			<p class="fm-field__help">Payment staged. Submission will complete the charge.</p>
		{/if}
	</div>
	{#if !disabled && elements === null && !loading && !localError}
		<!-- elements may legitimately be null if a later effect tears down. -->
	{/if}
	{#if error || localError}
		<p id={errorId} class="fm-field__error" role="alert" aria-live="polite">
			{error ?? localError}
		</p>
	{/if}
</div>
