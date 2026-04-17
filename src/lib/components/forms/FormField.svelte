<!--
  FORM-10: Field-type dispatcher.

  Picks the correct leaf renderer based on the `FieldSchema` discriminator.
  Each of the 33 field variants from `backend/src/forms/schema.rs` maps to a
  dedicated sub-component under `./fields/*`. Unknown variants fall through
  silently so a newer backend does not crash this client.
-->
<script lang="ts">
	import type { FieldProps } from './types.ts';

	import TextField from './fields/TextField.svelte';
	import EmailField from './fields/EmailField.svelte';
	import PhoneField from './fields/PhoneField.svelte';
	import UrlField from './fields/UrlField.svelte';
	import TextareaField from './fields/TextareaField.svelte';
	import NumberField from './fields/NumberField.svelte';
	import CheckboxField from './fields/CheckboxField.svelte';
	import HiddenField from './fields/HiddenField.svelte';

	const props: FieldProps = $props();
	const type = $derived(props.field.type);
</script>

{#if type === 'text'}
	<TextField {...props} />
{:else if type === 'email'}
	<EmailField {...props} />
{:else if type === 'phone'}
	<PhoneField {...props} />
{:else if type === 'url'}
	<UrlField {...props} />
{:else if type === 'textarea'}
	<TextareaField {...props} />
{:else if type === 'number'}
	<NumberField {...props} />
{:else if type === 'checkbox'}
	<CheckboxField {...props} />
{:else if type === 'hidden'}
	<HiddenField {...props} />
{/if}
