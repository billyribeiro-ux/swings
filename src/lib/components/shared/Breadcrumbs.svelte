<!--
  Breadcrumbs — hierarchical navigation trail.

  A11y:
  - `<nav aria-label="Breadcrumb">` (WAI-ARIA breadcrumb pattern).
  - `<ol>` enforces the ordered relationship.
  - Last item is `aria-current="page"` and rendered as plain text (never a link).
  - Separators are purely decorative and rendered via `::before` on each
    non-first item so they never land in the reading order (`aria-hidden`
    inferred from pseudo-element semantics).
  - LTR uses `›`, RTL uses `‹` — swapped via `:dir(rtl)` so the glyph always
    points in the reading direction.
-->
<script lang="ts" module>
	export type { BreadcrumbItem, BreadcrumbsProps } from './Breadcrumbs.types';
</script>

<script lang="ts">
	import type { BreadcrumbsProps as Props } from './Breadcrumbs.types';

	const { items, 'aria-label': ariaLabel = 'Breadcrumb' }: Props = $props();
</script>

{#if items.length > 0}
	<nav aria-label={ariaLabel} class="breadcrumbs">
		<ol class="list">
			{#each items as item, i (i)}
				{@const isLast = i === items.length - 1}
				<li class="item">
					{#if isLast || !item.href}
						<span class="current" aria-current={isLast ? 'page' : undefined}
							>{item.label}</span
						>
					{:else}
						<!-- eslint-disable-next-line svelte/no-navigation-without-resolve -- href is a caller-supplied prop; resolve() must be applied at the call site -->
						<a class="link" href={item.href}>{item.label}</a>
					{/if}
				</li>
			{/each}
		</ol>
	</nav>
{/if}

<style>
	.breadcrumbs {
		font-size: var(--fs-xs);
		color: var(--surface-fg-muted);
	}
	.list {
		list-style: none;
		margin: 0;
		padding: 0;
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: var(--space-1);
	}
	.item {
		display: inline-flex;
		align-items: center;
		gap: var(--space-1);
	}
	.item + .item::before {
		content: '\203A'; /* › */
		color: var(--surface-fg-muted);
		padding-inline: var(--space-1);
		user-select: none;
	}
	:global([dir='rtl']) .item + .item::before {
		content: '\2039'; /* ‹ */
	}
	.link {
		color: var(--brand-teal-600);
		text-decoration: none;
	}
	.link:hover {
		text-decoration: underline;
	}
	.current {
		color: var(--surface-fg-default);
		font-weight: var(--w-medium);
	}
</style>
