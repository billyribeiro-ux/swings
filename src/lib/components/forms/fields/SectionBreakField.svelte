<!--
  FORM-10: Visual section divider + optional heading.
  Rendered as `<hr>` + `<h3>` so SR users hear the level change.
-->
<script lang="ts">
	import type { FieldProps } from '../types.ts';

	const { field }: FieldProps = $props();
</script>

<div class="fm-field fm-section">
	{#if field.label}
		<h3 class="fm-section__title">{field.label}</h3>
	{/if}
	{#if field.helpText}
		<p class="fm-section__help">{field.helpText}</p>
	{/if}
	<hr class="fm-section__rule" />
</div>
