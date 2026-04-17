<!--
  CONSENT-07 — third-party services CRUD.

  A service row tells the consent-gate scanner "when category X is granted,
  activate these scripts / load these cookies". Grouped by category in the
  table. No delete — flip `is_active` to false to retire.
-->
<script lang="ts">
	import { onMount } from 'svelte';
	import { Button, Dialog } from '$lib/components/shared';
	import {
		listServices,
		createService,
		updateService,
		listCategories,
		type AdminService,
		type AdminCategory,
		type ServiceUpsertBody
	} from '$lib/api/admin-consent';

	let services = $state<AdminService[]>([]);
	let categories = $state<AdminCategory[]>([]);
	let loading = $state(true);
	let errorMsg = $state<string | null>(null);

	let editorOpen = $state(false);
	let editingId = $state<string | null>(null);
	let slug = $state('');
	let name = $state('');
	let vendor = $state('');
	let category = $state('analytics');
	let domains = $state('');
	let privacyUrl = $state('');
	let description = $state('');
	let isActive = $state(true);

	async function refresh() {
		loading = true;
		errorMsg = null;
		try {
			[services, categories] = await Promise.all([listServices(), listCategories()]);
			if (!category && categories.length > 0) category = categories[0].key;
		} catch (err) {
			errorMsg = err instanceof Error ? err.message : String(err);
		} finally {
			loading = false;
		}
	}

	onMount(() => void refresh());

	function openCreate() {
		editingId = null;
		slug = '';
		name = '';
		vendor = '';
		category = categories[0]?.key ?? 'analytics';
		domains = '';
		privacyUrl = '';
		description = '';
		isActive = true;
		editorOpen = true;
	}

	function openEdit(s: AdminService) {
		editingId = s.id;
		slug = s.slug;
		name = s.name;
		vendor = s.vendor;
		category = s.category;
		domains = s.domains.join(', ');
		privacyUrl = s.privacy_url ?? '';
		description = s.description ?? '';
		isActive = s.is_active;
		editorOpen = true;
	}

	async function save() {
		const body: ServiceUpsertBody = {
			slug,
			name,
			vendor,
			category,
			domains: domains
				.split(',')
				.map((d) => d.trim())
				.filter(Boolean),
			privacy_url: privacyUrl || null,
			description: description || null,
			is_active: isActive
		};
		try {
			if (editingId) {
				await updateService(editingId, body);
			} else {
				await createService(body);
			}
			editorOpen = false;
			await refresh();
		} catch (err) {
			errorMsg = err instanceof Error ? err.message : String(err);
		}
	}
</script>

<svelte:head>
	<title>Services · Consent · Admin</title>
</svelte:head>

<header class="head">
	<h1>Third-party services</h1>
	<Button variant="primary" size="md" onclick={openCreate}>New service</Button>
</header>

{#if errorMsg}<div class="error">{errorMsg}</div>{/if}

{#if loading}
	<p class="muted">Loading…</p>
{:else if services.length === 0}
	<p class="muted">No services configured yet.</p>
{:else}
	<table class="table">
		<thead>
			<tr>
				<th>Slug</th>
				<th>Name</th>
				<th>Vendor</th>
				<th>Category</th>
				<th>Active</th>
				<th></th>
			</tr>
		</thead>
		<tbody>
			{#each services as s (s.id)}
				<tr>
					<td><code>{s.slug}</code></td>
					<td>{s.name}</td>
					<td>{s.vendor}</td>
					<td><code>{s.category}</code></td>
					<td>{s.is_active ? 'yes' : 'no'}</td>
					<td><Button variant="tertiary" size="sm" onclick={() => openEdit(s)}>Edit</Button></td>
				</tr>
			{/each}
		</tbody>
	</table>
{/if}

<Dialog
	open={editorOpen}
	onclose={() => (editorOpen = false)}
	title={editingId ? 'Edit service' : 'New service'}
	size="md"
>
	<div class="form">
		<div class="row">
			<label class="field">
				<span>Slug</span>
				<input type="text" bind:value={slug} />
			</label>
			<label class="field">
				<span>Name</span>
				<input type="text" bind:value={name} />
			</label>
		</div>
		<div class="row">
			<label class="field">
				<span>Vendor</span>
				<input type="text" bind:value={vendor} />
			</label>
			<label class="field">
				<span>Category</span>
				<select bind:value={category}>
					{#each categories as c (c.key)}
						<option value={c.key}>{c.key} — {c.label}</option>
					{/each}
				</select>
			</label>
		</div>
		<label class="field">
			<span>Domains (comma-separated)</span>
			<input type="text" bind:value={domains} placeholder="example.com, *.example.com" />
		</label>
		<label class="field">
			<span>Privacy URL</span>
			<input type="text" bind:value={privacyUrl} />
		</label>
		<label class="field">
			<span>Description</span>
			<textarea rows="3" bind:value={description}></textarea>
		</label>
		<label class="field field--toggle">
			<input type="checkbox" bind:checked={isActive} />
			<span>Active</span>
		</label>
	</div>
	{#snippet footer()}
		<Button variant="tertiary" onclick={() => (editorOpen = false)}>Cancel</Button>
		<Button variant="primary" onclick={save}>Save</Button>
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
	.row {
		display: flex;
		gap: var(--space-4);
		flex-wrap: wrap;
	}
	.field {
		display: flex;
		flex-direction: column;
		gap: var(--space-1-5);
		flex: 1 1 14rem;
		font-size: var(--fs-sm);
	}
	.field--toggle {
		flex-direction: row;
		align-items: center;
		gap: var(--space-2);
	}
	input[type='text'],
	select,
	textarea {
		padding: var(--space-2) var(--space-3);
		border: 1px solid var(--surface-border-subtle);
		border-radius: var(--radius-md);
		background-color: var(--surface-bg-canvas);
		color: var(--surface-fg-default);
		font: inherit;
	}
</style>
