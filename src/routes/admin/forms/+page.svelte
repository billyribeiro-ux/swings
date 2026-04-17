<script lang="ts">
	import { onMount } from 'svelte';
	import Plus from 'phosphor-svelte/lib/Plus';
	import PencilSimple from 'phosphor-svelte/lib/PencilSimple';
	import ListBullets from 'phosphor-svelte/lib/ListBullets';
	import { goto } from '$app/navigation';
	import { api } from '$lib/api/client';

	type FormRow = {
		id: string;
		slug: string;
		name: string;
		description: string | null;
		is_active: boolean;
		updated_at: string;
	};

	let forms = $state<FormRow[]>([]);
	let loading = $state(true);
	let err = $state<string | null>(null);

	onMount(async () => {
		try {
			forms = await api.get<FormRow[]>('/admin/forms');
		} catch (e) {
			err = e instanceof Error ? e.message : 'Failed to load forms.';
		} finally {
			loading = false;
		}
	});
</script>

<svelte:head><title>Forms · Admin</title></svelte:head>

<header class="page-header">
	<h1>Forms</h1>
	<button class="btn btn--primary" type="button" onclick={() => goto('/admin/forms/new')}>
		<Plus size={16} />New form
	</button>
</header>

{#if loading}
	<p>Loading…</p>
{:else if err}
	<p class="error">{err}</p>
{:else if forms.length === 0}
	<p class="empty">No forms yet — create one to start collecting submissions.</p>
{:else}
	<table class="table">
		<thead>
			<tr><th>Name</th><th>Slug</th><th>Status</th><th>Updated</th><th></th></tr>
		</thead>
		<tbody>
			{#each forms as f (f.id)}
				<tr>
					<td>{f.name}</td>
					<td><code>{f.slug}</code></td>
					<td>
						<span class="status" class:status--active={f.is_active}>
							{f.is_active ? 'Active' : 'Draft'}
						</span>
					</td>
					<td>{new Date(f.updated_at).toLocaleString()}</td>
					<td class="actions">
						<a class="link" href="/admin/forms/{f.id}"><PencilSimple size={14} />Edit</a>
						<a class="link" href="/admin/forms/{f.id}/submissions"><ListBullets size={14} />Submissions</a>
					</td>
				</tr>
			{/each}
		</tbody>
	</table>
{/if}

<style>
	.page-header {
		display: flex; align-items: center; justify-content: space-between;
		margin-block-end: var(--space-4);
	}
	.btn {
		display: inline-flex; align-items: center; gap: var(--space-1);
		padding: var(--space-1) var(--space-3); border-radius: var(--radius-sm);
		font-size: var(--font-size-sm); cursor: pointer; border: 1px solid var(--color-border);
		background: var(--color-surface-2); color: var(--color-text);
	}
	.btn--primary { background: var(--color-accent); color: var(--color-on-accent); border-color: transparent; }
	.btn:focus-visible { outline: 2px solid var(--color-focus-ring); outline-offset: 2px; }

	.table { inline-size: 100%; border-collapse: collapse; }
	.table th, .table td {
		padding: var(--space-2);
		border-block-end: 1px solid var(--color-border);
		text-align: start;
		font-size: var(--font-size-sm);
	}
	.table th {
		font-size: var(--font-size-xs);
		text-transform: uppercase;
		letter-spacing: 0.05em;
		color: var(--color-text-muted);
	}
	.actions { display: flex; gap: var(--space-3); }
	.link {
		display: inline-flex; align-items: center; gap: 4px;
		color: var(--color-accent); text-decoration: none;
	}
	.link:hover { text-decoration: underline; }
	.status {
		font-size: var(--font-size-2xs); text-transform: uppercase; letter-spacing: 0.05em;
		padding: 2px var(--space-2); border-radius: var(--radius-sm);
		background: var(--color-surface-3); color: var(--color-text-muted);
	}
	.status--active { background: var(--color-accent-soft); color: var(--color-accent); }
	.empty, .error { padding: var(--space-6); text-align: center; }
	.error { color: var(--color-danger); }
</style>
