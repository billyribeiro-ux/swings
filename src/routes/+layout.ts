import { dev } from '$app/environment';
import { injectSpeedInsights } from '@vercel/speed-insights/sveltekit';
import { injectAnalytics } from '@vercel/analytics/sveltekit';

// Vercel Speed Insights (client-only); enable in project → Speed Insights. Latest: @vercel/speed-insights@2.0.0.
injectSpeedInsights();

// Vercel Web Analytics (client-only); enable in project → Analytics. Latest: @vercel/analytics@2.0.1.
injectAnalytics({ mode: dev ? 'development' : 'production' });

export const prerender = true;
export const trailingSlash = 'never';
