<script lang="ts">
	import { onMount } from 'svelte';
	import { api, ApiError } from '$lib/api/client';
	import type { UserResponse, BillingAddress } from '$lib/api/types';
	import CheckCircleIcon from 'phosphor-svelte/lib/CheckCircleIcon';
	import XCircleIcon from 'phosphor-svelte/lib/XCircleIcon';

	const COUNTRIES: { code: string; name: string }[] = [
		{ code: 'US', name: 'United States' },
		{ code: 'CA', name: 'Canada' },
		{ code: 'GB', name: 'United Kingdom' },
		{ code: 'AU', name: 'Australia' },
		{ code: 'DE', name: 'Germany' },
		{ code: 'FR', name: 'France' },
		{ code: 'ES', name: 'Spain' },
		{ code: 'IT', name: 'Italy' },
		{ code: 'NL', name: 'Netherlands' },
		{ code: 'BR', name: 'Brazil' },
		{ code: 'MX', name: 'Mexico' },
		{ code: 'JP', name: 'Japan' },
		{ code: 'IN', name: 'India' },
		{ code: 'OTHER', name: 'Other' }
	];

	let loading = $state(true);
	let saving = $state(false);
	let loadError = $state('');
	let success = $state('');
	let saveError = $state('');

	let phone = $state('');
	let line1 = $state('');
	let line2 = $state('');
	let city = $state('');
	let stateRegion = $state('');
	let postalCode = $state('');
	let country = $state('US');
	let countryOther = $state('');

	async function load() {
		loading = true;
		loadError = '';
		try {
			const me = await api.get<UserResponse>('/api/member/profile');
			phone = me.phone ?? '';
			line1 = me.billing_line1 ?? '';
			line2 = me.billing_line2 ?? '';
			city = me.billing_city ?? '';
			stateRegion = me.billing_state ?? '';
			postalCode = me.billing_postal_code ?? '';
			const cc = (me.billing_country ?? '').toUpperCase();
			if (cc && COUNTRIES.some((c) => c.code === cc)) {
				country = cc;
				countryOther = '';
			} else if (cc) {
				country = 'OTHER';
				countryOther = cc;
			} else {
				country = 'US';
				countryOther = '';
			}
		} catch (e) {
			loadError = e instanceof ApiError ? e.message : 'Failed to load profile';
		} finally {
			loading = false;
		}
	}

	function resolvedCountry(): string {
		if (country === 'OTHER') return countryOther.trim().toUpperCase();
		return country;
	}

	function hasAddressInput(): boolean {
		return Boolean(
			line1.trim() || line2.trim() || city.trim() || stateRegion.trim() || postalCode.trim()
		);
	}

	async function save(e: SubmitEvent) {
		e.preventDefault();
		saveError = '';
		success = '';
		// If user typed any address field, line1 is required.
		if (hasAddressInput() && !line1.trim()) {
			saveError = 'Address Line 1 is required when entering an address.';
			return;
		}
		const billing_address: BillingAddress = {
			line1: line1.trim() || null,
			line2: line2.trim() || null,
			city: city.trim() || null,
			state: stateRegion.trim() || null,
			postal_code: postalCode.trim() || null,
			country: resolvedCountry() || null
		};
		const body: { phone?: string | null; billing_address: BillingAddress } = {
			phone: phone.trim() || null,
			billing_address
		};
		saving = true;
		try {
			const updated = await api.put<UserResponse>('/api/member/profile', body);
			// Re-sync from server response so we display the canonical values
			phone = updated.phone ?? '';
			line1 = updated.billing_line1 ?? '';
			line2 = updated.billing_line2 ?? '';
			city = updated.billing_city ?? '';
			stateRegion = updated.billing_state ?? '';
			postalCode = updated.billing_postal_code ?? '';
			const cc = (updated.billing_country ?? '').toUpperCase();
			if (cc && COUNTRIES.some((c) => c.code === cc)) {
				country = cc;
				countryOther = '';
			} else if (cc) {
				country = 'OTHER';
				countryOther = cc;
			}
			success = 'Billing details saved.';
		} catch (err) {
			saveError = err instanceof ApiError ? err.message : 'Failed to save billing address';
		} finally {
			saving = false;
		}
	}

	onMount(() => {
		void load();
	});
