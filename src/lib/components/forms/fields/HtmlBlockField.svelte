<!--
  FORM-10: Decorative HTML block. The admin builder is trusted, but the
  backend sanitises before serving; we inject via `{@html}` because the
  string is already safe by contract.
-->
<script lang="ts">
	import { safeHtml } from '$lib/utils/safeHtml';
	import type { FieldProps, FieldSchema } from '../types.ts';

	const { field }: FieldProps = $props();
	const block = $derived(field as Extract<FieldSchema, { type: 'html_block' }>);
</script>

<!-- eslint-disable-next-line svelte/no-at-html-tags -- admin-authored HTML, sanitized via DOMPurify in safeHtml() -->
<div class="fm-field fm-block">{@html safeHtml(block.html)}</div>
