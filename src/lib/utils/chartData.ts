/** OHLC row used by chart generators and Apex adapters */
export interface OhlcCandle {
	time: string;
	open: number;
	high: number;
	low: number;
	close: number;
}

export interface VolumeBar {
	time: string;
	value: number;
	color: string;
}

export interface LinePoint {
	time: string;
	value: number;
}

// Generate realistic OHLC candlestick data
export function generateCandlestickData(
	count: number = 30,
	basePrice: number = 100,
	volatility: number = 0.02
): OhlcCandle[] {
	const data: OhlcCandle[] = [];
	let currentPrice = basePrice;
	const now = new Date();

	for (let i = 0; i < count; i++) {
		const time = new Date(now.getTime() - (count - i) * 24 * 60 * 60 * 1000);

		const change = (Math.random() - 0.5) * volatility * currentPrice;
		const open = currentPrice;
		const close = currentPrice + change;

		const high = Math.max(open, close) + Math.random() * Math.abs(change) * 0.5;
		const low = Math.min(open, close) - Math.random() * Math.abs(change) * 0.5;

		data.push({
			time: time.toISOString().split('T')[0],
			open: parseFloat(open.toFixed(2)),
			high: parseFloat(high.toFixed(2)),
			low: parseFloat(low.toFixed(2)),
			close: parseFloat(close.toFixed(2))
		});

		currentPrice = close;
	}

	return data;
}

// Generate trending data (bullish or bearish)
export function generateTrendData(
	count: number = 30,
	basePrice: number = 100,
	trend: 'up' | 'down' | 'sideways' = 'up',
	strength: number = 0.5
): OhlcCandle[] {
	const data: OhlcCandle[] = [];
	let currentPrice = basePrice;
	const now = new Date();
	const trendDirection = trend === 'up' ? 1 : trend === 'down' ? -1 : 0;

	for (let i = 0; i < count; i++) {
		const time = new Date(now.getTime() - (count - i) * 24 * 60 * 60 * 1000);

		const trendComponent = trendDirection * strength * 0.5;
		const noise = (Math.random() - 0.5) * 1.5;
		const change = (trendComponent + noise) * 0.02 * currentPrice;

		const open = currentPrice;
		const close = currentPrice + change;
		const high = Math.max(open, close) + Math.random() * Math.abs(change) * 0.6;
		const low = Math.min(open, close) - Math.random() * Math.abs(change) * 0.6;

		data.push({
			time: time.toISOString().split('T')[0],
			open: parseFloat(open.toFixed(2)),
			high: parseFloat(high.toFixed(2)),
			low: parseFloat(low.toFixed(2)),
			close: parseFloat(close.toFixed(2))
		});

		currentPrice = close;
	}

	return data;
}

// Generate volume data matching candlesticks
export function generateVolumeData(candleData: OhlcCandle[], baseVolume: number = 1000000): VolumeBar[] {
	return candleData.map((candle) => {
		const volumeMultiplier = 0.5 + Math.random();
		const volume = Math.floor(baseVolume * volumeMultiplier);

		return {
			time: candle.time,
			value: volume,
			color: candle.close >= candle.open ? '#22b573' : '#e04848'
		};
	});
}

// Generate line chart data (for trend visualization)
export function generateLineData(
	count: number = 30,
	baseValue: number = 100,
	trend: 'up' | 'down' | 'volatile' = 'up'
): LinePoint[] {
	const data: LinePoint[] = [];
	let currentValue = baseValue;
	const now = new Date();

	for (let i = 0; i < count; i++) {
		const time = new Date(now.getTime() - (count - i) * 24 * 60 * 60 * 1000);

		let change: number;
		switch (trend) {
			case 'up':
				change = (Math.random() * 0.02 + 0.005) * currentValue;
				break;
			case 'down':
				change = -(Math.random() * 0.02 + 0.005) * currentValue;
				break;
			case 'volatile':
				change = (Math.random() - 0.5) * 0.06 * currentValue;
				break;
			default:
				change = (Math.random() - 0.5) * 0.02 * currentValue;
		}

		currentValue += change;

		data.push({
			time: time.toISOString().split('T')[0],
			value: parseFloat(currentValue.toFixed(2))
		});
	}

	return data;
}

// Sample stock tickers for price ticker
export const sampleTickers = [
	{ symbol: 'SPY', name: 'SPDR S&P 500', basePrice: 450 },
	{ symbol: 'QQQ', name: 'Invesco QQQ', basePrice: 380 },
	{ symbol: 'AAPL', name: 'Apple Inc.', basePrice: 175 },
	{ symbol: 'MSFT', name: 'Microsoft', basePrice: 330 },
	{ symbol: 'NVDA', name: 'NVIDIA', basePrice: 480 },
	{ symbol: 'TSLA', name: 'Tesla Inc.', basePrice: 240 },
	{ symbol: 'AMD', name: 'AMD', basePrice: 145 },
	{ symbol: 'META', name: 'Meta Platforms', basePrice: 320 },
	{ symbol: 'AMZN', name: 'Amazon', basePrice: 145 },
	{ symbol: 'GOOGL', name: 'Alphabet', basePrice: 140 }
];

// Generate live price updates
export function generateLivePrice(basePrice: number): {
	price: number;
	change: number;
	changePercent: number;
} {
	const change = (Math.random() - 0.5) * basePrice * 0.02;
	const price = basePrice + change;
	const changePercent = (change / basePrice) * 100;

	return {
		price: parseFloat(price.toFixed(2)),
		change: parseFloat(change.toFixed(2)),
		changePercent: parseFloat(changePercent.toFixed(2))
	};
}

// Format price for display
export function formatPrice(price: number): string {
	return price.toLocaleString('en-US', {
		minimumFractionDigits: 2,
		maximumFractionDigits: 2
	});
}

// Format percentage
export function formatPercent(value: number): string {
	const sign = value >= 0 ? '+' : '';
	return `${sign}${value.toFixed(2)}%`;
}
