/** Published USD prices (keep in sync with Stripe and admin pricing plans). */
export const PRICING_MONTHLY_USD = 49;
export const PRICING_ANNUAL_USD = 399;

const monthlyYearTotalUsd = PRICING_MONTHLY_USD * 12;

/** Savings vs paying the monthly rate for 12 months ($588 − $399 = $189). */
export const PRICING_ANNUAL_SAVINGS_USD = monthlyYearTotalUsd - PRICING_ANNUAL_USD;

/**
 * Percent off the full monthly-year total, rounded to the nearest integer.
 * (189 / 588) × 100 ≈ 32.14 → 32%.
 */
export const PRICING_ANNUAL_SAVINGS_PERCENT_ROUNDED = Math.round(
	(PRICING_ANNUAL_SAVINGS_USD / monthlyYearTotalUsd) * 100
);

export const PRICING_ANNUAL_SAVINGS_PCT_LABEL = `Save ${PRICING_ANNUAL_SAVINGS_PERCENT_ROUNDED}%`;

export interface PricingPlan {
	id: string;
	name: string;
	amount: number;
	suffix: string;
	note: string;
	cta: string;
	variant: 'outline' | 'primary';
	featured?: boolean | undefined;
	badge?: string | undefined;
	savings?: string | undefined;
}

export const pricingPlans: PricingPlan[] = [
	{
		id: 'monthly',
		name: 'Monthly',
		amount: PRICING_MONTHLY_USD,
		suffix: '/mo',
		note: 'Cancel anytime. No commitment.',
		cta: 'Start Monthly',
		variant: 'outline'
	},
	{
		id: 'annual',
		name: 'Annual',
		amount: PRICING_ANNUAL_USD,
		suffix: '/yr',
		note: 'Billed once per year.',
		cta: 'Start Annual Plan',
		variant: 'primary',
		featured: true,
		badge: 'Best Value',
		savings: PRICING_ANNUAL_SAVINGS_PCT_LABEL
	}
];
