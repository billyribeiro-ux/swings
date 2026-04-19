<script lang="ts">
	import { onMount } from 'svelte';
	import Users from 'phosphor-svelte/lib/Users';
	import MagnifyingGlass from 'phosphor-svelte/lib/MagnifyingGlass';
	import UserPlus from 'phosphor-svelte/lib/UserPlus';
	import ArrowClockwise from 'phosphor-svelte/lib/ArrowClockwise';
	import { ApiError } from '$lib/api/client';
	import {
		adminMembersTyped,
		type AdminMemberSearchQuery,
		type AdminMemberSearchResponse,
		type CreateMemberRequest
	} from '$lib/api/admin-members';

	let envelope = $state<AdminMemberSearchResponse | null>(null);
	let loading = $state(true);
	let error = $state('');
	let toast = $state('');

	let filters = $state<AdminMemberSearchQuery>({
		q: '',
		role: '',
		status: '',
		limit: 25,
		offset: 0
	});

	let showCreate = $state(false);
	let createBusy = $state(false);
	let cEmail = $state('');
	let cName = $state('');
	let cRole = $state<CreateMemberRequest['role']>('member');
	let cTempPw = $state('');
	let cVerified = $state(false);
	let createResult = $state<{ id: string; needsPwSetup: boolean } | null>(null);

	function flash(msg: string) {
		toast = msg;
		setTimeout(() => (toast = ''), 2500);
	}

	async function refresh() {
		loading = true;
		error = '';
		try {
			envelope = await adminMembersTyped.search(filters);
		} catch (e) {
			error = e instanceof ApiError ? `${e.status}: ${e.message}` : 'Search failed';
		} finally {
			loading = false;
		}
	}

	function applyFilters(e: Event) {
		e.preventDefault();
		filters.offset = 0;
		void refresh();
	}

	function nextPage() {
		if (!envelope) return;
		if ((envelope.page ?? 1) >= (envelope.total_pages ?? 1)) return;
		filters.offset = (filters.offset ?? 0) + (filters.limit ?? 25);
		void refresh();
	}

	function prevPage() {
		filters.offset = Math.max(0, (filters.offset ?? 0) - (filters.limit ?? 25));
		void refresh();
	}

	async function create(e: Event) {
		e.preventDefault();
		createBusy = true;
		error = '';
		createResult = null;
		try {
			const res = await adminMembersTyped.create({
				email: cEmail.trim(),
				name: cName.trim(),
				role: cRole,
				temp_password: cTempPw.trim() ? cTempPw.trim() : undefined,
				email_verified: cVerified
			});
			createResult = { id: res.user.id, needsPwSetup: res.requires_password_setup };
			flash(`Created ${res.user.email}`);
			cEmail = '';
			cName = '';
			cTempPw = '';
			cVerified = false;
			await refresh();
		} catch (e) {
			error = e instanceof ApiError ? `${e.status}: ${e.message}` : 'Create failed';
		} finally {
			createBusy = false;
		}
	}

	function statusBadge(u: { suspended_at?: string | null; banned_at?: string | null; email_verified_at?: string | null }) {
		if (u.banned_at) return { label: 'banned', cls: 'badge--err' };
		if (u.suspended_at) return { label: 'suspended', cls: 'badge--warn' };
		if (!u.email_verified_at) return { label: 'unverified', cls: 'badge--off' };
		return { label: 'active', cls: 'badge--ok' };
	}

	onMount(refresh);
</script>

<svelte:head>
	<title>Members search · Admin</title>
</svelte:head>

