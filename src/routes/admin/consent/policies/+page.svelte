<!--
  CONSENT-07 — policy versioning.

  Each new version is an INSERT — there is no edit-in-place. Creating a new
  version bumps the effective `policy_version` that the banner carries, and
  the ConsentStore's envelope versioning compares stored `policyVersion` vs
  the current response; a mismatch triggers a re-consent prompt.

  The editor is a plain textarea (markdown). No WYSIWYG — deliberate: the
  admin should be able to paste from their legal review tool without the
  UI rewriting whitespace.
-->
<script lang="ts">
	import { onMount } from 'svelte';
	import { Button, Dialog } from '$lib/components/shared';
	import {
		listPolicies,
		createPolicy,
		type AdminPolicy,
		type PolicyCreateBody
	} from '$lib/api/admin-consent';

	let policies = $state<AdminPolicy[]>([]);
	let loading = $state(true);
	let errorMsg = $state<string | null>(null);

	let editorOpen = $state(false);
	let markdown = $state('');
	let locale = $state('en');
	let viewPolicy = $state<AdminPolicy | null>(null);

	async function refresh() {
		loading = true;
		errorMsg = null;
		try {
			policies = await listPolicies();
		} catch (err) {
			errorMsg = err instanceof Error ? err.message : String(err);
		} finally {
			loading = false;
		}
	}

	onMount(() => void refresh());

	function openNew() {
		markdown = '';
		locale = 'en';
		editorOpen = true;
	}

	async function save() {
		const body: PolicyCreateBody = { markdown, locale };
		try {
			await createPolicy(body);
			editorOpen = false;
			await refresh();
		} catch (err) {
			errorMsg = err instanceof Error ? err.message : String(err);
		}
	}
</script>

<svelte:head>
	<title>Policies · Consent · Admin</title>
</svelte:head>

<header class="head">
	<h1>Privacy policy versions</h1>
	<Button variant="primary" size="md" onclick={openNew}>New version</Button>
</header>

{#if errorMsg}<div class="error">{errorMsg}</div>{/if}

{#if loading}
	<p class="muted">Loading…</p>
{:else if policies.length === 0}
	<p class="muted">No policy versions yet.</p>
{:else}
	<table class="table">
		<thead>
			<tr>
				<th>Version</th>
				<th>Locale</th>
				<th>Effective at</th>
				<th>Created at</th>
				<th></th>
			</tr>
		</thead>
		<tbody>
			{#each policies as p (p.id)}
				<tr>
					<td>v{p.version}</td>
					<td><code>{p.locale}</code></td>
					<td>{new Date(p.effective_at).toISOString()}</td>
					<td>{new Date(p.created_at).toISOString()}</td>
					<td><Button variant="tertiary" size="sm" onclick={() => (viewPolicy = p)}>View</Button></td>
				</tr>
			{/each}
		</tbody>
	</table>
{/if}

<Dialog open={editorOpen} onclose={() => (editorOpen = false)} title="New policy version" size="lg">
	<div class="form">
		<label class="field">
			<span>Locale</span>
			<input type="text" bind:value={locale} />
		</label>
		<label class="field">
			<span>Markdown</span>
			<textarea rows="14" bind:value={markdown}></textarea>
		</label>
		<p class="muted">
			Creating a new version bumps the banner's <code>policy_version</code>, which forces
			re-consent UI for every subject on their next page-load.
		</p>
	</div>
	{#snippet footer()}
		<Button variant="tertiary" onclick={() => (editorOpen = false)}>Cancel</Button>
		<Button variant="primary" onclick={save}>Publish</Button>
	{/snippet}
</Dialog>

<Dialog
	open={viewPolicy !== null}
	onclose={() => (viewPolicy = null)}
	title="Policy version"
	size="lg"
>
	{#if viewPolicy}
		<div class="view-pane">
			<p class="muted">v{viewPolicy.version} · <code>{viewPolicy.locale}</code></p>
			<pre>{viewPolicy.markdown}</pre>
		</div>
	{/if}
	{#snippet footer()}
		<Button variant="primary" onclick={() => (viewPolicy = null)}>Close</Button>
	{/snippet}
</Dialog>

<style>
	.head {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: var(--space-4);
		margin-block-end: var(--space-5);
	}
	.head h1 {
		margin: 0;
		font-family: var(--font-heading);
		font-size: var(--fs-2xl);
		font-weight: var(--w-bold);
	}
	.error {
		padding: var(--space-3) var(--space-4);
		border-radius: var(--radius-md);
		border: 1px solid var(--surface-border-subtle);
		background-color: var(--surface-bg-canvas);
		margin-block-end: var(--space-4);
	}
	.muted {
		color: var(--surface-fg-muted);
	}
	.table {
		inline-size: 100%;
		border-collapse: collapse;
	}
	.table th,
	.table td {
		padding: var(--space-3);
		border-block-end: 1px solid var(--surface-border-subtle);
		font-size: var(--fs-sm);
		text-align: start;
	}
	.form {
		display: flex;
		flex-direction: column;
		gap: var(--space-3);
	}
	.field {
		display: flex;
		flex-direction: column;
		gap: var(--space-1-5);
		font-size: var(--fs-sm);
	}
	input[type='text'],
	textarea {
		padding: var(--space-2) var(--space-3);
		border: 1px solid var(--surface-border-subtle);
		border-radius: var(--radius-md);
		background-color: var(--surface-bg-canvas);
		color: var(--surface-fg-default);
		font: inherit;
	}
	.view-pane pre {
		max-block-size: 24rem;
		overflow: auto;
		padding: var(--space-3);
		border: 1px solid var(--surface-border-subtle);
		border-radius: var(--radius-md);
		background-color: var(--surface-bg-canvas);
		white-space: pre-wrap;
		font-family: var(--font-mono);
		font-size: var(--fs-sm);
	}
</style>
