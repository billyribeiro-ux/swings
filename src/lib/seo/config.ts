export const SITE = {
	name: 'Explosive Swings',
	url: 'https://explosiveswings.com',
	title: 'Explosive Swings - Early Stock Alerts You Can Actually Use',
	description:
		'Every Sunday night, get 5-7 top stock picks with defined entries, targets, exits, and stops. Created by Billy Ribeiro, former lead trader at Simpler Trading.',
	locale: 'en_US',
	twitterHandle: '@explosiveswings',
	logo: 'https://explosiveswings.com/favicon.svg',
	ogImage: 'https://explosiveswings.com/og-image.png',
	foundingDate: '2022',
	priceRange: '$49–$399'
} as const;

export const FOUNDERS = {
	billy: {
		name: 'Billy Ribeiro',
		role: 'Co-Founder & Head Trader',
		url: 'https://explosiveswings.com/about',
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
		url: 'https://explosiveswings.com/about',
		image: 'https://explosiveswings.com/images/freddie-ferber.jpg',
		sameAs: [] as string[],
		description:
			'A practitioner-turned-educator who brings real-world trading clarity to traders at every level.',
		jobTitle: 'Lead Trading Educator',
		knowsAbout: ['Options Trading', 'Trading Education', 'Swing Trading', 'Technical Analysis']
	},
	shaowan: {
		name: 'Shao Wan',
		role: 'Lead Educator',
		url: 'https://explosiveswings.com/about',
		image: 'https://explosiveswings.com/images/shao-wan.jpg',
		sameAs: [] as string[],
		description:
			'A practitioner-turned-educator who brings real-world trading clarity to traders at every level.',
		jobTitle: 'Lead Trading Educator',
		knowsAbout: ['Options Trading', 'Trading Education', 'Swing Trading', 'Technical Analysis']
	}
} as const;
