/**
 * Consent API client.
 *
 * Stub phase: these functions return hard-coded defaults / round-trip through
 * localStorage so the UI can be developed end-to-end while CONSENT-01 is still
 * landing the backend. Every function points at its future real endpoint via a
 * `// CONSENT-01 TODO:` comment so swapping in the live implementation is a
 * single-file edit per endpoint.
 *
 * When CONSENT-01 ships, the following endpoints will supersede the stubs:
 *   - GET  /api/consent/banner  → current geo-resolved banner config
 *   - POST /api/consent/record  → write a consent event row
 *   - GET  /api/consent/me      → current subject's consent state
 *
 * The TypeScript contracts here are deliberately narrow (and locally owned) so
 * that once `schema.d.ts` exports the real `ConsentBannerConfig`,
 * `ConsentRecord`, and `ConsentState` shapes, the only change needed is to
 * switch these `interface` declarations for `type X = components['schemas']['X']`
 * re-exports. Consumers of this module import from here exclusively, so the
 * swap is transparent.
 */

// TODO CONSENT-01: once backend schemas are generated, replace these with:
//   import type { components } from '$lib/api/schema';
//   export type BannerConfig = components['schemas']['ConsentBannerConfig'];
//   ...

export type BannerLayout = 'bar' | 'box' | 'popup';
export type BannerPosition = 'top' | 'bottom' | 'center' | 'bottom-start' | 'bottom-end';

export interface ConsentCategoryDef {
	/** Stable key — NEVER change after a category has been used in production. */
	readonly key: string;
	readonly label: string;
	readonly description: string;
	/** When true, toggle is disabled and the category is always granted. */
	readonly required: boolean;
	/** Off unless user opts in; purely informational. */
	readonly defaultEnabled: boolean;
}

export interface BannerCopy {
	readonly title: string;
	readonly body: string;
	readonly acceptAll: string;
	readonly rejectAll: string;
	readonly customize: string;
	readonly savePreferences: string;
	readonly privacyPolicyHref?: string;
	readonly privacyPolicyLabel?: string;
}

export interface BannerConfig {
	readonly version: number;
	/** Policy version — bumped when the underlying privacy policy changes. */
	readonly policyVersion: number;
	readonly layout: BannerLayout;
	readonly position: BannerPosition;
	readonly categories: readonly ConsentCategoryDef[];
	readonly copy: BannerCopy;
	/** Locale tag (BCP-47). Present so CONSENT-06 can slot in translations. */
	readonly locale: string;
}

export type ConsentAction = 'granted' | 'denied' | 'updated' | 'revoked';

export interface ConsentRecordInput {
	readonly action: ConsentAction;
	readonly categories: Readonly<Record<string, boolean>>;
}

export interface ConsentRecordResponse {
	readonly id: string;
}

export interface FetchMyConsentResponse {
	readonly categories: Readonly<Record<string, boolean>>;
	readonly decidedAt: string;
}

/** Canonical default category set per AUDIT_PHASE3_PLAN.md §3. */
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

/** Default banner copy in English. CONSENT-06 will swap this for translated messages. */
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

/** Hard-coded stub config used until CONSENT-01 exposes the real endpoint. */
export const STUB_BANNER_CONFIG: BannerConfig = {
	version: 1,
	policyVersion: 1,
	layout: 'bar',
	position: 'bottom',
	categories: DEFAULT_CATEGORIES,
	copy: DEFAULT_COPY,
	locale: 'en'
};

/**
 * Fetch the current banner config.
 *
 * CONSENT-01 TODO: replace with `api.get<BannerConfig>('/api/consent/banner')`.
 * Geo-resolution happens server-side; the UI does not need to know the region.
 */
export async function fetchBannerConfig(): Promise<BannerConfig> {
	// Preserve async shape so swapping to `fetch` is mechanical.
	return Promise.resolve(STUB_BANNER_CONFIG);
}

/**
 * Record a consent event.
 *
 * CONSENT-01 TODO: replace with
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
 * CONSENT-01 TODO: replace with
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
