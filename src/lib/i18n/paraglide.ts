/**
 * CONSENT-06 — narrow-scope i18n shim.
 *
 * This module mirrors the `@inlang/paraglide-js` ergonomic surface (a `m.*`
 * message accessor object, a `setLocale(tag)` side-door, `getLocale()`)
 * WITHOUT pulling the full paraglide runtime at build time. The messages are
 * loaded from JSON catalogues under `messages/<locale>.json`, seeded by the
 * `project.inlang/settings.json` config in repo root.
 *
 * Scope is intentionally narrow:
 *
 *   - Consent banner + preferences copy (`consent_banner_*`, `consent_preferences_*`)
 *   - DSAR flow (`dsar_form_*`)
 *   - Unsubscribe flow (`unsubscribe_*`)
 *
 * No other user-facing strings in the app are routed through here — the
 * broader i18n rollout is a separate subsystem. When we swap to the full
 * paraglide runtime, `m.*` stays byte-for-byte compatible.
 *
 * TODO (paraglide swap):
 *   pnpm add @inlang/paraglide-js
 *   npx paraglide compile --project ./project.inlang
 *   then delete this shim and import from `./paraglide/runtime.js` +
 *   `./paraglide/messages.js` generated outputs.
 */

import en from '../../../messages/en.json';
import es from '../../../messages/es.json';
import fr from '../../../messages/fr.json';
import de from '../../../messages/de.json';
import it from '../../../messages/it.json';
import ptBR from '../../../messages/pt-BR.json';
import nl from '../../../messages/nl.json';
import sv from '../../../messages/sv.json';
import da from '../../../messages/da.json';
import nb from '../../../messages/nb.json';
import fi from '../../../messages/fi.json';
import pl from '../../../messages/pl.json';
import cs from '../../../messages/cs.json';
import ja from '../../../messages/ja.json';
import ko from '../../../messages/ko.json';
import zhHans from '../../../messages/zh-Hans.json';
import zhHant from '../../../messages/zh-Hant.json';
import ar from '../../../messages/ar.json';

export type Locale =
	| 'en'
	| 'es'
	| 'fr'
	| 'de'
	| 'it'
	| 'pt-BR'
	| 'nl'
	| 'sv'
	| 'da'
	| 'nb'
	| 'fi'
	| 'pl'
	| 'cs'
	| 'ja'
	| 'ko'
	| 'zh-Hans'
	| 'zh-Hant'
	| 'ar';

export const LOCALES: readonly Locale[] = [
	'en',
	'es',
	'fr',
	'de',
	'it',
	'pt-BR',
	'nl',
	'sv',
	'da',
	'nb',
	'fi',
	'pl',
	'cs',
	'ja',
	'ko',
	'zh-Hans',
	'zh-Hant',
	'ar'
] as const;

export const BASE_LOCALE: Locale = 'en';

type Catalogue = Readonly<Record<string, string>>;

const CATALOGUES: Readonly<Record<Locale, Catalogue>> = {
	en: en as Catalogue,
	es: es as Catalogue,
	fr: fr as Catalogue,
	de: de as Catalogue,
	it: it as Catalogue,
	'pt-BR': ptBR as Catalogue,
	nl: nl as Catalogue,
	sv: sv as Catalogue,
	da: da as Catalogue,
	nb: nb as Catalogue,
	fi: fi as Catalogue,
	pl: pl as Catalogue,
	cs: cs as Catalogue,
	ja: ja as Catalogue,
	ko: ko as Catalogue,
	'zh-Hans': zhHans as Catalogue,
	'zh-Hant': zhHant as Catalogue,
	ar: ar as Catalogue
};

let currentLocale: Locale = BASE_LOCALE;

/**
 * Try to resolve a BCP-47 tag to one of our supported locales, with
 * progressive fallback: exact match → primary language match → base locale.
 */
export function resolveLocale(tag: string | undefined | null): Locale {
	if (!tag) return BASE_LOCALE;
	const normal = tag.trim();
	if (!normal) return BASE_LOCALE;

	// Exact match (case-insensitive on the script tag).
	const exact = LOCALES.find((l) => l.toLowerCase() === normal.toLowerCase());
	if (exact) return exact;

	// `pt-BR` should match even when the client sent `pt-br`.
	const normalNoCase = normal.toLowerCase().replace(/_/g, '-');
	const lowerMatch = LOCALES.find((l) => l.toLowerCase() === normalNoCase);
	if (lowerMatch) return lowerMatch;

	// Primary subtag match (e.g. `en-GB` → `en`).
	const primary = normalNoCase.split('-')[0];
	const primaryMatch = LOCALES.find((l) => l.toLowerCase() === primary);
	if (primaryMatch) return primaryMatch;

	// Special case: anything `zh-*` that isn't Hant falls back to Hans.
	if (primary === 'zh') return 'zh-Hans';

	return BASE_LOCALE;
}

/**
 * Active locale getter. Mirrors paraglide's `getLocale()` so swap-in is a
 * mechanical rename.
 */
