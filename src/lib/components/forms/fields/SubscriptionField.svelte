<!--
  FORM-10: Stripe subscription plan. Similar shape to PaymentField but
  targets the subscriptions endpoint; the value carries `{ plan_id,
  client_secret }` so FORM-03 links the row to a subscription.
-->
<script lang="ts">
	import { env } from '$env/dynamic/public';
	import { onMount } from 'svelte';
	import { api } from '$lib/api/client';
	import type { FieldProps, FieldSchema } from '../types.ts';

	interface StripeLike {
		elements(options: Record<string, unknown>): StripeElements;
	}
	interface StripeElements {
		create(kind: string, options?: Record<string, unknown>): StripeElementHandle;
	}
	interface StripeElementHandle {
		mount(selector: string | HTMLElement): void;
	}

	const { field, value: _value, error, onChange }: FieldProps = $props();
	const sub = $derived(field as Extract<FieldSchema, { type: 'subscription' }>);
	const controlId = $derived(`form-field-${field.key}`);
	const mountId = $derived(`${controlId}-stripe`);
	const errorId = $derived(error ? `${controlId}-err` : undefined);

	let loading = $state(true);
	let localError = $state('');

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
				'/api/payments/subscriptions/intents',
				{ plan_id: sub.plan_id, field_key: field.key },
				{ skipAuth: true }
			);
			const el = stripe.elements({ clientSecret: intent.client_secret });
			el.create('payment').mount(`#${mountId}`);
			onChange(field.key, { plan_id: sub.plan_id, client_secret: intent.client_secret });
		} catch (err) {
			localError = err instanceof Error ? err.message : 'Subscription setup failed.';
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
			{field.label ?? 'Subscription'}
			{#if field.required}
				<span class="fm-field__required" aria-hidden="true">*</span>
				<span class="fm-field__sr">(required)</span>
			{/if}
		</label>
	</div>
	<div
		id={controlId}
		role="group"
		aria-label={field.label ?? 'Subscription'}
		aria-busy={loading}
		class="fm-payment"
	>
		<div id={mountId} class="fm-payment__mount"></div>
		{#if loading}
			<p class="fm-field__help">Loading plan details…</p>
		{/if}
	</div>
	{#if error || localError}
		<p id={errorId} class="fm-field__error" role="alert" aria-live="polite">{error ?? localError}</p>
	{/if}
</div>
