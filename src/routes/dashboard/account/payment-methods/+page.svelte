<script lang="ts">
	import { env } from '$env/dynamic/public';
	import { onMount } from 'svelte';
	import { resolve } from '$app/paths';
	import { api, ApiError } from '$lib/api/client';
	import type { components } from '$lib/api/schema';
	import CreditCardIcon from 'phosphor-svelte/lib/CreditCardIcon';
	import PlusIcon from 'phosphor-svelte/lib/PlusIcon';
	import TrashIcon from 'phosphor-svelte/lib/TrashIcon';
	import CheckCircleIcon from 'phosphor-svelte/lib/CheckCircleIcon';
	import WarningIcon from 'phosphor-svelte/lib/WarningIcon';
	import XIcon from 'phosphor-svelte/lib/XIcon';

	type PaymentMethodSummary = components['schemas']['PaymentMethodSummary'];
	type MemberPaymentMethodsResponse = components['schemas']['MemberPaymentMethodsResponse'];
	type SetupIntentResponse = components['schemas']['SetupIntentResponse'];

	// ── Stripe.js shape (no @stripe/stripe-js dep — we load via <script>) ──
	interface StripeError {
		message?: string;
		code?: string;
	}
	interface StripeSetupIntent {
		id: string;
		status: string;
		payment_method?: string | null;
	}
	interface StripeCardElement {
		mount(target: string | HTMLElement): void;
		unmount(): void;
		on(event: string, handler: (ev: { error?: StripeError }) => void): void;
	}
	interface StripeElements {
		create(kind: 'card', options?: Record<string, unknown>): StripeCardElement;
	}
	interface StripeLike {
		elements(options?: Record<string, unknown>): StripeElements;
		confirmCardSetup(
			clientSecret: string,
			data: { payment_method: { card: StripeCardElement } }
		): Promise<{ setupIntent?: StripeSetupIntent; error?: StripeError }>;
	}
	interface StripeWindow extends Window {
		Stripe?: (key: string) => StripeLike;
	}

	// ── State ──────────────────────────────────────────────────────────────
	let loading = $state(true);
	let error = $state('');
	let paymentMethods = $state<PaymentMethodSummary[]>([]);
	let defaultPaymentMethodId = $state<string | null>(null);
	let busyPmId = $state<string | null>(null);
	let confirmDeletePm = $state<PaymentMethodSummary | null>(null);

	// Add-card modal state.
	let modalOpen = $state(false);
	let modalLoading = $state(false);
	let modalSubmitting = $state(false);
	let modalError = $state('');
	let stripeInstance = $state<StripeLike | null>(null);
	let cardElement = $state<StripeCardElement | null>(null);
	let cardSlotMounted = $state(false);
	let cardError = $state('');

	const STRIPE_KEY = env.PUBLIC_STRIPE_PUBLISHABLE_KEY ?? '';

	// ── Loading + initial fetch ────────────────────────────────────────────
	async function loadPaymentMethods() {
		loading = true;
		error = '';
		try {
			const res = await api.get<MemberPaymentMethodsResponse>('/api/member/payment-methods');
			paymentMethods = res.payment_methods;
			defaultPaymentMethodId = res.default_payment_method_id ?? null;
		} catch (e) {
			if (e instanceof ApiError && e.status === 404) {
				// Member has no Stripe customer yet — present empty state.
				paymentMethods = [];
				defaultPaymentMethodId = null;
				error = '';
			} else {
				error = e instanceof ApiError ? e.message : 'Failed to load payment methods';
			}
		} finally {
			loading = false;
		}
	}

	// ── Set default ────────────────────────────────────────────────────────
	async function setDefault(pm: PaymentMethodSummary) {
		if (busyPmId !== null || pm.is_default) return;
		busyPmId = pm.id;
		error = '';
		try {
			await api.post(`/api/member/payment-methods/${pm.id}/set-default`, {});
			await loadPaymentMethods();
		} catch (e) {
			error = e instanceof ApiError ? e.message : 'Failed to set default payment method';
		} finally {
			busyPmId = null;
		}
	}

	// ── Delete ─────────────────────────────────────────────────────────────
	function askDelete(pm: PaymentMethodSummary) {
		confirmDeletePm = pm;
	}
	function cancelDelete() {
		confirmDeletePm = null;
	}
	async function confirmDelete() {
		if (!confirmDeletePm) return;
		const pm = confirmDeletePm;
		busyPmId = pm.id;
		error = '';
		try {
			await api.delete(`/api/member/payment-methods/${pm.id}`);
			confirmDeletePm = null;
			await loadPaymentMethods();
		} catch (e) {
			error = e instanceof ApiError ? e.message : 'Failed to remove payment method';
		} finally {
			busyPmId = null;
		}
	}

	// ── Add-card modal ─────────────────────────────────────────────────────
	async function ensureStripe(): Promise<StripeLike | null> {
		if (typeof window === 'undefined') return null;
		const w = window as StripeWindow;
		if (w.Stripe) return w.Stripe(STRIPE_KEY);
		await new Promise<void>((res, rej) => {
			const s = document.createElement('script');
			s.src = 'https://js.stripe.com/v3/';
			s.async = true;
			s.onload = () => res();
			s.onerror = () => rej(new Error('Failed to load Stripe.js'));
			document.head.appendChild(s);
		});
		const loaded = (window as StripeWindow).Stripe;
		return loaded ? loaded(STRIPE_KEY) : null;
	}

	async function openAddCardModal() {
		modalOpen = true;
		modalLoading = true;
		modalError = '';
		cardError = '';
		cardSlotMounted = false;
		try {
			if (!STRIPE_KEY) {
				modalError =
					'Card collection is unavailable: PUBLIC_STRIPE_PUBLISHABLE_KEY is not configured.';
				return;
			}
			const stripe = await ensureStripe();
			if (!stripe) {
				modalError = 'Unable to load Stripe.js.';
				return;
			}
			stripeInstance = stripe;
			const intent = await api.post<SetupIntentResponse>(
				'/api/member/payment-methods/setup-intent',
				{}
			);
			const elements = stripe.elements();
			const card = elements.create('card', {
				style: {
					base: {
						color: '#ffffff',
						fontFamily: 'inherit',
						fontSize: '15px',
						'::placeholder': { color: '#7f8b9c' }
					},
					invalid: { color: '#e04848' }
				}
			});
			card.on('change', (ev) => {
				cardError = ev.error?.message ?? '';
			});
			cardElement = card;
			// `elements` itself is no longer needed once `card` is in our hand;
			// Stripe holds the back-reference internally. Keeping a state slot
			// for it would just trip the unused-binding lint.
			void elements;
			// Defer mount until the slot is in the DOM (after this tick).
			queueMicrotask(() => {
				const target = document.getElementById('pm-card-slot');
				if (target) {
					card.mount(target);
					cardSlotMounted = true;
				}
			});
			// Stash the client_secret on the card element scope via closure.
			modalSetupSecret = intent.client_secret;
		} catch (e) {
			modalError =
				e instanceof ApiError
					? e.message
					: e instanceof Error
						? e.message
						: 'Failed to start card setup';
		} finally {
			modalLoading = false;
		}
	}

	let modalSetupSecret = $state('');

	function closeAddCardModal() {
		try {
			cardElement?.unmount();
		} catch {
			// Best-effort — Stripe throws when unmounting an already-detached element.
		}
		cardElement = null;
		stripeInstance = null;
		modalOpen = false;
		modalLoading = false;
		modalSubmitting = false;
		modalError = '';
		cardError = '';
		modalSetupSecret = '';
		cardSlotMounted = false;
	}

	async function submitNewCard(ev: SubmitEvent) {
		ev.preventDefault();
		if (!stripeInstance || !cardElement || !modalSetupSecret) return;
		modalSubmitting = true;
		modalError = '';
		try {
			const result = await stripeInstance.confirmCardSetup(modalSetupSecret, {
				payment_method: { card: cardElement }
			});
			if (result.error) {
				modalError = result.error.message ?? 'Card setup failed';
				return;
			}
			// Success — close modal, refetch list.
			closeAddCardModal();
			await loadPaymentMethods();
		} catch (e) {
			modalError = e instanceof Error ? e.message : 'Card setup failed';
		} finally {
			modalSubmitting = false;
		}
	}

	function brandLabel(brand: string): string {
		const map: Record<string, string> = {
			visa: 'Visa',
			mastercard: 'Mastercard',
			amex: 'American Express',
			discover: 'Discover',
			diners: 'Diners Club',
			jcb: 'JCB',
			unionpay: 'UnionPay'
		};
		return map[brand?.toLowerCase()] ?? 'Card';
	}

	function expLabel(month: number, year: number): string {
		const m = String(month).padStart(2, '0');
		const yy = String(year).slice(-2);
		return `${m}/${yy}`;
	}

	onMount(() => {
		void loadPaymentMethods();
	});
