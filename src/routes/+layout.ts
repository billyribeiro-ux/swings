import { dev } from '$app/environment';
import { injectSpeedInsights } from '@vercel/speed-insights/sveltekit';
import { injectAnalytics } from '@vercel/analytics/sveltekit';

const vercelObservabilityInDev =
	import.meta.env.PUBLIC_VERCEL_OBSERVABILITY_IN_DEV === '1' ||
	import.meta.env.PUBLIC_VERCEL_OBSERVABILITY_IN_DEV === 'true';

// Vercel injects scripts that call `history.pushState`; SvelteKit patches that API in
// dev and warns once per session. Skip locally unless explicitly enabled.
if (!dev || vercelObservabilityInDev) {
	// Latest: @vercel/speed-insights@2.0.0, @vercel/analytics@2.0.1
	injectSpeedInsights();
	injectAnalytics({ mode: dev ? 'development' : 'production' });
}

export const prerender = true;
export const trailingSlash = 'never';
