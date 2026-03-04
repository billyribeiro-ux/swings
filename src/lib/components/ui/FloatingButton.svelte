<script lang="ts">
	import { openModal } from '$lib/stores/modal.svelte';
	import UsersThree from 'phosphor-svelte/lib/UsersThree';
</script>

<button
	onclick={openModal}
	aria-label="Meet the traders"
	class="group from-navy to-navy-mid border-teal/30 fixed right-8 bottom-8 z-40 flex items-center gap-2 rounded-full border bg-linear-to-r px-4 py-3 shadow-lg transition-all duration-300 hover:-translate-y-1 hover:shadow-xl"
>
	<!-- Pulse Animation -->
	<span
		class="bg-teal/20 absolute inset-0 animate-ping rounded-full opacity-0 group-hover:opacity-100"
	></span>

	<UsersThree size={24} weight="duotone" color="#15C5D1" />
	<span class="hidden text-sm font-semibold text-white sm:inline">Meet The Traders</span>
</button>
