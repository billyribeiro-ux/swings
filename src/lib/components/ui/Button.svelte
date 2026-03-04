<script lang="ts">
	import { type Snippet } from 'svelte';

	interface Props {
		variant?: 'primary' | 'ghost' | 'outline';
		href?: string;
		onclick?: () => void;
		disabled?: boolean;
		children: Snippet;
	}

	let { variant = 'primary', href, onclick, disabled = false, children }: Props = $props();

	const baseClasses =
		'inline-flex items-center justify-center gap-2 px-6 py-3 rounded-xl font-ui font-semibold text-sm transition-all duration-300 ease-out focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-teal/70 focus-visible:ring-offset-2 focus-visible:ring-offset-navy active:scale-[0.97] disabled:pointer-events-none disabled:opacity-50';

	const variantClasses = {
		primary:
			'bg-teal text-white hover:bg-teal-light hover:-translate-y-px shadow-lg shadow-teal/25 hover:shadow-xl hover:shadow-teal/30',
		ghost:
			'bg-white/8 text-white border border-white/15 hover:bg-white/15 hover:-translate-y-px backdrop-blur-sm',
		outline:
			'bg-transparent text-navy border-2 border-navy/80 hover:bg-navy hover:text-white hover:-translate-y-px hover:shadow-lg hover:shadow-navy/15'
	};

	const classes = $derived(`${baseClasses} ${variantClasses[variant]}`);
</script>

{#if href}
	<a {href} class={classes} aria-disabled={disabled || undefined}>
		{@render children()}
	</a>
{:else}
	<button {onclick} {disabled} class={classes}>
		{@render children()}
	</button>
{/if}
