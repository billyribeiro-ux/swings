import { api } from '$lib/api/client';
import type { PricingPlan } from '$lib/api/types';

let activePlansPromise: Promise<PricingPlan[]> | null = null;

export async function getActivePricingPlans(forceRefresh = false): Promise<PricingPlan[]> {
	if (!activePlansPromise || forceRefresh) {
		activePlansPromise = api.get<PricingPlan[]>('/api/pricing/plans', { skipAuth: true });
	}
	return activePlansPromise;
}

export async function getPricingPlanBySlug(slug: string): Promise<PricingPlan | null> {
	const plans = await getActivePricingPlans();
	return plans.find((plan) => plan.slug === slug) ?? null;
}
