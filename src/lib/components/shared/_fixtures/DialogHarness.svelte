<!--
  Test fixture: Dialog with a couple of focusable controls inside so focus
  trap assertions have something to hit.
-->
<script lang="ts">
	import { untrack } from 'svelte';
	import Dialog from '../Dialog.svelte';

	interface HarnessProps {
		initialOpen?: boolean;
		closeOnBackdrop?: boolean;
		closeOnEscape?: boolean;
		title?: string;
		description?: string;
	}

	const {
		initialOpen = true,
		closeOnBackdrop = true,
		closeOnEscape = true,
		title = 'Test dialog',
		description = 'A test description'
	}: HarnessProps = $props();

	// Initialize once from the prop; further updates to the prop should not
	// reset the dialog's open state (that is the component under test's job).
	let open = $state(untrack(() => initialOpen));
</script>

<button type="button" data-testid="trigger" onclick={() => (open = true)}>Open</button>

<Dialog bind:open {title} {description} {closeOnBackdrop} {closeOnEscape}>
	<p>Body content goes here.</p>
	<button type="button" data-testid="first-focusable">First</button>
	<input type="text" data-testid="text-input" />
	<button type="button" data-testid="last-focusable">Last</button>
</Dialog>
