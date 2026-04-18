<!--
  FORM-10: Hidden field. Emits a default value on mount so UTM / referrer
  captures work without user interaction. Never rendered visibly so no
  accessibility scaffolding is required, but an `aria-hidden` attribute is
  declared for belt-and-braces clarity.
-->
<script lang="ts">
	import type { FieldProps, FieldSchema } from '../types.ts';

	const { field, value, onChange }: FieldProps = $props();
	const hidden = $derived(field as Extract<FieldSchema, { type: 'hidden' }>);
	const current = $derived(typeof value === 'string' ? value : '');

	$effect(() => {
		// Seed with server-supplied default on first render only (value is empty).
		if ((value === undefined || value === null || value === '') && hidden.default_value !== undefined) {
			onChange(field.key, hidden.default_value);
		}
	});
</script>

<input
	type="hidden"
	name={field.key}
	value={current}
	aria-hidden="true"
/>
