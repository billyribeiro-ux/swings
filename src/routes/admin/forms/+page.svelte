<!--
  FORM-09: Forms list.

  Row actions:
  - Edit           → /admin/forms/{id}         (builder)
  - Submissions    → /admin/forms/{id}/submissions
  - Archive        → PATCH /admin/forms/{id} { is_active: false } (soft)
  - Unarchive      → PATCH /admin/forms/{id} { is_active: true }
  - Preview        → /admin/forms/{id}/preview
  - Versions       → /admin/forms/{id}/versions
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
	import StackIcon from 'phosphor-svelte/lib/StackIcon';
	import WarningIcon from 'phosphor-svelte/lib/WarningIcon';

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

<div class="forms-page">
	<header class="forms-page__header">
		<div class="forms-page__title-row">
			<StackIcon size={28} weight="duotone" />
			<div class="forms-page__copy">
				<h1 class="forms-page__title">Forms</h1>
				<p class="forms-page__subtitle">Build, preview, and manage collection forms.</p>
			</div>
		</div>
		<button class="btn btn--primary" type="button" onclick={() => goto('/admin/forms/new')}>
			<PlusIcon size={16} weight="bold" />
			<span>New form</span>
		</button>
	</header>

	{#if err}
		<div class="error" role="alert">
			<WarningIcon size={16} weight="fill" />
			<span>{err}</span>
		</div>
	{/if}

	{#if loading}
		<div class="state state--loading">
			<div class="state__spinner" aria-hidden="true"></div>
			<span>Loading forms…</span>
		</div>
	{:else if forms.length === 0}
		<div class="empty">
			<StackIcon size={48} weight="duotone" />
			<p class="empty__title">No forms yet</p>
			<p class="empty__sub">Create one to start collecting submissions.</p>
			<button class="btn btn--primary" type="button" onclick={() => goto('/admin/forms/new')}>
				<PlusIcon size={16} weight="bold" />
				<span>Create your first form</span>
			</button>
		</div>
	{:else}
		<section class="card table-card">
			<div class="table-wrap">
				<table class="table">
					<thead>
						<tr>
							<th scope="col">Name</th>
							<th scope="col">Slug</th>
							<th scope="col">Status</th>
							<th scope="col">Updated</th>
							<th scope="col" class="table__actions-th">Actions</th>
						</tr>
					</thead>
					<tbody>
						{#each forms as f (f.id)}
							<tr>
								<td>
									<a class="table__name" href={`/admin/forms/${f.id}`}>{f.name}</a>
									{#if f.description}<div class="table__desc">{f.description}</div>{/if}
								</td>
								<td><code class="table__slug">{f.slug}</code></td>
								<td>
									<span class="pill {f.is_active ? 'pill--success' : 'pill--neutral'}">
										{f.is_active ? 'Active' : 'Archived'}
									</span>
								</td>
								<td class="table__ts">{new Date(f.updated_at).toLocaleString()}</td>
								<td>
									<div class="actions">
										<a
											class="action-btn"
											href={`/admin/forms/${f.id}`}
											aria-label={`Edit ${f.name}`}
											title="Edit"
										>
											<PencilSimpleIcon size={14} weight="bold" />
											<span>Edit</span>
										</a>
										<a
											class="action-btn"
											href={`/admin/forms/${f.id}/preview`}
											aria-label={`Preview ${f.name}`}
											title="Preview"
										>
											<EyeIcon size={14} weight="bold" />
											<span>Preview</span>
										</a>
										<a
											class="action-btn"
											href={`/admin/forms/${f.id}/submissions`}
											aria-label={`Submissions for ${f.name}`}
											title="Submissions"
										>
											<ListBulletsIcon size={14} weight="bold" />
											<span>Submissions</span>
										</a>
										<a
											class="action-btn"
											href={`/admin/forms/${f.id}/versions`}
											aria-label={`Versions of ${f.name}`}
											title="Versions"
										>
											<ClockCounterClockwiseIcon size={14} weight="bold" />
											<span>Versions</span>
										</a>
										<button
											type="button"
											class="action-btn action-btn--destructive"
											onclick={() => toggleArchive(f)}
											disabled={acting === f.id}
											aria-label={f.is_active ? `Archive ${f.name}` : `Restore ${f.name}`}
											title={f.is_active ? 'Archive' : 'Restore'}
										>
											{#if f.is_active}
												<ArchiveIcon size={14} weight="bold" />
												<span>Archive</span>
											{:else}
												<ArrowCounterClockwiseIcon size={14} weight="bold" />
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
		</section>
	{/if}
</div>

<style>
	.forms-page {
		max-width: 80rem;
		padding: 0 0 3rem;
	}
	.forms-page__header {
		display: flex;
		flex-direction: column;
		gap: 1rem;
		margin-bottom: 1.5rem;
	}
	.forms-page__title-row {
		display: flex;
		align-items: flex-start;
		gap: 0.85rem;
		color: var(--color-white);
	}
	.forms-page__copy {
		min-width: 0;
	}
	.forms-page__title {
		margin: 0;
		font-family: var(--font-heading);
		font-size: 1.5rem;
		font-weight: 700;
		color: var(--color-white);
		letter-spacing: -0.01em;
		line-height: 1.2;
	}
	.forms-page__subtitle {
		margin: 0.35rem 0 0;
		font-size: 0.875rem;
		color: var(--color-grey-400);
		max-width: 42rem;
		line-height: 1.5;
	}

	.btn {
		display: inline-flex;
		align-items: center;
		gap: 0.5rem;
		min-height: 2.5rem;
		padding: 0 0.875rem;
		border-radius: var(--radius-lg);
		font-size: 0.875rem;
		font-weight: 600;
		font-family: inherit;
		border: 1px solid transparent;
		background: transparent;
		color: var(--color-grey-300);
		cursor: pointer;
		text-decoration: none;
		align-self: flex-start;
		transition:
			background-color 150ms,
			border-color 150ms,
			color 150ms,
			box-shadow 150ms,
			transform 150ms;
	}
	.btn--primary {
		background: linear-gradient(135deg, var(--color-teal), var(--color-teal-dark, #0d8a94));
		color: var(--color-white);
		box-shadow: 0 6px 16px -4px rgba(15, 164, 175, 0.45);
	}
	.btn--primary:hover:not(:disabled) {
		transform: translateY(-1px);
		box-shadow: 0 8px 18px -4px rgba(15, 164, 175, 0.55);
	}
	.btn:focus-visible {
		outline: none;
		box-shadow: 0 0 0 3px rgba(15, 164, 175, 0.35);
	}

	.error {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		padding: 0.75rem 1rem;
		border-radius: var(--radius-lg);
		font-size: 0.875rem;
		margin-bottom: 1rem;
		background: rgba(239, 68, 68, 0.1);
		border: 1px solid rgba(239, 68, 68, 0.3);
		color: #fca5a5;
	}

	.state {
		display: flex;
		align-items: center;
		justify-content: center;
		gap: 0.75rem;
		padding: 4rem 0;
		color: var(--color-grey-400);
		font-size: 0.875rem;
	}
	.state__spinner {
		width: 1.25rem;
		height: 1.25rem;
		border: 2px solid rgba(255, 255, 255, 0.1);
		border-top-color: var(--color-teal);
		border-radius: 50%;
		animation: spin 0.7s linear infinite;
	}
	@keyframes spin {
		to {
			transform: rotate(360deg);
		}
	}

	.empty {
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 0.5rem;
		padding: 3rem 1rem;
		background: var(--color-navy-mid);
		border: 1px dashed rgba(255, 255, 255, 0.1);
		border-radius: var(--radius-xl);
		color: var(--color-grey-500);
		text-align: center;
	}
	.empty__title {
		margin: 0.5rem 0 0;
		font-size: 1rem;
		font-weight: 600;
		color: var(--color-white);
	}
	.empty__sub {
		margin: 0 0 0.5rem;
		font-size: 0.875rem;
		color: var(--color-grey-400);
	}

	.card {
		background: var(--color-navy-mid);
		border: 1px solid rgba(255, 255, 255, 0.06);
		border-radius: var(--radius-xl);
		box-shadow:
			0 1px 0 rgba(255, 255, 255, 0.03) inset,
			0 12px 32px rgba(0, 0, 0, 0.18);
	}
	.table-card {
		overflow: hidden;
	}
	.table-wrap {
		overflow-x: auto;
	}
	.table {
		inline-size: 100%;
		border-collapse: collapse;
		font-size: 0.875rem;
	}
	.table th,
	.table td {
		padding: 0.875rem 1rem;
		text-align: left;
		vertical-align: top;
		border-bottom: 1px solid rgba(255, 255, 255, 0.04);
	}
	.table th {
		font-size: 0.6875rem;
		font-weight: 700;
		text-transform: uppercase;
		letter-spacing: 0.05em;
		color: var(--color-grey-500);
		background: rgba(255, 255, 255, 0.02);
		border-bottom: 1px solid rgba(255, 255, 255, 0.06);
		white-space: nowrap;
	}
	.table__actions-th {
		text-align: right;
	}
	.table tbody tr:hover td {
		background: rgba(255, 255, 255, 0.02);
	}
	.table tbody tr:last-child td {
		border-bottom: none;
	}
	.table__name {
		color: var(--color-white);
		font-weight: 600;
		text-decoration: none;
		transition: color 150ms;
	}
	.table__name:hover {
		color: var(--color-teal-light);
	}
	.table__desc {
		margin-top: 0.25rem;
		font-size: 0.75rem;
		color: var(--color-grey-400);
		line-height: 1.45;
	}
	.table__slug {
		font-family: var(--font-mono);
		font-size: 0.75rem;
		background: rgba(255, 255, 255, 0.06);
		padding: 0.125rem 0.4rem;
		border-radius: var(--radius-sm);
		color: var(--color-grey-200);
	}
	.table__ts {
		color: var(--color-grey-400);
		font-variant-numeric: tabular-nums;
		white-space: nowrap;
	}

	.pill {
		display: inline-flex;
		padding: 0.15rem 0.5rem;
		border-radius: var(--radius-full);
		font-size: 0.6875rem;
		font-weight: 600;
		text-transform: uppercase;
		letter-spacing: 0.05em;
	}
	.pill--success {
		background: rgba(15, 164, 175, 0.12);
		color: #5eead4;
	}
	.pill--neutral {
		background: rgba(255, 255, 255, 0.06);
		color: var(--color-grey-300);
	}

	.actions {
		display: inline-flex;
		gap: 0.4rem;
		flex-wrap: wrap;
		justify-content: flex-end;
	}
	.action-btn {
		display: inline-flex;
		align-items: center;
		gap: 0.35rem;
		min-height: 2rem;
		padding: 0 0.65rem;
		font-size: 0.75rem;
		font-weight: 600;
		border: 1px solid rgba(255, 255, 255, 0.1);
		background: rgba(255, 255, 255, 0.05);
		color: var(--color-grey-200);
		border-radius: var(--radius-lg);
		text-decoration: none;
		cursor: pointer;
		font-family: inherit;
		transition:
			background-color 150ms,
			border-color 150ms,
			color 150ms;
	}
	.action-btn:hover:not(:disabled) {
		background: rgba(255, 255, 255, 0.1);
		border-color: rgba(255, 255, 255, 0.18);
		color: var(--color-white);
	}
	.action-btn:focus-visible {
		outline: none;
		box-shadow: 0 0 0 3px rgba(15, 164, 175, 0.35);
	}
	.action-btn:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}
	.action-btn--destructive {
		background: rgba(239, 68, 68, 0.1);
		border-color: rgba(239, 68, 68, 0.3);
		color: #fca5a5;
	}
	.action-btn--destructive:hover:not(:disabled) {
		background: rgba(239, 68, 68, 0.18);
		border-color: rgba(239, 68, 68, 0.4);
	}

	@media (min-width: 768px) {
		.forms-page__header {
			flex-direction: row;
			justify-content: space-between;
			align-items: flex-start;
		}
	}
</style>
