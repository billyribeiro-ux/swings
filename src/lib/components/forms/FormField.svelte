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
	import SelectField from './fields/SelectField.svelte';
	import MultiSelectField from './fields/MultiSelectField.svelte';
	import RadioField from './fields/RadioField.svelte';
	import DateField from './fields/DateField.svelte';
	import TimeField from './fields/TimeField.svelte';
	import DatetimeField from './fields/DatetimeField.svelte';
	import SliderField from './fields/SliderField.svelte';
	import RatingField from './fields/RatingField.svelte';
	import FileUploadField from './fields/FileUploadField.svelte';
	import ImageUploadField from './fields/ImageUploadField.svelte';
	import SignatureField from './fields/SignatureField.svelte';
	import RichTextField from './fields/RichTextField.svelte';
	import AddressField from './fields/AddressField.svelte';

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
{:else if type === 'select'}
	<SelectField {...props} />
{:else if type === 'multi_select'}
	<MultiSelectField {...props} />
{:else if type === 'radio'}
	<RadioField {...props} />
{:else if type === 'date'}
	<DateField {...props} />
{:else if type === 'time'}
	<TimeField {...props} />
{:else if type === 'datetime'}
	<DatetimeField {...props} />
{:else if type === 'slider'}
	<SliderField {...props} />
{:else if type === 'rating'}
	<RatingField {...props} />
{:else if type === 'file_upload'}
	<FileUploadField {...props} />
{:else if type === 'image_upload'}
	<ImageUploadField {...props} />
{:else if type === 'signature'}
	<SignatureField {...props} />
{:else if type === 'rich_text'}
	<RichTextField {...props} />
{:else if type === 'address'}
	<AddressField {...props} />
{/if}
