import { SITE, FOUNDERS } from './config';

export function organizationSchema() {
	return {
		'@type': 'Organization',
		'@id': `${SITE.url}/#organization`,
		name: SITE.name,
		url: SITE.url,
		logo: {
			'@type': 'ImageObject',
			url: SITE.logo,
			width: 512,
			height: 512
		},
		foundingDate: SITE.foundingDate,
		description: SITE.description,
		sameAs: ['https://twitter.com/explosiveswings', 'https://www.youtube.com/@explosiveswings'],
		founder: [
			{
				'@type': 'Person',
				'@id': `${SITE.url}/#billy-ribeiro`,
				name: FOUNDERS.billy.name,
				jobTitle: FOUNDERS.billy.jobTitle,
				description: FOUNDERS.billy.description,
				url: FOUNDERS.billy.url,
				sameAs: FOUNDERS.billy.sameAs,
				knowsAbout: FOUNDERS.billy.knowsAbout
			},
			{
				'@type': 'Person',
				'@id': `${SITE.url}/#freddie-ferber`,
				name: FOUNDERS.freddie.name,
				jobTitle: FOUNDERS.freddie.jobTitle,
				description: FOUNDERS.freddie.description,
				url: FOUNDERS.freddie.url,
				image: FOUNDERS.freddie.image,
				knowsAbout: FOUNDERS.freddie.knowsAbout
			}
		],
		contactPoint: {
			'@type': 'ContactPoint',
			contactType: 'customer service',
			url: `${SITE.url}/about`
		}
	};
}

export function webSiteSchema() {
	return {
		'@type': 'WebSite',
		'@id': `${SITE.url}/#website`,
		url: SITE.url,
		name: SITE.name,
		description: SITE.description,
		publisher: { '@id': `${SITE.url}/#organization` },
		inLanguage: 'en-US'
	};
}

export function webPageSchema(opts: {
	path: string;
	title: string;
	description: string;
	datePublished?: string;
	dateModified?: string;
	speakable?: string;
}) {
	const page: Record<string, unknown> = {
		'@type': 'WebPage',
		'@id': `${SITE.url}${opts.path}/#webpage`,
		url: `${SITE.url}${opts.path}`,
		name: opts.title,
		description: opts.description,
		isPartOf: { '@id': `${SITE.url}/#website` },
		about: { '@id': `${SITE.url}/#organization` },
		inLanguage: 'en-US'
	};

	if (opts.datePublished) page.datePublished = opts.datePublished;
	if (opts.dateModified) page.dateModified = opts.dateModified;

	if (opts.speakable) {
		page.speakable = {
			'@type': 'SpeakableSpecification',
			cssSelector: opts.speakable
		};
	}

	return page;
}

export function courseSchema(opts: {
	name: string;
	description: string;
	slug: string;
	level: string;
	duration: string;
	modules: number;
}) {
	return {
		'@type': 'Course',
		'@id': `${SITE.url}/courses/${opts.slug}/#course`,
		name: opts.name,
		description: opts.description,
		url: `${SITE.url}/courses/${opts.slug}`,
		provider: { '@id': `${SITE.url}/#organization` },
		educationalLevel: opts.level,
		timeRequired: opts.duration,
		numberOfCredits: opts.modules,
		hasCourseInstance: {
			'@type': 'CourseInstance',
			courseMode: 'online',
			courseWorkload: opts.duration
		},
		inLanguage: 'en-US'
	};
}

export function articleSchema(opts: {
	title: string;
	description: string;
	path: string;
	datePublished: string;
	dateModified?: string;
	authorName?: string;
	image?: string;
}) {
	return {
		'@type': 'BlogPosting',
		'@id': `${SITE.url}${opts.path}/#article`,
		headline: opts.title,
		description: opts.description,
		url: `${SITE.url}${opts.path}`,
		datePublished: opts.datePublished,
		dateModified: opts.dateModified || opts.datePublished,
		author: {
			'@type': 'Person',
			'@id': `${SITE.url}/#billy-ribeiro`,
			name: opts.authorName || FOUNDERS.billy.name
		},
		publisher: { '@id': `${SITE.url}/#organization` },
		isPartOf: { '@id': `${SITE.url}/#website` },
		inLanguage: 'en-US',
		...(opts.image ? { image: opts.image } : {})
	};
}

export function productSchema(opts: {
	name: string;
	description: string;
	price: string;
	priceCurrency?: string;
	path: string;
	billingPeriod: string;
}) {
	return {
		'@type': 'Product',
		name: opts.name,
		description: opts.description,
		url: `${SITE.url}${opts.path}`,
		brand: { '@id': `${SITE.url}/#organization` },
		offers: {
			'@type': 'Offer',
			price: opts.price,
			priceCurrency: opts.priceCurrency || 'USD',
			availability: 'https://schema.org/InStock',
			priceValidUntil: new Date(Date.now() + 365 * 24 * 60 * 60 * 1000).toISOString().split('T')[0],
			url: `${SITE.url}${opts.path}`
		}
	};
}

export function buildJsonLd(items: Record<string, unknown>[]) {
	// Escape `</` to `<\/` so the serialized JSON cannot break out of the
	// surrounding inline `<script type="application/ld+json">` tag if any field
	// ever contains the literal substring `</script>`.
	return JSON.stringify({
		'@context': 'https://schema.org',
		'@graph': items
	}).replace(/</g, '\\u003c');
}
