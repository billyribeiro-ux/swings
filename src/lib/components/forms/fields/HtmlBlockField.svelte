<!--
  FORM-10: Decorative HTML block. The admin builder is trusted, but the
  backend sanitises before serving; we inject via `{@html}` because the
  string is already safe by contract.
-->
<script lang="ts">
	import type { FieldProps, FieldSchema } from '../types.ts';

	const { field }: FieldProps = $props();
	const block = $derived(field as Extract<FieldSchema, { type: 'html_block' }>);
</script>

<div class="fm-field fm-block">{@html block.html}</div>
