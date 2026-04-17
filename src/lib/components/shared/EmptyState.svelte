<!--
  EmptyState — placeholder for empty lists / no-data / zero results states.

  A11y:
  - `role="status"` so screen readers announce the state change when content
    transitions from a populated collection to empty (or vice versa).
  - Headline uses an `<h2>` so heading hierarchy stays intact; consumers may
    override the level via a wrapping outline if needed.
  - Icon snippet is wrapped in aria-hidden; the title carries the accessible
    meaning.
-->
<script lang="ts" module>
	import type { Snippet } from 'svelte';

	export interface EmptyStateProps {
		title: string;
		description?: string;
		icon?: Snippet;
		action?: Snippet;
		/** Element used as the title tag. Defaults to h2. */
		titleTag?: 'h1' | 'h2' | 'h3' | 'h4';
	}
</script>

<script lang="ts">
	const { title, description, icon, action, titleTag = 'h2' }: EmptyStateProps = $props();
</script>

<div class="empty-state" role="status">
	{#if icon}
		<span class="icon" aria-hidden="true">{@render icon()}</span>
	{/if}
	<svelte:element this={titleTag} class="title">{title}</svelte:element>
	{#if description}
		<p class="description">{description}</p>
	{/if}
	{#if action}
		<div class="action">{@render action()}</div>
	{/if}
</div>

<style>
	.empty-state {
		display: grid;
		justify-items: center;
		text-align: center;
		gap: var(--space-3);
		padding-block: var(--space-10);
		padding-inline: var(--space-6);
		color: var(--surface-fg-default);
	}
	.icon {
		color: var(--surface-fg-muted);
		font-size: 3rem;
		line-height: 1;
	}
	.title {
		margin: 0;
		font-family: var(--font-heading);
		font-size: var(--fs-lg);
		font-weight: var(--w-bold);
	}
	.description {
		margin: 0;
		max-inline-size: 48ch;
		color: var(--surface-fg-muted);
		font-size: var(--fs-sm);
		line-height: var(--lh-relaxed);
	}
	.action {
		margin-block-start: var(--space-2);
	}
</style>
