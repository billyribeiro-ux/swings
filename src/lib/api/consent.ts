/**
 * Consent API client.
 *
 * CONSENT-01 status: banner config endpoint is live. The category set is
 * sourced from the server response — the `DEFAULT_CATEGORIES` constant below
 * is retained only as an SSR/test fallback (fixtures, storybook-style
 * harnesses, and the admin preview route). The real endpoint always returns
 * an authoritative list, so consumers should prefer the server response.
 *
 * CONSENT-03 status: real endpoints wired.
 *   - `recordConsent`     → POST /api/consent/record  (public, IP rate-limited)
 *   - `fetchMyConsent`    → GET  /api/consent/me      (authed; falls back to
 *                                                      the localStorage envelope
 *                                                      for anonymous sessions
 *                                                      or when the API returns
 *                                                      401)
 *   - `submitDsarRequest` → POST /api/dsar            (public)
 *
 * The TypeScript contracts below alias the generated OpenAPI shapes. When
 * CONSENT-03 lands its own schemas the remaining local interfaces become
 * aliases the same way `BannerConfig` already is.
 */

import { api, ApiError } from '$lib/api/client';
import type { components } from '$lib/api/schema';

/** Generated OpenAPI shape — single source of truth for the banner payload. */
export type BannerConfig = components['schemas']['BannerConfig'];
export type BannerCopy = components['schemas']['BannerCopy'];
export type ConsentCategoryDef = components['schemas']['ConsentCategoryDef'];

export type BannerLayout = 'bar' | 'box' | 'popup' | 'fullscreen';
export type BannerPosition = 'top' | 'bottom' | 'center' | 'bottom-start' | 'bottom-end';

/**
 * The server accepts the full action enum from the CONSENT-03 migration.
 * The UI currently emits only the four "decision" actions; `expired` and
 * `prefill` land in CONSENT-04 when session expiry + GPC prefill ship.
 */
export type ConsentAction = 'granted' | 'denied' | 'updated' | 'revoked' | 'expired' | 'prefill';

export type DsarKind = 'access' | 'delete' | 'portability' | 'rectification' | 'opt_out_sale';

export type ConsentRecordResponse = components['schemas']['ConsentRecordResponse'];
export type MyConsentResponse = components['schemas']['MyConsentResponse'];
export type DsarSubmitResponse = components['schemas']['DsarSubmitResponse'];
export type DsarRow = components['schemas']['DsarRow'];

/**
 * Local alias so existing callers of `fetchMyConsent` keep their old shape
 * (they only consume `categories` + `decidedAt`). When every call site
 * migrates to `MyConsentResponse` this interface can go away.
 */
export interface FetchMyConsentResponse {
	readonly categories: Readonly<Record<string, boolean>>;
	readonly decidedAt: string;
}

/**
 * Canonical default category set. Used as an SSR/offline fallback so
 * the banner can render before (or without) the network round-trip, and by
 * `ConsentStore` as the seed shape for its preference record.
 *
 * Keep in sync with the seed in `backend/migrations/024_consent.sql` — the
 * test in `backend/src/handlers/consent.rs::seed_copy_json_parses` is the
 * backstop if they drift.
 */
export const DEFAULT_CATEGORIES: readonly ConsentCategoryDef[] = [
	{
		key: 'necessary',
		label: 'Strictly necessary',
		description:
			'Essential for the site to function — authentication, fraud prevention, load balancing, and your preferences. Cannot be turned off.',
		required: true,
		defaultEnabled: true
	},
	{
		key: 'functional',
		label: 'Functional',
		description:
			'Remembers choices you make (language, region, saved filters) to give you a better experience.',
		required: false,
		defaultEnabled: false
	},
	{
		key: 'analytics',
		label: 'Analytics',
		description:
			'Helps us understand how visitors use the site so we can improve it. All data is aggregated and never sold.',
		required: false,
		defaultEnabled: false
	},
	{
		key: 'marketing',
		label: 'Marketing',
		description:
			'Lets us measure campaign performance and show you relevant offers on other sites.',
		required: false,
		defaultEnabled: false
	},
	{
		key: 'personalization',
		label: 'Personalization',
		description:
			'Tailors what you see — recommendations, homepage layout, trader highlights — to your interests.',
		required: false,
		defaultEnabled: false
	}
];

/** SSR/offline fallback copy. Real copy flows from the server response. */
export const DEFAULT_COPY: BannerCopy = {
	title: 'We value your privacy',
	body: 'We use cookies and similar technologies to power the site, understand usage, and — with your permission — personalize content. You can accept everything, reject non-essential categories, or choose exactly what to allow.',
	acceptAll: 'Accept all',
	rejectAll: 'Reject all',
	customize: 'Customize',
	savePreferences: 'Save preferences',
	privacyPolicyHref: '/privacy',
	privacyPolicyLabel: 'Privacy policy'
};

/** SSR/offline fallback config. Real config flows from `/api/consent/banner`. */
export const STUB_BANNER_CONFIG: BannerConfig = {
	version: 1,
	policyVersion: 1,
	layout: 'bar',
	position: 'bottom',
	locale: 'en',
	region: 'default',
	categories: [...DEFAULT_CATEGORIES],
	copy: DEFAULT_COPY,
	theme: {}
};

/**
 * Fetch the current banner config from `/api/consent/banner`.
 *
 * Returns `STUB_BANNER_CONFIG` when the endpoint is unreachable (network
 * error, 5xx) so the UI degrades to the default experience rather than
 * failing closed. Logs the underlying error at debug-level so that
 * observability still picks it up without polluting the console in the
 * common-case dev flow where the backend is offline.
 *
 * `locale` accepts any BCP-47 tag. When omitted, the caller's
 * `navigator.language` is used so the server can pick a matching banner
 * locale without a second round-trip.
 *
 * `region` is normally NOT passed by the client — the server resolves the
 * regulatory bucket from request headers (CF-IPCountry etc.). The override
 * exists for admin-preview flows where the admin wants to see the EU
 * banner without spoofing headers.
 */
