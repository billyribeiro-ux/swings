<!--
  FORM-09: Forms list.

  Row actions:
  - Edit           → /admin/forms/{id}         (builder)
  - Submissions    → /admin/forms/{id}/submissions
  - ArchiveIcon        → PATCH /admin/forms/{id} { is_active: false } (soft)
  - Unarchive      → PATCH /admin/forms/{id} { is_active: true }
  - Preview        → /admin/forms/{id}/preview
  - Versions       → /admin/forms/{id}/versions

  A11y: the table uses `<th scope="col">`; the action column's buttons
  carry `aria-label` so SR users know what they target.
-->
<script lang="ts">
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import { api } from '$lib/api/client';
	import PlusIcon from 'phosphor-svelte/lib/PlusIcon';
	import PencilSimpleIcon from 'phosphor-svelte/lib/PencilSimpleIcon';
	import ListBulletsIcon from 'phosphor-svelte/lib/ListBulletsIcon';
	import ArchiveIcon from 'phosphor-svelte/lib/ArchiveIcon';
	import ArrowCounterClockwiseIcon from 'phosphor-svelte/lib/ArrowCounterClockwiseIcon';
	import EyeIcon from 'phosphor-svelte/lib/EyeIcon';
	import ClockCounterClockwiseIcon from 'phosphor-svelte/lib/ClockCounterClockwiseIcon';

	interface FormRow {
		readonly id: string;
		readonly slug: string;
		readonly name: string;
		readonly description: string | null;
		readonly is_active: boolean;
		readonly updated_at: string;
	}

	let forms = $state<FormRow[]>([]);
	let loading = $state(true);
	let err = $state<string | null>(null);
	let acting = $state<string | null>(null);

	async function load() {
		loading = true;
		err = null;
		try {
			forms = await api.get<FormRow[]>('/admin/forms');
		} catch (e) {
			err = e instanceof Error ? e.message : 'Failed to load forms.';
		} finally {
			loading = false;
		}
	}

	onMount(load);

	async function toggleArchive(row: FormRow) {
		acting = row.id;
		try {
			await api.put(`/admin/forms/${row.id}`, { is_active: !row.is_active });
			await load();
		} catch (e) {
			err = e instanceof Error ? e.message : 'Failed to update form.';
		} finally {
			acting = null;
		}
	}
</script>

<svelte:head><title>Forms · Admin</title></svelte:head>

<header class="af-header">
	<div>
		<h1 class="af-title">Forms</h1>
		<p class="af-subtitle">Build, preview, and manage collection forms.</p>
	</div>
	<button class="af-btn af-btn--primary" type="button" onclick={() => goto('/admin/forms/new')}>
		<PlusIcon size={16} weight="bold" />
		<span>New form</span>
	</button>
</header>