</script>

<svelte:head><title>Payment Methods - Precision Options Signals</title></svelte:head>

<section class="pm">
	<header class="pm__header">
		<h1 class="pm__title">Payment Methods</h1>
		<p class="pm__sub">
			Manage the cards we charge for your subscription and any future purchases. Cards are stored
			securely by Stripe — we never see or store the full card number.
		</p>
	</header>

	{#if loading}
		<p class="pm__muted">Loading saved cards…</p>
	{:else if error}
		<div class="pm__error" role="alert">
			<WarningIcon size={18} weight="fill" />
			<span>{error}</span>
		</div>
	{:else if paymentMethods.length === 0}
		<div class="pm__empty">
			<CreditCardIcon size={36} weight="duotone" />
			<p>You don't have any saved cards yet. Add one to manage subscriptions and make purchases.</p>
			<button type="button" class="btn btn--primary" onclick={openAddCardModal}>
				<PlusIcon size={16} weight="bold" />
				Add a new card
			</button>
			{#if defaultPaymentMethodId === null && paymentMethods.length === 0}
				<a href={resolve('/pricing')} class="btn btn--ghost">View plans</a>
			{/if}
		</div>
	{:else}
		<ul class="pm__list">
			{#each paymentMethods as pm (pm.id)}
				<li class="pm__card" class:pm__card--default={pm.is_default}>
					<div class="pm__cardLeft">
						<div class="pm__icon" aria-hidden="true">
							<CreditCardIcon size={22} weight="duotone" />
						</div>
						<div class="pm__cardMeta">
							<div class="pm__brandRow">
								<span class="pm__brand">{brandLabel(pm.brand)}</span>
								<span class="pm__last4">•••• {pm.last4}</span>
								{#if pm.is_default}
									<span class="pm__badge">
										<CheckCircleIcon size={12} weight="fill" />
										Default
									</span>
								{/if}
							</div>
							<div class="pm__exp">Expires {expLabel(pm.exp_month, pm.exp_year)}</div>
						</div>
					</div>
					<div class="pm__cardActions">
						{#if !pm.is_default}
							<button
								type="button"
								class="btn btn--ghost btn--sm"
								disabled={busyPmId !== null}
								onclick={() => setDefault(pm)}
							>
								{busyPmId === pm.id ? 'Setting…' : 'Set as default'}
							</button>
						{/if}
						<button
							type="button"
							class="btn btn--danger btn--sm"
							disabled={busyPmId !== null}
							aria-label={`Remove ${brandLabel(pm.brand)} ending ${pm.last4}`}
							onclick={() => askDelete(pm)}
						>
							<TrashIcon size={14} weight="bold" />
							{busyPmId === pm.id && confirmDeletePm?.id === pm.id ? 'Removing…' : 'Delete'}
						</button>
					</div>
				</li>
			{/each}
		</ul>

		<button type="button" class="btn btn--primary pm__addBtn" onclick={openAddCardModal}>
			<PlusIcon size={16} weight="bold" />
			Add a new card
		</button>
	{/if}
</section>

{#if confirmDeletePm}
	<div
		class="pm-modal-backdrop"
		role="presentation"
		onclick={cancelDelete}
		onkeydown={(e) => {
			if (e.key === 'Escape') cancelDelete();
		}}
	>
		<div
			class="pm-modal"
			role="dialog"
			aria-modal="true"
			aria-labelledby="pm-confirm-title"
			tabindex="-1"
			onclick={(e) => e.stopPropagation()}
			onkeydown={(e) => e.stopPropagation()}
		>
			<h2 id="pm-confirm-title" class="pm-modal__title">Remove this card?</h2>
			<p class="pm-modal__copy">
				{brandLabel(confirmDeletePm.brand)} ending in {confirmDeletePm.last4} will be detached from
				your account. You can add it back any time.
			</p>
			<div class="pm-modal__actions">
				<button
					type="button"
					class="btn btn--ghost"
					onclick={cancelDelete}
					disabled={busyPmId !== null}
				>
					Cancel
				</button>
				<button
					type="button"
					class="btn btn--danger"
					onclick={confirmDelete}
					disabled={busyPmId !== null}
				>
					{busyPmId === confirmDeletePm.id ? 'Removing…' : 'Remove card'}
				</button>
			</div>
		</div>
	</div>
{/if}

{#if modalOpen}
	<div
		class="pm-modal-backdrop"
		role="presentation"
		onclick={closeAddCardModal}
		onkeydown={(e) => {
			if (e.key === 'Escape') closeAddCardModal();
		}}
	>
		<div
			class="pm-modal"
			role="dialog"
			aria-modal="true"
			aria-labelledby="pm-add-title"
			tabindex="-1"
			onclick={(e) => e.stopPropagation()}
			onkeydown={(e) => e.stopPropagation()}
		>
			<div class="pm-modal__header">
				<h2 id="pm-add-title" class="pm-modal__title">Add a new card</h2>
				<button
					type="button"
					class="pm-modal__close"
					aria-label="Close"
					onclick={closeAddCardModal}
				>
					<XIcon size={18} weight="bold" />
				</button>
			</div>
			{#if modalLoading}
				<p class="pm__muted">Loading secure card form…</p>
			{:else if modalError}
				<div class="pm__error" role="alert">
					<WarningIcon size={18} weight="fill" />
					<span>{modalError}</span>
				</div>
			{:else}
				<form onsubmit={submitNewCard} class="pm-form">
					<label class="pm-form__label" for="pm-card-slot">Card details</label>
					<div id="pm-card-slot" class="pm-form__cardSlot" aria-busy={!cardSlotMounted}></div>
					{#if cardError}
						<p class="pm-form__err" role="alert">{cardError}</p>
					{/if}
					<p class="pm-form__hint">
						Your card details are sent directly to Stripe and never touch our servers.
					</p>
					<div class="pm-modal__actions">
						<button
							type="button"
							class="btn btn--ghost"
							onclick={closeAddCardModal}
							disabled={modalSubmitting}
						>
							Cancel
						</button>
						<button
							type="submit"
							class="btn btn--primary"
							disabled={modalSubmitting || !cardSlotMounted}
						>
							{modalSubmitting ? 'Saving…' : 'Save card'}
						</button>
					</div>
				</form>
			{/if}
		</div>
	</div>
{/if}

<style>
	.pm {
		display: flex;
		flex-direction: column;
		gap: 1.25rem;
	}
	.pm__header {
		display: flex;
		flex-direction: column;
		gap: 0.4rem;
	}
	.pm__title {
		font-size: var(--fs-2xl);
		font-weight: var(--w-bold);
		color: var(--color-white);
		font-family: var(--font-heading);
	}
	.pm__sub {
		color: var(--color-grey-400);
		font-size: var(--fs-sm);
		max-width: 44rem;
		line-height: 1.55;
	}
	.pm__muted {
		color: var(--color-grey-400);
		font-size: var(--fs-sm);
	}

	.pm__error {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		padding: 0.85rem 1rem;
		border-radius: var(--radius-lg);
		background-color: rgba(224, 72, 72, 0.1);
		border: 1px solid rgba(224, 72, 72, 0.25);
		color: var(--color-red);
		font-size: var(--fs-sm);
		max-width: 44rem;
	}

	.pm__empty {
		display: flex;
		flex-direction: column;
		gap: 0.85rem;
		align-items: flex-start;
		background-color: var(--color-navy-mid);
		border: 1px dashed rgba(255, 255, 255, 0.1);
		border-radius: var(--radius-xl);
		padding: 1.5rem;
		max-width: 44rem;
		color: var(--color-grey-300);
	}

	.pm__list {
		list-style: none;
		padding: 0;
		margin: 0;
		display: flex;
		flex-direction: column;
		gap: 0.85rem;
		max-width: 44rem;
	}

	.pm__card {
		display: flex;
		justify-content: space-between;
		align-items: center;
		gap: 1.25rem;
		background-color: var(--color-navy-mid);
		border: 1px solid rgba(255, 255, 255, 0.06);
		border-radius: var(--radius-xl);
		padding: 1rem 1.15rem;
	}
	.pm__card--default {
		border-color: rgba(15, 164, 175, 0.4);
		box-shadow: 0 0 0 1px rgba(15, 164, 175, 0.2);
	}

	.pm__cardLeft {
		display: flex;
		gap: 0.85rem;
		align-items: center;
		min-width: 0;
	}
	.pm__icon {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		width: 2.5rem;
		height: 2.5rem;
		border-radius: var(--radius-lg);
		background-color: rgba(15, 164, 175, 0.12);
		color: var(--color-teal);
		flex-shrink: 0;
	}
	.pm__cardMeta {
		display: flex;
		flex-direction: column;
		gap: 0.15rem;
		min-width: 0;
	}
	.pm__brandRow {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		flex-wrap: wrap;
	}
	.pm__brand {
		font-weight: var(--w-semibold);
		color: var(--color-white);
	}
	.pm__last4 {
		color: var(--color-grey-300);
		font-variant-numeric: tabular-nums;
	}
	.pm__badge {
		display: inline-flex;
		align-items: center;
		gap: 0.25rem;
		padding: 0.15rem 0.5rem;
		border-radius: var(--radius-full);
		background-color: rgba(34, 181, 115, 0.12);
		color: var(--color-green);
		font-size: var(--fs-xs);
		font-weight: var(--w-semibold);
		letter-spacing: 0.02em;
	}
	.pm__exp {
		color: var(--color-grey-400);
		font-size: var(--fs-xs);
		font-variant-numeric: tabular-nums;
	}

	.pm__cardActions {
		display: flex;
		gap: 0.5rem;
		align-items: center;
		flex-shrink: 0;
	}

	.pm__addBtn {
		align-self: flex-start;
	}

	.btn {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		gap: 0.4rem;
		padding: 0.6rem 1rem;
		border-radius: var(--radius-lg);
		font-size: var(--fs-sm);
		font-weight: var(--w-semibold);
		cursor: pointer;
		border: 1px solid transparent;
		transition:
			opacity 200ms var(--ease-out),
			background-color 200ms var(--ease-out);
		text-decoration: none;
	}
	.btn:disabled {
		opacity: 0.55;
		cursor: not-allowed;
	}
	.btn--sm {
		padding: 0.4rem 0.7rem;
		font-size: var(--fs-xs);
	}
	.btn--primary {
		background: linear-gradient(135deg, var(--color-teal), #0d8a94);
		color: var(--color-white);
	}
	.btn--primary:not(:disabled):hover {
		opacity: 0.9;
	}
	.btn--ghost {
		background-color: transparent;
		border-color: rgba(255, 255, 255, 0.12);
		color: var(--color-grey-300);
	}
	.btn--ghost:hover {
		color: var(--color-white);
		border-color: rgba(15, 164, 175, 0.3);
		background-color: rgba(15, 164, 175, 0.06);
	}
	.btn--danger {
		background-color: transparent;
		border-color: rgba(224, 72, 72, 0.3);
		color: var(--color-red);
	}
	.btn--danger:not(:disabled):hover {
		background-color: rgba(224, 72, 72, 0.08);
		border-color: rgba(224, 72, 72, 0.5);
	}

	/* ── Modal ─────────────────────────────────────────────────────────── */
	.pm-modal-backdrop {
		position: fixed;
		inset: 0;
		background-color: rgba(0, 0, 0, 0.55);
		display: flex;
		align-items: center;
		justify-content: center;
		padding: 1.25rem;
		z-index: 50;
	}
	.pm-modal {
		background-color: var(--color-navy-mid);
		border: 1px solid rgba(255, 255, 255, 0.08);
		border-radius: var(--radius-xl);
		padding: 1.5rem;
		max-width: 30rem;
		width: 100%;
		display: flex;
		flex-direction: column;
		gap: 1rem;
	}
	.pm-modal__header {
		display: flex;
		justify-content: space-between;
		align-items: center;
		gap: 1rem;
	}
	.pm-modal__title {
		font-size: var(--fs-lg);
		font-weight: var(--w-bold);
		color: var(--color-white);
		font-family: var(--font-heading);
	}
	.pm-modal__copy {
		color: var(--color-grey-300);
		font-size: var(--fs-sm);
		line-height: 1.55;
	}
	.pm-modal__actions {
		display: flex;
		justify-content: flex-end;
		gap: 0.5rem;
	}
	.pm-modal__close {
		background: transparent;
		border: none;
		color: var(--color-grey-300);
		cursor: pointer;
		padding: 0.25rem;
		border-radius: var(--radius-md);
		display: inline-flex;
		align-items: center;
		justify-content: center;
	}
	.pm-modal__close:hover {
		color: var(--color-white);
		background-color: rgba(255, 255, 255, 0.06);
	}

	.pm-form {
		display: flex;
		flex-direction: column;
		gap: 0.85rem;
	}
	.pm-form__label {
		font-size: var(--fs-xs);
		font-weight: var(--w-semibold);
		color: var(--color-grey-300);
		text-transform: uppercase;
		letter-spacing: 0.04em;
	}
	.pm-form__cardSlot {
		padding: 0.85rem 1rem;
		border-radius: var(--radius-lg);
		background-color: rgba(255, 255, 255, 0.04);
		border: 1px solid rgba(255, 255, 255, 0.1);
		min-height: 2.75rem;
	}
	.pm-form__hint {
		color: var(--color-grey-400);
		font-size: var(--fs-xs);
	}
	.pm-form__err {
		color: var(--color-red);
		font-size: var(--fs-xs);
	}

	@media (max-width: 640px) {
		.pm__card {
			flex-direction: column;
			align-items: stretch;
			gap: 0.85rem;
		}
		.pm__cardActions {
			justify-content: flex-end;
		}
	}
</style>
