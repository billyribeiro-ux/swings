import type { ApexOptions } from 'apexcharts';
import { heroChartTheme } from '$lib/utils/chartThemes';
import type { OhlcCandle } from '$lib/utils/chartData';

/** Apex candlestick OHLC order: [open, high, low, close] */
export function ohlcToApexSeries(rows: OhlcCandle[]) {
	return rows.map((r) => ({
		x: new Date(`${r.time}T12:00:00.000Z`).getTime(),
		y: [r.open, r.high, r.low, r.close] as [number, number, number, number]
	}));
}

const c = heroChartTheme.candlestick;

export function buildHeroCandleOptions(opts: {
	height: number;
	series: ApexOptions['series'];
}): ApexOptions {
	return {
		...(opts.series !== undefined ? { series: opts.series } : {}),
		chart: {
			type: 'candlestick',
			height: opts.height,
			width: '100%',
			background: 'transparent',
			toolbar: { show: false },
			zoom: { enabled: false },
			animations: { enabled: true, speed: 800 },
			fontFamily: 'Inter, ui-sans-serif, system-ui, sans-serif',
			selection: { enabled: false },
			sparkline: { enabled: false }
		},
		theme: { mode: 'dark' },
		plotOptions: {
			candlestick: {
				colors: {
					upward: c.upColor,
					downward: c.downColor
				}
			}
		},
		grid: {
			show: true,
			borderColor: 'rgba(15, 164, 175, 0.06)',
			strokeDashArray: 0,
			position: 'back',
			xaxis: { lines: { show: true } },
			yaxis: { lines: { show: true } },
			padding: { top: 4, right: 4, bottom: 4, left: 4 }
		},
		xaxis: {
			type: 'datetime',
			labels: { show: false },
			axisBorder: { show: false },
			axisTicks: { show: false },
			crosshairs: { show: false },
			tooltip: { enabled: false }
		},
		yaxis: {
			labels: { show: false },
			crosshairs: { show: false },
			tooltip: { enabled: false }
		},
		tooltip: { enabled: false },
		legend: { show: false },
		dataLabels: { enabled: false },
		states: {
			hover: { filter: { type: 'none' } },
			active: { filter: { type: 'none' } }
		}
	};
}

export function buildMiniCandleOptions(opts: {
	height: number;
	series: ApexOptions['series'];
	showcase: boolean;
}): ApexOptions {
	const base = buildHeroCandleOptions({ height: opts.height, series: opts.series });
	if (!opts.showcase) {
		return {
			...base,
			chart: {
				...base.chart,
				animations: { enabled: true, speed: 600 }
			},
			grid: { ...base.grid, show: false }
		};
	}
	return {
		...base,
		chart: {
			...base.chart,
			animations: { enabled: true, speed: 600 }
		},
		grid: {
			...base.grid,
			show: true,
			xaxis: { lines: { show: false } },
			yaxis: { lines: { show: true } }
		}
	};
}