{#if loading}
	<p class="af-status">Loading…</p>
{:else if err}
	<p class="af-status af-status--error" role="alert">{err}</p>
{:else if forms.length === 0}
	<div class="af-empty">
		<p>No forms yet — create one to start collecting submissions.</p>
		<button class="af-btn af-btn--primary" type="button" onclick={() => goto('/admin/forms/new')}>
			<PlusIcon size={16} weight="bold" />
			<span>Create your first form</span>
		</button>
	</div>
{:else}
	<div class="af-table-wrap">
		<table class="af-table">
			<thead>
				<tr>
					<th scope="col">Name</th>
					<th scope="col">Slug</th>
					<th scope="col">Status</th>
					<th scope="col">Updated</th>
					<th scope="col" class="af-table__actions-head">Actions</th>
				</tr>
			</thead>
			<tbody>
				{#each forms as f (f.id)}
					<tr>
						<td>
							<a class="af-table__name" href={`/admin/forms/${f.id}`}>{f.name}</a>
							{#if f.description}<div class="af-table__desc">{f.description}</div>{/if}
						</td>
						<td><code class="af-table__slug">{f.slug}</code></td>
						<td>
							<span class="af-status-pill" class:af-status-pill--active={f.is_active}>
								{f.is_active ? 'Active' : 'Archived'}
							</span>
						</td>
						<td class="af-table__ts">{new Date(f.updated_at).toLocaleString()}</td>
						<td>
							<div class="af-actions">
								<a class="af-action" href={`/admin/forms/${f.id}`} aria-label={`Edit ${f.name}`}>
									<PencilSimpleIcon size={14} />
									<span>Edit</span>
								</a>
								<a class="af-action" href={`/admin/forms/${f.id}/preview`} aria-label={`Preview ${f.name}`}>
									<EyeIcon size={14} />
									<span>Preview</span>
								</a>
								<a class="af-action" href={`/admin/forms/${f.id}/submissions`} aria-label={`Submissions for ${f.name}`}>
									<ListBulletsIcon size={14} />
									<span>Submissions</span>
								</a>
								<a class="af-action" href={`/admin/forms/${f.id}/versions`} aria-label={`Versions of ${f.name}`}>
									<ClockCounterClockwiseIcon size={14} />
									<span>Versions</span>
								</a>
								<button
									type="button"
									class="af-action af-action--destructive"
									onclick={() => toggleArchive(f)}
									disabled={acting === f.id}
									aria-label={f.is_active ? `ArchiveIcon ${f.name}` : `Restore ${f.name}`}
								>
									{#if f.is_active}
										<ArchiveIcon size={14} />
										<span>ArchiveIcon</span>
									{:else}
										<ArrowCounterClockwiseIcon size={14} />
										<span>Restore</span>
									{/if}
								</button>
							</div>
						</td>
					</tr>
				{/each}
			</tbody>
		</table>
	</div>
{/if}

<style>
	.af-header {
		display: flex;
		flex-direction: column;
		gap: var(--space-3);
		margin-block-end: var(--space-5);
	}

	.af-title {
		margin: 0;
		font-family: var(--font-heading);
		font-size: var(--fs-2xl);
		font-weight: var(--w-semibold);
		color: var(--surface-fg-default);
	}

	.af-subtitle {
		margin: var(--space-1) 0 0;
		color: var(--surface-fg-muted);
		font-size: var(--fs-sm);
	}

	.af-btn {
		display: inline-flex;
		align-items: center;
		gap: var(--space-2);
		padding-block: var(--space-2);
		padding-inline: var(--space-4);
		border: 1px solid transparent;
		border-radius: var(--radius-md);
		background-color: var(--brand-teal-500);
		color: var(--neutral-0);
		font-family: inherit;
		font-size: var(--fs-sm);
		font-weight: var(--w-semibold);
		cursor: pointer;
		align-self: flex-start;
		text-decoration: none;
	}

	.af-btn:hover {
		background-color: var(--brand-teal-600);
	}

	.af-btn:focus-visible {
		outline: 2px solid var(--brand-teal-500);
		outline-offset: 2px;
	}

	.af-status {
		padding: var(--space-6);
		text-align: center;
		color: var(--surface-fg-muted);
	}

	.af-status--error {
		color: var(--status-danger-700);
	}

	.af-empty {
		padding: var(--space-8);
		text-align: center;
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: var(--space-3);
		background-color: var(--surface-bg-subtle);
		border: 1px dashed var(--surface-border-default);
		border-radius: var(--radius-md);
	}

	.af-table-wrap {
		overflow-x: auto;
		border: 1px solid var(--surface-border-subtle);
		border-radius: var(--radius-md);
		background-color: var(--surface-bg-canvas);
	}

	.af-table {
		inline-size: 100%;
		border-collapse: collapse;
		font-size: var(--fs-sm);
	}

	.af-table th,
	.af-table td {
		padding-block: var(--space-3);
		padding-inline: var(--space-3);
		text-align: start;
		border-block-end: 1px solid var(--surface-border-subtle);
		vertical-align: top;
	}

	.af-table thead th {
		font-size: var(--fs-xs);
		text-transform: uppercase;
		letter-spacing: 0.05em;
		color: var(--surface-fg-muted);
		font-weight: var(--w-semibold);
		background-color: var(--surface-bg-subtle);
	}

	.af-table__name {
		color: var(--surface-fg-default);
		text-decoration: none;
		font-weight: var(--w-semibold);
	}

	.af-table__name:hover {
		color: var(--brand-teal-600);
	}

	.af-table__desc {
		margin-block-start: var(--space-1);
		font-size: var(--fs-xs);
		color: var(--surface-fg-muted);
	}

	.af-table__slug {
		font-family: var(--font-mono);
		font-size: var(--fs-xs);
		background-color: var(--surface-bg-muted);
		padding-block: 2px;
		padding-inline: var(--space-1-5);
		border-radius: var(--radius-sm);
	}

	.af-table__ts {
		color: var(--surface-fg-muted);
		font-variant-numeric: tabular-nums;
	}

	.af-status-pill {
		display: inline-flex;
		padding-block: 2px;
		padding-inline: var(--space-2-5);
		border-radius: var(--radius-full);
		font-size: var(--fs-xs);
		font-weight: var(--w-semibold);
		text-transform: uppercase;
		letter-spacing: 0.04em;
		background-color: var(--surface-bg-muted);
		color: var(--surface-fg-muted);
	}

	.af-status-pill--active {
		background-color: var(--status-success-50);
		color: var(--status-success-700);
	}

	.af-actions {
		display: inline-flex;
		gap: var(--space-1);
		flex-wrap: wrap;
	}

	.af-action {
		display: inline-flex;
		align-items: center;
		gap: var(--space-1);
		padding-block: var(--space-1);
		padding-inline: var(--space-2);
		font-size: var(--fs-xs);
		border: 1px solid var(--surface-border-subtle);
		border-radius: var(--radius-sm);
		color: var(--surface-fg-default);
		background-color: var(--surface-bg-canvas);
		text-decoration: none;
		cursor: pointer;
	}

	.af-action:hover {
		background-color: var(--surface-bg-muted);
	}

	.af-action:focus-visible {
		outline: 2px solid var(--brand-teal-500);
		outline-offset: 1px;
	}

	.af-action:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}

	.af-action--destructive:hover {
		color: var(--status-danger-700);
		border-color: var(--status-danger-500);
	}

	@media (min-width: 768px) {
		.af-header {
			flex-direction: row;
			justify-content: space-between;
			align-items: flex-end;
		}
	}
</style>
