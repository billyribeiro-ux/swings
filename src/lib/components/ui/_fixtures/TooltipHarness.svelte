<!--
  Test fixture: wraps Tooltip with a button trigger so tests can hover/focus
  the trigger and assert on the floating tooltip element. vitest-browser-svelte's
  render() can't accept snippets as bare JS values, hence this wrapper.
-->
<script lang="ts">
	import Tooltip from '../Tooltip.svelte';

	type Placement = 'top' | 'right' | 'bottom' | 'left';
	interface Props {
		label: string;
		placement?: Placement;
		hotkey?: string;
		delay?: number;
		disabled?: boolean;
		buttonLabel?: string;
	}
	const {
		label,
		placement = 'top',
		hotkey,
		delay = 0,
		disabled = false,
		buttonLabel = 'Trigger'
	}: Props = $props();
</script>

<Tooltip {label} {placement} {hotkey} {delay} {disabled}>
	<button type="button" data-testid="tooltip-trigger">{buttonLabel}</button>
</Tooltip>
