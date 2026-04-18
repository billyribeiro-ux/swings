<!--
  FORM-10: Step boundary marker. The renderer pre-partitions the schema
  into steps around this field, so it never actually renders content; the
  component exists only to satisfy the dispatcher's exhaustive handling of
  the 33-variant FieldSchema contract.
-->
<script lang="ts">
	import type { FieldProps } from '../types.ts';
	// Accept the full FieldProps bag for dispatcher-splat compatibility; the
	// body renders nothing, so the prop values are intentionally unused.
	// eslint-disable-next-line @typescript-eslint/no-unused-vars
	const _ignored: FieldProps = $props();
</script>
