export interface Course {
	id: string;
	slug: string;
	title: string;
	level: 'Beginner' | 'Intermediate' | 'Advanced';
	description: string;
	meta: string;
	icon: string;
	gradient: { from: string; to: string };
	price: number;
	duration: string;
	modules: number;
	features: string[];
	curriculum: { title: string; lessons: string[] }[];
	whatYouLearn: string[];
}

export const courses: Course[] = [
	{
		id: 'beginning-options',
		slug: 'beginning-options-trading',
		title: 'Beginning to Options Trading',
		level: 'Beginner',
		description:
			'Start from scratch. Learn what options are, how they work, and how to place your first trades with confidence -- no prior experience needed.',
		meta: 'Self-Paced | All Levels',
		icon: 'BookOpen',
		gradient: { from: '#0B1D3A', to: '#1A3A6B' },
		price: 197,
		duration: '4 weeks',
		modules: 6,
		features: [
			'Lifetime access to all course materials',
			'Video lessons with real chart examples',
			'Downloadable cheat sheets and guides',
			'Private community access',
			'Certificate of completion'
		],
		whatYouLearn: [
			'What options are and how they work',
			'The difference between calls and puts',
			'How to read an options chain',
			'Basic options terminology and Greeks',
			'How to place your first options trade',
			'Risk management fundamentals'
		],
		curriculum: [
			{
				title: 'Module 1: Options Basics',
				lessons: [
					'What Are Options?',
					'Calls vs Puts Explained',
					'Options Terminology You Need to Know',
					'How Options Pricing Works'
				]
			},
			{
				title: 'Module 2: Reading the Options Chain',
				lessons: [
					'Understanding Strike Prices',
					'Expiration Dates and Time Decay',
					'Bid/Ask Spreads',
					'Volume and Open Interest'
				]
			},
			{
				title: 'Module 3: Your First Trade',
				lessons: [
					'Choosing the Right Strike and Expiration',
					'Placing a Call Option Trade',
					'Placing a Put Option Trade',
					'Managing Your Position'
				]
			},
			{
				title: 'Module 4: Risk Management',
				lessons: [
					'Position Sizing Basics',
					'When to Take Profit',
					'When to Cut Losses',
					'Common Beginner Mistakes to Avoid'
				]
			}
		]
	},
	{
		id: 'options-101',
		slug: 'options-trading-101',
		title: 'Options Trading 101',
		level: 'Intermediate',
		description:
			'Go deeper into calls, puts, spreads, and real strategies. Learn how to read the options chain, manage risk, and build consistent setups that work.',
		meta: 'Self-Paced | Intermediate',
		icon: 'Pulse',
		gradient: { from: '#1A3A6B', to: '#0FA4AF' },
		price: 297,
		duration: '6 weeks',
		modules: 8,
		features: [
			'Lifetime access to all course materials',
			'Advanced video lessons with live trade examples',
			'Strategy templates and scanners',
			'Private community access',
			'Weekly live Q&A sessions',
			'Certificate of completion'
		],
		whatYouLearn: [
			'Advanced options strategies (spreads, iron condors)',
			'How to use the Greeks for better entries',
			'Building a consistent trading system',
			'Advanced risk management techniques',
			'How to scan for high-probability setups',
			'Position management and scaling'
		],
		curriculum: [
			{
				title: 'Module 1: Advanced Options Strategies',
				lessons: [
					'Vertical Spreads (Bull & Bear)',
					'Iron Condors and Credit Spreads',
					'Calendar Spreads',
					'Diagonal Spreads'
				]
			},
			{
				title: 'Module 2: The Greeks Deep Dive',
				lessons: [
					'Delta and Directional Bias',
					'Theta and Time Decay Strategies',
					'Vega and Volatility Trading',
					'Gamma and Position Acceleration'
				]
			},
			{
				title: 'Module 3: Building Your System',
				lessons: [
					'Defining Your Trading Style',
					'Creating a Watchlist Process',
					'Entry and Exit Rules',
					'Journaling and Review'
				]
			},
			{
				title: 'Module 4: Advanced Risk Management',
				lessons: [
					'Portfolio Heat and Position Sizing',
					'Hedging Strategies',
					'Adjusting Losing Trades',
					'When to Walk Away'
				]
			},
			{
				title: 'Module 5: Scanning and Setup Selection',
				lessons: [
					'Technical Setups for Options',
					'Using Volume and Price Action',
					'Earnings and Event Trades',
					'Building Your Scan Criteria'
				]
			}
		]
	}
];
