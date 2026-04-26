<!--
  FORM-10: Step boundary marker. The renderer pre-partitions the schema
  into steps around this field, so it never actually renders content; the
  component exists only to satisfy the dispatcher's exhaustive handling of
  the 33-variant FieldSchema contract.
-->
<script lang="ts">
	import type { FieldProps } from '../types.ts';
	// Accept the full FieldProps bag for dispatcher-splat compatibility; the
	// body renders nothing, so the prop values are intentionally unused. The
	// `_props` binding silences `noUnusedLocals`; the leading underscore
	// satisfies `@typescript-eslint/no-unused-vars`'s `^_` ignore pattern.
	const _props: FieldProps = $props();
	// svelte-ignore state_referenced_locally
	void _props;
</script>
