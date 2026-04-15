import { describe, it, expect } from 'vitest';
import { TZDate } from '@date-fns/tz';
import {
	EASTERN_TZ,
	getNextWatchlistDelivery,
	getNextMarketOpen,
	getWatchlistCountdown,
	getMarketCountdown,
	pad2
} from './tradingSchedule';

function et(y: number, month0: number, d: number, h: number, min: number, sec = 0): Date {
	return new Date(new TZDate(y, month0, d, h, min, sec, 0, EASTERN_TZ).getTime());
}

describe('getNextWatchlistDelivery', () => {
	it('from Monday noon ET returns upcoming Sunday 6:00 PM', () => {
		const now = et(2026, 6, 6, 12, 0); // Mon Jul 6, 2026
		expect(getNextWatchlistDelivery(now)).toEqual(et(2026, 6, 12, 18, 0));
	});

	it('Sunday before 6:00 PM returns same Sunday', () => {
		const now = et(2026, 6, 12, 17, 30);
		expect(getNextWatchlistDelivery(now)).toEqual(et(2026, 6, 12, 18, 0));
	});

	it('Sunday at or after 6:00 PM returns following Sunday', () => {
		const now = et(2026, 6, 12, 18, 0);
		expect(getNextWatchlistDelivery(now)).toEqual(et(2026, 6, 19, 18, 0));
		const later = et(2026, 6, 12, 18, 30);
		expect(getNextWatchlistDelivery(later)).toEqual(et(2026, 6, 19, 18, 0));
	});

	it('winter (EST): Monday to next Sunday', () => {
		const now = et(2026, 0, 5, 10, 0); // Mon Jan 5, 2026
		expect(getNextWatchlistDelivery(now)).toEqual(et(2026, 0, 11, 18, 0));
	});
});

describe('getNextMarketOpen', () => {
	it('weekday before 9:30 AM returns same day open', () => {
		const now = et(2026, 6, 8, 9, 0); // Wed Jul 8
		expect(getNextMarketOpen(now)).toEqual(et(2026, 6, 8, 9, 30));
	});

	it('weekday after 9:30 AM returns next weekday open', () => {
		const now = et(2026, 6, 8, 15, 0);
		expect(getNextMarketOpen(now)).toEqual(et(2026, 6, 9, 9, 30));
	});

	it('Friday evening returns Monday open', () => {
		const now = et(2026, 6, 10, 18, 0);
		expect(getNextMarketOpen(now)).toEqual(et(2026, 6, 13, 9, 30));
	});

	it('Saturday returns Monday open', () => {
		const now = et(2026, 6, 11, 12, 0);
		expect(getNextMarketOpen(now)).toEqual(et(2026, 6, 13, 9, 30));
	});
});

describe('countdown helpers', () => {
	it('getWatchlistCountdown floors to minute parts', () => {
		const target = new Date('2026-01-01T00:02:30.000Z');
		const now = new Date('2026-01-01T00:00:00.000Z');
		expect(getWatchlistCountdown(now, target)).toEqual({ days: 0, hours: 0, minutes: 2 });
	});

	it('getMarketCountdown uses total hours past 24', () => {
		const now = new Date('2026-01-01T00:00:00.000Z');
		const target = new Date('2026-01-02T02:03:04.000Z');
		expect(getMarketCountdown(now, target)).toEqual({ hours: 26, minutes: 3, seconds: 4 });
	});

	it('pad2', () => {
		expect(pad2(3)).toBe('03');
		expect(pad2(12)).toBe('12');
	});
});
