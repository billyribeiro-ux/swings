<!--
  FORM-10: Chained country → state dropdown. Countries load from a
  canonical endpoint on mount; states repopulate whenever the selected
  country changes. Stored as `{ country, state }`.
-->
<script lang="ts">
	import { onMount } from 'svelte';
	import { api } from '$lib/api/client';
	import type { FieldProps } from '../types.ts';

	interface CountryState {
		country: string;
		state: string;
	}

	const { field, value, error, disabled = false, onChange }: FieldProps = $props();
	const current = $derived<CountryState>(toCS(value));
	const base = $derived(`form-field-${field.key}`);
	const countryId = $derived(`${base}-country`);
	const stateId = $derived(`${base}-state`);
	const errorId = $derived(error ? `${base}-err` : undefined);

	let countries = $state<{ value: string; label: string }[]>([]);
	let states = $state<{ value: string; label: string }[]>([]);
	let loading = $state(true);

	function toCS(v: unknown): CountryState {
		if (v && typeof v === 'object') {
			const rec = v as Record<string, unknown>;
			return {
				country: typeof rec.country === 'string' ? rec.country : '',
				state: typeof rec.state === 'string' ? rec.state : ''
			};
		}
		return { country: '', state: '' };
	}

	onMount(async () => {
		try {
			countries = await api.get<{ value: string; label: string }[]>(
				'/api/forms/geo/countries',
				{ skipAuth: true }
			);
		} catch {
			countries = [];
		} finally {
			loading = false;
		}
	});

	$effect(() => {
		const country = current.country;
		if (!country) {
			states = [];
			return;
		}
		let cancelled = false;
		(async () => {
			try {
				const res = await api.get<{ value: string; label: string }[]>(
					`/api/forms/geo/states?country=${encodeURIComponent(country)}`,
					{ skipAuth: true }
				);
				if (!cancelled) states = res;
			} catch {
				if (!cancelled) states = [];
			}
		})();
		return () => {
			cancelled = true;
		};
	});

	function setCountry(v: string) {
		onChange(field.key, { country: v, state: '' });
	}
	function setState(v: string) {
		onChange(field.key, { ...current, state: v });
	}
</script>

<fieldset
	class="fm-field fm-field--group fm-field--country-state"
	class:fm-field--invalid={!!error}
	aria-describedby={errorId}
>
	<legend class="fm-field__label">
		{field.label ?? field.key}
		{#if field.required}
			<span class="fm-field__required" aria-hidden="true">*</span>
			<span class="fm-field__sr">(required)</span>
		{/if}
	</legend>
	<div class="fm-address">
		<div class="fm-address__row">
			<label for={countryId} class="fm-field__label">Country</label>
			<select
				id={countryId}
				class="fm-select"
				value={current.country}
				disabled={disabled || loading}
				aria-busy={loading}
				onchange={(e) => setCountry((e.currentTarget as HTMLSelectElement).value)}
			>
				<option value="" disabled={field.required ?? false}>
					{loading ? 'Loading…' : 'Select country'}
				</option>
				{#each countries as c (c.value)}
					<option value={c.value}>{c.label}</option>
				{/each}
			</select>
		</div>
		<div class="fm-address__row">
			<label for={stateId} class="fm-field__label">State / region</label>
			<select
				id={stateId}
				class="fm-select"
				value={current.state}
				disabled={disabled || states.length === 0}
				onchange={(e) => setState((e.currentTarget as HTMLSelectElement).value)}
			>
				<option value="" disabled={field.required ?? false}>Select state</option>
				{#each states as s (s.value)}
					<option value={s.value}>{s.label}</option>
				{/each}
			</select>
		</div>
	</div>
	{#if error}
		<p id={errorId} class="fm-field__error" role="alert" aria-live="polite">{error}</p>
	{/if}
</fieldset>
