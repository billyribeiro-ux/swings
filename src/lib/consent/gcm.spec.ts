/**
 * CONSENT-04 — Google Consent Mode v2 mapping unit tests.
 *
 * The bridge wiring (`installGcmBridge` + event subscription) is covered by
 * integration in the browser spec; this file pins the pure category → GCM
 * signal mapping so regressions show up as focused diffs.
 */
import { describe, expect, it } from 'vitest';
import { mapCategoriesToGcm } from './gcm';

describe('mapCategoriesToGcm', () => {
	it('maps analytics → analytics_storage', () => {
		expect(mapCategoriesToGcm({ analytics: true })).toEqual({
			analytics_storage: 'granted'
		});
		expect(mapCategoriesToGcm({ analytics: false })).toEqual({
			analytics_storage: 'denied'
		});
	});

	it('fans out marketing to ad_storage / ad_user_data / ad_personalization', () => {
		expect(mapCategoriesToGcm({ marketing: true })).toEqual({
			ad_storage: 'granted',
			ad_user_data: 'granted',
			ad_personalization: 'granted'
		});
		expect(mapCategoriesToGcm({ marketing: false })).toEqual({
			ad_storage: 'denied',
			ad_user_data: 'denied',
			ad_personalization: 'denied'
		});
	});

	it('maps functional → functional_storage and personalization → personalization_storage', () => {
		expect(mapCategoriesToGcm({ functional: true, personalization: false })).toEqual({
			functional_storage: 'granted',
			personalization_storage: 'denied'
		});
	});

	it('omits signals whose category is not in the input map', () => {
		// A pure-`necessary` envelope produces no GCM signals — `necessary`
		// is always-on and is not a GCM signal at all.
		expect(mapCategoriesToGcm({ necessary: true })).toEqual({});
	});

	it('combines all categories into a full GCM frame', () => {
		const frame = mapCategoriesToGcm({
			necessary: true,
			functional: true,
			analytics: false,
			marketing: true,
			personalization: false
		});
		expect(frame).toEqual({
			analytics_storage: 'denied',
			ad_storage: 'granted',
			ad_user_data: 'granted',
			ad_personalization: 'granted',
			functional_storage: 'granted',
			personalization_storage: 'denied'
		});
	});

	it('treats undefined category values as denied (never crashes)', () => {
		const frame = mapCategoriesToGcm({
			analytics: undefined as unknown as boolean,
			marketing: true
		});
		expect(frame.analytics_storage).toBe('denied');
		expect(frame.ad_storage).toBe('granted');
	});
});
