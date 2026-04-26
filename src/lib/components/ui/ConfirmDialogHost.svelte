<script lang="ts">
	// Mounts the active confirm dialog from the imperative `confirms` store.
	// Drop a single instance into the root layout; consumers call
	// `await confirmDialog({ ... })` from anywhere in the app.
	import ConfirmDialog from './ConfirmDialog.svelte';
	import { confirms } from '$lib/stores/confirm.svelte';

	const current = $derived(confirms.current);

	function handleResolve(value: boolean): void {
		confirms.resolveCurrent(value);
	}
</script>

{#if current}
	{#key current.id}
		<ConfirmDialog
			title={current.title}
			message={current.message}
			confirmLabel={current.confirmLabel}
			cancelLabel={current.cancelLabel}
			variant={current.variant}
			onresolve={handleResolve}
		/>
	{/key}
{/if}
