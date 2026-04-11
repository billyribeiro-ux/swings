<script lang="ts">
	import { page } from '$app/state';
	import { SITE } from './config';

	interface Props {
		title?: string;
		description?: string;
		ogTitle?: string;
		ogDescription?: string;
		ogImage?: string;
		ogType?: string;
		noindex?: boolean;
		canonical?: string;
		jsonLd?: string;
	}

	let {
		title = SITE.title,
		description = SITE.description,
		ogTitle,
		ogDescription,
		ogImage = SITE.ogImage,
		ogType = 'website',
		noindex = false,
		canonical,
		jsonLd
	}: Props = $props();

	const resolvedCanonical = $derived(canonical || `${SITE.url}${page.url.pathname}`);
	const resolvedOgTitle = $derived(ogTitle || title);
	const resolvedOgDesc = $derived(ogDescription || description);
</script>

<svelte:head>
	<title>{title}</title>
	<meta name="description" content={description} />
	<link rel="canonical" href={resolvedCanonical} />

	{#if noindex}
		<meta name="robots" content="noindex, nofollow" />
	{/if}

	<!-- Open Graph -->
	<meta property="og:title" content={resolvedOgTitle} />
	<meta property="og:description" content={resolvedOgDesc} />
	<meta property="og:type" content={ogType} />
	<meta property="og:url" content={resolvedCanonical} />
	<meta property="og:image" content={ogImage} />
	<meta property="og:site_name" content={SITE.name} />
	<meta property="og:locale" content={SITE.locale} />

	<!-- Twitter Card -->
	<meta name="twitter:card" content="summary_large_image" />
	<meta name="twitter:site" content={SITE.twitterHandle} />
	<meta name="twitter:title" content={resolvedOgTitle} />
	<meta name="twitter:description" content={resolvedOgDesc} />
	<meta name="twitter:image" content={ogImage} />

	{#if jsonLd}
		<!-- buildJsonLd() escapes `<` to `\u003c` so the inner JSON cannot break out -->
		{@html '<script type="application/ld+json">' + jsonLd + '<' + '/script>'}
	{/if}
</svelte:head>
