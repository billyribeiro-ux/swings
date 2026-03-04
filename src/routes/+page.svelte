<script lang="ts">
	import Hero from '$lib/components/landing/Hero.svelte';
	import WhyDifferent from '$lib/components/landing/WhyDifferent.svelte';
	import WhatYouGet from '$lib/components/landing/WhatYouGet.svelte';
	import WhoItsFor from '$lib/components/landing/WhoItsFor.svelte';
	import Pricing from '$lib/components/landing/Pricing.svelte';
	import Courses from '$lib/components/landing/Courses.svelte';
	import GreeksPdfCta from '$lib/components/landing/GreeksPdfCta.svelte';
	import About from '$lib/components/landing/About.svelte';
	import FinalCta from '$lib/components/landing/FinalCta.svelte';
</script>

<svelte:head>
	<title>Explosive Swings — Early Stock Alerts You Can Actually Use</title>
	<meta
		name="description"
		content="Every Sunday night, get 5–7 top stock picks with defined entries, targets, exits, and stops. Created by Billy Ribeiro, former lead trader at Simpler Trading."
	/>
	<meta property="og:title" content="Explosive Swings — Weekly Stock Watchlist" />
	<meta
		property="og:description"
		content="Clear, actionable trade ideas delivered every Sunday night. 5–7 picks with entries, targets, exits, and stops."
	/>
	<meta property="og:type" content="website" />
	<meta name="twitter:card" content="summary_large_image" />
</svelte:head>

<Hero />
<WhyDifferent />
<WhatYouGet />
<WhoItsFor />
<Pricing />
<Courses />
<GreeksPdfCta />
<About />
<FinalCta />
