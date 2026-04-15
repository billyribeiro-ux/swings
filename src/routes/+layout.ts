import { injectSpeedInsights } from '@vercel/speed-insights/sveltekit';

// Vercel Speed Insights (client-only); enable in project → Speed Insights. Latest: @vercel/speed-insights@2.0.0.
injectSpeedInsights();

export const prerender = true;
export const trailingSlash = 'never';
