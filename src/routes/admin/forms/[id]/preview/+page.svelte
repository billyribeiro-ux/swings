<!--
  FORM-09: Live preview of the current draft at all 9 PE7 breakpoints.

  The preview is hosted in a sandboxed <iframe> pointing at
  /forms/{slug}?preview=1 so we get the real renderer (including
  validation, honeypot, multi-step, and stylesheet layering) inside a
  width-constrained frame. Toolbar buttons clamp the iframe width to one
  of the 9 tiers — the page itself is an admin-only viewport manager.

  A11y: each breakpoint button sets aria-pressed so SR users can identify
  the active viewport; the iframe has a title attribute and the page
  surfaces the form's slug + name so the context is clear.
-->
<script lang="ts">
	import { onMount } from 'svelte';
	import { page } from '$app/state';
	import { resolve } from '$app/paths';
	import { api } from '$lib/api/client';
	import DeviceMobileIcon from 'phosphor-svelte/lib/DeviceMobileIcon';
	import DeviceTabletIcon from 'phosphor-svelte/lib/DeviceTabletIcon';
	import DesktopIcon from 'phosphor-svelte/lib/DesktopIcon';
	import MonitorIcon from 'phosphor-svelte/lib/MonitorIcon';

	interface FormDef {
		id: string;
		slug: string;
		name: string;
	}

	interface Breakpoint {
		readonly key: string;
		readonly label: string;
		readonly width: number;
		readonly icon: typeof DeviceMobileIcon;
	}

	const breakpoints: readonly Breakpoint[] = [
		{ key: 'xs', label: 'XS · 320', width: 320, icon: DeviceMobileIcon },
		{ key: 'sm', label: 'SM · 480', width: 480, icon: DeviceMobileIcon },
		{ key: 'md', label: 'MD · 640', width: 640, icon: DeviceMobileIcon },
		{ key: 'lg', label: 'LG · 768', width: 768, icon: DeviceTabletIcon },
		{ key: 'xl', label: 'XL · 1024', width: 1024, icon: DeviceTabletIcon },
		{ key: '2xl', label: '2XL · 1280', width: 1280, icon: DesktopIcon },
		{ key: '3xl', label: '3XL · 1536', width: 1536, icon: DesktopIcon },
		{ key: '4xl', label: '4XL · 1920', width: 1920, icon: MonitorIcon },
		{ key: '5xl', label: '5XL · 3840', width: 3840, icon: MonitorIcon }
	];

	const id = $derived(page.params.id ?? '');
	let form = $state<FormDef | null>(null);
	let active = $state<Breakpoint>(breakpoints[3]!);
	let err = $state<string | null>(null);

	onMount(async () => {
		try {
			form = await api.get<FormDef>(`/admin/forms/${id}`);
		} catch (e) {
			err = e instanceof Error ? e.message : 'Failed to load form.';
		}
	});

	const previewUrl = $derived(form ? `/forms/${encodeURIComponent(form.slug)}?preview=1` : '');
	const frameStyle = $derived(`--preview-width: ${active.width}px`);
</script>

<svelte:head><title>{form?.name ?? 'Form'} · Preview</title></svelte:head>

<header class="preview__header">
	<div>
		<a class="preview__back" href={resolve('/admin/forms/[id]', { id })}>← Builder</a>
		<h1 class="preview__title">{form?.name ?? 'Loading…'}</h1>
		{#if form}<p class="preview__slug">/forms/{form.slug}</p>{/if}
	</div>
	<div class="preview__toolbar" role="toolbar" aria-label="Viewport size">
		{#each breakpoints as bp (bp.key)}
			<button
				type="button"
				class="preview__bp"
				class:preview__bp--active={active.key === bp.key}
				aria-pressed={active.key === bp.key}
				onclick={() => (active = bp)}
				title={bp.label}
			>
				<bp.icon size={16} />
				<span>{bp.key}</span>
			</button>
		{/each}
	</div>
</header>

{#if err}
	<p class="preview__error" role="alert">{err}</p>
{:else if form}
	<div class="preview__stage">
		<div class="preview__meta">
			{active.label}px · {active.width}px wide
		</div>
		<iframe
			class="preview__frame"
			src={previewUrl}
			title={`${form.name} rendered at ${active.label}`}
			style={frameStyle}
			sandbox="allow-same-origin allow-scripts allow-forms"
			loading="lazy"
		></iframe>
	</div>
{:else}
	<p class="preview__loading">Loading…</p>
{/if}

<style>
	.preview__header {
		display: flex;
		flex-direction: column;
		gap: var(--space-3);
		padding-block: var(--space-4);
		border-block-end: 1px solid var(--surface-border-subtle);
		margin-block-end: var(--space-4);
	}

	.preview__back {
		color: var(--surface-fg-muted);
		font-size: var(--fs-sm);
		text-decoration: none;
	}

	.preview__back:hover {
		color: var(--surface-fg-default);
	}

	.preview__title {
		margin: 0;
		font-family: var(--font-heading);
		font-size: var(--fs-2xl);
		color: var(--surface-fg-default);
	}

	.preview__slug {
		margin: var(--space-1) 0 0;
		font-family: var(--font-mono);
		font-size: var(--fs-sm);
		color: var(--surface-fg-muted);
	}

	.preview__toolbar {
		display: inline-flex;
		flex-wrap: wrap;
		gap: var(--space-1);
		padding: var(--space-1);
		background-color: var(--surface-bg-subtle);
		border: 1px solid var(--surface-border-subtle);
		border-radius: var(--radius-md);
	}

	.preview__bp {
		display: inline-flex;
		align-items: center;
		gap: var(--space-1);
		padding-block: var(--space-1-5);
		padding-inline: var(--space-2-5);
		background: transparent;
		color: var(--surface-fg-muted);
		border: 1px solid transparent;
		border-radius: var(--radius-sm);
		font-size: var(--fs-xs);
		font-family: var(--font-mono);
		cursor: pointer;
	}

	.preview__bp:hover {
		color: var(--surface-fg-default);
	}

	.preview__bp--active {
		background-color: var(--brand-teal-500);
		color: var(--neutral-0);
	}

	.preview__bp:focus-visible {
		outline: 2px solid var(--brand-teal-500);
		outline-offset: 2px;
	}

	.preview__stage {
		padding: var(--space-4);
		background-color: var(--surface-bg-muted);
		border-radius: var(--radius-md);
		min-block-size: calc(100vh - 14rem);
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: var(--space-3);
	}

	.preview__meta {
		font-size: var(--fs-xs);
		font-family: var(--font-mono);
		color: var(--surface-fg-muted);
	}

	.preview__frame {
		inline-size: var(--preview-width, 768px);
		max-inline-size: 100%;
		block-size: min(calc(100vh - 18rem), 900px);
		background-color: var(--surface-bg-canvas);
		border: 1px solid var(--surface-border-default);
		border-radius: var(--radius-md);
		box-shadow: var(--shadow-lg);
	}

	.preview__error {
		color: var(--status-danger-700);
		padding: var(--space-4);
	}

	.preview__loading {
		padding: var(--space-6);
		text-align: center;
		color: var(--surface-fg-muted);
	}

	@media (min-width: 768px) {
		.preview__header {
			flex-direction: row;
			justify-content: space-between;
			align-items: flex-end;
		}
	}
</style>
