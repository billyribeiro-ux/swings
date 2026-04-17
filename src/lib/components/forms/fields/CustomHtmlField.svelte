<!--
  FORM-10: Admin-authored raw HTML. Identical to HtmlBlockField at runtime,
  but the admin builder surfaces a richer editor for this variant.
-->
<script lang="ts">
	import type { FieldProps, FieldSchema } from '../types.ts';

	const { field }: FieldProps = $props();
	const block = $derived(field as Extract<FieldSchema, { type: 'custom_html' }>);
</script>

<div class="fm-field fm-block">{@html block.html}</div>
