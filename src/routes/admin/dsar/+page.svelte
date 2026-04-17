<script lang="ts">
	import { onMount } from 'svelte';
	import { api, ApiError } from '$lib/api/client';
	import { Button, Dialog, EmptyState, Spinner } from '$lib/components/shared';
	import ShieldCheck from 'phosphor-svelte/lib/ShieldCheck';
	import CheckCircle from 'phosphor-svelte/lib/CheckCircle';
	import CaretLeft from 'phosphor-svelte/lib/CaretLeft';
	import CaretRight from 'phosphor-svelte/lib/CaretRight';
	import type { components } from '$lib/api/schema';

	type DsarRow = components['schemas']['DsarRow'];
	type DsarListResponse = components['schemas']['DsarListResponse'];
	type DsarFulfillResponse = components['schemas']['DsarFulfillResponse'];

	type StatusFilter =
		| ''
		| 'pending'
		| 'verifying'
		| 'in_progress'
		| 'fulfilled'
		| 'denied'
		| 'cancelled';

	const STATUS_OPTIONS: readonly { readonly value: StatusFilter; readonly label: string }[] = [
		{ value: '', label: 'All' },
		{ value: 'pending', label: 'Pending' },
		{ value: 'verifying', label: 'Verifying' },
		{ value: 'in_progress', label: 'In progress' },
		{ value: 'fulfilled', label: 'Fulfilled' },
		{ value: 'denied', label: 'Denied' },
		{ value: 'cancelled', label: 'Cancelled' }
	];

	let rows = $state<DsarRow[]>([]);
	let total = $state(0);
	let page = $state(1);
	let perPage = $state(25);
	let totalPages = $state(1);
	let statusFilter = $state<StatusFilter>('');
	let loading = $state(true);
	let errorMessage = $state<string | null>(null);

	let fulfillOpen = $state(false);
	let fulfillTarget = $state<DsarRow | null>(null);
	let fulfillUrl = $state('');
	let fulfillNotes = $state('');
	let fulfillBusy = $state(false);
	let fulfillMessage = $state<string | null>(null);
	let lastExport = $state<DsarFulfillResponse['export'] | null>(null);

	let pending = $derived(rows.filter((r) => r.status === 'pending').length);

	async function load() {
		loading = true;
		errorMessage = null;
		try {
			const parts = [
				`page=${encodeURIComponent(String(page))}`,
				`perPage=${encodeURIComponent(String(perPage))}`
			];
			if (statusFilter) parts.push(`status=${encodeURIComponent(statusFilter)}`);
			const resp = await api.get<DsarListResponse>(`/api/admin/consent/dsar?${parts.join('&')}`);
			rows = resp.data;
			total = resp.total;
			totalPages = Math.max(1, resp.totalPages);
		} catch (err) {
			errorMessage =
				err instanceof ApiError ? err.message : 'Could not load DSAR requests.';
		} finally {
			loading = false;
		}
	}

	onMount(load);

	function onFilterChange(e: Event) {
		const t = e.currentTarget as HTMLSelectElement;
		statusFilter = t.value as StatusFilter;
		page = 1;
		void load();
	}

	function openFulfill(row: DsarRow) {
		fulfillTarget = row;
		fulfillUrl = '';
		fulfillNotes = '';
		fulfillMessage = null;
		lastExport = null;
		fulfillOpen = true;
	}

	function closeFulfill() {
		fulfillOpen = false;
		fulfillTarget = null;
		fulfillBusy = false;
	}

	async function submitFulfill() {
		if (!fulfillTarget) return;
		fulfillBusy = true;
		fulfillMessage = null;
		try {
			const body: Record<string, unknown> = {};
			if (fulfillUrl.trim()) body.fulfillmentUrl = fulfillUrl.trim();
			if (fulfillNotes.trim()) body.adminNotes = fulfillNotes.trim();
			const resp = await api.post<DsarFulfillResponse>(
				`/api/admin/consent/dsar/${fulfillTarget.id}/fulfill`,
				body
			);
			lastExport = resp.export ?? null;
			fulfillMessage = 'Request marked as fulfilled.';
			await load();
		} catch (err) {
			fulfillMessage = err instanceof ApiError ? err.message : 'Failed to fulfill.';
		} finally {
			fulfillBusy = false;
		}
	}

	function downloadExport() {
		if (!lastExport) return;
		const blob = new Blob([JSON.stringify(lastExport, null, 2)], {
			type: 'application/json'
		});
		const url = URL.createObjectURL(blob);
		const a = document.createElement('a');
		a.href = url;
		a.download = `dsar-${fulfillTarget?.id ?? 'export'}.json`;
		document.body.appendChild(a);
		a.click();
		document.body.removeChild(a);
		URL.revokeObjectURL(url);
	}

	function prettyStatus(s: string): string {
		return s.replace(/_/g, ' ');
	}

	function prettyKind(s: string): string {
		return s.replace(/_/g, ' ');
	}

	function formatDate(iso: string): string {
		try {
			return new Date(iso).toLocaleString();
		} catch {
			return iso;
		}
	}
</script>

