<!--
  Test fixture: wraps Button with a simple text label so tests don't have to
  author a snippet inline (vitest-browser-svelte's render() can't receive
  snippets as bare JS values).
-->
<script lang="ts">
	import Button from '../Button.svelte';
	import type { ButtonProps } from '../Button.svelte';

	type Props = Omit<ButtonProps, 'children'> & { label: string };

	const { label, ...rest }: Props = $props();
</script>

<Button {...rest}>
	{label}
</Button>
