<script lang="ts">
	import { onMount } from 'svelte';
	import { SvelteSet } from 'svelte/reactivity';
	import { resolve } from '$app/paths';
	import UserGearIcon from 'phosphor-svelte/lib/UserGearIcon';
	import ArrowClockwiseIcon from 'phosphor-svelte/lib/ArrowClockwiseIcon';
	import FloppyDiskIcon from 'phosphor-svelte/lib/FloppyDiskIcon';
	import { ApiError } from '$lib/api/client';
	import { roleMatrix, type MatrixResponse } from '$lib/api/admin-security';

	let matrix = $state<MatrixResponse | null>(null);
	let loading = $state(true);
	let error = $state('');
	let toast = $state('');

	// Local mutable copy: a Set of "role|permission" keys for O(1) toggles.
	let pairs: SvelteSet<string> = new SvelteSet();
	let original: SvelteSet<string> = new SvelteSet();
	let saving = $state(false);

	function flash(msg: string) {
		toast = msg;
		setTimeout(() => (toast = ''), 2500);
	}

	async function refresh() {
		loading = true;
		error = '';
		try {
			const res = await roleMatrix.list();
			matrix = res;
			const keys = res.matrix.map((p) => `${p.role}|${p.permission}`);
			pairs = new SvelteSet(keys);
			original = new SvelteSet(keys);
		} catch (e) {
			error = e instanceof ApiError ? `${e.status}: ${e.message}` : 'Failed to load matrix';
		} finally {
			loading = false;
		}
	}

	function isChecked(role: string, perm: string): boolean {
		return pairs.has(`${role}|${perm}`);
	}

	function toggle(role: string, perm: string) {
		const key = `${role}|${perm}`;
		if (pairs.has(key)) pairs.delete(key);
		else pairs.add(key);
	}

	const dirty = $derived.by(() => {
		if (pairs.size !== original.size) return true;
		for (const key of pairs) if (!original.has(key)) return true;
		return false;
	});

	async function save() {
		if (!matrix || !dirty) return;
		saving = true;
		error = '';
		try {
			// Replace per-role to keep the change atomic on the server side.
			for (const role of matrix.roles) {
				const next = matrix.permissions
					.map((p) => p.key)
					.filter((k) => pairs.has(`${role}|${k}`));
				const previous = matrix.permissions
					.map((p) => p.key)
					.filter((k) => original.has(`${role}|${k}`));
				if (next.length === previous.length && next.every((k, i) => k === previous[i]))
					continue;
				await roleMatrix.replace(role, next);
			}
			flash('Matrix saved & policy hot-reloaded');
			await refresh();
		} catch (e) {
			error = e instanceof ApiError ? `${e.status}: ${e.message}` : 'Failed to save';
		} finally {
			saving = false;
		}
	}

	function discard() {
		pairs = new SvelteSet(original);
	}

	async function reload() {
		try {
			await roleMatrix.reload();
			flash('Policy snapshot reloaded');
		} catch (e) {
			error = e instanceof ApiError ? `${e.status}: ${e.message}` : 'Failed to reload';
		}
	}

	onMount(refresh);
</script>

<svelte:head>
	<title>Roles · Security · Admin</title>
</svelte:head>

