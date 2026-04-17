<!--
  Test fixture: ConsentBanner with the stub config so tests don't have to
  construct a BannerConfig inline every time. Accepts per-test overrides
  for the fields we actually assert on.
-->
<script lang="ts">
	import { untrack } from 'svelte';
	import ConsentBanner from '../ConsentBanner.svelte';
	import { STUB_BANNER_CONFIG, type BannerLayout } from '$lib/api/consent';
	import { consent } from '$lib/stores/consent.svelte';

	interface Props {
		layout?: BannerLayout;
		/** Reset the store on mount so the banner appears. */
		resetOnMount?: boolean;
	}

	const { layout = 'bar', resetOnMount = true }: Props = $props();

	// Read the prop once at mount — deliberately NOT reactive. The harness's
	// contract is "reset once on mount if asked", not "keep resetting when
	// the prop toggles".
	untrack(() => {
		if (resetOnMount) consent.revokeAll();
	});

	const config = $derived({ ...STUB_BANNER_CONFIG, layout });
</script>

<ConsentBanner {config} />