<svelte:head>
	<title>DSAR Requests - Admin - Precision Options Signals</title>
</svelte:head>

<section class="dsar">
	<header class="dsar__header">
		<div>
			<h1 class="dsar__title">Data Subject Access Requests</h1>
			<p class="dsar__subtitle">
				{total} total · {pending} pending on this page
			</p>
		</div>
		<label class="dsar__filter">
			<span class="dsar__filter-label">Status</span>
			<select class="dsar__filter-select" value={statusFilter} onchange={onFilterChange}>
				{#each STATUS_OPTIONS as opt (opt.value)}
					<option value={opt.value}>{opt.label}</option>
				{/each}
			</select>
		</label>
	</header>

	{#if loading}
		<div class="dsar__loading">
			<Spinner size="md" label="Loading DSAR requests" />
		</div>
	{:else if errorMessage}
		<div class="dsar__error" role="alert">{errorMessage}</div>
	{:else if rows.length === 0}
		<EmptyState
			title="No DSAR requests"
			description="When visitors submit access, delete, portability, rectification, or opt-out requests they will appear here."
		>
			{#snippet icon()}
				<ShieldCheck size={40} weight="light" />
			{/snippet}
		</EmptyState>
	{:else}
		<div class="dsar__table-wrap">
			<table class="dsar__table">
				<thead>
					<tr>
						<th scope="col">Submitted</th>
						<th scope="col">Email</th>
						<th scope="col">Kind</th>
						<th scope="col">Status</th>
						<th scope="col">Actions</th>
					</tr>
				</thead>
				<tbody>
					{#each rows as row (row.id)}
						<tr>
							<td>{formatDate(row.createdAt)}</td>
							<td class="dsar__email">{row.email}</td>
							<td>
								<span class="dsar__kind">{prettyKind(row.kind)}</span>
							</td>
							<td>
								<span class="dsar__status dsar__status--{row.status}">
									{prettyStatus(row.status)}
								</span>
							</td>
							<td>
								{#if row.status === 'fulfilled'}
									<span class="dsar__fulfilled-at">
										<CheckCircle size={14} weight="fill" />
										{row.fulfilledAt ? formatDate(row.fulfilledAt) : ''}
									</span>
								{:else}
									<Button size="sm" variant="primary" onclick={() => openFulfill(row)}>
										Fulfill
									</Button>
								{/if}
							</td>
						</tr>
					{/each}
				</tbody>
			</table>
		</div>

		{#if totalPages > 1}
			<div class="dsar__pagination">
				<Button
					size="sm"
					variant="secondary"
					disabled={page <= 1}
					onclick={() => {
						page--;
						void load();
					}}
				>
					{#snippet iconLeading()}<CaretLeft size={14} />{/snippet}
					Prev
				</Button>
				<span class="dsar__page-info">Page {page} of {totalPages}</span>
				<Button
					size="sm"
					variant="secondary"
					disabled={page >= totalPages}
					onclick={() => {
						page++;
						void load();
					}}
				>
					{#snippet iconTrailing()}<CaretRight size={14} />{/snippet}
					Next
				</Button>
			</div>
		{/if}
	{/if}
</section>

<Dialog
	bind:open={fulfillOpen}
	onclose={closeFulfill}
	title="Fulfill DSAR request"
	description={fulfillTarget
		? `${prettyKind(fulfillTarget.kind)} request from ${fulfillTarget.email}`
		: undefined}
	size="md"
>
	<div class="dsar__dialog">
			<p class="dsar__hint">
				Leave the URL blank for access / portability requests — the server will generate a
				JSON export and inline it as a data: URI. Delete / rectification / opt-out requests
				require manual follow-up; record what was done in the notes field.
			</p>
			<label class="dsar__field">
				<span class="dsar__field-label">Fulfillment URL (optional)</span>
				<input
					type="url"
					class="dsar__input"
					bind:value={fulfillUrl}
					placeholder="https://…"
					disabled={fulfillBusy}
				/>
			</label>
			<label class="dsar__field">
				<span class="dsar__field-label">Admin notes (optional)</span>
				<textarea
					class="dsar__textarea"
					bind:value={fulfillNotes}
					rows="4"
					disabled={fulfillBusy}
				></textarea>
			</label>
			{#if fulfillMessage}
				<div class="dsar__result" role="status">{fulfillMessage}</div>
			{/if}
			{#if lastExport}
				<Button variant="secondary" size="sm" onclick={downloadExport}>
					Download export JSON
				</Button>
			{/if}
		</div>
	{#snippet footer()}
		<Button variant="tertiary" onclick={closeFulfill} disabled={fulfillBusy}>
			Cancel
		</Button>
		<Button variant="primary" onclick={submitFulfill} loading={fulfillBusy}>
			Mark fulfilled
		</Button>
	{/snippet}
</Dialog>

<style>
	.dsar {
		display: flex;
		flex-direction: column;
		gap: 1rem;
	}

	.dsar__header {
		display: flex;
		flex-direction: column;
		gap: 0.75rem;
		margin-bottom: 0.5rem;
	}

	.dsar__title {
		font-size: var(--fs-xl);
		font-weight: var(--w-bold);
		color: var(--color-white);
		font-family: var(--font-heading);
	}

	.dsar__subtitle {
		font-size: var(--fs-xs);
		color: var(--color-grey-400);
	}

	.dsar__filter {
		display: flex;
		align-items: center;
		gap: 0.5rem;
	}

	.dsar__filter-label {
		font-size: var(--fs-xs);
		color: var(--color-grey-400);
		text-transform: uppercase;
		letter-spacing: 0.05em;
	}

	.dsar__filter-select {
		padding: 0.4rem 0.6rem;
		background-color: var(--color-navy-mid);
		color: var(--color-white);
		font-size: var(--fs-sm);
		border: 1px solid rgba(255, 255, 255, 0.08);
		border-radius: var(--radius-md);
	}

	.dsar__loading {
		display: flex;
		justify-content: center;
		padding: 3rem 0;
	}

	.dsar__error {
		padding: 0.85rem 1rem;
		background-color: rgba(239, 68, 68, 0.08);
		border: 1px solid rgba(239, 68, 68, 0.2);
		border-radius: var(--radius-lg);
		color: #fca5a5;
		font-size: var(--fs-sm);
	}

	.dsar__table-wrap {
		overflow-x: auto;
		background-color: var(--color-navy-mid);
		border: 1px solid rgba(255, 255, 255, 0.06);
		border-radius: var(--radius-xl);
	}

	.dsar__table {
		width: 100%;
		border-collapse: collapse;
	}

	.dsar__table th {
		text-align: left;
		font-size: var(--fs-xs);
		font-weight: var(--w-semibold);
		color: var(--color-grey-400);
		text-transform: uppercase;
		letter-spacing: 0.05em;
		padding: 0.85rem 1rem;
		border-bottom: 1px solid rgba(255, 255, 255, 0.06);
	}

	.dsar__table td {
		padding: 0.85rem 1rem;
		font-size: var(--fs-sm);
		color: var(--color-grey-300);
		border-bottom: 1px solid rgba(255, 255, 255, 0.04);
	}

	.dsar__email {
		color: var(--color-white);
		font-weight: var(--w-medium);
	}

	.dsar__kind,
	.dsar__status {
		font-size: var(--fs-xs);
		font-weight: var(--w-semibold);
		padding: 0.15rem 0.55rem;
		border-radius: var(--radius-full);
		text-transform: capitalize;
		white-space: nowrap;
	}

	.dsar__kind {
		background-color: rgba(15, 164, 175, 0.12);
		color: var(--color-teal-light);
	}

	.dsar__status--pending {
		background-color: rgba(212, 168, 67, 0.12);
		color: var(--color-gold-light);
	}

	.dsar__status--verifying,
	.dsar__status--in_progress {
		background-color: rgba(59, 130, 246, 0.12);
		color: #60a5fa;
	}

	.dsar__status--fulfilled {
		background-color: rgba(34, 197, 94, 0.12);
		color: #4ade80;
	}

	.dsar__status--denied,
	.dsar__status--cancelled {
		background-color: rgba(239, 68, 68, 0.08);
		color: #fca5a5;
	}

	.dsar__fulfilled-at {
		display: inline-flex;
		align-items: center;
		gap: 0.35rem;
		color: #4ade80;
		font-size: var(--fs-xs);
	}

	.dsar__pagination {
		display: flex;
		align-items: center;
		justify-content: center;
		gap: 0.75rem;
		margin-top: 1rem;
	}

	.dsar__page-info {
		font-size: var(--fs-xs);
		color: var(--color-grey-400);
	}

	.dsar__dialog {
		display: flex;
		flex-direction: column;
		gap: 0.85rem;
	}

	.dsar__hint {
		font-size: var(--fs-xs);
		color: var(--color-grey-400);
		line-height: 1.5;
	}

	.dsar__field {
		display: flex;
		flex-direction: column;
		gap: 0.35rem;
	}

	.dsar__field-label {
		font-size: var(--fs-xs);
		color: var(--color-grey-400);
		text-transform: uppercase;
		letter-spacing: 0.05em;
	}

	.dsar__input,
	.dsar__textarea {
		width: 100%;
		padding: 0.55rem 0.75rem;
		background-color: var(--color-navy-mid);
		color: var(--color-white);
		font-size: var(--fs-sm);
		border: 1px solid rgba(255, 255, 255, 0.08);
		border-radius: var(--radius-md);
	}

	.dsar__textarea {
		resize: vertical;
		min-height: 4.5rem;
		font-family: inherit;
	}

	.dsar__result {
		padding: 0.5rem 0.75rem;
		background-color: rgba(34, 197, 94, 0.08);
		border: 1px solid rgba(34, 197, 94, 0.2);
		border-radius: var(--radius-md);
		color: #4ade80;
		font-size: var(--fs-xs);
	}

	@media (min-width: 768px) {
		.dsar__header {
			flex-direction: row;
			align-items: center;
			justify-content: space-between;
		}

		.dsar__title {
			font-size: var(--fs-2xl);
		}
	}
</style>