<div class="page" data-testid="security-roles">
	<header class="page__header">
		<a href={resolve('/admin/security')} class="page__back">← Security</a>
		<div class="page__title-row">
			<UserGearIcon size={26} weight="duotone" />
			<h1 class="page__title">Role / permission matrix</h1>
		</div>
		<p class="page__subtitle">
			Toggle permissions per role, then save to persist and hot-reload the in-memory policy
			cache. Self-lock guards (revoking <code>admin.role.manage</code> or
			<code>admin.dashboard.read</code> from the admin role) are enforced server-side.
		</p>
	</header>

	{#if toast}
		<div class="toast">{toast}</div>
	{/if}
	{#if error}
		<div class="error" role="alert" data-testid="roles-error">{error}</div>
	{/if}

	<div class="actions">
		<button class="btn btn--ghost" onclick={refresh}>
			<ArrowClockwiseIcon size={16} weight="bold" />
			Refresh
		</button>
		<button class="btn btn--ghost" onclick={reload} title="Reload policy snapshot">
			Hot-reload
		</button>
		<div class="actions__spacer"></div>
		<button class="btn btn--ghost" onclick={discard} disabled={!dirty}>Discard</button>
		<button
			class="btn btn--primary"
			onclick={save}
			disabled={!dirty || saving}
			data-testid="roles-save"
		>
			<FloppyDiskIcon size={16} weight="bold" />
			{saving ? 'Saving…' : 'Save changes'}
		</button>
	</div>

	{#if loading}
		<p class="muted">Loading…</p>
	{:else if matrix}
		<div class="card matrix-wrap">
			<table class="matrix" data-testid="roles-matrix">
				<thead>
					<tr>
						<th class="permission-col">Permission</th>
						{#each matrix.roles as role (role)}
							<th class="role-col">{role}</th>
						{/each}
					</tr>
				</thead>
				<tbody>
					{#each matrix.permissions as perm (perm.key)}
						<tr>
							<td class="permission-cell">
								<div class="permission-key">{perm.key}</div>
								{#if perm.description}
									<div class="permission-desc">{perm.description}</div>
								{/if}
							</td>
							{#each matrix.roles as role (role)}
								<td class="cell">
									<input
										type="checkbox"
										aria-label={`${role} - ${perm.key}`}
										checked={isChecked(role, perm.key)}
										onchange={() => toggle(role, perm.key)}
										data-testid={`cell-${role}-${perm.key}`}
									/>
								</td>
							{/each}
						</tr>
					{/each}
				</tbody>
			</table>
		</div>
	{/if}
</div>

<style>
	.page {
		max-width: 1200px;
	}
	.page__header {
		margin-bottom: var(--space-6);
	}
	.page__back {
		display: inline-block;
		margin-bottom: var(--space-2);
		font-size: var(--fs-sm);
		color: var(--color-teal);
		text-decoration: none;
	}
	.page__back:hover {
		text-decoration: underline;
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
		max-width: 70ch;
	}
	.toast {
		padding: var(--space-3) var(--space-4);
		background: rgba(34, 181, 115, 0.12);
		border: 1px solid rgba(34, 181, 115, 0.25);
		color: var(--color-green);
		border-radius: var(--radius-2xl);
		font-size: var(--fs-sm);
		margin-bottom: var(--space-4);
	}
	.error {
		padding: var(--space-3) var(--space-4);
		background: rgba(239, 68, 68, 0.1);
		border: 1px solid rgba(239, 68, 68, 0.3);
		color: #fca5a5;
		border-radius: var(--radius-2xl);
		font-size: var(--fs-sm);
		margin-bottom: var(--space-4);
	}
	.muted {
		color: var(--color-grey-400);
		font-size: var(--fs-sm);
	}
	.actions {
		display: flex;
		gap: var(--space-2);
		margin-bottom: var(--space-4);
		align-items: center;
	}
	.actions__spacer {
		flex: 1;
	}
	.btn {
		display: inline-flex;
		align-items: center;
		gap: var(--space-2);
		padding: var(--space-2) var(--space-3);
		border-radius: var(--radius-2xl);
		font-size: var(--fs-sm);
		font-weight: var(--w-semibold);
		border: 1px solid transparent;
		background: transparent;
		color: var(--color-grey-300);
		cursor: pointer;
		transition: all 200ms var(--ease-out);
	}
	.btn:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}
	.btn--primary {
		background: var(--color-teal);
		color: var(--color-white);
	}
	.btn--primary:hover:not(:disabled) {
		opacity: 0.9;
	}
	.btn--ghost {
		border-color: rgba(255, 255, 255, 0.1);
		background: rgba(255, 255, 255, 0.03);
	}
	.btn--ghost:hover:not(:disabled) {
		background: rgba(255, 255, 255, 0.08);
		color: var(--color-white);
	}
	.card {
		background: rgba(19, 43, 80, 0.35);
		backdrop-filter: blur(24px);
		-webkit-backdrop-filter: blur(24px);
		border: 1px solid rgba(255, 255, 255, 0.06);
		border-radius: var(--radius-2xl);
		padding: var(--space-4);
	}
	.matrix-wrap {
		overflow-x: auto;
	}
	.matrix {
		width: 100%;
		border-collapse: collapse;
		font-size: var(--fs-sm);
	}
	.matrix th,
	.matrix td {
		padding: var(--space-2) var(--space-3);
		text-align: left;
	}
	.matrix th {
		font-weight: var(--w-medium);
		color: var(--color-grey-400);
		border-bottom: 1px solid rgba(255, 255, 255, 0.08);
		position: sticky;
		top: 0;
		background: rgba(19, 43, 80, 0.35);
		backdrop-filter: blur(24px);
		-webkit-backdrop-filter: blur(24px);
	}
	.permission-col {
		min-width: 18rem;
	}
	.role-col {
		text-align: center;
		text-transform: capitalize;
		min-width: 6rem;
	}
	.matrix tbody tr {
		border-bottom: 1px solid rgba(255, 255, 255, 0.04);
	}
	.matrix tbody tr:hover {
		background: rgba(255, 255, 255, 0.025);
	}
	.permission-cell {
		vertical-align: top;
	}
	.permission-key {
		font-family: var(--font-mono, monospace);
		color: var(--color-white);
		font-size: var(--fs-xs);
	}
	.permission-desc {
		color: var(--color-grey-400);
		font-size: var(--fs-xs);
		margin-top: var(--space-1);
		line-height: var(--lh-snug);
	}
	.cell {
		text-align: center;
	}
	.cell input[type='checkbox'] {
		width: 1.05rem;
		height: 1.05rem;
		accent-color: var(--color-teal);
		cursor: pointer;
	}
</style>
