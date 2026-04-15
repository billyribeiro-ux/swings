const SITE_ORIGIN = 'https://precisionoptionsignals.com';

export const SITE = {
	name: 'Precision Options Signals',
	url: SITE_ORIGIN,
	title: 'Precision Options Signals — Weekly Options Watchlists & Trading Education',
	description:
		'Every Sunday evening (ET), get 5–7 options-ready stock setups with defined entries, targets, exits, and stops—plus courses that explain the thesis behind each plan.',
	locale: 'en_US',
	/** Set when you have a handle; omitted from meta when empty. */
	twitterHandle: '' as string,
	logo: `${SITE_ORIGIN}/favicon.svg`,
	ogImage: `${SITE_ORIGIN}/og-image.png`,
	foundingDate: '2022',
	priceRange: '$49–$399',
	supportEmail: 'support@precisionoptionsignals.com',
	/** Nav / footer wordmark (two-line lockup). */
	logoBrandPrimary: 'Precision',
	logoBrandAccent: 'Options Signals',
	/** Organization `sameAs` profile URLs; add real links when available. */
	sameAs: [] as readonly string[]
} as const;

export const FOUNDERS = {
	billy: {
		name: 'Billy Ribeiro',
		role: 'Co-Founder & Head Trader',
		url: `${SITE_ORIGIN}/about`,
		sameAs: ['https://twitter.com/billyribeiro', 'https://www.linkedin.com/in/billyribeiro'],
		description:
			"Over a decade of institutional-grade market experience, including head trader at ZMC Capital. Mentored by Mark McGoldrick, Goldman Sachs' former Global Head of Proprietary Trading.",
		jobTitle: 'Head Trader & Options Strategist',
		knowsAbout: [
			'Options Trading',
			'Swing Trading',
			'Technical Analysis',
			'Risk Management',
			'Stock Market Analysis'
		]
	},
	freddie: {
		name: 'Freddie Ferber',
		role: 'Co-Founder & Lead Educator',
		url: `${SITE_ORIGIN}/about`,
		image: `${SITE_ORIGIN}/images/freddie-ferber.jpg`,
		sameAs: [] as string[],
		description:
			'A practitioner-turned-educator who brings real-world trading clarity to traders at every level.',
		jobTitle: 'Lead Trading Educator',
		knowsAbout: ['Options Trading', 'Trading Education', 'Swing Trading', 'Technical Analysis']
	},
	shaowan: {
		name: 'Shao Wan',
		role: 'Lead Educator',
		url: `${SITE_ORIGIN}/about`,
		image: `${SITE_ORIGIN}/images/shao-wan.jpg`,
		sameAs: [] as string[],
		description:
			'A practitioner-turned-educator who brings real-world trading clarity to traders at every level.',
		jobTitle: 'Lead Trading Educator',
		knowsAbout: ['Options Trading', 'Trading Education', 'Swing Trading', 'Technical Analysis']
	}
} as const;
