/**
 * CONSENT-06 — paraglide shim unit tests.
 *
 * Covers locale resolution, base-locale fallback, and the
 * `translateOrFallback` override semantics the consent banner depends on.
 */
import { afterEach, describe, expect, it } from 'vitest';
import {
	BASE_LOCALE,
	LOCALES,
	getLocale,
	m,
	resolveLocale,
	setLocale,
	translateOrFallback
} from './paraglide';

describe('resolveLocale', () => {
	afterEach(() => {
		setLocale(BASE_LOCALE);
	});

	it('returns base locale for nullish input', () => {
		expect(resolveLocale(null)).toBe('en');
		expect(resolveLocale(undefined)).toBe('en');
		expect(resolveLocale('')).toBe('en');
	});

	it('exact-matches a listed locale', () => {
		expect(resolveLocale('fr')).toBe('fr');
		expect(resolveLocale('pt-BR')).toBe('pt-BR');
	});

	it('normalises case + underscore to dash', () => {
		expect(resolveLocale('PT-br')).toBe('pt-BR');
		expect(resolveLocale('zh_hans')).toBe('zh-Hans');
	});

	it('falls back to primary subtag when the region is unsupported', () => {
		expect(resolveLocale('en-GB')).toBe('en');
		expect(resolveLocale('de-AT')).toBe('de');
	});

	it('routes unknown zh variants to Simplified', () => {
		expect(resolveLocale('zh-CN')).toBe('zh-Hans');
	});

	it('falls back to base locale for unknown languages', () => {
		expect(resolveLocale('xx')).toBe('en');
		expect(resolveLocale('wookieepedia-basic')).toBe('en');
	});
});

describe('getLocale / setLocale', () => {
	afterEach(() => {
		setLocale(BASE_LOCALE);
	});

	it('round-trips the active locale', () => {
		expect(getLocale()).toBe('en');
		setLocale('fr');
		expect(getLocale()).toBe('fr');
	});

	it('normalises tags via resolveLocale', () => {
		setLocale('PT-br');
		expect(getLocale()).toBe('pt-BR');
	});
});

describe('m.* accessors', () => {
	afterEach(() => {
		setLocale(BASE_LOCALE);
	});

	it('serves the French title after setLocale(fr)', () => {
		setLocale('fr');
		expect(m.consent_banner_title()).toMatch(/vie privée/i);
	});

	it('serves the Japanese title after setLocale(ja)', () => {
		setLocale('ja');
		expect(m.consent_banner_title()).toContain('プライバシー');
	});

	it('falls back to English when a key is missing in a partial catalogue', () => {
		setLocale('sv');
		// sv.json deliberately omits consent_banner_body.
		expect(m.consent_banner_body()).toBe(m.consent_banner_body.call(null));
		// Loop through base locale + sv to ensure fallback works — the body
		// string must match the English one.
		const swedish = m.consent_banner_body();
		setLocale('en');
		const english = m.consent_banner_body();
		expect(swedish).toBe(english);
	});

	it('interpolates unsubscribe_body vars', () => {
		setLocale('en');
		const out = m.unsubscribe_body({ listName: 'Swings Daily' });
		expect(out).toContain('Swings Daily');
	});
});

describe('translateOrFallback', () => {
	afterEach(() => {
		setLocale(BASE_LOCALE);
	});

	it('returns the translation when present', () => {
		setLocale('fr');
		const out = translateOrFallback('consent_banner_accept_all', 'fallback');
		expect(out).toBe('Tout accepter');
	});

	it('returns the fallback when the key is missing everywhere', () => {
		expect(translateOrFallback('this_key_does_not_exist_anywhere', 'server copy')).toBe(
			'server copy'
		);
	});

	it('falls back to the base-locale entry when a partial catalogue is missing the key', () => {
		setLocale('sv');
		// sv.json has consent_banner_title but not consent_banner_body.
		// The catalogue's base-en fallback is the result — fallback arg is
		// only used if EVERY catalogue misses the key.
		expect(translateOrFallback('consent_banner_body', 'server copy')).toMatch(
			/power the site/i
		);
	});
});

describe('LOCALES catalogue', () => {
	it('includes 18 locales', () => {
		expect(LOCALES).toHaveLength(18);
	});

	it('starts with en as base locale', () => {
		expect(LOCALES[0]).toBe('en');
		expect(BASE_LOCALE).toBe('en');
	});
});
