<!--
  PE7 shared UI primitives — showcase / dogfooding page.

  Intended for development use only. Reachable at `/admin/_ui-kit`.
  The leading underscore in the route segment is a convention signalling
  "internal" — this page is NOT part of the public nav.

  Authz: gated by the admin shell in `src/routes/admin/+layout.svelte` —
  the layout's `{:else}` branch only renders children when
  `auth.isAuthenticated && auth.isAdmin && adminSessionReady`, so non-admin
  visitors see the admin login form instead of this preview surface. The
  in-page `$effect` below adds a defense-in-depth redirect in case the
  layout gate is ever lifted.
-->
<script lang="ts">
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import { auth } from '$lib/stores/auth.svelte';
	import PackageIcon from 'phosphor-svelte/lib/PackageIcon';
	import ArrowRightIcon from 'phosphor-svelte/lib/ArrowRightIcon';
	import DownloadIcon from 'phosphor-svelte/lib/DownloadIcon';
	import TrashIcon from 'phosphor-svelte/lib/TrashIcon';
	import CaretRightIcon from 'phosphor-svelte/lib/CaretRightIcon';
	import {
		Button,
		Dialog,
		Drawer,
		Toast,
		ToastRegion,
		Stepper,
		Spinner,
		FormField,
		Breadcrumbs,
		EmptyState,
		VisuallyHidden
	} from '$lib/components/shared';
	import { toasts } from '$lib/stores/toasts.svelte';

	// Defense-in-depth: if a future refactor lifts the admin-shell gate,
	// this redirect still keeps the dev preview off the public surface.
	$effect(() => {
		if (!auth.loading && !auth.isAdmin) {
			void goto('/admin', { replaceState: true });
		}
	});

	let dialogOpen = $state(false);
	let drawerOpen = $state(false);
	let drawerPosition = $state<'start' | 'end' | 'top' | 'bottom'>('end');
	let emailValue = $state('');
	let emailError = $state<string | undefined>(undefined);
	let currentStep = $state('plan');

	const steps = [
		{ id: 'plan', label: 'Plan', description: 'Choose your plan' },
		{ id: 'billing', label: 'Billing', description: 'Enter payment details' },
		{ id: 'review', label: 'Review', description: 'Confirm and complete' }
	];

	function validateEmail() {
		if (!emailValue.includes('@')) {
			emailError = 'Please enter a valid email address.';
		} else {
			emailError = undefined;
		}
	}

	function pushInfoToast() {
		toasts.push({
			kind: 'info',
			title: 'Heads up',
			description: 'This is an informational toast.'
		});
	}
	function pushSuccessToast() {
		toasts.push({ kind: 'success', title: 'Saved', description: 'Your changes were saved.' });
	}
	function pushWarningToast() {
		toasts.push({
			kind: 'warning',
			title: 'Unsaved changes',
			description: 'You have pending edits.'
		});
	}
	function pushDangerToast() {
		toasts.push({
			kind: 'danger',
			title: 'Request failed',
			description: 'We could not reach the server.'
		});
	}

	onMount(() => {
		return () => toasts.clear();
	});
</script>

<svelte:head>
	<title>PE7 UI Kit (dev)</title>
</svelte:head>

