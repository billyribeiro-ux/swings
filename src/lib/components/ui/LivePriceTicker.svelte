<script lang="ts">
	import { untrack } from 'svelte';
	import {
		sampleTickers,
		generateLivePrice,
		formatPrice,
		formatPercent
	} from '$lib/utils/chartData';

	interface TickerItem {
		symbol: string;
		name: string;
		price: number;
		change: number;
		changePercent: number;
	}

	let tickers = $state<TickerItem[]>([]);

	$effect(() => {
		// Initialize with base prices.
		tickers = sampleTickers.map((t) => ({
			...t,
			...generateLivePrice(t.basePrice)
		}));

		// Update prices every 3 seconds. `untrack` keeps the read of `tickers` from
		// becoming a dependency of this effect — otherwise the assignment below would
		// re-trigger the effect on every tick and create a tight loop.
		const interval = setInterval(() => {
			tickers = untrack(() => tickers).map((t) => {
				const live = generateLivePrice(t.price);
				return {
					...t,
					price: live.price,
					change: live.change,
					changePercent: live.changePercent
				};
			});
		}, 3000);

		return () => clearInterval(interval);
	});

	function getChangeColor(change: number): string {
		return change >= 0 ? 'var(--color-green)' : 'var(--color-red)';
	}
</script>

<div class="ticker-tape">
	<div class="ticker-content">
		{#each tickers as ticker (ticker.symbol)}
			<div class="ticker-item">
				<span class="ticker-symbol">{ticker.symbol}</span>
				<span class="ticker-price">{formatPrice(ticker.price)}</span>
				<span class="ticker-change" style="color: {getChangeColor(ticker.change)}">
					{formatPercent(ticker.changePercent)}
				</span>
			</div>
		{/each}
		<!-- Duplicate for seamless loop -->
		{#each tickers as ticker (ticker.symbol + '-dup')}
			<div class="ticker-item">
				<span class="ticker-symbol">{ticker.symbol}</span>
				<span class="ticker-price">{formatPrice(ticker.price)}</span>
				<span class="ticker-change" style="color: {getChangeColor(ticker.change)}">
					{formatPercent(ticker.changePercent)}
				</span>
			</div>
		{/each}
	</div>
</div>

<style>
	.ticker-tape {
		width: 100%;
		overflow: hidden;
		background: rgba(11, 29, 58, 0.8);
		backdrop-filter: blur(8px);
		border-top: 1px solid rgba(15, 164, 175, 0.2);
		border-bottom: 1px solid rgba(15, 164, 175, 0.2);
		padding: 0.75rem 0;
	}

	.ticker-content {
		display: flex;
		gap: 3rem;
		animation: scroll 40s linear infinite;
		width: max-content;
	}

	.ticker-tape:hover .ticker-content {
		animation-play-state: paused;
	}

	.ticker-item {
		display: flex;
		align-items: center;
		gap: 0.75rem;
		font-family: var(--font-ui);
		font-size: var(--fs-sm);
		white-space: nowrap;
	}

	.ticker-symbol {
		font-weight: var(--w-bold);
		color: var(--color-teal);
		letter-spacing: 0.05em;
	}

	.ticker-price {
		font-weight: var(--w-semibold);
		color: var(--color-white);
		font-variant-numeric: tabular-nums;
	}

	.ticker-change {
		font-weight: var(--w-medium);
		font-size: var(--fs-xs);
		font-variant-numeric: tabular-nums;
	}

	@keyframes scroll {
		0% {
			transform: translateX(0);
		}
		100% {
			transform: translateX(-50%);
		}
	}

	@media (prefers-reduced-motion: reduce) {
		.ticker-content {
			animation: none;
		}
	}
</style>
