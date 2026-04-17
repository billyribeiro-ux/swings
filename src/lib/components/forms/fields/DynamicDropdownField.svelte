<!--
  FORM-10: Dropdown populated via XHR at render time. The endpoint is
  expected to return `{ options: ChoiceOption[] }`. Loading + failure
  states are surfaced so users know the control is in flight.
-->
<script lang="ts">
	import FieldFrame from './FieldFrame.svelte';
	import { onMount } from 'svelte';
	import { api } from '$lib/api/client';
	import type { FieldProps, FieldSchema } from '../types.ts';

	interface Opt {
		value: string;
		label: string;
		disabled?: boolean;
	}

	const { field, value, error, disabled = false, onChange }: FieldProps = $props();
	const dd = $derived(field as Extract<FieldSchema, { type: 'dynamic_dropdown' }>);
	const current = $derived(typeof value === 'string' ? value : '');
	const controlId = $derived(`form-field-${field.key}`);

	let options = $state<Opt[]>([]);
	let loading = $state(true);
	let localError = $state('');

	onMount(async () => {
		try {
			const res = await api.get<{ options: Opt[] } | Opt[]>(dd.endpoint, { skipAuth: true });
			options = Array.isArray(res) ? res : (res.options ?? []);
		} catch (err) {
			localError = err instanceof Error ? err.message : 'Failed to load options.';
		} finally {
			loading = false;
		}
	});

	function handleChange(e: Event) {
		const target = e.currentTarget as HTMLSelectElement;
		onChange(field.key, target.value);
	}
</script>

<FieldFrame
	{controlId}
	label={field.label ?? field.key}
	helpText={field.helpText}
	error={error ?? (localError || undefined)}
	required={field.required ?? false}
>
	{#snippet children({ describedBy, invalid, required })}
		<select
			id={controlId}
			name={field.key}
			class="fm-select"
			value={current}
			disabled={disabled || loading}
			aria-describedby={describedBy}
			aria-invalid={invalid}
			aria-required={required}
			aria-busy={loading}
			onchange={handleChange}
		>
			<option value="" disabled={required}>{loading ? 'Loading…' : (field.placeholder ?? 'Select an option')}</option>
			{#each options as opt (opt.value)}
				<option value={opt.value} disabled={opt.disabled ?? false}>{opt.label}</option>
			{/each}
		</select>
	{/snippet}
</FieldFrame>
