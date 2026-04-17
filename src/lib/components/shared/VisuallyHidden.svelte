<!--
  VisuallyHidden — renders content accessibly to assistive tech while hiding
  it visually. Equivalent to the `visually-hidden` utility class in
  `$styles/utilities.css`, but component-shaped for inline use in templates.

  A11y: preserved in the accessibility tree. Works with screen readers
  because the element keeps layout dimensions of 1px and uses `clip` /
  `clip-path` rather than `display: none` or `visibility: hidden`.
-->
<script lang="ts">
	import type { Snippet } from 'svelte';

	interface Props {
		children: Snippet;
	}

	const { children }: Props = $props();
</script>

<span class="visually-hidden">{@render children()}</span>

<style>
	.visually-hidden {
		position: absolute;
		inline-size: 1px;
		block-size: 1px;
		padding: 0;
		margin: -1px;
		overflow: hidden;
		clip: rect(0, 0, 0, 0);
		clip-path: inset(50%);
		white-space: nowrap;
		border: 0;
	}
</style>
