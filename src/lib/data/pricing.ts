export interface PricingPlan {
  id: string;
  name: string;
  amount: number;
  suffix: string;
  note: string;
  cta: string;
  variant: 'outline' | 'primary';
  featured?: boolean;
  badge?: string;
  savings?: string;
}

export const pricingPlans: PricingPlan[] = [
  {
    id: 'monthly',
    name: 'Monthly',
    amount: 49,
    suffix: '/mo',
    note: 'Cancel anytime. No commitment.',
    cta: 'Start Monthly',
    variant: 'outline',
  },
  {
    id: 'annual',
    name: 'Annual',
    amount: 399,
    suffix: '/yr',
    note: 'Billed once per year.',
    cta: 'Start Annual Plan',
    variant: 'primary',
    featured: true,
    badge: 'Best Value',
    savings: 'Save 32%',
  },
];
