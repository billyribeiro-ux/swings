<!--
  FORM-10: Structured address block. Stores a nested object:
  `{ street, street2, city, state, postal, country }`.

  Uses a `<fieldset>` so SR announces the composite label, with each sub-
  field getting its own `for`/`id` pair. validate.ts treats this as a single
  field; the server unpacks when persisting.
-->
<script lang="ts">
	import type { FieldProps } from '../types.ts';

	interface Address {
		street: string;
		street2: string;
		city: string;
		state: string;
		postal: string;
		country: string;
	}

	const { field, value, error, disabled = false, onChange }: FieldProps = $props();
	const current = $derived<Address>(toAddress(value));
	const base = $derived(`form-field-${field.key}`);
	const helpId = $derived(field.helpText ? `${base}-help` : undefined);
	const errorId = $derived(error ? `${base}-err` : undefined);
	const describedBy = $derived(
		[helpId, errorId].filter((s): s is string => typeof s === 'string').join(' ') || undefined
	);

	function toAddress(v: unknown): Address {
		if (v && typeof v === 'object') {
			const rec = v as Record<string, unknown>;
			return {
				street: typeof rec.street === 'string' ? rec.street : '',
				street2: typeof rec.street2 === 'string' ? rec.street2 : '',
				city: typeof rec.city === 'string' ? rec.city : '',
				state: typeof rec.state === 'string' ? rec.state : '',
				postal: typeof rec.postal === 'string' ? rec.postal : '',
				country: typeof rec.country === 'string' ? rec.country : ''
			};
		}
		return { street: '', street2: '', city: '', state: '', postal: '', country: '' };
	}

	function set(key: keyof Address, v: string) {
		onChange(field.key, { ...current, [key]: v });
	}
</script>

<fieldset
	class="fm-field fm-field--group fm-field--address"
	class:fm-field--invalid={!!error}
	aria-describedby={describedBy}
>
	<legend class="fm-field__label">
		{field.label ?? field.key}
		{#if field.required}
			<span class="fm-field__required" aria-hidden="true">*</span>
			<span class="fm-field__sr">(required)</span>
		{/if}
	</legend>
	{#if field.helpText}
		<p id={helpId} class="fm-field__help">{field.helpText}</p>
	{/if}
	<div class="fm-address">
		<div class="fm-address__row fm-address__row--full">
			<label for="{base}-street" class="fm-field__label">Street</label>
			<input
				id="{base}-street"
				class="fm-input"
				autocomplete="address-line1"
				{disabled}
				value={current.street}
				oninput={(e) => set('street', (e.currentTarget as HTMLInputElement).value)}
			/>
		</div>
		<div class="fm-address__row fm-address__row--full">
			<label for="{base}-street2" class="fm-field__label">Apt / suite</label>
			<input
				id="{base}-street2"
				class="fm-input"
				autocomplete="address-line2"
				{disabled}
				value={current.street2}
				oninput={(e) => set('street2', (e.currentTarget as HTMLInputElement).value)}
			/>
		</div>
		<div class="fm-address__row">
			<label for="{base}-city" class="fm-field__label">City</label>
			<input
				id="{base}-city"
				class="fm-input"
				autocomplete="address-level2"
				{disabled}
				value={current.city}
				oninput={(e) => set('city', (e.currentTarget as HTMLInputElement).value)}
			/>
		</div>
		<div class="fm-address__row">
			<label for="{base}-state" class="fm-field__label">State / region</label>
			<input
				id="{base}-state"
				class="fm-input"
				autocomplete="address-level1"
				{disabled}
				value={current.state}
				oninput={(e) => set('state', (e.currentTarget as HTMLInputElement).value)}
			/>
		</div>
		<div class="fm-address__row">
			<label for="{base}-postal" class="fm-field__label">Postal code</label>
			<input
				id="{base}-postal"
				class="fm-input"
				autocomplete="postal-code"
				{disabled}
				value={current.postal}
				oninput={(e) => set('postal', (e.currentTarget as HTMLInputElement).value)}
			/>
		</div>
		<div class="fm-address__row">
			<label for="{base}-country" class="fm-field__label">Country</label>
			<input
				id="{base}-country"
				class="fm-input"
				autocomplete="country-name"
				{disabled}
				value={current.country}
				oninput={(e) => set('country', (e.currentTarget as HTMLInputElement).value)}
			/>
		</div>
	</div>
	{#if error}
		<p id={errorId} class="fm-field__error" role="alert" aria-live="polite">{error}</p>
	{/if}
</fieldset>