<div class="kit">
	<header class="kit-header">
		<h1>PE7 shared UI primitives</h1>
		<p class="lede">
			Canonical component library under <code>$lib/components/shared</code>. Every variant
			rendered below is WCAG 2.2 AA baseline.
		</p>
	</header>

	<section class="tile">
		<h2>Button</h2>
		<div class="grid">
			<article>
				<h3>Variants</h3>
				<div class="cluster">
					<Button variant="primary">Primary</Button>
					<Button variant="secondary">Secondary</Button>
					<Button variant="tertiary">Tertiary</Button>
					<Button variant="danger">Danger</Button>
					<Button variant="ghost">Ghost</Button>
					<Button variant="link">Link</Button>
				</div>
			</article>
			<article>
				<h3>Sizes</h3>
				<div class="cluster">
					<Button size="sm">Small</Button>
					<Button size="md">Medium</Button>
					<Button size="lg">Large</Button>
				</div>
			</article>
			<article>
				<h3>States</h3>
				<div class="cluster">
					<Button disabled>Disabled</Button>
					<Button loading>Loading</Button>
					<Button fullWidth>Full width</Button>
				</div>
			</article>
			<article>
				<h3>With icons</h3>
				<div class="cluster">
					{#snippet downloadIcon()}
						<DownloadIcon size="1rem" weight="bold" />
					{/snippet}
					{#snippet arrowIcon()}
						<ArrowRightIcon size="1rem" weight="bold" />
					{/snippet}
					{#snippet trashIcon()}
						<TrashIcon size="1rem" weight="bold" />
					{/snippet}
					<Button iconLeading={downloadIcon}>DownloadIcon</Button>
					<Button iconTrailing={arrowIcon}>Continue</Button>
					<Button variant="danger" iconLeading={trashIcon}>Delete</Button>
				</div>
			</article>
			<article>
				<h3>As link</h3>
				<div class="cluster">
					<Button href="/admin">Go to admin</Button>
					<Button href="https://svelte.dev" target="_blank" variant="link"
						>External</Button
					>
				</div>
			</article>
		</div>
	</section>

	<section class="tile">
		<h2>Spinner</h2>
		<div class="cluster align-center">
			<Spinner size="sm" label="Loading small" />
			<Spinner size="md" label="Loading medium" />
			<Spinner size="lg" label="Loading large" />
		</div>
	</section>

	<section class="tile">
		<h2>Dialog</h2>
		<div class="cluster">
			<Button onclick={() => (dialogOpen = true)}>Open dialog</Button>
		</div>
		<Dialog
			bind:open={dialogOpen}
			title="Confirm action"
			description="This cannot be undone."
			size="sm"
		>
			<p>Are you sure you want to proceed? This is a destructive operation.</p>
			{#snippet footer()}
				<Button variant="tertiary" onclick={() => (dialogOpen = false)}>Cancel</Button>
				<Button variant="danger" onclick={() => (dialogOpen = false)}>Yes, delete</Button>
			{/snippet}
		</Dialog>
	</section>

	<section class="tile">
		<h2>Drawer</h2>
		<div class="cluster">
			{#each ['start', 'end', 'top', 'bottom'] as const as p (p)}
				<Button
					variant="tertiary"
					onclick={() => {
						drawerPosition = p;
						drawerOpen = true;
					}}
				>
					From {p}
				</Button>
			{/each}
		</div>
		<Drawer
			bind:open={drawerOpen}
			title="Drawer from {drawerPosition}"
			position={drawerPosition}
		>
			<p>Drawers are useful for detail panels, filter pickers, and settings.</p>
			{#snippet footer()}
				<Button onclick={() => (drawerOpen = false)}>Close</Button>
			{/snippet}
		</Drawer>
	</section>

	<section class="tile">
		<h2>Toasts</h2>
		<div class="cluster">
			<Button variant="tertiary" onclick={pushInfoToast}>Info</Button>
			<Button variant="tertiary" onclick={pushSuccessToast}>Success</Button>
			<Button variant="tertiary" onclick={pushWarningToast}>Warning</Button>
			<Button variant="tertiary" onclick={pushDangerToast}>Danger</Button>
		</div>
		<h3>Standalone (no store)</h3>
		<div class="cluster align-start">
			<Toast kind="info" title="Info" description="Standalone info toast." duration={0} />
			<Toast kind="success" title="Saved" duration={0} />
		</div>
	</section>

	<section class="tile">
		<h2>Stepper</h2>
		<Stepper {steps} current={currentStep} interactive onselect={(id) => (currentStep = id)} />
		<h3>Vertical</h3>
		<Stepper {steps} current={currentStep} orientation="vertical" />
	</section>

	<section class="tile">
		<h2>FormField</h2>
		<FormField
			for="demo-email"
			label="Email"
			description="We use this for billing receipts only."
			required
			error={emailError}
		>
			{#snippet children({ describedBy, invalid, required })}
				<input
					id="demo-email"
					type="email"
					bind:value={emailValue}
					aria-describedby={describedBy}
					aria-invalid={invalid || undefined}
					aria-required={required || undefined}
					onblur={validateEmail}
				/>
			{/snippet}
		</FormField>
	</section>

	<section class="tile">
		<h2>Breadcrumbs</h2>
		<Breadcrumbs
			items={[
				{ label: 'Admin', href: '/admin' },
				{ label: 'Blog', href: '/admin/blog' },
				{ label: 'New post' }
			]}
		/>
	</section>

	<section class="tile">
		<h2>EmptyState</h2>
		<EmptyState title="No posts yet" description="When you publish a post it will appear here.">
			{#snippet icon()}
				<PackageIcon size="3rem" weight="duotone" />
			{/snippet}
			{#snippet action()}
				<Button>
					Write your first post
					{#snippet iconTrailing()}<CaretRightIcon size="1rem" weight="bold" />{/snippet}
				</Button>
			{/snippet}
		</EmptyState>
	</section>

	<section class="tile">
		<h2>VisuallyHidden</h2>
		<p>
			The next word is hidden visually but announced by screen readers:
			<VisuallyHidden>ACCESSIBLE</VisuallyHidden>
			— inspect with a screen reader to verify.
		</p>
	</section>
</div>

<ToastRegion />

<style>
	.kit {
		max-inline-size: var(--container-max);
		margin-inline: auto;
		padding-block: var(--space-8);
		padding-inline: var(--space-6);
		display: flex;
		flex-direction: column;
		gap: var(--space-8);
	}
	.kit-header {
		display: flex;
		flex-direction: column;
		gap: var(--space-2);
	}
	.lede {
		margin: 0;
		color: var(--surface-fg-muted);
	}
	code {
		font-family: var(--font-mono);
		font-size: 0.95em;
	}
	.tile {
		background-color: var(--surface-bg-subtle);
		border: 1px solid var(--surface-border-subtle);
		border-radius: var(--radius-2xl);
		padding: var(--space-6);
		display: flex;
		flex-direction: column;
		gap: var(--space-4);
	}
	.tile h2 {
		margin: 0;
		font-size: var(--fs-lg);
	}
	.tile h3 {
		margin: 0;
		font-size: var(--fs-sm);
		color: var(--surface-fg-muted);
		text-transform: uppercase;
		letter-spacing: var(--ls-wide);
	}
	.grid {
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(18rem, 1fr));
		gap: var(--space-4);
	}
	article {
		background-color: var(--surface-bg-canvas);
		border: 1px solid var(--surface-border-subtle);
		border-radius: var(--radius-2xl);
		padding: var(--space-4);
		display: flex;
		flex-direction: column;
		gap: var(--space-3);
	}
	.cluster {
		display: flex;
		flex-wrap: wrap;
		gap: var(--space-3);
	}
	.cluster.align-center {
		align-items: center;
	}
	.cluster.align-start {
		align-items: flex-start;
	}
	input[type='email'] {
		inline-size: 100%;
		max-inline-size: 24rem;
	}
</style>
