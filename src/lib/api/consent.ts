/**
 * Consent API client.
 *
 * CONSENT-01 status: banner config endpoint is live. The category set is
 * sourced from the server response — the `DEFAULT_CATEGORIES` constant below
 * is retained only as an SSR/test fallback (fixtures, storybook-style
 * harnesses, and the admin preview route). The real endpoint always returns
 * an authoritative list, so consumers should prefer the server response.
 *
 * CONSENT-03 stubs (still localStorage-backed):
 *   - `recordConsent` — will write to `POST /api/consent/record`
 *   - `fetchMyConsent` — will read from `GET /api/consent/me`
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

export type ConsentAction = 'granted' | 'denied' | 'updated' | 'revoked';

export interface ConsentRecordResponse {
	readonly id: string;
}

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
 * `locale` accepts any BCP-47 tag; the server normalises to its primary
 * subtag until CONSENT-06 lands the translation catalogues.
 */
export async function fetchBannerConfig(locale?: string): Promise<BannerConfig> {
	const qs = locale ? `?locale=${encodeURIComponent(locale)}` : '';
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
 * Record a consent event.
 *
 * CONSENT-03 TODO: replace with
 *   `api.post<ConsentRecordResponse>('/api/consent/record', { action, categories })`.
 * The backend will enrich the row with subject_id / anonymous_id, IP hash,
 * user-agent, banner_version, policy_version, and — critically — the GPC
 * signal bit for regulatory audit. Never re-derive those fields on the client.
 */
export async function recordConsent(
	action: ConsentAction,
	categories: Readonly<Record<string, boolean>>
): Promise<ConsentRecordResponse> {
	const id = generateUuid();
	if (typeof console !== 'undefined') {
		console.debug('[consent:stub] recordConsent', { id, action, categories });
	}
	return Promise.resolve({ id });
}

/**
 * Fetch the current subject's consent state.
 *
 * CONSENT-03 TODO: replace with
 *   `api.get<FetchMyConsentResponse>('/api/consent/me')`.
 * The stub reads from the same localStorage envelope the store persists to
 * so the rest of the app sees a consistent view during development.
 */
export async function fetchMyConsent(): Promise<FetchMyConsentResponse | null> {
	if (typeof window === 'undefined') return Promise.resolve(null);
	try {
		const raw = window.localStorage.getItem('swings_consent_v1');
		if (!raw) return Promise.resolve(null);
		const parsed = JSON.parse(raw) as unknown;
		if (!parsed || typeof parsed !== 'object') return Promise.resolve(null);
		const envelope = parsed as Partial<{ categories: Record<string, boolean>; decidedAt: string }>;
		if (!envelope.categories || !envelope.decidedAt) return Promise.resolve(null);
		return Promise.resolve({
			categories: envelope.categories,
			decidedAt: envelope.decidedAt
		});
	} catch {
		return Promise.resolve(null);
	}
}

function generateUuid(): string {
	if (typeof crypto !== 'undefined' && typeof crypto.randomUUID === 'function') {
		return crypto.randomUUID();
	}
	return `consent_${Math.random().toString(36).slice(2)}_${Date.now().toString(36)}`;
}
