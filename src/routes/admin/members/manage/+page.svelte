<script lang="ts">
	import { onMount } from 'svelte';
	import UsersIcon from 'phosphor-svelte/lib/UsersIcon';
	import MagnifyingGlassIcon from 'phosphor-svelte/lib/MagnifyingGlassIcon';
	import UserPlusIcon from 'phosphor-svelte/lib/UserPlusIcon';
	import ArrowClockwiseIcon from 'phosphor-svelte/lib/ArrowClockwiseIcon';
	import { api, ApiError } from '$lib/api/client';
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
	let cSendSetupEmail = $state(true);
	let createResult = $state<{
		id: string;
		needsPwSetup: boolean;
		inviteDispatched?: boolean | undefined;
	} | null>(null);

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
			let inviteDispatched: boolean | undefined;
			if (res.requires_password_setup && cSendSetupEmail && !cTempPw.trim()) {
				try {
					const fr = await api.post<{ reset_url_dispatched: boolean }>(
						`/api/admin/members/${res.user.id}/force-password-reset`,
						{}
					);
					inviteDispatched = fr.reset_url_dispatched;
				} catch (invErr) {
					flash(
						invErr instanceof ApiError
							? `Created ${res.user.email}, but invite email failed: ${invErr.message}`
							: `Created ${res.user.email}, but invite email failed.`
					);
					createResult = {
						id: res.user.id,
						needsPwSetup: res.requires_password_setup,
						inviteDispatched: false
					};
					cEmail = '';
					cName = '';
					cTempPw = '';
					cVerified = false;
					await refresh();
					return;
				}
			}
			createResult = {
				id: res.user.id,
				needsPwSetup: res.requires_password_setup,
				inviteDispatched
			};
			flash(
				res.requires_password_setup && !cTempPw.trim() && cSendSetupEmail
					? inviteDispatched
						? `Created ${res.user.email} — setup email sent`
						: `Created ${res.user.email} — account saved, but email was not dispatched (check email / notifications)`
					: `Created ${res.user.email}`
			);
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

	function statusBadge(u: {
		suspended_at?: string | null;
		banned_at?: string | null;
		email_verified_at?: string | null;
	}) {
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
			<UsersIcon size={28} weight="duotone" />
			<h1 class="page__title">Members — search &amp; manual create</h1>
		</div>
		<p class="page__subtitle">
			Indexed search backed by <code>pg_trgm</code> over email + name. Filters by role and
			lifecycle status. Manual create gates on <code>admin.member.create</code>. Administrator
			accounts require <code>admin.role.manage</code>. Leaving the temp password blank
			disables login until the user sets a password — you can send a setup link by checking
			&quot;Send password setup email&quot; (outbound email must be configured).
		</p>
	</header>

	{#if toast}<div class="toast">{toast}</div>{/if}
	{#if error}<div class="error" role="alert">{error}</div>{/if}

	<form class="filters card" onsubmit={applyFilters}>
		<div class="filters__grid">
			<div class="field field--wide">
				<label class="field__label" for="m-q">Search</label>
				<div class="search-input">
					<MagnifyingGlassIcon size={16} />
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
					<ArrowClockwiseIcon size={16} weight="bold" />
				</button>
				<button
					class="btn btn--primary"
					type="button"
					onclick={() => (showCreate = !showCreate)}
				>
					<UserPlusIcon size={16} weight="bold" />
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
						<option value="admin">admin</option>
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
				<label class="field field--checkbox" class:field--disabled={!!cTempPw.trim()}>
					<input
						type="checkbox"
						bind:checked={cSendSetupEmail}
						disabled={!!cTempPw.trim()}
					/>
					<span
						>Send password setup email (uses the same secure link as &quot;force
						reset&quot;)</span
					>
				</label>
				<label class="field field--checkbox">
					<input type="checkbox" bind:checked={cVerified} />
					<span>Mark email verified</span>
				</label>
				<div class="form-actions">
					<button
						class="btn btn--ghost"
						type="button"
						onclick={() => (showCreate = false)}
					>
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
						{#if createResult.inviteDispatched === true}
							— setup link emailed (or queued).
						{:else if createResult.inviteDispatched === false}
							— account awaits a password; invite was not sent (check logs / email
							config) or use Force reset from the member profile.
						{:else}
							— created in disabled state; send a reset link from the member profile
							or run create again with &quot;Send password setup email&quot;.
						{/if}
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
						{@const s = statusBadge(
							u as {
								suspended_at?: string | null;
								banned_at?: string | null;
								email_verified_at?: string | null;
							}
						)}
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
			<button class="btn btn--ghost" disabled={(envelope.page ?? 1) <= 1} onclick={prevPage}>
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
	/* Layout matches the rest of admin (audit, security, etc.) — width is
	 * already bounded by `.admin__shell-inner` (75rem) in the layout, this
	 * page just provides bottom breathing room. No max-width here so we
	 * inherit the layout's responsive container. */
	.page {
		padding: 0 0 3rem;
	}
	.page__header {
		margin-bottom: 1.25rem;
	}
	.page__title-row {
		display: flex;
		align-items: flex-start;
		gap: 0.85rem;
		color: var(--color-white);
	}
	.page__title {
		margin: 0;
		font-family: var(--font-heading);
		font-size: 1.5rem;
		font-weight: 700;
		letter-spacing: -0.01em;
		line-height: 1.2;
		color: var(--color-white);
	}
	.page__subtitle {
		margin: 0.5rem 0 0;
		font-size: 0.875rem;
		line-height: 1.5;
		color: var(--color-grey-400);
		max-width: 60rem;
	}
	.page__subtitle code {
		font-size: 0.85em;
		padding: 0.1em 0.35em;
		border-radius: 0.25rem;
		background: rgba(255, 255, 255, 0.06);
		color: var(--color-grey-200);
	}
	.toast {
		padding: 0.85rem 1rem;
		background: rgba(34, 181, 115, 0.12);
		border: 1px solid rgba(34, 181, 115, 0.25);
		color: var(--color-green);
		border-radius: var(--radius-2xl);
		font-size: 0.875rem;
		margin-bottom: 1rem;
	}
	.error {
		padding: 0.85rem 1rem;
		background: rgba(239, 68, 68, 0.1);
		border: 1px solid rgba(239, 68, 68, 0.3);
		color: #fca5a5;
		border-radius: var(--radius-2xl);
		font-size: 0.875rem;
		margin-bottom: 1rem;
	}
	.muted {
		color: var(--color-grey-400);
		font-size: 0.875rem;
		padding: 1.5rem 1rem;
		text-align: center;
	}
	.card {
		background: rgba(19, 43, 80, 0.35);
		backdrop-filter: blur(24px);
		-webkit-backdrop-filter: blur(24px);
		border: 1px solid rgba(255, 255, 255, 0.06);
		border-radius: var(--radius-2xl);
		padding: 1.25rem;
		margin-bottom: 1.25rem;
		box-shadow:
			0 1px 0 rgba(255, 255, 255, 0.03) inset,
			0 12px 32px rgba(0, 0, 0, 0.18);
	}
	.card__title {
		margin: 0 0 1rem 0;
		font-family: var(--font-heading);
		font-size: 1.0625rem;
		font-weight: 700;
		color: var(--color-white);
	}
	.filters__grid {
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(min(100%, 200px), 1fr));
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
		grid-column: 1 / -1;
		flex-direction: row;
		flex-wrap: wrap;
		gap: var(--space-2);
		align-items: center;
	}
	.field--actions .btn {
		flex: 0 1 auto;
	}
	.field--actions .btn--ghost {
		flex: 0 0 auto;
		min-width: 2.75rem;
		min-height: 2.75rem;
		justify-content: center;
		padding-inline: var(--space-2-5);
	}
	.field--checkbox {
		flex-direction: row;
		align-items: center;
		gap: var(--space-2);
		font-size: var(--fs-sm);
		color: var(--color-grey-300);
	}
	.field--disabled {
		opacity: 0.55;
	}
	.field__label {
		font-size: var(--fs-xs);
		color: var(--color-grey-300);
	}
	.field__input {
		min-height: 2.5rem;
		padding: var(--space-2-5) var(--space-3);
		background: rgba(255, 255, 255, 0.05);
		border: 1px solid rgba(255, 255, 255, 0.1);
		color: var(--color-white);
		border-radius: var(--radius-lg);
		font-size: var(--fs-sm);
		font-family: inherit;
		color-scheme: dark;
		width: 100%;
		transition:
			border-color 150ms var(--ease-out),
			box-shadow 150ms var(--ease-out);
	}
	.field__input::placeholder {
		color: var(--color-grey-500);
	}
	.field__input:focus {
		outline: none;
		border-color: var(--color-teal);
		box-shadow: 0 0 0 3px rgba(15, 164, 175, 0.15);
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
		justify-content: center;
		gap: var(--space-2);
		min-height: 2.5rem;
		padding: 0 1rem;
		border-radius: var(--radius-lg);
		font-size: var(--fs-sm);
		font-weight: var(--w-semibold);
		border: 1px solid transparent;
		background: transparent;
		color: var(--color-grey-300);
		cursor: pointer;
		white-space: nowrap;
		transition:
			background-color 150ms var(--ease-out),
			border-color 150ms var(--ease-out),
			color 150ms var(--ease-out),
			box-shadow 150ms var(--ease-out),
			transform 150ms var(--ease-out);
	}
	.btn:disabled {
		opacity: 0.4;
		cursor: not-allowed;
	}
	.btn--primary {
		background: linear-gradient(135deg, var(--color-teal), var(--color-teal-dark));
		color: var(--color-white);
		border-color: var(--color-teal-dark);
		box-shadow:
			0 1px 0 rgba(255, 255, 255, 0.15) inset,
			0 4px 12px rgba(15, 164, 175, 0.2);
	}
	.btn--primary:hover:not(:disabled) {
		transform: translateY(-1px);
		box-shadow:
			0 1px 0 rgba(255, 255, 255, 0.18) inset,
			0 6px 16px rgba(15, 164, 175, 0.32);
	}
	.btn--ghost {
		border-color: rgba(255, 255, 255, 0.12);
		background: rgba(255, 255, 255, 0.03);
		color: var(--color-grey-200);
	}
	.btn--ghost:hover:not(:disabled) {
		background: rgba(255, 255, 255, 0.08);
		color: var(--color-white);
		border-color: rgba(255, 255, 255, 0.2);
	}
	.create-form {
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(min(100%, 200px), 1fr));
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
		border-radius: var(--radius-2xl);
		font-size: var(--fs-sm);
		color: var(--color-grey-200);
	}
	.table-wrap {
		overflow-x: auto;
		padding: 0;
	}
	.table {
		width: 100%;
		border-collapse: collapse;
		font-size: var(--fs-sm);
	}
	.table th {
		text-align: left;
		font-weight: 600;
		font-size: 0.75rem;
		color: var(--color-grey-400);
		text-transform: uppercase;
		letter-spacing: 0.05em;
		padding: 0.85rem 1rem;
		border-bottom: 1px solid rgba(255, 255, 255, 0.08);
		background: rgba(255, 255, 255, 0.02);
	}
	.table td {
		padding: 0.85rem 1rem;
		border-bottom: 1px solid rgba(255, 255, 255, 0.04);
		color: var(--color-grey-200);
		vertical-align: middle;
	}
	.table tbody tr {
		transition: background-color 120ms var(--ease-out);
	}
	.table tbody tr:hover {
		background-color: rgba(255, 255, 255, 0.02);
	}
	.table tbody tr:last-child td {
		border-bottom: none;
	}
	.badge {
		display: inline-flex;
		align-items: center;
		padding: 0.2rem 0.6rem;
		border-radius: var(--radius-full);
		font-size: 0.6875rem;
		font-weight: 700;
		text-transform: uppercase;
		letter-spacing: 0.04em;
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

	@media (max-width: 36rem) {
		.page__header {
			overflow-wrap: anywhere;
		}
		.page__title-row {
			flex-wrap: wrap;
		}
		.filters__grid {
			grid-template-columns: 1fr;
		}
		.field--wide {
			grid-column: auto;
		}
		.field--actions {
			display: grid;
			grid-template-columns: 1fr auto;
			align-items: stretch;
		}
		.field--actions .btn:nth-child(1) {
			grid-column: 1;
		}
		.field--actions .btn:nth-child(2) {
			grid-column: 2;
		}
		.field--actions .btn:nth-child(3) {
			grid-column: 1 / -1;
		}
		.pager {
			flex-wrap: wrap;
			row-gap: var(--space-2);
		}
	}
</style>