export async function fetchBannerConfig(locale?: string, region?: string): Promise<BannerConfig> {
	const params = new URLSearchParams();
	const resolvedLocale =
		locale ??
		(typeof navigator !== 'undefined' && navigator.language ? navigator.language : undefined);
	if (resolvedLocale) params.set('locale', resolvedLocale);
	if (region) params.set('region', region);
	const qs = params.toString() ? `?${params.toString()}` : '';
	try {
		return await api.get<BannerConfig>(`/api/consent/banner${qs}`, { skipAuth: true });
	} catch (err) {
		if (typeof console !== 'undefined') {
			const reason = err instanceof ApiError ? `HTTP ${err.status}` : String(err);
			console.debug('[consent] fetchBannerConfig fell back to stub:', reason);
		}
		return STUB_BANNER_CONFIG;
	}
}

/**
 * Anonymous-id cookie used for linking consent decisions across sessions
 * before the subject authenticates. 1y SameSite=Lax per ICO guidance —
 * "strictly necessary" treatment so the banner isn't gated on itself.
 */
const ANON_ID_KEY = 'swings_consent_anon_v1';

function readOrCreateAnonId(): string | undefined {
	if (typeof window === 'undefined') return undefined;
	try {
		const existing = window.localStorage.getItem(ANON_ID_KEY);
		if (existing && /^[0-9a-fA-F-]{32,36}$/.test(existing)) return existing;
		const fresh = generateUuid();
		window.localStorage.setItem(ANON_ID_KEY, fresh);
		return fresh;
	} catch {
		return undefined;
	}
}

/**
 * Record a consent event.
 *
 * The server enriches the row with `subject_id` (from the JWT if present),
 * `ip_hash`, `user_agent`, `country`, `banner_version`, and `policy_version`.
 * The client supplies the categories payload, any per-service overrides, the
 * GPC signal (if `navigator.globalPrivacyControl === true`), and an
 * anonymous-id UUID for unauthenticated subjects.
 */
export async function recordConsent(
	action: ConsentAction,
	categories: Readonly<Record<string, boolean>>,
	extras?: {
		readonly services?: Readonly<Record<string, boolean>>;
		readonly tcfString?: string;
		readonly gpcSignal?: boolean;
		readonly bannerVersion?: number;
		readonly policyVersion?: number;
	}
): Promise<ConsentRecordResponse> {
	const anonymousId = readOrCreateAnonId();
	const body: Record<string, unknown> = {
		action,
		categories
	};
	if (extras?.services) body.services = extras.services;
	if (extras?.tcfString) body.tcfString = extras.tcfString;
	if (typeof extras?.gpcSignal === 'boolean') body.gpcSignal = extras.gpcSignal;
	if (typeof extras?.bannerVersion === 'number') body.bannerVersion = extras.bannerVersion;
	if (typeof extras?.policyVersion === 'number') body.policyVersion = extras.policyVersion;
	if (anonymousId) body.anonymousId = anonymousId;
	return api.post<ConsentRecordResponse>('/api/consent/record', body, { skipAuth: false });
}

/**
 * Fetch the current subject's consent state.
 *
 * Authenticated users: calls `GET /api/consent/me` and returns the newest
 * row's `categories` + `decidedAt`. Anonymous users (401 response) fall back
 * to the localStorage envelope persisted by the consent store so the UI
 * still sees a consistent view.
 */
export async function fetchMyConsent(): Promise<FetchMyConsentResponse | null> {
	try {
		const resp = await api.get<MyConsentResponse>('/api/consent/me');
		if (!resp.decidedAt) return readLocalConsent();
		return {
			categories: (resp.categories ?? {}) as Record<string, boolean>,
			decidedAt: resp.decidedAt
		};
	} catch (err) {
		if (err instanceof ApiError && err.status === 401) {
			return readLocalConsent();
		}
		if (typeof console !== 'undefined') {
			const reason = err instanceof ApiError ? `HTTP ${err.status}` : String(err);
			console.debug('[consent] fetchMyConsent fell back to localStorage:', reason);
		}
		return readLocalConsent();
	}
}

/**
 * Submit a Data Subject Access Request.
 *
 * Returns the DSAR id the backend minted. The subject will receive a
 * verification e-mail; actual fulfilment is an admin-side workflow.
 */
export async function submitDsarRequest(
	email: string,
	kind: DsarKind,
	payload?: Readonly<Record<string, unknown>>
): Promise<DsarSubmitResponse> {
	return api.post<DsarSubmitResponse>('/api/dsar', { email, kind, payload }, { skipAuth: true });
}

function readLocalConsent(): FetchMyConsentResponse | null {
	if (typeof window === 'undefined') return null;
	try {
		const raw = window.localStorage.getItem('swings_consent_v1');
		if (!raw) return null;
		const parsed = JSON.parse(raw) as unknown;
		if (!parsed || typeof parsed !== 'object') return null;
		const envelope = parsed as Partial<{
			categories: Record<string, boolean>;
			decidedAt: string;
		}>;
		if (!envelope.categories || !envelope.decidedAt) return null;
		return { categories: envelope.categories, decidedAt: envelope.decidedAt };
	} catch {
		return null;
	}
}

function generateUuid(): string {
	if (typeof crypto !== 'undefined' && typeof crypto.randomUUID === 'function') {
		return crypto.randomUUID();
	}
	return `consent_${Math.random().toString(36).slice(2)}_${Date.now().toString(36)}`;
}
