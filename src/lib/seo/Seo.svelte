<script lang="ts">
	import { page } from '$app/state';
	import { SITE } from './config';

	interface Props {
		title?: string | undefined;
		description?: string | undefined;
		ogTitle?: string | undefined;
		ogDescription?: string | undefined;
		ogImage?: string | undefined;
		ogType?: string | undefined;
		noindex?: boolean | undefined;
		canonical?: string | undefined;
		jsonLd?: string | undefined;
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

	function absolutizeUrl(url: string): string {
		try {
			return new URL(url, SITE.url).toString();
		} catch {
			return SITE.url;
		}
	}

	const resolvedCanonical = $derived(
		canonical ? absolutizeUrl(canonical) : `${SITE.url}${page.url.pathname}`
	);
	const resolvedOgImage = $derived(absolutizeUrl(ogImage));
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
	<meta property="og:image" content={resolvedOgImage} />
	<meta property="og:site_name" content={SITE.name} />
	<meta property="og:locale" content={SITE.locale} />

	<!-- Twitter Card -->
	<meta name="twitter:card" content="summary_large_image" />
	{#if SITE.twitterHandle}
		<meta name="twitter:site" content={SITE.twitterHandle} />
	{/if}
	<meta name="twitter:title" content={resolvedOgTitle} />
	<meta name="twitter:description" content={resolvedOgDesc} />
	<meta name="twitter:image" content={resolvedOgImage} />

	{#if jsonLd}
		<!-- prettier-ignore -->
		<script type="application/ld+json">
{jsonLd}
		</script>
	{/if}
</svelte:head>
