<script lang="ts">
	import Hero from '$lib/components/landing/Hero.svelte';
	import WhyDifferent from '$lib/components/landing/WhyDifferent.svelte';
	import WhatYouGet from '$lib/components/landing/WhatYouGet.svelte';
	import WhoItsFor from '$lib/components/landing/WhoItsFor.svelte';
	import Pricing from '$lib/components/landing/Pricing.svelte';
	import Testimonials from '$lib/components/landing/Testimonials.svelte';
	import Courses from '$lib/components/landing/Courses.svelte';
	import GreeksPdfCta from '$lib/components/landing/GreeksPdfCta.svelte';
	import About from '$lib/components/landing/About.svelte';
	import FinalCta from '$lib/components/landing/FinalCta.svelte';
	import Seo from '$lib/seo/Seo.svelte';
	import { webPageSchema, buildJsonLd } from '$lib/seo/jsonld';

	const jsonLd = buildJsonLd([
		webPageSchema({
			path: '/',
			title: 'Explosive Swings - Weekly Options Watchlists and Trading Education',
			description:
				'Get a weekly options watchlist with clear entry zones, targets, and risk levels, plus practical courses that explain the trade thesis behind every setup.',
			speakable: '.hero__title, .hero__subtitle'
		})
	]);
</script>

<Seo
	title="Explosive Swings - Weekly Options Watchlists and Trading Education"
	description="Get a weekly options watchlist with clear entry zones, targets, and risk levels, plus practical courses that explain the trade thesis behind every setup."
	ogTitle="Explosive Swings - Weekly Options Watchlists"
	ogDescription="Trade with a documented plan each week: setup rationale, entry zones, targets, and risk controls."
	{jsonLd}
/>

<Hero />
<WhyDifferent />
<WhatYouGet />
<WhoItsFor />
<Pricing />
<Testimonials />
<Courses />
<GreeksPdfCta />
<About />
<FinalCta />