export function getLocale(): Locale {
	return currentLocale;
}

/**
 * Active locale setter. Mirrors paraglide's `setLocale(tag)`. Accepts any
 * BCP-47-ish string and normalises it via `resolveLocale`.
 */
export function setLocale(tag: string | Locale): Locale {
	currentLocale = resolveLocale(tag);
	return currentLocale;
}

/** Raw lookup used by `m.*` — not exported from the module's public API. */
function lookup(key: string): string | undefined {
	const cat = CATALOGUES[currentLocale];
	if (cat[key]) return cat[key];
	// Fall back to the base locale so a partial catalogue never shows a raw key.
	const base = CATALOGUES[BASE_LOCALE];
	return base[key];
}

/**
 * Interpolate `{name}` placeholders using the provided vars.
 *
 * Paraglide's real output is compiled JS rather than runtime interpolation,
 * but the shape is identical: `m.unsubscribe_body({ listName: 'foo' })`.
 */
function interpolate(template: string, vars?: Readonly<Record<string, string>>): string {
	if (!vars) return template;
	return template.replace(/\{(\w+)\}/g, (_, name: string) => {
		return name in vars ? vars[name] : `{${name}}`;
	});
}

/**
 * The `m.*` message object. Each accessor is a function — matches paraglide's
 * runtime signature so components can call `m.consent_banner_title()` without
 * conditional check. Missing keys fall through to the base catalogue;
 * missing base keys return the raw key (which is a loud dev-time failure).
 */
export const m = {
	consent_banner_title: (): string => lookup('consent_banner_title') ?? 'consent_banner_title',
	consent_banner_body: (): string => lookup('consent_banner_body') ?? 'consent_banner_body',
	consent_banner_accept_all: (): string =>
		lookup('consent_banner_accept_all') ?? 'consent_banner_accept_all',
	consent_banner_reject_all: (): string =>
		lookup('consent_banner_reject_all') ?? 'consent_banner_reject_all',
	consent_banner_customize: (): string =>
		lookup('consent_banner_customize') ?? 'consent_banner_customize',
	consent_banner_save_preferences: (): string =>
		lookup('consent_banner_save_preferences') ?? 'consent_banner_save_preferences',
	consent_banner_privacy_policy_label: (): string =>
		lookup('consent_banner_privacy_policy_label') ?? 'consent_banner_privacy_policy_label',
	consent_preferences_title: (): string =>
		lookup('consent_preferences_title') ?? 'consent_preferences_title',
	consent_preferences_description: (): string =>
		lookup('consent_preferences_description') ?? 'consent_preferences_description',
	consent_preferences_cancel: (): string =>
		lookup('consent_preferences_cancel') ?? 'consent_preferences_cancel',
	consent_preferences_required_tag: (): string =>
		lookup('consent_preferences_required_tag') ?? 'consent_preferences_required_tag',
	dsar_form_title: (): string => lookup('dsar_form_title') ?? 'dsar_form_title',
	dsar_form_description: (): string => lookup('dsar_form_description') ?? 'dsar_form_description',
	dsar_form_email_label: (): string => lookup('dsar_form_email_label') ?? 'dsar_form_email_label',
	dsar_form_request_type_label: (): string =>
		lookup('dsar_form_request_type_label') ?? 'dsar_form_request_type_label',
	dsar_form_request_type_access: (): string =>
		lookup('dsar_form_request_type_access') ?? 'dsar_form_request_type_access',
	dsar_form_request_type_delete: (): string =>
		lookup('dsar_form_request_type_delete') ?? 'dsar_form_request_type_delete',
	dsar_form_request_type_port: (): string =>
		lookup('dsar_form_request_type_port') ?? 'dsar_form_request_type_port',
	dsar_form_request_type_correct: (): string =>
		lookup('dsar_form_request_type_correct') ?? 'dsar_form_request_type_correct',
	dsar_form_submit: (): string => lookup('dsar_form_submit') ?? 'dsar_form_submit',
	dsar_form_submitted: (): string => lookup('dsar_form_submitted') ?? 'dsar_form_submitted',
	unsubscribe_title: (): string => lookup('unsubscribe_title') ?? 'unsubscribe_title',
	unsubscribe_body: (vars?: { listName: string }): string =>
		interpolate(lookup('unsubscribe_body') ?? 'unsubscribe_body', vars),
	unsubscribe_confirm: (): string => lookup('unsubscribe_confirm') ?? 'unsubscribe_confirm',
	unsubscribe_confirmed: (): string => lookup('unsubscribe_confirmed') ?? 'unsubscribe_confirmed'
} as const;

/**
 * Return the translated message for `key`, or `undefined` if no translation
 * exists in the current locale AND no fallback in the base locale. Consumers
 * (the banner / preferences components) use this to decide whether to show
 * the translation or the server-provided copy.
 */
export function translateOrFallback(key: string, fallback: string): string {
	const v = lookup(key);
	return v && v.length > 0 ? v : fallback;
}