</script>

<svelte:head><title>Billing Address - Precision Options Signals</title></svelte:head>

<section class="ba">
	<header class="ba__header">
		<h1 class="ba__title">Billing Address</h1>
		<p class="ba__sub">Used for invoices and payment confirmations.</p>
	</header>

	{#if loading}
		<p class="ba__muted">Loading…</p>
	{:else if loadError}
		<div class="ba__error" role="alert">{loadError}</div>
	{:else}
		<form class="ba__card" onsubmit={save} novalidate>
			<div class="ba__field">
				<label class="ba__label" for="ba-phone">Phone</label>
				<input
					id="ba-phone"
					class="ba__input"
					type="tel"
					autocomplete="tel"
					bind:value={phone}
					disabled={saving}
					placeholder="Optional"
				/>
			</div>

			<div class="ba__field">
				<label class="ba__label" for="ba-line1">Address Line 1</label>
				<input
					id="ba-line1"
					class="ba__input"
					type="text"
					autocomplete="address-line1"
					bind:value={line1}
					disabled={saving}
					placeholder="Street, PO box"
				/>
			</div>

			<div class="ba__field">
				<label class="ba__label" for="ba-line2">Address Line 2</label>
				<input
					id="ba-line2"
					class="ba__input"
					type="text"
					autocomplete="address-line2"
					bind:value={line2}
					disabled={saving}
					placeholder="Apt, suite, etc. (optional)"
				/>
			</div>

			<div class="ba__row">
				<div class="ba__field">
					<label class="ba__label" for="ba-city">City</label>
					<input
						id="ba-city"
						class="ba__input"
						type="text"
						autocomplete="address-level2"
						bind:value={city}
						disabled={saving}
					/>
				</div>
				<div class="ba__field">
					<label class="ba__label" for="ba-state">State / Province</label>
					<input
						id="ba-state"
						class="ba__input"
						type="text"
						autocomplete="address-level1"
						bind:value={stateRegion}
						disabled={saving}
					/>
				</div>
			</div>

			<div class="ba__row">
				<div class="ba__field">
					<label class="ba__label" for="ba-postal">Postal Code</label>
					<input
						id="ba-postal"
						class="ba__input"
						type="text"
						autocomplete="postal-code"
						bind:value={postalCode}
						disabled={saving}
					/>
				</div>
				<div class="ba__field">
					<label class="ba__label" for="ba-country">Country</label>
					<select
						id="ba-country"
						class="ba__select"
						bind:value={country}
						disabled={saving}
					>
						{#each COUNTRIES as c (c.code)}
							<option value={c.code}>{c.name}</option>
						{/each}
					</select>
				</div>
			</div>

			{#if country === 'OTHER'}
				<div class="ba__field">
					<label class="ba__label" for="ba-country-other"
						>Country code (ISO 3166-1 alpha-2)</label
					>
					<input
						id="ba-country-other"
						class="ba__input"
						type="text"
						maxlength="2"
						bind:value={countryOther}
						disabled={saving}
						placeholder="e.g. SE"
					/>
				</div>
			{/if}

			{#if success}
				<p class="ba__success" role="status">
					<CheckCircleIcon size={14} weight="fill" />
					{success}
				</p>
			{/if}
			{#if saveError}
				<p class="ba__inline-error" role="alert">
					<XCircleIcon size={14} weight="fill" />
					{saveError}
				</p>
			{/if}

			<div class="ba__actions">
				<button type="submit" class="btn btn--primary" disabled={saving}>
					{saving ? 'Saving…' : 'Save changes'}
				</button>
			</div>
		</form>
	{/if}
</section>

<style>
	.ba {
		display: flex;
		flex-direction: column;
		gap: 1.25rem;
	}
	.ba__header {
		display: flex;
		flex-direction: column;
		gap: 0.25rem;
	}
	.ba__title {
		font-size: var(--fs-2xl);
		font-weight: var(--w-bold);
		color: var(--color-white);
		font-family: var(--font-heading);
	}
	.ba__sub {
		color: var(--color-grey-400);
		font-size: var(--fs-sm);
	}
	.ba__muted {
		color: var(--color-grey-400);
		font-size: var(--fs-sm);
	}
	.ba__error {
		padding: 0.85rem 1rem;
		border-radius: var(--radius-lg);
		background-color: rgba(224, 72, 72, 0.1);
		border: 1px solid rgba(224, 72, 72, 0.25);
		color: var(--color-red);
		font-size: var(--fs-sm);
	}

	.ba__card {
		background-color: var(--color-navy-mid);
		border: 1px solid rgba(255, 255, 255, 0.06);
		border-radius: var(--radius-xl);
		padding: 1.5rem;
		display: flex;
		flex-direction: column;
		gap: 0.85rem;
		max-width: 40rem;
	}

	.ba__field {
		display: flex;
		flex-direction: column;
		gap: 0.35rem;
	}
	.ba__row {
		display: grid;
		grid-template-columns: 1fr 1fr;
		gap: 0.85rem;
	}
	.ba__label {
		font-size: var(--fs-xs);
		color: var(--color-grey-400);
		font-weight: var(--w-semibold);
		text-transform: uppercase;
		letter-spacing: 0.04em;
	}

	.ba__input,
	.ba__select {
		padding: 0.65rem 0.85rem;
		border-radius: var(--radius-lg);
		background-color: rgba(255, 255, 255, 0.04);
		border: 1px solid rgba(255, 255, 255, 0.1);
		color: var(--color-white);
		font-size: var(--fs-sm);
		width: 100%;
	}
	.ba__input:focus,
	.ba__select:focus {
		outline: none;
		border-color: var(--color-teal);
		background-color: rgba(15, 164, 175, 0.06);
	}
	.ba__input:disabled,
	.ba__select:disabled {
		opacity: 0.6;
		cursor: not-allowed;
	}
	.ba__select {
		appearance: none;
	}

	.btn {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		gap: 0.4rem;
		padding: 0.65rem 1.1rem;
		border-radius: var(--radius-lg);
		font-size: var(--fs-sm);
		font-weight: var(--w-semibold);
		cursor: pointer;
		border: 1px solid transparent;
		transition: opacity 200ms var(--ease-out);
	}
	.btn:disabled {
		opacity: 0.55;
		cursor: not-allowed;
	}
	.btn--primary {
		background: linear-gradient(135deg, var(--color-teal), #0d8a94);
		color: var(--color-white);
	}
	.btn--primary:not(:disabled):hover {
		opacity: 0.9;
	}

	.ba__actions {
		display: flex;
		justify-content: flex-end;
		padding-top: 0.5rem;
	}

	.ba__success {
		display: inline-flex;
		align-items: center;
		gap: 0.4rem;
		padding: 0.65rem 0.85rem;
		border-radius: var(--radius-lg);
		background-color: rgba(34, 181, 115, 0.1);
		border: 1px solid rgba(34, 181, 115, 0.25);
		color: var(--color-green);
		font-size: var(--fs-sm);
	}
	.ba__inline-error {
		display: inline-flex;
		align-items: center;
		gap: 0.4rem;
		padding: 0.65rem 0.85rem;
		border-radius: var(--radius-lg);
		background-color: rgba(224, 72, 72, 0.1);
		border: 1px solid rgba(224, 72, 72, 0.25);
		color: var(--color-red);
		font-size: var(--fs-sm);
	}

	@media (max-width: 640px) {
		.ba__card {
			padding: 1rem;
		}
		.ba__row {
			grid-template-columns: 1fr;
		}
	}
</style>
