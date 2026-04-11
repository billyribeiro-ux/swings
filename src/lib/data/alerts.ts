export interface Alert {
	ticker: string;
	entryZone: string;
	invalidation: string;
	profitZones: string[];
	notes: string;
}

export const sampleAlert: Alert = {
	ticker: 'AAPL',
	entryZone: '182.50 – 183.20',
	invalidation: 'Below 181.90',
	profitZones: ['185.20', '186.40', '188.00'],
	notes:
		'Watching for a breakout above the 20-day MA with strong volume. Early entry zone gives room before momentum kicks in.'
};
