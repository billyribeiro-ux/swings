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
			title: 'Explosive Swings - Early Stock Alerts You Can Actually Use',
			description:
				'Every Sunday night, get 5-7 top stock picks with defined entries, targets, exits, and stops. Created by Billy Ribeiro, former lead trader at Simpler Trading.',
			speakable: '.hero__title, .hero__subtitle'
		})
	]);
</script>

<Seo
	title="Explosive Swings - Early Stock Alerts You Can Actually Use"
	description="Every Sunday night, get 5-7 top stock picks with defined entries, targets, exits, and stops. Created by Billy Ribeiro, former lead trader at Simpler Trading."
	ogTitle="Explosive Swings - Weekly Stock Watchlist"
	ogDescription="Clear, actionable trade ideas delivered every Sunday night. 5-7 picks with entries, targets, exits, and stops."
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
