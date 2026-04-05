// Chart theme configuration aligned with Explosive Swings brand
export const heroChartTheme = {
	// Background
	background: {
		type: 'solid',
		color: 'transparent'
	},
	// Grid
	grid: {
		vertLines: {
			color: 'rgba(15, 164, 175, 0.06)',
			style: 1,
			visible: true
		},
		horzLines: {
			color: 'rgba(15, 164, 175, 0.06)',
			style: 1,
			visible: true
		}
	},
	// Crosshair
	crosshair: {
		mode: 1,
		vertLine: {
			color: '#0fa4af',
			width: 1,
			style: 2,
			visible: true,
			labelVisible: false
		},
		horzLine: {
			color: '#0fa4af',
			width: 1,
			style: 2,
			visible: true,
			labelVisible: false
		}
	},
	// Right price scale
	rightPriceScale: {
		borderColor: 'rgba(15, 164, 175, 0.15)',
		scaleMargins: {
			top: 0.1,
			bottom: 0.1
		}
	},
	// Time scale
	timeScale: {
		borderColor: 'rgba(15, 164, 175, 0.15)',
		timeVisible: false,
		secondsVisible: false
	},
	// Candlestick colors
	candlestick: {
		upColor: '#22b573',
		downColor: '#e04848',
		borderUpColor: '#22b573',
		borderDownColor: '#e04848',
		wickUpColor: 'rgba(34, 181, 115, 0.5)',
		wickDownColor: 'rgba(224, 72, 72, 0.5)'
	},
	// Area chart colors
	area: {
		lineColor: '#0fa4af',
		topColor: 'rgba(15, 164, 175, 0.4)',
		bottomColor: 'rgba(15, 164, 175, 0.05)',
		lineWidth: 2
	},
	// Histogram (volume)
	histogram: {
		upColor: 'rgba(34, 181, 115, 0.6)',
		downColor: 'rgba(224, 72, 72, 0.6)'
	},
	// Handle scale
	handleScale: {
		axisPressedMouseMove: {
			time: false,
			price: false
		}
	},
	// Tracking mode
	trackingMode: {
		exitMode: 1
	}
};

// Compact theme for mini charts (alert cards, small widgets)
export const miniChartTheme = {
	...heroChartTheme,
	grid: {
		vertLines: { visible: false },
		horzLines: { visible: false }
	},
	crosshair: {
		mode: 0,
		vertLine: { visible: false },
		horzLine: { visible: false }
	},
	rightPriceScale: { visible: false },
	timeScale: { visible: false }
};

// Dark theme variant (for dashboard)
export const darkChartTheme = {
	...heroChartTheme,
	background: {
		type: 'solid',
		color: '#0b1d3a'
	},
	grid: {
		vertLines: {
			color: 'rgba(255, 255, 255, 0.05)'
		},
		horzLines: {
			color: 'rgba(255, 255, 255, 0.05)'
		}
	}
};

// Chart options presets
export const chartOptions = {
	mini: {
		layout: {
			background: { type: 'solid', color: 'transparent' },
			textColor: '#8b95a8',
			fontSize: 10,
			fontFamily: "'Inter', sans-serif"
		},
		grid: {
			vertLines: { visible: false },
			horzLines: { visible: false }
		},
		crosshair: { mode: 0 },
		rightPriceScale: { visible: false },
		timeScale: { visible: false },
		handleScroll: false,
		handleScale: false
	},
	hero: {
		layout: {
			background: { type: 'solid', color: 'transparent' },
			textColor: '#8b95a8',
			fontSize: 11,
			fontFamily: "'Inter', sans-serif"
		},
		grid: {
			vertLines: {
				color: 'rgba(15, 164, 175, 0.08)',
				visible: true
			},
			horzLines: {
				color: 'rgba(15, 164, 175, 0.08)',
				visible: true
			}
		},
		crosshair: {
			mode: 0,
			vertLine: { visible: false },
			horzLine: { visible: false }
		},
		rightPriceScale: { visible: false },
		timeScale: { visible: false },
		handleScroll: false,
		handleScale: false
	},
	dashboard: {
		layout: {
			background: { type: 'solid', color: '#132b50' },
			textColor: '#8b95a8',
			fontSize: 12,
			fontFamily: "'Inter', sans-serif"
		},
		rightPriceScale: {
			visible: true,
			borderColor: 'rgba(255, 255, 255, 0.1)',
			scaleMargins: { top: 0.1, bottom: 0.2 }
		},
		timeScale: {
			visible: true,
			borderColor: 'rgba(255, 255, 255, 0.1)',
			timeVisible: true
		}
	}
};
