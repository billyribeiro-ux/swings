<!--
  TableSkeleton — admin list-page placeholder.

  Consolidates the row-shimmer pattern used on members/dashboard/watchlists
  so every admin list page gets the same CLS-stable skeleton. Match the
  card height on mobile and the table-row height on tablet+ via the
  `mobileRowHeight` / `tabletRowHeight` props.

  Usage:
    {#if loading}
      <TableSkeleton rows={5} />
    {:else}
      …real rows…
    {/if}

  Why a primitive: half a dozen list pages were re-implementing the
  shimmer keyframe + row-stack. This locks the look (rounded corners,
  navy-on-white-alpha gradient, prefers-reduced-motion respect) and the
  vertical footprint, so swapping skeleton → data doesn't reflow.
-->
<script lang="ts">
	interface Props {
		/** How many shimmer rows to render. Default 5 — matches default `per_page`. */
		rows?: number;
		/** Card-row height on mobile (used <768px). Defaults align with member/watchlist cards. */
		mobileRowHeight?: string;
		/** Row height on tablet+ (≥768px). Defaults align with `.m-table` row metrics. */
		tabletRowHeight?: string;
		/** ARIA-friendly label so SR users hear what's loading. */
		label?: string;
	}

	let {
		rows = 5,
		mobileRowHeight = '5rem',
		tabletRowHeight = '3.6rem',
		label = 'Loading rows'
	}: Props = $props();
</script>

<div
	class="ts"
	role="status"
	aria-live="polite"
	aria-label={label}
	style:--ts-mobile-h={mobileRowHeight}
	style:--ts-tablet-h={tabletRowHeight}
>
	{#each Array(rows) as _, i (i)}
		<div class="ts__row" aria-hidden="true"></div>
	{/each}
</div>

<style>
	.ts {
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
	}

	.ts__row {
		height: var(--ts-mobile-h, 5rem);
		border-radius: var(--radius-2xl);
		background: linear-gradient(
			90deg,
			rgba(255, 255, 255, 0.03) 0%,
			rgba(255, 255, 255, 0.06) 50%,
			rgba(255, 255, 255, 0.03) 100%
		);
		background-size: 200% 100%;
		animation: ts-shimmer 1.6s ease-in-out infinite;
	}

	@keyframes ts-shimmer {
		0% {
			background-position: -200% 0;
		}
		100% {
			background-position: 200% 0;
		}
	}

	@media (min-width: 768px) {
		.ts__row {
			height: var(--ts-tablet-h, 3.6rem);
			border-radius: var(--radius-md);
		}
	}

	@media (prefers-reduced-motion: reduce) {
		.ts__row {
			animation: none;
		}
	}
</style>