<div class="page" data-testid="admin-members-manage-page">
	<header class="page__header">
		<div class="page__title-row">
			<Users size={28} weight="duotone" />
			<h1 class="page__title">Members — search &amp; manual create</h1>
		</div>
		<p class="page__subtitle">
			Indexed search backed by <code>pg_trgm</code> over email + name. Filters by role and
			lifecycle status. Manual create gates on <code>admin.member.create</code>; passing a
			temp password skips the invite flow.
		</p>
	</header>

	{#if toast}<div class="toast">{toast}</div>{/if}
	{#if error}<div class="error" role="alert">{error}</div>{/if}

	<form class="filters card" onsubmit={applyFilters}>
		<div class="filters__grid">
			<div class="field field--wide">
				<label class="field__label" for="m-q">Search</label>
				<div class="search-input">
					<MagnifyingGlass size={16} />
					<input
						id="m-q"
						class="field__input"
						placeholder="email or name substring"
						bind:value={filters.q}
						data-testid="members-q-input"
					/>
				</div>
			</div>
			<div class="field">
				<label class="field__label" for="m-role">Role</label>
				<select id="m-role" class="field__input" bind:value={filters.role}>
					<option value="">Any</option>
					<option value="member">member</option>
					<option value="author">author</option>
					<option value="support">support</option>
					<option value="admin">admin</option>
				</select>
			</div>
			<div class="field">
				<label class="field__label" for="m-status">Status</label>
				<select id="m-status" class="field__input" bind:value={filters.status}>
					<option value="">Any</option>
					<option value="active">active</option>
					<option value="suspended">suspended</option>
					<option value="banned">banned</option>
					<option value="unverified">unverified</option>
				</select>
			</div>
			<div class="field field--actions">
				<button class="btn btn--primary" type="submit">Apply</button>
				<button class="btn btn--ghost" type="button" onclick={refresh}>
					<ArrowClockwise size={16} weight="bold" />
				</button>
				<button
					class="btn btn--primary"
					type="button"
					onclick={() => (showCreate = !showCreate)}
				>
					<UserPlus size={16} weight="bold" />
					New member
				</button>
			</div>
		</div>
	</form>

	{#if showCreate}
		<section class="card">
			<h2 class="card__title">Create member</h2>
			<form class="create-form" onsubmit={create}>
				<div class="field">
					<label class="field__label" for="c-email">Email</label>
					<input
						id="c-email"
						class="field__input"
						type="email"
						bind:value={cEmail}
						required
						data-testid="member-create-email"
					/>
				</div>
				<div class="field">
					<label class="field__label" for="c-name">Name</label>
					<input id="c-name" class="field__input" bind:value={cName} required />
				</div>
				<div class="field">
					<label class="field__label" for="c-role">Role</label>
					<select id="c-role" class="field__input" bind:value={cRole}>
						<option value="member">member</option>
						<option value="author">author</option>
						<option value="support">support</option>
					</select>
				</div>
				<div class="field">
					<label class="field__label" for="c-pw">Temp password (≥ 12 chars)</label>
					<input
						id="c-pw"
						class="field__input"
						type="text"
						placeholder="leave blank → invite via reset"
						minlength="12"
						bind:value={cTempPw}
					/>
				</div>
				<label class="field field--checkbox">
					<input type="checkbox" bind:checked={cVerified} />
					<span>Mark email verified</span>
				</label>
				<div class="form-actions">
					<button class="btn btn--ghost" type="button" onclick={() => (showCreate = false)}>
						Cancel
					</button>
					<button class="btn btn--primary" type="submit" disabled={createBusy}>
						{createBusy ? 'Creating…' : 'Create'}
					</button>
				</div>
			</form>
			{#if createResult}
				<div class="hint">
					<strong>{createResult.id}</strong>
					{#if createResult.needsPwSetup}
						created in disabled state — send reset link to enable login.
					{:else}
						created with the temp password you supplied.
					{/if}
				</div>
			{/if}
		</section>
	{/if}

	{#if loading}
		<p class="muted">Loading…</p>
	{:else if !envelope || envelope.data.length === 0}
		<p class="muted">No matching members.</p>
	{:else}
		<div class="card table-wrap">
			<table class="table">
				<thead>
					<tr>
						<th>Email</th>
						<th>Name</th>
						<th>Role</th>
						<th>Status</th>
						<th>Joined</th>
					</tr>
				</thead>
				<tbody>
					{#each envelope.data as u (u.id)}
						{@const s = statusBadge(u as { suspended_at?: string | null; banned_at?: string | null; email_verified_at?: string | null })}
						<tr>
							<td>{u.email}</td>
							<td>{u.name}</td>
							<td>{u.role}</td>
							<td><span class="badge {s.cls}">{s.label}</span></td>
							<td>{new Date(u.created_at).toLocaleDateString()}</td>
						</tr>
					{/each}
				</tbody>
			</table>
		</div>
		<div class="pager">
			<button
				class="btn btn--ghost"
				disabled={(envelope.page ?? 1) <= 1}
				onclick={prevPage}
			>
				Prev
			</button>
			<span class="pager__info">
				Page {envelope.page} / {envelope.total_pages || 1} · {envelope.total} members
			</span>
			<button
				class="btn btn--ghost"
				disabled={(envelope.page ?? 1) >= (envelope.total_pages ?? 1)}
				onclick={nextPage}
			>
				Next
			</button>
		</div>
	{/if}
</div>

<style>
	.page {
		max-width: 1200px;
	}
	.page__header {
		margin-bottom: var(--space-5);
	}
	.page__title-row {
		display: flex;
		align-items: center;
		gap: var(--space-3);
		color: var(--color-white);
	}
	.page__title {
		font-size: var(--fs-xl);
		font-weight: var(--w-bold);
		font-family: var(--font-heading);
		margin: 0;
	}
	.page__subtitle {
		margin-top: var(--space-2);
		font-size: var(--fs-sm);
		color: var(--color-grey-400);
		max-width: 75ch;
	}
	.toast {
		padding: var(--space-3) var(--space-4);
		background: rgba(34, 181, 115, 0.12);
		border: 1px solid rgba(34, 181, 115, 0.25);
		color: var(--color-green);
		border-radius: var(--radius-lg);
		font-size: var(--fs-sm);
		margin-bottom: var(--space-4);
	}
	.error {
		padding: var(--space-3) var(--space-4);
		background: rgba(239, 68, 68, 0.1);
		border: 1px solid rgba(239, 68, 68, 0.3);
		color: #fca5a5;
		border-radius: var(--radius-lg);
		font-size: var(--fs-sm);
		margin-bottom: var(--space-4);
	}
	.muted {
		color: var(--color-grey-400);
		font-size: var(--fs-sm);
	}
	.card {
		background: var(--color-navy-mid);
		border: 1px solid rgba(255, 255, 255, 0.06);
		border-radius: var(--radius-xl);
		padding: var(--space-5);
		margin-bottom: var(--space-4);
	}
	.card__title {
		font-size: var(--fs-md);
		font-weight: var(--w-bold);
		font-family: var(--font-heading);
		color: var(--color-white);
		margin: 0 0 var(--space-3) 0;
	}
	.filters__grid {
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(180px, 1fr));
		gap: var(--space-3);
		align-items: end;
	}
	.field {
		display: flex;
		flex-direction: column;
		gap: var(--space-1-5);
	}
	.field--wide {
		grid-column: span 2;
	}
	.field--actions {
		flex-direction: row;
		gap: var(--space-2);
		align-items: end;
	}
	.field--checkbox {
		flex-direction: row;
		align-items: center;
		gap: var(--space-2);
		font-size: var(--fs-sm);
		color: var(--color-grey-300);
	}
	.field__label {
		font-size: var(--fs-xs);
		color: var(--color-grey-300);
	}
	.field__input {
		padding: var(--space-2-5) var(--space-3);
		background: rgba(255, 255, 255, 0.05);
		border: 1px solid rgba(255, 255, 255, 0.1);
		color: var(--color-white);
		border-radius: var(--radius-lg);
		font-size: var(--fs-sm);
		width: 100%;
	}
	.field__input:focus {
		outline: none;
		border-color: var(--color-teal);
	}
	.search-input {
		position: relative;
	}
	.search-input :global(svg) {
		position: absolute;
		left: 0.7rem;
		top: 50%;
		transform: translateY(-50%);
		color: var(--color-grey-400);
		pointer-events: none;
	}
	.search-input .field__input {
		padding-left: 2rem;
	}
	.btn {
		display: inline-flex;
		align-items: center;
		gap: var(--space-2);
		padding: var(--space-2) var(--space-3);
		border-radius: var(--radius-lg);
		font-size: var(--fs-sm);
		font-weight: var(--w-semibold);
		border: 1px solid transparent;
		background: transparent;
		color: var(--color-grey-300);
		cursor: pointer;
	}
	.btn:disabled {
		opacity: 0.4;
		cursor: not-allowed;
	}
	.btn--primary {
		background: var(--color-teal);
		color: var(--color-white);
	}
	.btn--ghost {
		border-color: rgba(255, 255, 255, 0.1);
		background: rgba(255, 255, 255, 0.03);
	}
	.btn--ghost:hover:not(:disabled) {
		background: rgba(255, 255, 255, 0.08);
		color: var(--color-white);
	}
	.create-form {
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
		gap: var(--space-3);
		align-items: end;
	}
	.form-actions {
		grid-column: 1 / -1;
		display: flex;
		justify-content: flex-end;
		gap: var(--space-2);
	}
	.hint {
		margin-top: var(--space-3);
		padding: var(--space-3);
		background: rgba(15, 164, 175, 0.08);
		border: 1px solid rgba(15, 164, 175, 0.15);
		border-radius: var(--radius-lg);
		font-size: var(--fs-sm);
		color: var(--color-grey-200);
	}
	.table-wrap {
		overflow-x: auto;
		padding: var(--space-3);
	}
	.table {
		width: 100%;
		border-collapse: collapse;
		font-size: var(--fs-sm);
	}
	.table th {
		text-align: left;
		font-weight: var(--w-medium);
		color: var(--color-grey-400);
		padding: var(--space-2);
		border-bottom: 1px solid rgba(255, 255, 255, 0.08);
	}
	.table td {
		padding: var(--space-2);
		border-bottom: 1px solid rgba(255, 255, 255, 0.04);
		color: var(--color-grey-200);
	}
	.badge {
		display: inline-block;
		padding: 0.1rem 0.5rem;
		border-radius: var(--radius-full);
		font-size: var(--fs-xs);
		font-weight: var(--w-bold);
		text-transform: uppercase;
	}
	.badge--ok {
		background: rgba(34, 181, 115, 0.15);
		color: var(--color-green);
	}
	.badge--warn {
		background: rgba(245, 158, 11, 0.15);
		color: #fbbf24;
	}
	.badge--err {
		background: rgba(239, 68, 68, 0.15);
		color: #fca5a5;
	}
	.badge--off {
		background: rgba(255, 255, 255, 0.06);
		color: var(--color-grey-400);
	}
	.pager {
		display: flex;
		gap: var(--space-3);
		justify-content: center;
		align-items: center;
		margin-top: var(--space-4);
	}
	.pager__info {
		font-size: var(--fs-xs);
		color: var(--color-grey-400);
	}
</style>
