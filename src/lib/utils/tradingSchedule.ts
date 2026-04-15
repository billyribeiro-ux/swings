import { addDays } from 'date-fns';
import { TZDate } from '@date-fns/tz';

/** US Eastern for watchlist delivery and US cash session (handles EST/EDT). */
export const EASTERN_TZ = 'America/New_York';

const WATCHLIST_HOUR = 18;
const WATCHLIST_MINUTE = 0;
const MARKET_OPEN_HOUR = 9;
const MARKET_OPEN_MINUTE = 30;

/**
 * Next Sunday at 6:00 PM Eastern (strictly after `now`).
 */
export function getNextWatchlistDelivery(now: Date): Date {
	const z = new TZDate(now, EASTERN_TZ);
	const sod = new TZDate(z.getFullYear(), z.getMonth(), z.getDate(), 0, 0, 0, 0, EASTERN_TZ);
	const t = now.getTime();

	for (let i = 0; i < 14; i++) {
		const day = addDays(sod, i);
		if (day.getDay() !== 0) continue;
		const delivery = new TZDate(
			day.getFullYear(),
			day.getMonth(),
			day.getDate(),
			WATCHLIST_HOUR,
			WATCHLIST_MINUTE,
			0,
			0,
			EASTERN_TZ
		);
		if (delivery.getTime() > t) {
			return new Date(delivery.getTime());
		}
	}

	throw new Error('getNextWatchlistDelivery: no Sunday within 14 days');
}

/**
 * Next Mon–Fri session open at 9:30 AM Eastern (strictly after `now`).
 * Does not account for exchange holidays.
 */
export function getNextMarketOpen(now: Date): Date {
	const z = new TZDate(now, EASTERN_TZ);
	const sod = new TZDate(z.getFullYear(), z.getMonth(), z.getDate(), 0, 0, 0, 0, EASTERN_TZ);
	const t = now.getTime();

	for (let i = 0; i < 14; i++) {
		const day = addDays(sod, i);
		const dow = day.getDay();
		if (dow === 0 || dow === 6) continue;
		const open = new TZDate(
			day.getFullYear(),
			day.getMonth(),
			day.getDate(),
			MARKET_OPEN_HOUR,
			MARKET_OPEN_MINUTE,
			0,
			0,
			EASTERN_TZ
		);
		if (open.getTime() > t) {
			return new Date(open.getTime());
		}
	}

	throw new Error('getNextMarketOpen: no weekday open within 14 days');
}

export interface WatchlistCountdownParts {
	days: number;
	hours: number;
	minutes: number;
}

export interface MarketCountdownParts {
	hours: number;
	minutes: number;
	seconds: number;
}

export function getWatchlistCountdown(now: Date, target: Date): WatchlistCountdownParts {
	let ms = target.getTime() - now.getTime();
	if (ms < 0) ms = 0;
	const totalM = Math.floor(ms / 60_000);
	return {
		days: Math.floor(totalM / (60 * 24)),
		hours: Math.floor((totalM % (60 * 24)) / 60),
		minutes: totalM % 60
	};
}

/** Total hours (may exceed 24), minutes, seconds until target. */
export function getMarketCountdown(now: Date, target: Date): MarketCountdownParts {
	let ms = target.getTime() - now.getTime();
	if (ms < 0) ms = 0;
	const totalS = Math.floor(ms / 1000);
	return {
		hours: Math.floor(totalS / 3600),
		minutes: Math.floor((totalS % 3600) / 60),
		seconds: totalS % 60
	};
}

export function pad2(n: number): string {
	return String(n).padStart(2, '0');
}
